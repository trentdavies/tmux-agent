use std::collections::HashMap;

use crate::error::TaError;
use crate::tmux::session::list_all_panes;
use crate::tmux::TmuxClient;

use super::{run_picker, switch_to, PickerItem};

/// Worktree switcher — replaces wt() from zshrc.
/// Lists worktrees from the current repo, jumps to an existing window
/// at that path or creates a new window in the current session.
pub async fn switch_worktree(client: &TmuxClient) -> Result<(), TaError> {
    // Get the git root of the current pane's working directory
    let current_path = client
        .run(&["display-message", "-p", "#{pane_current_path}"])
        .await?;

    let root = git_root(&current_path)
        .await
        .ok_or_else(|| TaError::Other("Not in a git repository".to_string()))?;

    let worktrees = list_worktrees(&root)
        .await
        .ok_or_else(|| TaError::Other("Failed to list worktrees".to_string()))?;

    // Get all panes so we can match worktree paths to existing windows
    let panes = list_all_panes(client).await?;
    let mut path_to_panes: HashMap<&str, Vec<&crate::tmux::Pane>> = HashMap::new();
    for pane in &panes {
        path_to_panes
            .entry(pane.current_path.as_str())
            .or_default()
            .push(pane);
    }

    let items: Vec<PickerItem> = worktrees
        .iter()
        .map(|wt| {
            let leaf = wt.path.rsplit('/').next().unwrap_or(&wt.path);

            let display = format!(
                "\x1b[38;5;208m[{}]\x1b[0m \x1b[90m../{}\x1b[0m",
                wt.branch, leaf,
            );

            PickerItem {
                display,
                output: wt.path.clone(),
                search_text: Some(format!("{} {}", wt.branch, leaf)),
            }
        })
        .collect();

    // Preview: capture existing pane or show git log
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
        let path = selected_path
            .split_whitespace()
            .next()
            .unwrap_or(&selected_path);
        let path = expand_tilde(path);

        // Jump to existing window/pane if one exists at this path
        if let Some(panes) = path_to_panes.get(path.as_str()) {
            if let Some(first) = panes.first() {
                switch_to(client, &first.target()).await?;
                return Ok(());
            }
        }

        // No existing window — create a new one in the current session
        client.run_silent(&["new-window", "-c", &path]).await?;
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
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~') {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}{rest}");
        }
    }
    path.to_string()
}
