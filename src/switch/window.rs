use crate::agent::{detect_agent, resolve_display_status, snapshot_processes, task_from_title};
use crate::error::TaError;
use crate::tmux::session::list_all_panes;
use crate::tmux::TmuxClient;

use super::{git_branches, run_picker, switch_to, PickerItem};

pub async fn switch_window(client: &TmuxClient) -> Result<(), TaError> {
    let panes = list_all_panes(client).await?;
    let sys = snapshot_processes();

    // Group panes by session:window
    let mut windows: std::collections::BTreeMap<String, Vec<&crate::tmux::Pane>> =
        std::collections::BTreeMap::new();
    for pane in &panes {
        let key = format!("{}:{}", pane.session_name, pane.window_index);
        windows.entry(key).or_default().push(pane);
    }

    // Batch-resolve git branches
    let paths: Vec<String> = panes.iter().map(|p| p.current_path.clone()).collect();
    let branches = git_branches(&paths).await;

    let mut items: Vec<PickerItem> = Vec::new();

    for (key, win_panes) in &windows {
        let pane_count = win_panes.len();

        // Use the most common path in this window
        let dir = most_common_path(win_panes);
        let path = compress_path(&dir);
        let branch = branches
            .get(&dir)
            .map(|b| format!("[{}]", b))
            .unwrap_or_default();

        // Find all agents in this window
        let agents = find_window_agents(client, &sys, win_panes).await;

        let mut display = format!(
            "{} {} {} pane{}",
            key,
            path,
            pane_count,
            if pane_count == 1 { "" } else { "s" },
        );

        if !branch.is_empty() {
            display.push(' ');
            display.push_str(&branch);
        }

        for (status, agent_type, task) in &agents {
            display.push(' ');
            display.push_str(&status.colored_icon());
            display.push_str(agent_type);
            if !task.is_empty() {
                display.push(' ');
                display.push_str(task);
            }
        }

        items.push(PickerItem {
            display,
            output: key.clone(),
        });
    }

    let preview_cmd = "tmux capture-pane -p -t {1} 2>/dev/null || echo '(no preview)'";

    if let Some(target) = run_picker(items, Some(preview_cmd)) {
        let window_target = target.split_whitespace().next().unwrap_or(&target);
        switch_to(client, window_target).await?;
    }

    Ok(())
}

/// Find all agent panes in a window. Returns (status, type_tag, task) for each.
async fn find_window_agents(
    client: &TmuxClient,
    sys: &sysinfo::System,
    panes: &[&crate::tmux::Pane],
) -> Vec<(crate::agent::AgentStatus, String, String)> {
    let mut agents = Vec::new();

    for pane in panes {
        let Some(detection) = detect_agent(sys, &pane.command, pane.pid, &pane.title, "") else {
            continue;
        };

        let target = pane.target();
        let window_opt = client
            .run(&["show-option", "-wv", "-t", &target, "@workmux_status"])
            .await
            .ok()
            .filter(|s| !s.trim().is_empty());

        let output = client
            .run(&["capture-pane", "-p", "-t", &target, "-S", "-30"])
            .await
            .unwrap_or_default();

        let status = resolve_display_status(
            window_opt.as_deref(),
            &detection.agent_type,
            &pane.title,
            &output,
        );

        let type_tag = detection.agent_type.tag().to_string();
        let task = task_from_title(&pane.title).unwrap_or_default();
        agents.push((status, type_tag, task));
    }

    agents
}

fn most_common_path(panes: &[&crate::tmux::Pane]) -> String {
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for pane in panes {
        *counts.entry(&pane.current_path).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(p, _)| p.to_string())
        .unwrap_or_default()
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
