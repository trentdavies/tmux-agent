use crate::error::TaError;
use crate::tmux::session::list_sessions;
use crate::tmux::TmuxClient;

use super::{run_picker, switch_to, PickerItem};

pub async fn switch_session(client: &TmuxClient) -> Result<(), TaError> {
    let sessions = list_sessions(client).await?;

    let items: Vec<PickerItem> = sessions
        .iter()
        .map(|s| {
            let attached = if s.attached { " (attached)" } else { "" };
            let display = format!(
                "{:<20} {} window{}{:<12} {}",
                s.name,
                s.windows,
                if s.windows == 1 { " " } else { "s" },
                attached,
                tilde_path(&s.directory),
            );
            PickerItem {
                display,
                output: s.name.clone(),
                preview_target: Some(s.name.clone()),
            }
        })
        .collect();

    let ta_bin = std::env::current_exe()
        .unwrap_or_else(|_| "ta".into())
        .to_string_lossy()
        .to_string();
    let preview_cmd = format!("{} session show {{1}}", ta_bin);

    if let Some(target) = run_picker(items, Some(&preview_cmd)) {
        // Extract session name (first whitespace-delimited token)
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
