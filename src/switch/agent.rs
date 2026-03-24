use crate::agent::{detect_agent, detect_status, snapshot_processes, task_from_title};
use crate::error::TaError;
use crate::tmux::TmuxClient;
use crate::tmux::session::list_all_panes;

use super::{PickerItem, run_picker, switch_to};

/// Agent switcher — lists all detected agent panes with status.
/// Uses multi-method detection: process tree > content > title.
pub async fn switch_agent(client: &TmuxClient) -> Result<(), TaError> {
    let all_panes = list_all_panes(client).await?;

    // Snapshot the process table once (no shelling out)
    let sys = snapshot_processes();

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

        let type_label = detection.agent_type.display_name();

        let display = format!(
            "{} {:<18} {:<12} {:<12} {}",
            status.icon(),
            target,
            type_label,
            status.label(),
            truncate(&task, 45),
        );

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

    let preview_cmd =
        "tmux capture-pane -p -t $(echo {} | awk '{print $2}') 2>/dev/null || echo '(no preview)'";

    if let Some(target) = run_picker(items, Some(preview_cmd)) {
        let pane_target = target
            .split_whitespace()
            .nth(1)
            .unwrap_or(&target);
        switch_to(client, pane_target).await?;
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
