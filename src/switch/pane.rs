use crate::error::TaError;
use crate::tmux::pane::format_tags;
use crate::tmux::session::list_all_panes;
use crate::tmux::TmuxClient;

use super::{display_path, git_branches, path_tail, run_filterable_picker, switch_to, PickerItem};

/// General switcher — replaces tmux-pane-finder.
/// Shows all panes across all sessions with agent metadata, directory, and branch.
pub async fn switch_pane(
    client: &TmuxClient,
    current_session: &str,
    local: bool,
) -> Result<(), TaError> {
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
            let path = display_path(&pane.current_path);
            let tail = path_tail(&pane.current_path);
            let branch = branches
                .get(&pane.current_path)
                .map(|b| format!("[{}]", b))
                .unwrap_or_default();

            let mut display = target.clone();

            if label != "user" {
                display.push(' ');
                display.push_str(&label);
            }
            if !tags.is_empty() {
                display.push(' ');
                display.push_str(&tags);
            }
            if !branch.is_empty() {
                display.push_str(&format!(" \x1b[38;5;208m{}\x1b[0m", branch));
            }
            display.push(' ');
            display.push_str(&path);

            let mut search = format!("{} {}", pane.session_name, label);
            if !tags.is_empty() {
                search.push(' ');
                search.push_str(&tags);
            }
            search.push(' ');
            search.push_str(&tail);
            if let Some(b) = branches.get(&pane.current_path) {
                search.push(' ');
                search.push_str(b);
            }

            PickerItem {
                display,
                output: target.clone(),
                search_text: Some(search),
                session: Some(pane.session_name.clone()),
            }
        })
        .collect();

    let preview_cmd =
        "tmux capture-pane -p -t $(echo {} | awk '{print $1}') 2>/dev/null || echo '(no preview)'";

    if let Some(target) = run_filterable_picker(items, current_session, local, Some(preview_cmd)) {
        let pane_target = target.split_whitespace().next().unwrap_or(&target);
        switch_to(client, pane_target).await?;
    }

    Ok(())
}
