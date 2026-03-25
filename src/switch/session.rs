use std::collections::HashMap;

use crate::error::TaError;
use crate::tmux::session::{list_all_panes, list_sessions};
use crate::tmux::TmuxClient;

use super::{run_picker, switch_to, PickerItem};

pub async fn switch_session(client: &TmuxClient) -> Result<(), TaError> {
    let sessions = list_sessions(client).await?;
    let all_panes = list_all_panes(client).await?;

    // Build a map of session -> most common pane path (mode of current_path)
    let mut session_paths: HashMap<&str, HashMap<&str, usize>> = HashMap::new();
    for pane in &all_panes {
        *session_paths
            .entry(&pane.session_name)
            .or_default()
            .entry(&pane.current_path)
            .or_default() += 1;
    }

    let items: Vec<PickerItem> = sessions
        .iter()
        .map(|s| {
            let attached = if s.attached { " (attached)" } else { "" };

            // Use the most common pane path, not session_path
            let dir = session_paths
                .get(s.name.as_str())
                .and_then(|paths| paths.iter().max_by_key(|(_, count)| *count))
                .map(|(path, _)| tilde_path(path))
                .unwrap_or_else(|| tilde_path(&s.directory));

            let display = format!(
                "{:<20} {} window{}{:<12} {}",
                s.name,
                s.windows,
                if s.windows == 1 { " " } else { "s" },
                attached,
                dir,
            );
            PickerItem {
                display,
                output: s.name.clone(),
            }
        })
        .collect();

    let ta_bin = std::env::current_exe()
        .unwrap_or_else(|_| "ta".into())
        .to_string_lossy()
        .to_string();
    let preview_cmd = format!("{} session show {{1}}", ta_bin);

    if let Some(target) = run_picker(items, Some(&preview_cmd)) {
        let session = target.split_whitespace().next().unwrap_or(&target);
        switch_to(client, session).await?;
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
