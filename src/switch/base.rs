use crate::error::TaError;
use crate::tmux::TmuxClient;

const WINDOW_OPTION: &str = "@ta_base";

/// Jump to a window tagged with @ta_base, preferring the current session.
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

    // List all windows, including the @ta_base option value
    let output = client
        .run(&[
            "list-windows",
            "-a",
            "-F",
            &format!("#{{session_name}}:#{{window_index}} #{{{}}}", WINDOW_OPTION),
        ])
        .await?;

    let mut current_session_match: Option<&str> = None;
    let mut any_match: Option<&str> = None;

    for line in output.lines() {
        let mut parts = line.splitn(2, ' ');
        let target = parts.next().unwrap_or("");
        let option_val = parts.next().unwrap_or("").trim();

        // Empty means the option isn't set on this window
        if option_val.is_empty() {
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

        // If a command was provided and the pane is idle (at a shell prompt),
        // re-send the command.
        if let Some(command) = command {
            let pane_cmd = client
                .run(&["display-message", "-t", target, "-p", "#{pane_current_command}"])
                .await
                .unwrap_or_default()
                .trim()
                .to_string();

            let is_idle = matches!(
                pane_cmd.rsplit('/').next().unwrap_or(""),
                "zsh" | "bash" | "sh" | "fish"
            );

            if is_idle {
                client
                    .run_silent(&["send-keys", "-t", target, command, "Enter"])
                    .await?;
            }
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

        // Create a normal shell window first, then send the command as
        // keystrokes. This way the underlying shell survives if the
        // command exits or crashes.
        client
            .run_silent(&["new-window", "-n", name])
            .await?;
        client
            .run_silent(&["set-option", "-w", WINDOW_OPTION, "1"])
            .await?;
        client
            .run_silent(&["send-keys", command, "Enter"])
            .await?;
    }

    Ok(())
}
