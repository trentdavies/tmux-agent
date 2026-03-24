use ta::agent::*;
use ta::tmux::pane::{self, AgentType};

// ═══════════════════════════════════════════════════════════════════════════
// Command-based detection
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn detect_codex_from_command() {
    let sys = snapshot_processes();
    let d = detect_agent(&sys, "codex-aarch64-a", 0, "", "").unwrap();
    assert_eq!(d.agent_type, AgentType::Cod);
    assert!(d.confidence >= 0.9);
}

#[test]
fn detect_codex_from_command_plain() {
    let sys = snapshot_processes();
    let d = detect_agent(&sys, "codex", 0, "", "").unwrap();
    assert_eq!(d.agent_type, AgentType::Cod);
}

#[test]
fn detect_claude_from_command() {
    let sys = snapshot_processes();
    let d = detect_agent(&sys, "claude", 0, "", "").unwrap();
    assert_eq!(d.agent_type, AgentType::Cc);
}

#[test]
fn detect_gemini_from_command() {
    let sys = snapshot_processes();
    let d = detect_agent(&sys, "gemini-cli", 0, "", "").unwrap();
    assert_eq!(d.agent_type, AgentType::Gmi);
}

#[test]
fn detect_aider_from_command() {
    let sys = snapshot_processes();
    let d = detect_agent(&sys, "aider", 0, "", "").unwrap();
    assert_eq!(d.agent_type, AgentType::Aider);
}

#[test]
fn no_detection_for_shell() {
    let sys = snapshot_processes();
    assert!(detect_agent(&sys, "zsh", 0, "", "").is_none());
    assert!(detect_agent(&sys, "bash", 0, "", "").is_none());
    assert!(detect_agent(&sys, "nvim", 0, "", "").is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// Content-based detection
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn detect_claude_from_content() {
    let sys = snapshot_processes();
    let output = "Claude Code is ready\nAnthropic\n";
    let d = detect_agent(&sys, "zsh", 0, "", output).unwrap();
    assert_eq!(d.agent_type, AgentType::Cc);
}

#[test]
fn detect_codex_from_content() {
    let sys = snapshot_processes();
    let output = "OpenAI Codex CLI v1.2.3\ncodex> ";
    let d = detect_agent(&sys, "zsh", 0, "", output).unwrap();
    assert_eq!(d.agent_type, AgentType::Cod);
}

#[test]
fn detect_gemini_from_content() {
    let sys = snapshot_processes();
    let output = "gemini-2.0-flash-preview /model\ngemini> ";
    let d = detect_agent(&sys, "zsh", 0, "", output).unwrap();
    assert_eq!(d.agent_type, AgentType::Gmi);
}

// ═══════════════════════════════════════════════════════════════════════════
// Title-based detection
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn detect_claude_from_spinner_title() {
    let sys = snapshot_processes();
    let d = detect_agent(&sys, "zsh", 0, "✳ Review PR design flaws", "").unwrap();
    assert_eq!(d.agent_type, AgentType::Cc);
}

#[test]
fn detect_claude_from_braille_spinner() {
    let sys = snapshot_processes();
    let d = detect_agent(&sys, "zsh", 0, "⠐ tmux-agent-task-planner", "").unwrap();
    assert_eq!(d.agent_type, AgentType::Cc);
}

#[test]
fn detect_claude_from_title_keyword() {
    let sys = snapshot_processes();
    let d = detect_agent(&sys, "zsh", 0, "Claude Code", "").unwrap();
    assert_eq!(d.agent_type, AgentType::Cc);
}

#[test]
fn no_detection_from_hostname_title() {
    let sys = snapshot_processes();
    assert!(detect_agent(&sys, "zsh", 0, "TrentAdobeMac.local", "").is_none());
}

#[test]
fn no_detection_from_empty_title() {
    let sys = snapshot_processes();
    assert!(detect_agent(&sys, "zsh", 0, "", "").is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// Detection priority: process > content > title
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn process_beats_content() {
    let sys = snapshot_processes();
    // Command says codex, content says claude
    let d = detect_agent(&sys, "codex", 0, "", "Claude Code is ready").unwrap();
    assert_eq!(d.agent_type, AgentType::Cod);
    assert!(matches!(d.method, DetectionMethod::Process));
}

#[test]
fn content_beats_title() {
    let sys = snapshot_processes();
    // Content says codex, title says claude
    let d = detect_agent(&sys, "zsh", 0, "Claude Code", "OpenAI Codex CLI\ncodex> ").unwrap();
    assert_eq!(d.agent_type, AgentType::Cod);
    assert!(matches!(d.method, DetectionMethod::Content));
}

// ═══════════════════════════════════════════════════════════════════════════
// Task extraction from title
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn task_from_spinner_title() {
    assert_eq!(
        task_from_title("✳ Review PR design flaws"),
        Some("Review PR design flaws".to_string())
    );
}

#[test]
fn task_from_braille_title() {
    assert_eq!(
        task_from_title("⠐ tmux-agent-task-planner"),
        Some("tmux-agent-task-planner".to_string())
    );
}

#[test]
fn task_from_plain_title() {
    // No spinner prefix — return the whole thing
    assert_eq!(
        task_from_title("Claude Code"),
        Some("Claude Code".to_string())
    );
}

#[test]
fn task_from_empty_title() {
    assert_eq!(task_from_title(""), None);
    assert_eq!(task_from_title("   "), None);
}

// ═══════════════════════════════════════════════════════════════════════════
// Pane naming convention
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_pane_title_full() {
    let (t, i, v, tags) = pane::parse_pane_title("myproject__cc_1_opus[frontend,api]");
    assert_eq!(t, AgentType::Cc);
    assert_eq!(i, 1);
    assert_eq!(v.as_deref(), Some("opus"));
    assert_eq!(tags, vec!["frontend", "api"]);
}

#[test]
fn parse_pane_title_no_variant() {
    let (t, i, v, tags) = pane::parse_pane_title("proj__cod_2");
    assert_eq!(t, AgentType::Cod);
    assert_eq!(i, 2);
    assert!(v.is_none());
    assert!(tags.is_empty());
}

#[test]
fn parse_pane_title_non_matching() {
    let (t, i, _, _) = pane::parse_pane_title("just a regular title");
    assert_eq!(t, AgentType::User);
    assert_eq!(i, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Status detection — Claude Code
// ═══════════════════════════════════════════════════════════════════════════

/// Claude Code idle: prompt visible, no spinner, status bar below.
/// Captured from a real idle Claude Code pane (ap:2.0).
#[test]
fn cc_idle_with_status_bar() {
    let output = r#"
   design gets implemented. The single-client-for-everything issue (#3/#4) will be painful to fix later.
  The rest are worth discussing but less likely to bite you at launch.

✻ Sautéed for 1m 22s

──────────────────────────────────────────────────────────────────────────────────────────────────────────
❯
──────────────────────────────────────────────────────────────────────────────────────────────────────────
  ~/dev/active/agent-platform-repos/sessions/agents/prs  ctx:3%  opus4
  -- INSERT -- ⏸ plan mode on (shift+tab to cycle)
"#;
    // "Sautéed for 1m 22s" is a PAST tense completion marker, not an active spinner.
    // The ❯ prompt and -- INSERT -- status bar indicate idle.
    assert_eq!(detect_status(&AgentType::Cc, "", output), AgentStatus::Idle);
}

/// Claude Code working: active spinner with timing, prompt also visible above.
/// Captured from a real working Claude Code pane (ap:3.0).
#[test]
fn cc_working_with_active_spinner() {
    let output = r#"
     (ctrl+b ctrl+b (twice) to run in background)

✽ Twisting… (32s · ↓ 494 tokens)
  ⎿  Tip: Use /btw to ask a quick side question without interrupting Claude's current work

──────────────────────────────────────────────────────────────────────────────────────────────────────────
❯ next up - I want to
──────────────────────────────────────────────────────────────────────────────────────────────────────────
  ~/dev/active/agent-platform-repos/sessions/agents/e2b  ctx:14%  opus4
  -- INSERT -- ⏵⏵ bypass permissions on (shift+tab to cycle)
"#;
    // "Twisting… (32s)" is an ACTIVE spinner — agent is working.
    // Even though ❯ prompt is visible, spinner override kicks in.
    assert_eq!(
        detect_status(&AgentType::Cc, "", output),
        AgentStatus::Working
    );
}

/// Claude Code working: Running… spinner, tool execution in progress.
/// Captured from work:2.0 during active work.
#[test]
fn cc_working_running_spinner() {
    let output = r#"
⏺ Bash(mkdir -p /Users/tdavies/dev/tdavies/agent-tmux-tools/tests/fixtures…)
  ⎿  Running…

✽ Moseying… (1m 12s · ↓ 3.4k tokens)

──────────────────────────────────────────────────────────────────────────── tmux-agent-task-planner ──
❯
───────────────────────────────────────────────────────────────────────────────────────────────────────────
  /Users/tdavies/dev/active/agent-flywheel/ntm  ctx:32%  opus4
  -- INSERT -- ⏵⏵ bypass permissions on (shift+tab to cycle)
"#;
    // Both "Running…" and "Moseying… (1m 12s)" are active spinners.
    assert_eq!(
        detect_status(&AgentType::Cc, "", output),
        AgentStatus::Working
    );
}

/// Claude Code idle: completed work, shell prompt at bottom.
/// Captured from work:2.1 after finishing a git push.
#[test]
fn cc_idle_shell_prompt() {
    let output = r#"
 * [new branch]      HEAD -> main
branch 'main' set up to track 'origin/main'.
✓ Pushed commits to https://github.com/trentdavies/tmux-agent.git
❯ git st
## main...origin/main
 M src/agent.rs
❯ git diff
❯ git ci -am 'adjust agent status detection'
[main 65fcb15] adjust agent status detection
 1 file changed, 56 insertions(+), 38 deletions(-)
~/d/t/agent-tmux-tools main ⇡1
❯
"#;
    assert_eq!(detect_status(&AgentType::Cc, "", output), AgentStatus::Idle);
}

/// Claude Code idle: should not false-positive on "rate limit" in conversation prose.
#[test]
fn cc_idle_not_rate_limited_by_prose() {
    let output = r#"
The agent detection logic now checks rate limit only after confirming the agent
isn't idle. This prevents false positives when conversation contains "rate limit"
or "429" in prose. The rate limit scoped to last 50 lines avoids stale errors.

──────────────────────────────────────────────────────────────────────────────────────────────────────────
❯
──────────────────────────────────────────────────────────────────────────────────────────────────────────
  ~/dev/active/agent-flywheel/ntm  ctx:29%  opus4
  -- INSERT -- ⏵⏵ bypass permissions on (shift+tab to cycle)
"#;
    // Even though "rate limit" and "429" appear in the text,
    // the agent is idle (prompt visible, no spinner).
    assert_eq!(detect_status(&AgentType::Cc, "", output), AgentStatus::Idle);
}

/// Claude Code idle: should not false-positive on "Error:" in tool output.
#[test]
fn cc_idle_not_error_from_tool_output() {
    let output = r#"
⏺ Bash(git rebase dev)
  ⎿  Error: Exit code 1

  The rebase failed due to conflicts. Let me resolve them.

✻ Done (45s)

──────────────────────────────────────────────────────────────────────────────────────────────────────────
❯
──────────────────────────────────────────────────────────────────────────────────────────────────────────
  ~/dev/active/agent-platform-repos/sessions/agents/e2b  ctx:11%  opus4
  -- INSERT -- ⏵⏵ bypass permissions on (shift+tab to cycle)
"#;
    // "Error: Exit code 1" is a tool error the agent reported, not the agent's own state.
    // Agent is idle at the prompt.
    assert_eq!(detect_status(&AgentType::Cc, "", output), AgentStatus::Idle);
}

/// Past-tense completion marker is NOT an active spinner.
#[test]
fn cc_idle_past_tense_spinner() {
    let output = r#"
Summary of changes:
- Fixed the bug in the parser
- Added tests

✻ Sautéed for 2m 15s

❯
"#;
    // "Sautéed for 2m 15s" has the "… " pattern but it's NOT followed by "("
    // and uses past tense. The ❯ prompt indicates idle.
    assert_eq!(detect_status(&AgentType::Cc, "", output), AgentStatus::Idle);
}

/// Active spinner with parenthesized timing IS working.
#[test]
fn cc_working_spinner_with_timing() {
    let output = "✶ Kneading… (5m 50s · ↑ 11.8k tokens)\n  ⎿  Working on task\n";
    assert_eq!(
        detect_status(&AgentType::Cc, "", output),
        AgentStatus::Working
    );
}

/// "· thinking" spinner is working.
#[test]
fn cc_working_thinking() {
    let output = "Let me analyze this.\n\n· thinking\n";
    assert_eq!(
        detect_status(&AgentType::Cc, "", output),
        AgentStatus::Working
    );
}

/// Empty output should be unknown.
#[test]
fn cc_unknown_empty() {
    assert_eq!(
        detect_status(&AgentType::Cc, "", ""),
        AgentStatus::Unknown
    );
    assert_eq!(
        detect_status(&AgentType::Cc, "", "   \n\n  "),
        AgentStatus::Unknown
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Status detection — Codex
// ═══════════════════════════════════════════════════════════════════════════

/// Codex idle at the › prompt with status bar below.
/// Captured from work:2.2.
#[test]
fn codex_idle_with_prompt() {
    let output = r#"
  Verified with cargo fmt --all, cargo test, and cargo clippy --all-targets --all-features. Clippy is now
  clean.


› Use /skills to list available skills

  gpt-5.4 high · 72% left · ~/dev/tdavies/agent-tmux-tools
"#;
    assert_eq!(
        detect_status(&AgentType::Cod, "", output),
        AgentStatus::Idle
    );
}

/// Codex idle at codex> prompt.
#[test]
fn codex_idle_classic_prompt() {
    let output = "OpenAI Codex CLI v1.2.3\n47% context left · ? for shortcuts\ncodex> ";
    assert_eq!(
        detect_status(&AgentType::Cod, "", output),
        AgentStatus::Idle
    );
}

/// Codex working: no prompt visible, output streaming.
#[test]
fn codex_working_no_prompt() {
    let output = "Reading src/main.rs\nAnalyzing dependencies...\nPlanning changes to 3 files\n";
    assert_eq!(
        detect_status(&AgentType::Cod, "", output),
        AgentStatus::Working
    );
}

/// Codex empty output.
#[test]
fn codex_unknown_empty() {
    assert_eq!(
        detect_status(&AgentType::Cod, "", ""),
        AgentStatus::Unknown
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Status detection — edge cases
// ═══════════════════════════════════════════════════════════════════════════

/// Unknown agent type returns Unknown status.
#[test]
fn unknown_agent_returns_unknown() {
    assert_eq!(
        detect_status(&AgentType::User, "", "some output"),
        AgentStatus::Unknown
    );
    assert_eq!(
        detect_status(&AgentType::Gmi, "", "some output"),
        AgentStatus::Unknown
    );
}

/// Rate limit only triggers when agent is NOT idle.
#[test]
fn rate_limit_only_when_not_idle() {
    let output = "you've hit your limit\nPlease wait and try again.\n";
    assert_eq!(
        detect_status(&AgentType::Cc, "", output),
        AgentStatus::RateLimited
    );
}

/// Rate limit does NOT trigger when agent is idle with rate-limit text in scrollback.
#[test]
fn rate_limit_suppressed_when_idle() {
    let output = r#"
Previously you hit a rate limit, but it's resolved now.

❯
"#;
    assert_eq!(detect_status(&AgentType::Cc, "", output), AgentStatus::Idle);
}

// ═══════════════════════════════════════════════════════════════════════════
// Status detection — fixture-based tests
// ═══════════════════════════════════════════════════════════════════════════

fn load_fixture(name: &str) -> (String, String, String) {
    let path = format!(
        "{}/tests/fixtures/{}.txt",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    let content = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture: {}", path));
    let mut title = String::new();
    let mut command = String::new();
    let mut output_lines = Vec::new();
    let mut past_header = false;

    for line in content.lines() {
        if !past_header {
            if line == "---" {
                past_header = true;
                continue;
            }
            if let Some(t) = line.strip_prefix("# title: ") {
                title = t.to_string();
            }
            if let Some(c) = line.strip_prefix("# command: ") {
                command = c.to_string();
            }
            continue;
        }
        output_lines.push(line);
    }

    (title, command, output_lines.join("\n"))
}

#[test]
fn fixture_ap_2_0_idle_with_stale_spinner_title() {
    let (title, command, output) = load_fixture("ap_2_0");
    assert_eq!(command, "zsh");
    assert!(title.starts_with('✳')); // Has spinner in title
    // But the output shows idle (prompt visible, past-tense "Sautéed")
    assert_eq!(detect_status(&AgentType::Cc, &title, &output), AgentStatus::Idle);
}

#[test]
fn fixture_ap_3_0_working_active_spinner() {
    let (title, _command, output) = load_fixture("ap_3_0");
    assert!(title.contains("Rebase"));
    // Active spinner "Twisting… (32s)" should override the ❯ prompt
    assert_eq!(
        detect_status(&AgentType::Cc, &title, &output),
        AgentStatus::Working
    );
}

#[test]
fn fixture_work_2_0_working_running() {
    let (_title, _command, output) = load_fixture("work_2_0");
    // "Moseying… (1m 12s)" and "Running…" are active spinners
    assert_eq!(
        detect_status(&AgentType::Cc, "", &output),
        AgentStatus::Working
    );
}

#[test]
fn fixture_work_2_1_idle_shell() {
    let (_title, _command, output) = load_fixture("work_2_1");
    // Simple shell prompt ❯ at the end, no spinner
    assert_eq!(
        detect_status(&AgentType::Cc, "", &output),
        AgentStatus::Idle
    );
}

#[test]
fn fixture_work_2_2_codex_idle() {
    let (_title, command, output) = load_fixture("work_2_2");
    assert!(command.contains("codex"));
    // Codex at › prompt
    assert_eq!(
        detect_status(&AgentType::Cod, "", &output),
        AgentStatus::Idle
    );
}
