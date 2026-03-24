use std::collections::{HashMap, HashSet};

use crate::error::TaError;
use crate::tmux::TmuxClient;
use crate::tmux::session::list_all_panes;

use super::{PickerItem, run_picker, switch_to};

/// Worktree switcher — replaces wt() from zshrc.
/// Shows all git worktrees discovered from pane working directories,
/// cross-referenced with which panes exist at each path.
pub async fn switch_worktree(client: &TmuxClient) -> Result<(), TaError> {
    let panes = list_all_panes(client).await?;

    // Collect unique paths and find git roots
    let unique_paths: HashSet<&str> = panes.iter().map(|p| p.current_path.as_str()).collect();

    // For each unique path, find the git root and list worktrees
    let mut all_worktrees: Vec<WorktreeInfo> = Vec::new();
    let mut seen_roots: HashSet<String> = HashSet::new();

    for path in &unique_paths {
        if let Some(root) = git_root(path).await {
            if seen_roots.insert(root.clone()) {
                if let Some(wts) = list_worktrees(&root).await {
                    all_worktrees.extend(wts);
                }
            }
        }
    }

    // Build a map: worktree path -> panes at that path
    let mut path_to_panes: HashMap<&str, Vec<&crate::tmux::Pane>> = HashMap::new();
    for pane in &panes {
        path_to_panes
            .entry(pane.current_path.as_str())
            .or_default()
            .push(pane);
    }

    let items: Vec<PickerItem> = all_worktrees
        .iter()
        .map(|wt| {
            let path_display = tilde_path(&wt.path);
            let branch_display = format!("[{}]", wt.branch);

            let pane_info = if let Some(panes) = path_to_panes.get(wt.path.as_str()) {
                // Find the window these panes are in
                if let Some(first) = panes.first() {
                    let count = panes.len();
                    format!(
                        "{}:{} ({} pane{})",
                        first.session_name,
                        first.window_index,
                        count,
                        if count == 1 { "" } else { "s" }
                    )
                } else {
                    "(no window)".to_string()
                }
            } else {
                "(no window)".to_string()
            };

            let display = format!(
                "{:<40} {:<20} {}",
                path_display, branch_display, pane_info,
            );

            // Output encodes the worktree path so we can resolve later
            PickerItem {
                display,
                output: wt.path.clone(),
                preview_target: Some(wt.path.clone()),
            }
        })
        .collect();

    // Preview: if panes exist at the path, capture the first pane; otherwise git log
    let preview_cmd = concat!(
        "path=$(echo {} | awk '{print $1}'); ",
        "target=$(tmux list-panes -a -F '#{pane_current_path} #{session_name}:#{window_index}.#{pane_index}' ",
        "| awk -v p=\"$path\" '$1 == p {print $2; exit}'); ",
        "if [ -n \"$target\" ]; then ",
        "  tmux capture-pane -p -t \"$target\" 2>/dev/null; ",
        "else ",
        "  git -C \"$path\" log --oneline -10 2>/dev/null || echo '(not a git repo)'; ",
        "fi"
    );

    if let Some(selected_path) = run_picker(items, Some(preview_cmd)) {
        let path = selected_path.split_whitespace().next().unwrap_or(&selected_path);
        // Expand tilde back
        let path = expand_tilde(path);

        // Check if a pane already exists at this path
        if let Some(panes) = path_to_panes.get(path.as_str()) {
            if let Some(first) = panes.first() {
                let target = first.target();
                switch_to(client, &target).await?;
                return Ok(());
            }
        }

        // No existing pane — open a new window at this path
        client
            .run_silent(&["new-window", "-c", &path])
            .await?;
    }

    Ok(())
}

struct WorktreeInfo {
    path: String,
    branch: String,
}

async fn git_root(path: &str) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["-C", path, "rev-parse", "--show-toplevel"])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        Some(
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string(),
        )
    } else {
        None
    }
}

async fn list_worktrees(git_root: &str) -> Option<Vec<WorktreeInfo>> {
    let output = tokio::process::Command::new("git")
        .args(["-C", git_root, "worktree", "list", "--porcelain"])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch = String::new();

    for line in text.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(path.to_string());
            current_branch = String::new();
        } else if let Some(branch) = line.strip_prefix("branch ") {
            // Strip refs/heads/ prefix
            current_branch = branch
                .strip_prefix("refs/heads/")
                .unwrap_or(branch)
                .to_string();
        } else if line.is_empty() {
            if let Some(path) = current_path.take() {
                let branch = if current_branch.is_empty() {
                    "(detached)".to_string()
                } else {
                    std::mem::take(&mut current_branch)
                };
                worktrees.push(WorktreeInfo { path, branch });
            }
        }
    }

    // Handle last entry (no trailing blank line)
    if let Some(path) = current_path {
        let branch = if current_branch.is_empty() {
            "(detached)".to_string()
        } else {
            current_branch
        };
        worktrees.push(WorktreeInfo { path, branch });
    }

    Some(worktrees)
}

fn tilde_path(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Some(rest) = path.strip_prefix(&home) {
            return format!("~{rest}");
        }
    }
    path.to_string()
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~') {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}{rest}");
        }
    }
    path.to_string()
}
