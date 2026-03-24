use std::path::PathBuf;
use std::time::Duration;

use crate::error::TaError;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct TmuxClient {
    binary: PathBuf,
    remote: Option<String>,
    timeout: Duration,
}

impl TmuxClient {
    pub fn local() -> Result<Self, TaError> {
        let binary = Self::resolve_binary()?;
        Ok(Self {
            binary,
            remote: None,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    pub fn remote(host: String) -> Result<Self, TaError> {
        let binary = Self::resolve_binary()?;
        Ok(Self {
            binary,
            remote: Some(host),
            timeout: DEFAULT_TIMEOUT,
        })
    }

    fn resolve_binary() -> Result<PathBuf, TaError> {
        which::which("tmux").map_err(|_| TaError::TmuxNotInstalled)
    }

    pub async fn run(&self, args: &[&str]) -> Result<String, TaError> {
        let output = if let Some(host) = &self.remote {
            let tmux_cmd = std::iter::once(self.binary.to_string_lossy().into_owned())
                .chain(args.iter().map(|a| shell_quote(a)))
                .collect::<Vec<_>>()
                .join(" ");

            tokio::time::timeout(
                self.timeout,
                tokio::process::Command::new("ssh")
                    .args(["--", host, "/bin/sh", "-c", &shell_quote(&tmux_cmd)])
                    .output(),
            )
            .await
        } else {
            tokio::time::timeout(
                self.timeout,
                tokio::process::Command::new(&self.binary)
                    .args(args)
                    .output(),
            )
            .await
        };

        let output = output
            .map_err(|_| TaError::Timeout(self.timeout))?
            .map_err(TaError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr = stderr.trim();
            // Detect common tmux errors
            if stderr.contains("no server running") || stderr.contains("no current client") {
                return Err(TaError::NotInTmux);
            }
            if stderr.contains("can't find session") || stderr.contains("session not found") {
                // Try to extract session name
                return Err(TaError::SessionNotFound(stderr.to_string()));
            }
            return Err(TaError::TmuxCommand(format!(
                "tmux {}: {}",
                args.join(" "),
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub async fn run_silent(&self, args: &[&str]) -> Result<(), TaError> {
        self.run(args).await?;
        Ok(())
    }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
