use crate::error::TaError;
use crate::tmux::TmuxClient;
use crate::tmux::session::list_all_panes;

use super::{PickerItem, run_picker, switch_to};

pub async fn switch_window(client: &TmuxClient) -> Result<(), TaError> {
    let panes = list_all_panes(client).await?;

    // Group panes by session:window
    let mut windows: std::collections::BTreeMap<String, Vec<&crate::tmux::Pane>> =
        std::collections::BTreeMap::new();
    for pane in &panes {
        let key = format!("{}:{}", pane.session_name, pane.window_index);
        windows.entry(key).or_default().push(pane);
    }

    let items: Vec<PickerItem> = windows
        .iter()
        .map(|(key, panes)| {
            let pane_labels: Vec<String> = panes.iter().map(|p| p.label()).collect();
            let pane_count = panes.len();
            let window_title = panes
                .first()
                .map(|p| p.title.clone())
                .unwrap_or_default();

            let display = format!(
                "{:<16} {:<14} {} pane{}  [{}]",
                key,
                truncate(&window_title, 14),
                pane_count,
                if pane_count == 1 { " " } else { "s" },
                pane_labels.join(", "),
            );

            PickerItem {
                display,
                output: key.clone(),
                preview_target: Some(key.clone()),
            }
        })
        .collect();

    let preview_cmd = "tmux capture-pane -p -t {1} 2>/dev/null || echo '(no preview)'";

    if let Some(target) = run_picker(items, Some(preview_cmd)) {
        let window_target = target.split_whitespace().next().unwrap_or(&target);
        switch_to(client, window_target).await?;
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
