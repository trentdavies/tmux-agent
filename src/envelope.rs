use chrono::Utc;
use serde::Serialize;

use crate::error::{ErrorCode, TaError};

const VERSION: &str = "1.0.0";

#[derive(Debug, Serialize)]
pub struct Envelope<T: Serialize> {
    pub success: bool,
    pub timestamp: String,
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            timestamp: Utc::now().to_rfc3339(),
            version: VERSION,
            error: None,
            error_code: None,
            hint: None,
            data: Some(data),
        }
    }
}

impl Envelope<()> {
    pub fn err(e: &TaError) -> Self {
        Self {
            success: false,
            timestamp: Utc::now().to_rfc3339(),
            version: VERSION,
            error: Some(e.to_string()),
            error_code: Some(e.error_code()),
            hint: e.hint(),
            data: None,
        }
    }
}

/// Print a success envelope to stdout and exit 0.
pub fn print_ok<T: Serialize>(data: T) {
    let envelope = Envelope::ok(data);
    println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
}

/// Print an error envelope to stdout and exit 1.
pub fn print_err(e: &TaError) {
    let envelope = Envelope::<()>::err(e);
    println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
}
