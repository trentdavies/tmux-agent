use crate::error::TaError;
use crate::tmux::session::list_all_panes;
use crate::tmux::TmuxClient;

use super::{compress_path, git_branches, path_tail, run_picker, switch_to, PickerItem};

pub async fn switch_window(client: &TmuxClient) -> Result<(), TaError> {
    let panes = list_all_panes(client).await?;

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
        // Use the most common path in this window
        let dir = most_common_path(win_panes);
        let path = compress_path(&dir);
        let branch = branches
            .get(&dir)
            .map(|b| format!("[{}]", b))
            .unwrap_or_default();

        let tail = path_tail(&dir);

        let mut display = key.clone();

        if !branch.is_empty() {
            display.push_str(&format!(" \x1b[38;5;208m{}\x1b[0m", branch));
        }

        display.push_str(&format!(" \x1b[90m{}\x1b[0m", path));

        let mut search = key.clone();
        search.push(' ');
        search.push_str(&tail);
        if let Some(b) = branches.get(&dir) {
            search.push(' ');
            search.push_str(b);
        }

        items.push(PickerItem {
            display,
            output: key.clone(),
            search_text: Some(search),
        });
    }

    let preview_cmd = "tmux capture-pane -p -t {1} 2>/dev/null || echo '(no preview)'";

    if let Some(target) = run_picker(items, Some(preview_cmd)) {
        let window_target = target.split_whitespace().next().unwrap_or(&target);
        switch_to(client, window_target).await?;
    }

    Ok(())
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
