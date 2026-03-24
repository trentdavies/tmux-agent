use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    SessionNotFound,
    PaneNotFound,
    InvalidFlag,
    Timeout,
    TmuxNotInstalled,
    InternalError,
    CursorExpired,
    NotInTmux,
}

#[derive(Debug, thiserror::Error)]
pub enum TaError {
    #[error("tmux command failed: {0}")]
    TmuxCommand(String),

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("pane not found: {0}")]
    PaneNotFound(String),

    #[error("tmux not installed or not in PATH")]
    TmuxNotInstalled,

    #[error("not inside a tmux session")]
    NotInTmux,

    #[error("invalid flag: {0}")]
    InvalidFlag(String),

    #[error("cursor expired: requested {requested}, earliest {earliest}")]
    CursorExpired { requested: i64, earliest: i64 },

    #[error("timeout after {0:?}")]
    Timeout(Duration),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl TaError {
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::SessionNotFound(_) => ErrorCode::SessionNotFound,
            Self::PaneNotFound(_) => ErrorCode::PaneNotFound,
            Self::TmuxNotInstalled => ErrorCode::TmuxNotInstalled,
            Self::NotInTmux => ErrorCode::NotInTmux,
            Self::InvalidFlag(_) => ErrorCode::InvalidFlag,
            Self::CursorExpired { .. } => ErrorCode::CursorExpired,
            Self::Timeout(_) => ErrorCode::Timeout,
            _ => ErrorCode::InternalError,
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            Self::SessionNotFound(_) => {
                Some("Use 'ta session list' to see available sessions".into())
            }
            Self::PaneNotFound(_) => {
                Some("Use 'ta pane list <session>' to see available panes".into())
            }
            Self::TmuxNotInstalled => Some("Install tmux: brew install tmux".into()),
            Self::NotInTmux => Some("Run this command inside a tmux session".into()),
            _ => None,
        }
    }
}
