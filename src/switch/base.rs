use crate::error::TaError;
use crate::tmux::TmuxClient;

/// Jump to a window with the given name, preferring the current session.
/// If no matching window exists, creates one running the specified command.
pub async fn jump_to_base(
    client: &TmuxClient,
    name: &str,
    command: Option<&str>,
) -> Result<(), TaError> {
    let current_session = client
        .run(&["display-message", "-p", "#{session_name}"])
        .await?
        .trim()
        .to_string();

    // List all windows across all sessions: "session_name:window_index window_name"
    let output = client
        .run(&[
            "list-windows",
            "-a",
            "-F",
            "#{session_name}:#{window_index} #{window_name}",
        ])
        .await?;

    let mut current_session_match: Option<&str> = None;
    let mut any_match: Option<&str> = None;

    for line in output.lines() {
        let mut parts = line.splitn(2, ' ');
        let target = parts.next().unwrap_or("");
        let window_name = parts.next().unwrap_or("");

        if window_name != name {
            continue;
        }

        if any_match.is_none() {
            any_match = Some(target);
        }
        if target.starts_with(&format!("{}:", current_session)) {
            current_session_match = Some(target);
            break;
        }
    }

    if let Some(target) = current_session_match.or(any_match) {
        client.run_silent(&["select-window", "-t", target]).await?;
        if !target.starts_with(&format!("{}:", current_session)) {
            client.run_silent(&["switch-client", "-t", target]).await?;
        }
    } else {
        let command = command.ok_or_else(|| {
            TaError::Other(
                "No base window found and no --command provided to create one.\n\
                 \n\
                 To configure a base binding, run:\n\
                 \n\
                   ta setup tmux --base-command '<your command>'\n\
                 \n\
                 This binds prefix-b to jump to (or launch) your base window."
                    .to_string(),
            )
        })?;
        client
            .run_silent(&["new-window", "-n", name, command])
            .await?;
    }

    Ok(())
}
