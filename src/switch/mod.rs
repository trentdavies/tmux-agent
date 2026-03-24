pub mod agent;
pub mod pane;
pub mod session;
pub mod window;
pub mod worktree;

use std::borrow::Cow;
use std::sync::Arc;

use skim::prelude::*;
use skim::{AnsiString, DisplayContext};

use crate::error::TaError;
use crate::tmux::TmuxClient;

/// A generic item for the skim picker.
/// `display` is what the user sees (may contain ANSI color codes),
/// `output` is the value returned on selection.
#[derive(Clone)]
pub struct PickerItem {
    pub display: String,
    pub output: String,
    pub preview_target: Option<String>,
}

/// Strip ANSI escape codes from a string.
fn strip_ansi(s: &str) -> String {
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap());
    RE.replace_all(s, "").to_string()
}

impl SkimItem for PickerItem {
    /// Plain text for fuzzy matching (ANSI stripped).
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(strip_ansi(&self.display))
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

/// Run the skim picker with the given items and optional preview command.
/// Returns the selected item's output string, or None if the user cancelled.
pub fn run_picker(
    items: Vec<PickerItem>,
    preview_cmd: Option<&str>,
) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    let mut options = SkimOptionsBuilder::default();
    options.height(Some("100%")).multi(false).reverse(true);

    if let Some(cmd) = preview_cmd {
        options.preview(Some(cmd));
        options.preview_window(Some("right:50%"));
    }

    let options = options.build().unwrap();

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in items {
        let _ = tx.send(Arc::new(item));
    }
    drop(tx);

    let result = Skim::run_with(&options, Some(rx))?;

    if result.is_abort {
        return None;
    }

    result
        .selected_items
        .first()
        .map(|item| item.output().to_string())
}

/// Switch tmux client to the given target.
pub async fn switch_to(client: &TmuxClient, target: &str) -> Result<(), TaError> {
    client
        .run_silent(&["switch-client", "-t", target])
        .await
}

/// Get the git branch for a directory path. Returns None if not a git repo.
pub async fn git_branch(path: &str) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["-C", path, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        Some(
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string(),
        )
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
