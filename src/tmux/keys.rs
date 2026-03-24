use super::client::TmuxClient;
use crate::error::TaError;

/// Send keys to a tmux pane.
/// If `enter` is true, appends Enter after the text.
pub async fn send_keys(
    client: &TmuxClient,
    target: &str,
    text: &str,
    enter: bool,
) -> Result<(), TaError> {
    // For long text, chunk to avoid tmux buffer limits.
    // tmux send-keys has a practical limit around 500 bytes per call.
    const CHUNK_SIZE: usize = 400;

    if text.len() <= CHUNK_SIZE {
        let mut args = vec!["send-keys", "-t", target, text];
        if enter {
            args.push("Enter");
        }
        client.run_silent(&args).await
    } else {
        // Send in chunks without Enter, then send Enter at the end
        for chunk in text.as_bytes().chunks(CHUNK_SIZE) {
            let chunk_str = String::from_utf8_lossy(chunk);
            client
                .run_silent(&["send-keys", "-t", target, "-l", &chunk_str])
                .await?;
        }
        if enter {
            client
                .run_silent(&["send-keys", "-t", target, "Enter"])
                .await?;
        }
        Ok(())
    }
}
