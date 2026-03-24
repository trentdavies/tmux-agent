use super::client::TmuxClient;
use crate::error::TaError;

/// Capture the visible content of a pane.
pub async fn capture_pane(
    client: &TmuxClient,
    target: &str,
    lines: u32,
) -> Result<String, TaError> {
    client
        .run(&[
            "capture-pane",
            "-p",
            "-t",
            target,
            "-S",
            &format!("-{}", lines),
        ])
        .await
}
