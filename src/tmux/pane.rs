use std::fmt;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

/// Regex matching the ta pane naming convention.
/// Format: {session}__{type}_{index}[_{variant}][tags]
/// Examples: myproject__cc_1, myproject__cc_1_opus, myproject__cc_1_opus[frontend,api]
static PANE_NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^.+__([\w-]+)_(\d+)(?:_([A-Za-z0-9._/@:+-]+))?(?:\[([^\]]*)\])?$").unwrap()
});

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Cc,
    Cod,
    Gmi,
    Cursor,
    Windsurf,
    Aider,
    Ollama,
    User,
    Other(String),
}

impl AgentType {
    pub fn from_str_tag(s: &str) -> Self {
        match s {
            "cc" => Self::Cc,
            "cod" => Self::Cod,
            "gmi" => Self::Gmi,
            "cursor" => Self::Cursor,
            "windsurf" => Self::Windsurf,
            "aider" => Self::Aider,
            "ollama" => Self::Ollama,
            "user" => Self::User,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn tag(&self) -> &str {
        match self {
            Self::Cc => "cc",
            Self::Cod => "cod",
            Self::Gmi => "gmi",
            Self::Cursor => "cursor",
            Self::Windsurf => "windsurf",
            Self::Aider => "aider",
            Self::Ollama => "ollama",
            Self::User => "user",
            Self::Other(s) => s,
        }
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.tag())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Pane {
    pub id: String,
    pub index: u32,
    pub window_index: u32,
    pub session_name: String,
    pub ta_index: u32,
    pub title: String,
    pub agent_type: AgentType,
    pub variant: Option<String>,
    pub tags: Vec<String>,
    pub command: String,
    pub width: u32,
    pub height: u32,
    pub active: bool,
    pub pid: u32,
    pub current_path: String,
}

impl Pane {
    /// Tmux target string for this pane: "session:window.pane"
    pub fn target(&self) -> String {
        format!("{}:{}.{}", self.session_name, self.window_index, self.index)
    }

    /// Short label like "cc_1_opus" or "user"
    pub fn label(&self) -> String {
        if self.ta_index == 0 && self.agent_type == AgentType::User {
            return "user".to_string();
        }
        let mut s = format!("{}_{}", self.agent_type.tag(), self.ta_index);
        if let Some(v) = &self.variant {
            s.push('_');
            s.push_str(v);
        }
        s
    }
}

/// Parse agent type, index, variant, and tags from a pane title.
/// Returns (User, 0, None, vec![]) if title doesn't match the naming convention.
pub fn parse_pane_title(title: &str) -> (AgentType, u32, Option<String>, Vec<String>) {
    let Some(caps) = PANE_NAME_RE.captures(title) else {
        return (AgentType::User, 0, None, vec![]);
    };

    let agent_type = AgentType::from_str_tag(&caps[1]);
    let index: u32 = caps[2].parse().unwrap_or(0);
    let variant = caps.get(3).map(|m| m.as_str().to_string());
    let tags = caps
        .get(4)
        .map(|m| parse_tags(m.as_str()))
        .unwrap_or_default();

    (agent_type, index, variant, tags)
}

/// Detect agent type from the running command name (fallback for untagged panes).
pub fn detect_agent_from_command(cmd: &str) -> AgentType {
    let cmd_lower = cmd.to_lowercase();
    if cmd_lower.contains("claude") {
        AgentType::Cc
    } else if cmd_lower.contains("codex") {
        AgentType::Cod
    } else if cmd_lower.contains("gemini") {
        AgentType::Gmi
    } else if cmd_lower.contains("cursor") {
        AgentType::Cursor
    } else if cmd_lower.contains("windsurf") {
        AgentType::Windsurf
    } else if cmd_lower.contains("aider") {
        AgentType::Aider
    } else if cmd_lower.contains("ollama") {
        AgentType::Ollama
    } else {
        AgentType::User
    }
}

/// Format tags as `[tag1,tag2]`.
pub fn format_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        String::new()
    } else {
        format!("[{}]", tags.join(","))
    }
}

fn parse_tags(tag_str: &str) -> Vec<String> {
    if tag_str.is_empty() {
        return vec![];
    }
    tag_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let (t, i, v, tags) = parse_pane_title("myproject__cc_1");
        assert_eq!(t, AgentType::Cc);
        assert_eq!(i, 1);
        assert!(v.is_none());
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_with_variant() {
        let (t, i, v, tags) = parse_pane_title("myproject__cc_1_opus");
        assert_eq!(t, AgentType::Cc);
        assert_eq!(i, 1);
        assert_eq!(v.as_deref(), Some("opus"));
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_with_tags() {
        let (t, i, v, tags) = parse_pane_title("myproject__cc_1_opus[frontend,api]");
        assert_eq!(t, AgentType::Cc);
        assert_eq!(i, 1);
        assert_eq!(v.as_deref(), Some("opus"));
        assert_eq!(tags, vec!["frontend", "api"]);
    }

    #[test]
    fn parse_unknown_type() {
        let (t, i, _, _) = parse_pane_title("proj__custom-agent_3");
        assert_eq!(t, AgentType::Other("custom-agent".to_string()));
        assert_eq!(i, 3);
    }

    #[test]
    fn parse_non_matching() {
        let (t, i, v, tags) = parse_pane_title("just a regular title");
        assert_eq!(t, AgentType::User);
        assert_eq!(i, 0);
        assert!(v.is_none());
        assert!(tags.is_empty());
    }
}
