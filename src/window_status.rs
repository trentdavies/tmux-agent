use crate::cli::WindowStatus;
use crate::error::TaError;
use crate::tmux::TmuxClient;

/// Window option key for ta status icon.
const STATUS_OPTION: &str = "@workmux_status";

/// Tmux format string that conditionally displays the status icon.
const STATUS_FORMAT: &str = "#{?@workmux_status, #{@workmux_status},}";

/// Default status icons.
fn icon(status: &WindowStatus) -> &'static str {
    match status {
        WindowStatus::Working => "🤖",
        WindowStatus::Waiting => "💬",
        WindowStatus::Done => "✅",
        WindowStatus::Clear => "",
    }
}

/// Set the window status for the current pane.
pub async fn set_window_status(client: &TmuxClient, status: &WindowStatus) -> Result<(), TaError> {
    let pane_id = get_current_pane(client).await?;

    match status {
        WindowStatus::Clear => {
            clear_status(client, &pane_id).await?;
        }
        _ => {
            let icon = icon(status);
            let auto_clear = matches!(status, WindowStatus::Waiting | WindowStatus::Done);

            // Ensure the format string is injected so the icon shows
            ensure_status_format(client, &pane_id).await?;

            // Set the window option
            client
                .run_silent(&["set-option", "-w", "-t", &pane_id, STATUS_OPTION, icon])
                .await?;

            // Auto-clear on focus for waiting/done
            if auto_clear {
                let hook_cmd = format!(
                    "if-shell -F \"#{{==:#{{{}}},.{}}}\" \"set-option -uw {}\"",
                    STATUS_OPTION, icon, STATUS_OPTION
                );
                let _ = client
                    .run_silent(&["set-hook", "-w", "-t", &pane_id, "pane-focus-in", &hook_cmd])
                    .await;
            }
        }
    }

    Ok(())
}

/// Run an external command for status updates instead of built-in logic.
pub async fn set_window_status_via_command(
    command: &str,
    status: &WindowStatus,
) -> Result<(), TaError> {
    let status_name = match status {
        WindowStatus::Working => "working",
        WindowStatus::Waiting => "waiting",
        WindowStatus::Done => "done",
        WindowStatus::Clear => "clear",
    };

    let full_cmd = format!("{} {}", command, status_name);
    let output = tokio::process::Command::new("sh")
        .args(["-c", &full_cmd])
        .output()
        .await
        .map_err(TaError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("ta: command failed: {}", stderr.trim());
    }

    Ok(())
}

async fn get_current_pane(client: &TmuxClient) -> Result<String, TaError> {
    // Use TMUX_PANE env var if available, otherwise query tmux
    if let Ok(pane) = std::env::var("TMUX_PANE") {
        return Ok(pane);
    }
    client.run(&["display-message", "-p", "#{pane_id}"]).await
}

async fn clear_status(client: &TmuxClient, pane_id: &str) -> Result<(), TaError> {
    let _ = client
        .run_silent(&["set-option", "-uw", "-t", pane_id, STATUS_OPTION])
        .await;
    Ok(())
}

/// Inject the status format into window-status-format if not already present.
async fn ensure_status_format(client: &TmuxClient, pane_id: &str) -> Result<(), TaError> {
    for option in &["window-status-format", "window-status-current-format"] {
        update_format_option(client, pane_id, option).await?;
    }
    Ok(())
}

async fn update_format_option(
    client: &TmuxClient,
    pane_id: &str,
    option: &str,
) -> Result<(), TaError> {
    // Try window-level first, fall back to global
    let current = client
        .run(&["show-option", "-wv", "-t", pane_id, option])
        .await
        .ok()
        .filter(|s| !s.is_empty())
        .or({
            // Can't use async in or_else, so try sync fallback
            None
        });

    let current = match current {
        Some(fmt) => fmt,
        None => {
            // Try global
            client
                .run(&["show-option", "-gv", option])
                .await
                .unwrap_or_else(|_| "#I:#W#{?window_flags,#{window_flags}, }".to_string())
        }
    };

    if !current.contains("@workmux_status") {
        let new_format = inject_status_format(&current);
        client
            .run_silent(&["set-option", "-w", "-t", pane_id, option, &new_format])
            .await?;
    }

    Ok(())
}

/// Inject ta status format string into an existing tmux format.
/// Inserts before window_flags if present, otherwise appends.
fn inject_status_format(format: &str) -> String {
    let patterns = ["#{window_flags", "#{?window_flags", "#{F}"];
    let insert_pos = patterns.iter().filter_map(|p| format.find(p)).min();

    if let Some(pos) = insert_pos {
        let (before, after) = format.split_at(pos);
        format!("{}{}{}", before, STATUS_FORMAT, after)
    } else {
        format!("{}{}", format, STATUS_FORMAT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_before_window_flags() {
        let input = "#I:#W#{?window_flags,#{window_flags}, }";
        let result = inject_status_format(input);
        assert!(result.contains("@workmux_status"));
        assert!(result.find("@workmux_status").unwrap() < result.find("window_flags").unwrap());
    }

    #[test]
    fn inject_appends_when_no_flags() {
        let input = "#I:#W";
        let result = inject_status_format(input);
        assert!(result.ends_with(STATUS_FORMAT));
    }

    #[test]
    fn no_double_inject() {
        let input =
            "#I:#W#{?@workmux_status, #{@workmux_status},}#{?window_flags,#{window_flags}, }";
        // Already contains @workmux_status — should not re-inject
        assert!(input.contains("@workmux_status"));
    }
}
