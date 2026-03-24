use crate::agent::{detect_agent, detect_status, snapshot_processes, task_from_title};
use crate::error::TaError;
use crate::tmux::TmuxClient;
use crate::tmux::session::list_all_panes;

use super::{PickerItem, git_branches, run_picker, switch_to};

/// Agent switcher — lists all detected agent panes with status.
/// Uses multi-method detection: process tree > content > title.
pub async fn switch_agent(client: &TmuxClient) -> Result<(), TaError> {
    let all_panes = list_all_panes(client).await?;

    // Snapshot the process table once (no shelling out)
    let sys = snapshot_processes();

    // Batch-resolve git branches for all pane paths
    let paths: Vec<String> = all_panes.iter().map(|p| p.current_path.clone()).collect();
    let branches = git_branches(&paths).await;

    let mut items: Vec<PickerItem> = Vec::new();

    for pane in &all_panes {
        let target = pane.target();

        // Capture output for detection and status
        let output = client
            .run(&["capture-pane", "-p", "-t", &target, "-S", "-30"])
            .await
            .unwrap_or_default();

        // Multi-method agent detection
        let Some(detection) = detect_agent(
            &sys,
            &pane.command,
            pane.pid,
            &pane.title,
            &output,
        ) else {
            continue; // Not an agent
        };

        let status = detect_status(&detection.agent_type, &pane.title, &output);
        let task = task_from_title(&pane.title).unwrap_or_default();
        let type_tag = detection.agent_type.tag();
        let path = compress_path(&pane.current_path);
        let branch = branches
            .get(&pane.current_path)
            .map(|b| format!("[{}]", b))
            .unwrap_or_default();

        // Option D: dense, no padding, everything searchable, colored status
        let mut display = format!(
            "{} {} {} {}",
            status.colored_icon(),
            target,
            status.colored_label(),
            type_tag,
        );
        if !task.is_empty() {
            display.push(' ');
            display.push_str(&task);
        }
        display.push(' ');
        display.push_str(&path);
        if !branch.is_empty() {
            display.push(' ');
            display.push_str(&branch);
        }

        items.push(PickerItem {
            display,
            output: target.clone(),
            preview_target: Some(target),
        });
    }

    if items.is_empty() {
        println!("No agent panes found.");
        return Ok(());
    }

    // Preview uses the second token (target) since first is the colored icon
    let preview_cmd =
        "tmux capture-pane -p -t $(echo {} | sed 's/\\x1b\\[[0-9;]*m//g' | awk '{print $2}') 2>/dev/null || echo '(no preview)'";

    if let Some(target) = run_picker(items, Some(preview_cmd)) {
        // Strip ANSI codes to extract the target (second whitespace token)
        let stripped = strip_ansi(&target);
        let pane_target = stripped
            .split_whitespace()
            .nth(1)
            .unwrap_or(&stripped);
        switch_to(client, pane_target).await?;
    }

    Ok(())
}

/// Compress a path for display. Keeps the last 2 segments full,
/// truncates intermediate segments to first char.
/// `/Users/tdavies/dev/active/agent-flywheel/ntm` → `~/d/a/agent-flywheel/ntm`
fn compress_path(path: &str) -> String {
    let path = tilde_path(path);
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() <= 3 {
        return path;
    }

    // Keep first segment (~ or empty for /) and last 2 segments full.
    // Compress everything in between to first char.
    let first = parts[0]; // "~" or ""
    let middle = &parts[1..parts.len() - 2];
    let last_two = &parts[parts.len() - 2..];

    let compressed_middle: Vec<String> = middle
        .iter()
        .map(|seg| {
            if seg.is_empty() {
                String::new()
            } else {
                seg.chars().next().unwrap().to_string()
            }
        })
        .collect();

    format!(
        "{}/{}/{}",
        first,
        compressed_middle.join("/"),
        last_two.join("/"),
    )
}

fn tilde_path(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Some(rest) = path.strip_prefix(&home) {
            return format!("~{rest}");
        }
    }
    path.to_string()
}

fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}
