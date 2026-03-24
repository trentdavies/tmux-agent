use crate::agent::{detect_agent, detect_status, snapshot_processes, task_from_title};
use crate::error::TaError;
use crate::tmux::session::list_all_panes;
use crate::tmux::TmuxClient;

use super::{git_branches, run_picker, switch_to, PickerItem};

/// Agent switcher — lists all detected agent panes with status.
/// Uses multi-method detection: process tree > content > title.
pub async fn switch_agent(client: &TmuxClient) -> Result<(), TaError> {
    let all_panes = list_all_panes(client).await?;
    let sys = snapshot_processes();

    // First pass: detect agents using process tree + command + title (no capture needed)
    let mut agent_panes = Vec::new();
    for pane in &all_panes {
        // Try detection without output first (process + title methods)
        if let Some(detection) =
            detect_agent(&sys, &pane.command, pane.pid, &pane.title, "")
        {
            agent_panes.push((pane, detection));
        }
    }

    if agent_panes.is_empty() {
        println!("No agent panes found.");
        return Ok(());
    }

    // Batch-resolve git branches only for agent pane paths
    let paths: Vec<String> = agent_panes.iter().map(|(p, _)| p.current_path.clone()).collect();
    let branches = git_branches(&paths).await;

    // Second pass: capture output only for detected agents (for status)
    let mut items: Vec<PickerItem> = Vec::new();
    for (pane, detection) in &agent_panes {
        let target = pane.target();

        let output = client
            .run(&["capture-pane", "-p", "-t", &target, "-S", "-30"])
            .await
            .unwrap_or_default();

        let status = detect_status(&detection.agent_type, &pane.title, &output);
        let task = task_from_title(&pane.title).unwrap_or_default();
        let type_tag = detection.agent_type.tag();
        let path = compress_path(&pane.current_path);
        let branch = branches
            .get(&pane.current_path)
            .map(|b| format!("[{}]", b))
            .unwrap_or_default();

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
        });
    }

    // Capture only the visible pane area, strip leading blank lines
    let preview_cmd = "tmux capture-pane -p -t {} | sed '/./,$!d'";

    if let Some(target) = run_picker(items, Some(preview_cmd)) {
        switch_to(client, &target).await?;
    }

    Ok(())
}

fn compress_path(path: &str) -> String {
    let path = tilde_path(path);
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() <= 3 {
        return path;
    }

    let first = parts[0];
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

