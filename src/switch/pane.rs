use crate::error::TaError;
use crate::tmux::TmuxClient;
use crate::tmux::session::list_all_panes;
use crate::tmux::pane::format_tags;

use super::{PickerItem, git_branches, run_picker, switch_to};

/// General switcher — replaces tmux-pane-finder.
/// Shows all panes across all sessions with agent metadata, directory, and branch.
pub async fn switch_pane(client: &TmuxClient) -> Result<(), TaError> {
    let panes = list_all_panes(client).await?;

    // Batch-resolve git branches
    let paths: Vec<String> = panes.iter().map(|p| p.current_path.clone()).collect();
    let branches = git_branches(&paths).await;

    let items: Vec<PickerItem> = panes
        .iter()
        .map(|pane| {
            let target = pane.target();
            let label = pane.label();
            let tags = format_tags(&pane.tags);
            let path = tilde_path(&pane.current_path);
            let branch = branches
                .get(&pane.current_path)
                .map(|b| format!("[{}]", b))
                .unwrap_or_default();

            let display = format!(
                "{:<18} {:<16} {:<12} {:<30} {}",
                target, label, tags, path, branch,
            );

            PickerItem {
                display,
                output: target.clone(),
                preview_target: Some(target),
            }
        })
        .collect();

    let preview_cmd =
        "tmux capture-pane -p -t $(echo {} | awk '{print $1}') 2>/dev/null || echo '(no preview)'";

    if let Some(target) = run_picker(items, Some(preview_cmd)) {
        let pane_target = target.split_whitespace().next().unwrap_or(&target);
        switch_to(client, pane_target).await?;
    }

    Ok(())
}

fn tilde_path(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Some(rest) = path.strip_prefix(&home) {
            return format!("~{rest}");
        }
    }
    path.to_string()
}
