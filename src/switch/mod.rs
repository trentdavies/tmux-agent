pub mod agent;
pub mod base;
pub mod pane;
pub mod session;
pub mod window;
pub mod worktree;

use std::borrow::Cow;
use std::sync::Arc;

use skim::prelude::*;
use skim::{AnsiString, DisplayContext};
use tuikit::key::Key;

use crate::error::TaError;
use crate::tmux::TmuxClient;

/// A generic item for the skim picker.
/// `display` is what the user sees (may contain ANSI color codes),
/// `output` is the value returned on selection.
/// `search_text` optionally overrides what skim fuzzy-matches against.
#[derive(Clone)]
pub struct PickerItem {
    pub display: String,
    pub output: String,
    pub search_text: Option<String>,
    /// Session name for filtering (None = always visible).
    pub session: Option<String>,
}

/// Strip ANSI escape codes from a string.
fn strip_ansi(s: &str) -> String {
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap());
    RE.replace_all(s, "").to_string()
}

impl SkimItem for PickerItem {
    /// Plain text for fuzzy matching.
    fn text(&self) -> Cow<'_, str> {
        match &self.search_text {
            Some(t) => Cow::Borrowed(t),
            None => Cow::Owned(strip_ansi(&self.display)),
        }
    }

    /// Rendered display with ANSI colors parsed.
    fn display<'a>(&'a self, _context: DisplayContext<'a>) -> AnsiString<'a> {
        AnsiString::parse(&self.display)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.output)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Global
    }
}

// ---------------------------------------------------------------------------
// Shared path utilities
// ---------------------------------------------------------------------------

/// Replace the home directory prefix with `~`.
pub fn tilde_path(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Some(rest) = path.strip_prefix(&home) {
            return format!("~{rest}");
        }
    }
    path.to_string()
}

/// Compress a path by abbreviating middle segments to their first character.
/// Keeps the first segment (e.g. `~`) and last two segments intact.
/// Example: `~/dev/tdavies/tmux-agent/src` → `~/d/t/tmux-agent/src`
pub fn compress_path(path: &str) -> String {
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

/// Render a path for display: leaf/parent in normal text, compressed ancestry dimmed.
/// Example: `~/dev/tdavies/tmux-agent/src` → `tmux-agent/src  ~/d/t`
/// Short paths (≤3 segments) are returned as-is with tilde substitution.
pub fn display_path(path: &str) -> String {
    let path = tilde_path(path);
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() <= 3 {
        return path;
    }

    let last_two = format!("{}/{}", parts[parts.len() - 1], parts[parts.len() - 2]);

    let first = parts[0];
    let middle = &parts[1..parts.len() - 2];
    let compressed: Vec<String> = middle
        .iter()
        .map(|seg| {
            if seg.is_empty() {
                String::new()
            } else {
                seg.chars().next().unwrap().to_string()
            }
        })
        .collect();
    let ancestry = format!("{}/{}", first, compressed.join("/"));

    format!("{}  \x1b[90m{}\x1b[0m", last_two, ancestry)
}

/// Return the last two segments of a path, space-separated for search.
/// Example: `~/dev/tdavies/tmux-agent/src` → `tmux-agent src`
pub fn path_tail(path: &str) -> String {
    let path = tilde_path(path);
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 2 {
        return path;
    }
    parts[parts.len() - 2..].join(" ")
}

// ---------------------------------------------------------------------------
// Number prefixes
// ---------------------------------------------------------------------------

/// Prepend bold number prefixes (0-9) to the first 10 items.
/// Items beyond 10 get blank padding for alignment.
fn add_number_prefixes(items: &mut [PickerItem]) {
    for (i, item) in items.iter_mut().enumerate() {
        let prefix = if i < 10 {
            format!("\x1b[1m{}\x1b[0m  ", i)
        } else {
            "   ".to_string()
        };
        item.display = format!("{}{}", prefix, item.display);
    }
}

/// Run the skim picker with numbered quick-select.
/// Items 0-9 can be selected instantly by pressing the digit key.
/// Any other keypress is handled by skim as normal fuzzy search.
pub fn run_picker(mut items: Vec<PickerItem>, preview_cmd: Option<&str>) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    add_number_prefixes(&mut items);

    let mut options = SkimOptionsBuilder::default();
    options
        .height(Some("100%"))
        .multi(false)
        .reverse(true)
        .expect(Some("0,1,2,3,4,5,6,7,8,9".to_owned()))
        .bind(vec!["shift-up:preview-up", "shift-down:preview-down"]);

    if let Some(cmd) = preview_cmd {
        options.preview(Some(cmd));
        options.preview_window(Some("right:50%"));
    }

    let options = options.build().unwrap();

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in &items {
        let _ = tx.send(Arc::new(item.clone()));
    }
    drop(tx);

    let result = Skim::run_with(&options, Some(rx))?;

    if result.is_abort {
        return None;
    }

    // Check if a digit key was pressed for quick-select
    if let Key::Char(c @ '0'..='9') = result.final_key {
        let idx = (c as u8 - b'0') as usize;
        if idx < items.len() {
            return Some(items[idx].output.clone());
        }
    }

    result
        .selected_items
        .first()
        .map(|item| item.output().to_string())
}

// ---------------------------------------------------------------------------
// Filterable picker (session toggle via ctrl-s)
// ---------------------------------------------------------------------------

enum PickerResult {
    Selected(String),
    Aborted,
    Toggle { query: String },
}

/// Run skim once, returning a structured result that distinguishes
/// selection, abort, and toggle (ctrl-s).
fn run_picker_inner(
    mut items: Vec<PickerItem>,
    preview_cmd: Option<&str>,
    prompt: &str,
    header: &str,
    query: Option<&str>,
) -> PickerResult {
    if items.is_empty() {
        return PickerResult::Aborted;
    }

    add_number_prefixes(&mut items);

    let mut options = SkimOptionsBuilder::default();
    options
        .height(Some("100%"))
        .multi(false)
        .reverse(true)
        .prompt(Some(prompt))
        .header(Some(header))
        .expect(Some("0,1,2,3,4,5,6,7,8,9,ctrl-s".to_owned()))
        .bind(vec!["shift-up:preview-up", "shift-down:preview-down"]);

    if let Some(q) = query {
        options.query(Some(q));
    }

    if let Some(cmd) = preview_cmd {
        options.preview(Some(cmd));
        options.preview_window(Some("right:50%"));
    }

    let options = options.build().unwrap();

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in &items {
        let _ = tx.send(Arc::new(item.clone()));
    }
    drop(tx);

    let result = match Skim::run_with(&options, Some(rx)) {
        Some(r) => r,
        None => return PickerResult::Aborted,
    };

    if result.is_abort {
        return PickerResult::Aborted;
    }

    // Toggle session filter
    if result.final_key == Key::Ctrl('s') {
        return PickerResult::Toggle {
            query: result.query.clone(),
        };
    }

    // Digit quick-select
    if let Key::Char(c @ '0'..='9') = result.final_key {
        let idx = (c as u8 - b'0') as usize;
        if idx < items.len() {
            return PickerResult::Selected(items[idx].output.clone());
        }
    }

    match result.selected_items.first() {
        Some(item) => PickerResult::Selected(item.output().to_string()),
        None => PickerResult::Aborted,
    }
}

/// Run the picker with session-filter toggle support.
/// Pressing ctrl-s toggles between showing all items and only items
/// matching `current_session`. The user's query text is preserved
/// across toggles.
pub fn run_filterable_picker(
    all_items: Vec<PickerItem>,
    current_session: &str,
    start_local: bool,
    preview_cmd: Option<&str>,
) -> Option<String> {
    let mut local_mode = start_local;
    let mut query: Option<String> = None;

    loop {
        let items: Vec<PickerItem> = if local_mode {
            all_items
                .iter()
                .filter(|item| item.session.as_deref() == Some(current_session))
                .cloned()
                .collect()
        } else {
            all_items.clone()
        };

        let prompt = if local_mode {
            format!("[{}] > ", current_session)
        } else {
            "> ".to_string()
        };

        let header = if local_mode {
            "ctrl-s: show all sessions".to_string()
        } else {
            "ctrl-s: filter to current session".to_string()
        };

        match run_picker_inner(
            items,
            preview_cmd,
            &prompt,
            &header,
            query.as_deref(),
        ) {
            PickerResult::Selected(s) => return Some(s),
            PickerResult::Aborted => return None,
            PickerResult::Toggle { query: q } => {
                local_mode = !local_mode;
                query = Some(q);
            }
        }
    }
}

/// Switch tmux client to the given target.
pub async fn switch_to(client: &TmuxClient, target: &str) -> Result<(), TaError> {
    client.run_silent(&["switch-client", "-t", target]).await
}

/// Get the git branch for a directory path. Returns None if not a git repo.
pub async fn git_branch(path: &str) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["-C", path, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Batch-resolve git branches for a set of unique paths.
/// Returns a map of path -> branch name.
pub async fn git_branches(paths: &[String]) -> std::collections::HashMap<String, String> {
    use std::collections::{HashMap, HashSet};

    let unique_paths: HashSet<&str> = paths.iter().map(|s| s.as_str()).collect();
    let mut results = HashMap::new();

    let futures: Vec<_> = unique_paths
        .into_iter()
        .map(|path| {
            let path = path.to_string();
            async move {
                let branch = git_branch(&path).await;
                (path, branch)
            }
        })
        .collect();

    let resolved = futures::future::join_all(futures).await;
    for (path, branch) in resolved {
        if let Some(b) = branch {
            results.insert(path, b);
        }
    }

    results
}
