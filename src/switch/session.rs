use std::collections::HashMap;

use crate::error::TaError;
use crate::tmux::session::{list_all_panes, list_sessions};
use crate::tmux::TmuxClient;

use super::{compress_path, path_tail, run_picker, switch_to, PickerItem};

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
            let raw_dir = session_paths
                .get(s.name.as_str())
                .and_then(|paths| paths.iter().max_by_key(|(_, count)| *count))
                .map(|(path, _)| path.to_string())
                .unwrap_or_else(|| s.directory.clone());

            let path = compress_path(&raw_dir);
            let tail = path_tail(&raw_dir);

            let display = format!("{}  \x1b[90m{}\x1b[0m", s.name, path,);

            PickerItem {
                display,
                output: s.name.clone(),
                search_text: Some(format!("{} {}", s.name, tail)),
                session: None,
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
