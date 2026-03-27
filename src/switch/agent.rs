use crate::agent::{detect_agent, resolve_display_status, snapshot_processes, task_from_title};
use crate::error::TaError;
use crate::tmux::session::list_all_panes;
use crate::tmux::TmuxClient;

use super::{compress_path, git_branches, run_picker, switch_to, PickerItem};

/// Agent switcher — lists all detected agent panes with status.
/// Uses multi-method detection: process tree > content > title.
/// Status priority: @ta_status/@workmux_status window option > output patterns > title.
pub async fn switch_agent(client: &TmuxClient) -> Result<(), TaError> {
    let all_panes = list_all_panes(client).await?;
    let sys = snapshot_processes();

    // First pass: detect agents using process tree + command + title (no capture needed)
    let mut agent_panes = Vec::new();
    for pane in &all_panes {
        if let Some(detection) = detect_agent(&sys, &pane.command, pane.pid, &pane.title, "") {
            agent_panes.push((pane, detection));
        }
    }

    if agent_panes.is_empty() {
        println!("No agent panes found.");
        return Ok(());
    }

    // Batch-resolve git branches only for agent pane paths
    let paths: Vec<String> = agent_panes
        .iter()
        .map(|(p, _)| p.current_path.clone())
        .collect();
    let branches = git_branches(&paths).await;

    // Second pass: capture output and read window status options
    let mut items: Vec<PickerItem> = Vec::new();
    for (pane, detection) in &agent_panes {
        let target = pane.target();

        // Read hook-set window status (@ta_status or @workmux_status)
        let window_opt = read_window_status(client, &target).await;

        let output = client
            .run(&["capture-pane", "-p", "-t", &target, "-S", "-30"])
            .await
            .unwrap_or_default();

        // Resolve status: window option (hooks) > output patterns > title
        let status = resolve_display_status(
            window_opt.as_deref(),
            &detection.agent_type,
            &pane.title,
            &output,
        );

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
            search_text: None,
        });
    }

    let preview_cmd = "tmux capture-pane -p -t {}";

    if let Some(target) = run_picker(items, Some(preview_cmd)) {
        switch_to(client, &target).await?;
    }

    Ok(())
}

/// Read @workmux_status window option for a pane.
/// Both ta and workmux write to the same option for a single source of truth.
async fn read_window_status(client: &TmuxClient, target: &str) -> Option<String> {
    if let Ok(val) = client
        .run(&["show-option", "-wv", "-t", target, "@workmux_status"])
        .await
    {
        let val = val.trim().to_string();
        if !val.is_empty() {
            return Some(val);
        }
    }
    None
}

