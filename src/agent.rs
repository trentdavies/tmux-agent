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
    Waiting,
    Done,
    Idle,
    RateLimited,
    Error,
    Unknown,
}

impl AgentStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Working => "🤖",
            Self::Waiting => "💬",
            Self::Done => "✅",
            Self::Idle => "💤",
            Self::RateLimited => "🚫",
            Self::Error => "❌",
            Self::Unknown => "❓",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Waiting => "waiting",
            Self::Done => "done",
            Self::Idle => "idle",
            Self::RateLimited => "rate-limited",
            Self::Error => "error",
            Self::Unknown => "unknown",
        }
    }

    fn ansi_code(&self) -> &'static str {
        match self {
            Self::Working => "\x1b[32m",     // green
            Self::Waiting => "\x1b[35m",     // magenta
            Self::Done => "\x1b[36m",        // cyan
            Self::Idle => "\x1b[33m",        // yellow
            Self::RateLimited => "\x1b[31m", // red
            Self::Error => "\x1b[1;31m",     // bold red
            Self::Unknown => "\x1b[90m",     // dim gray
        }
    }

    pub fn colored_icon(&self) -> String {
        format!("{}{}\x1b[0m", self.ansi_code(), self.icon())
    }

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

/// Detect agent status from captured pane output.
///
/// Follows ntm's approach:
/// 1. Rate limit: check last 50 lines (stale rate limits fade out)
/// 2. Idle: check last 12 lines for prompt patterns (Claude Code's TUI has
///    a status bar 5-8 lines below the prompt, so we need a wide window)
/// 3. Spinner override: if idle AND an active spinner pattern is present in
///    the same window, the agent is WORKING (spinner beats stale prompt)
/// 4. Working: check last 20 lines for working keywords
/// 5. Conflict: idle beats working (prompt at end overrides stale keywords)
/// 6. Error: check last 10 lines
pub fn status_from_output(agent_type: &AgentType, output: &str) -> AgentStatus {
    let patterns = match agent_type {
        AgentType::Cc => &*CC_STATUS,
        AgentType::Cod => &*COD_STATUS,
        _ => return AgentStatus::Unknown,
    };

    let last_50 = get_last_n_lines(output, 50);
    let last_20 = get_last_n_lines(output, 20);
    let last_10 = get_last_n_lines(output, 10);
    // Both Claude Code and Codex have TUI status bars below the prompt.
    // Use a wide window (12 lines) for all agents to catch the prompt.
    let idle_window = get_last_n_lines(output, 12);

    // 1. Idle detection FIRST — if there's a prompt visible, the agent
    //    is idle regardless of what keywords appear in scrollback.
    let is_idle = patterns.idle.iter().any(|re| re.is_match(&idle_window));

    // 2. Spinner override (CC only): if idle AND an active spinner is
    //    present, the agent is working. The spinner ("Kneading… (5m 50s)")
    //    appears below the ❯ prompt in Claude Code's TUI.
    if is_idle && *agent_type == AgentType::Cc {
        let has_spinner = CC_SPINNER_OUTPUT.iter().any(|re| re.is_match(&idle_window));
        if has_spinner {
            return AgentStatus::Working;
        }
        return AgentStatus::Idle;
    }
    if is_idle {
        return AgentStatus::Idle;
    }

    // 3. Rate limit — only checked if NOT idle. Broad patterns are fine
    //    here because we've already ruled out idle agents (whose scrollback
    //    might contain "rate limit" in conversation prose).
    for pat in &patterns.rate_limit {
        if last_50.contains(pat) {
            return AgentStatus::RateLimited;
        }
    }

    // 4. Working detection (CC spinner patterns in last 20 lines)
    if *agent_type == AgentType::Cc && CC_SPINNER_OUTPUT.iter().any(|re| re.is_match(&last_20)) {
        return AgentStatus::Working;
    }

    // 5. Error detection (last 10 lines) — only if not idle
    for pat in &patterns.error {
        if last_10.contains(pat) {
            return AgentStatus::Error;
        }
    }

    // 6. If there's content but no prompt and no spinner, assume working
    //    (output is streaming, hasn't settled to a prompt yet)
    if !output.trim().is_empty() {
        return AgentStatus::Working;
    }

    AgentStatus::Unknown
}

fn get_last_n_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Best-effort status from output patterns.
pub fn detect_status(agent_type: &AgentType, _title: &str, output: &str) -> AgentStatus {
    status_from_output(agent_type, output)
}

/// Map a window status icon (from @ta_status or @workmux_status) to AgentStatus.
/// These are the most authoritative signals — set by hooks at the exact moment
/// the agent changes state.
pub fn status_from_window_option(icon: &str) -> Option<AgentStatus> {
    let icon = icon.trim();
    if icon.is_empty() {
        return None;
    }
    // Match both ta and workmux default icons
    match icon {
        "🤖" => Some(AgentStatus::Working),
        "💬" => Some(AgentStatus::Waiting),
        "✅" => Some(AgentStatus::Done),
        _ => {
            // Try common text fallbacks
            let lower = icon.to_lowercase();
            if lower.contains("working") || lower.contains("running") {
                Some(AgentStatus::Working)
            } else if lower.contains("waiting") || lower.contains("input") {
                Some(AgentStatus::Waiting)
            } else if lower.contains("done") || lower.contains("finished") {
                Some(AgentStatus::Done)
            } else {
                None
            }
        }
    }
}

/// Combine window option (hook-set) with output-based detection.
/// Window option is highest priority since it's set by real-time hooks.
pub fn resolve_display_status(
    window_option: Option<&str>,
    agent_type: &AgentType,
    title: &str,
    output: &str,
) -> AgentStatus {
    // 1. Window option from hooks (most authoritative)
    if let Some(opt) = window_option {
        if let Some(status) = status_from_window_option(opt) {
            return status;
        }
    }

    // 2. Fall back to output-based detection
    detect_status(agent_type, title, output)
}

// ─── Status patterns ────────────────────────────────────────────────────────

struct StatusPatterns {
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
    idle: vec![
        Regex::new(r"❯\s*$").unwrap(),      // Claude Code prompt
        Regex::new(r">\s*$").unwrap(),      // Generic prompt
        Regex::new(r"(?m)^>\s*$").unwrap(), // Prompt on its own line
        Regex::new(r"Human:\s*$").unwrap(),
        Regex::new(r"\?\s*$").unwrap(),
        Regex::new(r"(?i)claude\s+code\s+v[\d.]+").unwrap(),
        Regex::new(r"(?i)welcome\s+back").unwrap(),
        Regex::new(r"╰─>\s*$").unwrap(),
        Regex::new(r"-- INSERT --").unwrap(), // Claude Code status bar (TUI idle)
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
        "you've hit your limit",
        "you.ve hit your limit",
        "rate limit exceeded",
        "usage limit",
        "request limit",
        "exceeded the limit",
        "too many requests",
        "please wait",
        "try again later",
        "overloaded",
    ],
});

static COD_STATUS: LazyLock<StatusPatterns> = LazyLock::new(|| StatusPatterns {
    idle: vec![
        Regex::new(r">\s*$").unwrap(),
        Regex::new(r"\?\s*for\s*shortcuts").unwrap(),
        Regex::new(r"codex>\s*$").unwrap(),
        Regex::new(r"(?m)^\s*›\s*.*$").unwrap(),
    ],
    error: vec!["error:", "Error:", "ERROR", "panic:", "fatal:"],
    rate_limit: vec![
        "you've reached your usage limit",
        "rate limit exceeded",
        "quota exceeded",
        "too many requests",
        "maximum requests",
    ],
});
