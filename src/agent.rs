use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use sysinfo::{Pid, System};

use crate::tmux::pane::AgentType;

/// Detected status of an agent pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Working,
    Idle,
    RateLimited,
    Error,
    Unknown,
}

impl AgentStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Working => "~",
            Self::Idle => ">",
            Self::RateLimited => "!",
            Self::Error => "x",
            Self::Unknown => "?",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Idle => "idle",
            Self::RateLimited => "rate-limited",
            Self::Error => "error",
            Self::Unknown => "unknown",
        }
    }

    /// ANSI color code for this status.
    fn ansi_code(&self) -> &'static str {
        match self {
            Self::Working => "\x1b[32m",     // green
            Self::Idle => "\x1b[33m",        // yellow
            Self::RateLimited => "\x1b[31m", // red
            Self::Error => "\x1b[1;31m",     // bold red
            Self::Unknown => "\x1b[90m",     // dim gray
        }
    }

    /// Icon with ANSI color.
    pub fn colored_icon(&self) -> String {
        format!("{}{}\x1b[0m", self.ansi_code(), self.icon())
    }

    /// Label with ANSI color.
    pub fn colored_label(&self) -> String {
        format!("{}{}\x1b[0m", self.ansi_code(), self.label())
    }
}

/// How the agent type was detected, with confidence.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionMethod {
    Process, // 0.95 — pane_current_command or child process
    Content, // 0.75 — output pattern matching
    Title,   // 0.60 — pane title keywords or spinner chars
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentDetection {
    pub agent_type: AgentType,
    pub method: DetectionMethod,
    pub confidence: f64,
}

// ─── Process-based detection ────────────────────────────────────────────────

/// Process name substrings that map to agent types.
const PROCESS_PATTERNS: &[(&str, AgentType)] = &[
    ("claude", AgentType::Cc),
    ("codex", AgentType::Cod),
    ("gemini", AgentType::Gmi),
    ("cursor", AgentType::Cursor),
    ("windsurf", AgentType::Windsurf),
    ("aider", AgentType::Aider),
    ("ollama", AgentType::Ollama),
];

/// Detect agent type from pane_current_command.
fn detect_from_command(command: &str) -> Option<AgentDetection> {
    let cmd = command.to_lowercase();
    for (pattern, agent_type) in PROCESS_PATTERNS {
        if cmd.contains(pattern) {
            return Some(AgentDetection {
                agent_type: agent_type.clone(),
                method: DetectionMethod::Process,
                confidence: 0.95,
            });
        }
    }
    None
}

/// Walk the process tree from a pane's shell PID to find agent child processes.
/// Uses `sysinfo` for portable process table access (no shelling out to ps/pgrep).
/// Walks up to 3 levels deep.
pub fn detect_from_process_tree(sys: &System, pid: u32) -> Option<AgentDetection> {
    let descendant_cmds = collect_descendants(sys, Pid::from_u32(pid), 3);

    for cmd in &descendant_cmds {
        let cmd_lower = cmd.to_lowercase();
        for (pattern, agent_type) in PROCESS_PATTERNS {
            if cmd_lower.contains(pattern) {
                return Some(AgentDetection {
                    agent_type: agent_type.clone(),
                    method: DetectionMethod::Process,
                    confidence: 0.95,
                });
            }
        }
    }
    None
}

/// Collect command names of all descendants of `root_pid`, up to `depth` levels.
fn collect_descendants(sys: &System, root_pid: Pid, depth: u32) -> Vec<String> {
    if depth == 0 {
        return vec![];
    }

    let mut result = Vec::new();
    for (pid, process) in sys.processes() {
        if process.parent() == Some(root_pid) {
            result.push(process.name().to_string_lossy().to_string());
            result.extend(collect_descendants(sys, *pid, depth - 1));
        }
    }
    result
}

/// Create a sysinfo System snapshot with process info loaded.
pub fn snapshot_processes() -> System {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys
}

// ─── Content-based detection ────────────────────────────────────────────────

static CONTENT_PATTERNS: LazyLock<Vec<(AgentType, Vec<Regex>)>> = LazyLock::new(|| {
    vec![
        (
            AgentType::Cc,
            vec![
                Regex::new(r"(?i)claude\s*(code|>|$)").unwrap(),
                Regex::new(r"(?i)anthropic").unwrap(),
                Regex::new(r"(?i)\[claude\]").unwrap(),
            ],
        ),
        (
            AgentType::Cod,
            vec![
                Regex::new(r"(?i)codex\s*(>|cli|$)").unwrap(),
                Regex::new(r"(?i)openai\s+codex").unwrap(),
            ],
        ),
        (
            AgentType::Gmi,
            vec![Regex::new(r"(?i)gemini\s*(>|cli|$)").unwrap()],
        ),
    ]
});

fn detect_from_content(output: &str) -> Option<AgentDetection> {
    for (agent_type, patterns) in CONTENT_PATTERNS.iter() {
        for re in patterns {
            if re.is_match(output) {
                return Some(AgentDetection {
                    agent_type: agent_type.clone(),
                    method: DetectionMethod::Content,
                    confidence: 0.75,
                });
            }
        }
    }
    None
}

// ─── Title-based detection ──────────────────────────────────────────────────

/// Spinner chars Claude Code uses in pane titles when working.
static TITLE_SPINNER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\u{2800}-\u{28FF}✳◐◑◒◓]").unwrap());

/// Title contains an agent keyword.
fn detect_from_title_keyword(title: &str) -> Option<AgentDetection> {
    let lower = title.to_lowercase();
    let agents: &[(&str, AgentType)] = &[
        ("claude", AgentType::Cc),
        ("codex", AgentType::Cod),
        ("gemini", AgentType::Gmi),
        ("cursor", AgentType::Cursor),
        ("windsurf", AgentType::Windsurf),
        ("aider", AgentType::Aider),
    ];
    for (keyword, agent_type) in agents {
        if lower.contains(keyword) {
            return Some(AgentDetection {
                agent_type: agent_type.clone(),
                method: DetectionMethod::Title,
                confidence: 0.60,
            });
        }
    }
    None
}

/// Claude Code sets pane title to spinner + task when working.
fn detect_from_title_spinner(title: &str) -> Option<AgentDetection> {
    let title = title.trim();
    if TITLE_SPINNER_RE.is_match(title) {
        // Spinner chars are used by Claude Code
        return Some(AgentDetection {
            agent_type: AgentType::Cc,
            method: DetectionMethod::Title,
            confidence: 0.70,
        });
    }
    None
}

// ─── Combined detection ─────────────────────────────────────────────────────

/// Multi-method agent detection. Priority: Process > Content > Title.
/// `command` is pane_current_command, `pid` is the shell PID,
/// `title` is pane_title, `output` is captured pane content.
/// `sys` is a pre-loaded sysinfo snapshot (call `snapshot_processes()` once).
pub fn detect_agent(
    sys: &System,
    command: &str,
    pid: u32,
    title: &str,
    output: &str,
) -> Option<AgentDetection> {
    // 1. Direct command match (codex shows as pane_current_command)
    if let Some(d) = detect_from_command(command) {
        return Some(d);
    }

    // 2. Process tree walk (claude is a child of the shell)
    if let Some(d) = detect_from_process_tree(sys, pid) {
        return Some(d);
    }

    // 3. Content patterns in pane output
    if let Some(d) = detect_from_content(output) {
        return Some(d);
    }

    // 4. Title spinner (Claude Code working indicator)
    if let Some(d) = detect_from_title_spinner(title) {
        return Some(d);
    }

    // 5. Title keyword
    if let Some(d) = detect_from_title_keyword(title) {
        return Some(d);
    }

    None
}

// ─── Status detection ───────────────────────────────────────────────────────

/// Extract the task description from a Claude Code pane title.
pub fn task_from_title(title: &str) -> Option<String> {
    let title = title.trim();
    if title.is_empty() {
        return None;
    }
    let task = TITLE_SPINNER_RE.replace(title, "");
    let task = task.trim();
    if task.is_empty() {
        None
    } else {
        Some(task.to_string())
    }
}

/// Detect whether the pane title indicates working (spinner prefix).
pub fn status_from_title(title: &str) -> Option<AgentStatus> {
    if TITLE_SPINNER_RE.is_match(title.trim()) {
        Some(AgentStatus::Working)
    } else {
        None
    }
}

/// Detect agent status from captured pane output.
///
/// Strategy: check the LAST FEW LINES for idle/prompt patterns first (most
/// reliable — if a prompt is at the bottom, the agent is idle regardless of
/// what appears earlier in the buffer). Then check recent lines for
/// rate-limit/error/working signals.
pub fn status_from_output(agent_type: &AgentType, output: &str) -> AgentStatus {
    let patterns = match agent_type {
        AgentType::Cc => &*CC_STATUS,
        AgentType::Cod => &*COD_STATUS,
        _ => return AgentStatus::Unknown,
    };

    // Get the last non-empty lines for prompt detection
    let lines: Vec<&str> = output.lines().collect();
    let tail: Vec<&str> = lines
        .iter()
        .rev()
        .filter(|l| !l.trim().is_empty())
        .take(5)
        .copied()
        .collect();
    let tail_text = tail.join("\n");

    // 1. Check tail for idle prompt — most authoritative signal.
    //    If the last visible line is a prompt, the agent is idle.
    for re in &patterns.idle {
        if re.is_match(&tail_text) {
            return AgentStatus::Idle;
        }
    }

    // 2. Check tail for rate limit (these appear at the bottom when active)
    for pat in &patterns.rate_limit {
        if tail_text.contains(pat) {
            return AgentStatus::RateLimited;
        }
    }

    // 3. Check tail for errors
    for pat in &patterns.error {
        if tail_text.contains(pat) {
            return AgentStatus::Error;
        }
    }

    // 4. Check tail for active spinner patterns (CC only)
    if *agent_type == AgentType::Cc {
        for re in &*CC_SPINNER_OUTPUT {
            if re.is_match(&tail_text) {
                return AgentStatus::Working;
            }
        }
    }

    // 5. If none of the above matched, the agent is probably working
    //    (output is streaming and hasn't settled to a prompt yet).
    //    But only if there's actual content in the tail.
    if !tail.is_empty() {
        return AgentStatus::Working;
    }

    AgentStatus::Unknown
}

/// Best-effort status from output patterns.
pub fn detect_status(agent_type: &AgentType, _title: &str, output: &str) -> AgentStatus {
    status_from_output(agent_type, output)
}

// ─── Status patterns ────────────────────────────────────────────────────────

struct StatusPatterns {
    working: Vec<&'static str>,
    idle: Vec<Regex>,
    error: Vec<&'static str>,
    rate_limit: Vec<&'static str>,
}

static CC_SPINNER_OUTPUT: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"\S+…\s+\(").unwrap(),
        Regex::new(r"·\s*thinking").unwrap(),
        Regex::new(r"·\s*thought\s+for").unwrap(),
        Regex::new(r"Running…").unwrap(),
    ]
});

static CC_STATUS: LazyLock<StatusPatterns> = LazyLock::new(|| StatusPatterns {
    working: vec![
        "```",
        "writing to ",
        "created ",
        "updated ",
        "deleted ",
        "reading ",
        "searching ",
        "running ",
        "executing ",
        "installing ",
        "building ",
        "compiling ",
    ],
    idle: vec![
        Regex::new(r">\s*$").unwrap(),
        Regex::new(r"(?m)^>\s*").unwrap(),
        Regex::new(r"Human:\s*$").unwrap(),
        Regex::new(r"\?\s*$").unwrap(),
        Regex::new(r"(?i)claude\s+code\s+v[\d.]+").unwrap(),
        Regex::new(r"(?i)welcome\s+back").unwrap(),
        Regex::new(r"╰─>\s*$").unwrap(),
        Regex::new(r"(?m)❯[\s\u{00a0}]*$").unwrap(),
    ],
    error: vec![
        "error:",
        "Error:",
        "ERROR",
        "panic:",
        "fatal:",
        "FATAL",
        "unhandled exception",
    ],
    rate_limit: vec![
        "rate limit",
        "Rate limit",
        "429",
        "too many requests",
        "overloaded",
    ],
});

static COD_STATUS: LazyLock<StatusPatterns> = LazyLock::new(|| StatusPatterns {
    working: vec![
        "```",
        "editing ",
        "creating ",
        "reading ",
        "running ",
        "applying ",
        "searching ",
        "writing ",
        "deleting ",
    ],
    idle: vec![
        Regex::new(r">\s*$").unwrap(),
        Regex::new(r"\?\s*for\s*shortcuts").unwrap(),
        Regex::new(r"codex>\s*$").unwrap(),
        Regex::new(r"(?m)^\s*›\s*.*$").unwrap(),
    ],
    error: vec!["error:", "Error:", "ERROR", "panic:", "fatal:"],
    rate_limit: vec!["rate limit", "Rate limit", "429", "too many requests"],
});
