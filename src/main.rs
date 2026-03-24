use clap::Parser;
use std::process::ExitCode;

mod agent;
mod cli;
mod envelope;
mod error;
mod switch;
mod tmux;
mod version;

use cli::{Cli, Command, PaneAction, SessionAction, SwitchTarget};
use error::TaError;
use tmux::TmuxClient;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            envelope::print_err(&e);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), TaError> {
    let client = match cli.remote {
        Some(host) => TmuxClient::remote(host)?,
        None => TmuxClient::local()?,
    };

    match cli.command {
        Command::Session { action } => match action {
            SessionAction::List => {
                let sessions = tmux::session::list_sessions(&client).await?;
                envelope::print_ok(sessions);
            }
            SessionAction::Show { name } => {
                let session = tmux::session::get_session(&client, &name).await?;
                envelope::print_ok(session);
            }
        },

        Command::Pane { action } => match action {
            PaneAction::List { session } => {
                let panes = tmux::session::list_panes(&client, &session).await?;
                envelope::print_ok(panes);
            }
            PaneAction::Capture {
                session,
                pane,
                lines,
            } => {
                let target = format!("{session}:.{pane}");
                let output = tmux::capture::capture_pane(&client, &target, lines).await?;
                envelope::print_ok(output);
            }
        },

        Command::Switch { target } => {
            // If inside tmux but not already in a popup, re-exec inside display-popup
            if should_popup() {
                return exec_in_popup(&target).await;
            }

            match target {
                None => {
                    switch::pane::switch_pane(&client).await?;
                }
                Some(SwitchTarget::Session) => {
                    switch::session::switch_session(&client).await?;
                }
                Some(SwitchTarget::Window) => {
                    switch::window::switch_window(&client).await?;
                }
                Some(SwitchTarget::Pane) => {
                    switch::pane::switch_pane(&client).await?;
                }
                Some(SwitchTarget::Worktree) => {
                    switch::worktree::switch_worktree(&client).await?;
                }
                Some(SwitchTarget::Agent) => {
                    switch::agent::switch_agent(&client).await?;
                }
            }
        }

        Command::Bind(args) => {
            run_bind(&client, args).await?;
        }

        Command::Shell { shell } => {
            run_shell(shell);
        }
    }

    Ok(())
}

/// Returns true if we're inside tmux but NOT already inside a ta popup.
fn should_popup() -> bool {
    std::env::var("TMUX").is_ok() && std::env::var("TA_POPUP").is_err()
}

/// Re-exec `ta switch [target]` inside a tmux display-popup.
async fn exec_in_popup(target: &Option<SwitchTarget>) -> Result<(), TaError> {
    let ta_bin = std::env::current_exe()
        .unwrap_or_else(|_| "ta".into())
        .to_string_lossy()
        .to_string();

    let subcmd = match target {
        None => "switch".to_string(),
        Some(SwitchTarget::Session) => "switch session".to_string(),
        Some(SwitchTarget::Window) => "switch window".to_string(),
        Some(SwitchTarget::Pane) => "switch pane".to_string(),
        Some(SwitchTarget::Worktree) => "switch worktree".to_string(),
        Some(SwitchTarget::Agent) => "switch agent".to_string(),
    };

    let inner_cmd = format!("TA_POPUP=1 {} {}", ta_bin, subcmd);

    // display-popup blocks until the popup is dismissed, so we need to
    // inherit stdio (not capture) so tmux can communicate properly.
    let tmux = which::which("tmux").map_err(|_| TaError::TmuxNotInstalled)?;
    let status = tokio::process::Command::new(tmux)
        .args(["display-popup", "-E", "-w", "80%", "-h", "60%", &inner_cmd])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .map_err(TaError::Io)?;

    if !status.success() {
        // User dismissed the popup — not an error
    }

    Ok(())
}

/// Config directory for ta: ~/.config/ta/
/// Uses ~/.config explicitly (not platform config_dir) to avoid spaces in path
/// which break tmux source-file.
fn config_dir() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~"))
        .join(".config")
        .join("ta")
}

/// Path to the persisted tmux bindings file: ~/.config/ta/tmux.conf
fn bindings_path() -> std::path::PathBuf {
    config_dir().join("tmux.conf")
}

/// Path to saved prior keybindings: ~/.config/ta/prior-keys.json
fn prior_keys_path() -> std::path::PathBuf {
    config_dir().join("prior-keys.json")
}

/// Snapshot the current binding for a key from the live tmux server.
/// Returns the full `bind-key ...` command line, or None if unbound.
async fn get_current_binding(client: &TmuxClient, key: &str) -> Option<String> {
    let output = client.run(&["list-keys"]).await.ok()?;
    // tmux list-keys output looks like:
    //   bind-key    -T prefix       s                    choose-tree -Zs
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Look for: bind-key -T prefix <key> <rest...>
        if parts.len() >= 5
            && parts[0] == "bind-key"
            && parts[1] == "-T"
            && parts[2] == "prefix"
            && parts[3] == key
        {
            // Check it's not already a ta binding
            let rest = parts[4..].join(" ");
            if rest.contains("TA_POPUP") {
                return None;
            }
            return Some(line.trim().to_string());
        }
    }
    None
}

/// Save prior bindings for the given keys to prior-keys.json.
/// Merges with any existing saved keys (doesn't overwrite unrelated entries).
async fn save_prior_bindings(client: &TmuxClient, keys: &[String]) -> Result<(), TaError> {
    let path = prior_keys_path();

    // Load existing saved bindings
    let mut saved: std::collections::HashMap<String, String> = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    // Only save if we haven't already saved for this key (don't overwrite
    // the original binding if user runs `ta bind` twice)
    for key in keys {
        if !saved.contains_key(key) {
            if let Some(binding) = get_current_binding(client, key).await {
                saved.insert(key.clone(), binding);
            }
        }
    }

    std::fs::create_dir_all(config_dir())?;
    std::fs::write(&path, serde_json::to_string_pretty(&saved)?)?;
    Ok(())
}

/// Restore prior bindings for the given keys. Returns the keys that were restored.
async fn restore_prior_bindings(
    client: &TmuxClient,
    keys: &[String],
) -> Result<Vec<String>, TaError> {
    let path = prior_keys_path();
    if !path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&path)?;
    let mut saved: std::collections::HashMap<String, String> =
        serde_json::from_str(&content).unwrap_or_default();

    let mut restored = vec![];

    for key in keys {
        if let Some(binding_line) = saved.remove(key) {
            // The saved line is the full `bind-key -T prefix s ...` output.
            // We need to replay it. Parse out the args after `bind-key`.
            let parts: Vec<&str> = binding_line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == "bind-key" {
                let args: Vec<&str> = parts[1..].to_vec();
                let _ = client
                    .run_silent(&[&["bind-key"], args.as_slice()].concat())
                    .await;
                restored.push(key.clone());
            }
        } else {
            // No prior binding saved — just unbind
            let _ = client.run_silent(&["unbind-key", key]).await;
        }
    }

    // Write back the remaining entries (other keys we haven't unbound)
    if saved.is_empty() {
        let _ = std::fs::remove_file(&path);
    } else {
        std::fs::write(&path, serde_json::to_string_pretty(&saved)?)?;
    }

    Ok(restored)
}

/// Build the list of (key, subcommand) bindings from CLI args.
fn resolve_bindings(args: &cli::BindArgs) -> Vec<(String, &'static str)> {
    if args.session {
        vec![(
            args.key.clone().unwrap_or_else(|| "s".into()),
            "switch session",
        )]
    } else if args.window {
        vec![(
            args.key.clone().unwrap_or_else(|| "w".into()),
            "switch window",
        )]
    } else if args.pane {
        vec![(
            args.key.clone().unwrap_or_else(|| "p".into()),
            "switch pane",
        )]
    } else if args.worktree {
        vec![(
            args.key.clone().unwrap_or_else(|| "t".into()),
            "switch worktree",
        )]
    } else if args.agent {
        vec![(
            args.key.clone().unwrap_or_else(|| "a".into()),
            "switch agent",
        )]
    } else {
        // Default: bind all
        vec![
            ("s".into(), "switch session"),
            ("w".into(), "switch window"),
            ("p".into(), "switch"),
            ("t".into(), "switch worktree"),
            ("a".into(), "switch agent"),
        ]
    }
}

/// Generate the tmux.conf content for current bindings.
fn generate_bindings_conf(bindings: &[(String, &str)], ta_bin: &str) -> String {
    let mut lines = vec![
        "# ta keybindings — managed by `ta bind`".to_string(),
        "# Source this from your tmux.conf:".to_string(),
        format!("#   source-file {}", bindings_path().display()),
        String::new(),
    ];
    for (key, subcmd) in bindings {
        lines.push(format!(
            "bind-key {} display-popup -E -w 80% -h 60% \"TA_POPUP=1 {} {}\"",
            key, ta_bin, subcmd
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}

/// Write bindings to ~/.config/ta/tmux.conf and source them into the live server.
/// Saves any prior bindings for the affected keys first.
async fn persist_and_apply(
    client: &TmuxClient,
    bindings: &[(String, &str)],
    ta_bin: &str,
) -> Result<(), TaError> {
    // Save prior bindings before overwriting
    let keys: Vec<String> = bindings.iter().map(|(k, _)| k.clone()).collect();
    save_prior_bindings(client, &keys).await?;

    let conf = generate_bindings_conf(bindings, ta_bin);
    let path = bindings_path();

    // Ensure config dir exists
    std::fs::create_dir_all(config_dir())?;
    std::fs::write(&path, &conf)?;

    // Source into the running server so they take effect immediately
    client
        .run_silent(&["source-file", &path.to_string_lossy()])
        .await?;

    Ok(())
}

async fn run_bind(client: &TmuxClient, args: cli::BindArgs) -> Result<(), TaError> {
    let ta_bin = std::env::current_exe()
        .unwrap_or_else(|_| "ta".into())
        .to_string_lossy()
        .to_string();
    let path = bindings_path();

    if args.show {
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            print!("{}", content);
        } else {
            println!("No ta keybindings configured.");
            println!("Run `ta bind` to set up default bindings.");
        }
        return Ok(());
    }

    if args.unbind {
        // Collect the keys ta bound
        let mut keys = vec![];
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("bind-key ") {
                    if let Some(key) = rest.split_whitespace().next() {
                        keys.push(key.to_string());
                    }
                }
            }
            std::fs::remove_file(&path)?;
        }

        // Restore prior bindings (or unbind if none were saved)
        let restored = restore_prior_bindings(client, &keys).await?;

        remove_source_from_tmux_conf()?;

        println!("Removed ta keybindings.");
        if !restored.is_empty() {
            println!("Restored prior bindings for: {}", restored.join(", "));
        }
        return Ok(());
    }

    let bindings = resolve_bindings(&args);
    persist_and_apply(client, &bindings, &ta_bin).await?;

    let path_display = path.display();
    for (key, subcmd) in &bindings {
        println!("Bound prefix-{} → ta {}", key, subcmd);
    }
    println!();
    println!("Persisted to: {}", path_display);

    if args.persist {
        add_source_to_tmux_conf(&path)?;
        println!("Added source-file to ~/.tmux.conf");
    } else {
        println!();
        println!("To load on tmux startup, add to your ~/.tmux.conf:");
        println!("  source-file {}", path_display);
        println!();
        println!("Or run: ta bind --persist");
    }

    Ok(())
}

const TMUX_CONF_MARKER: &str = "# ta bindings";

/// Add a `source-file` line to ~/.tmux.conf if not already present.
fn add_source_to_tmux_conf(bindings_path: &std::path::Path) -> Result<(), TaError> {
    let tmux_conf = dirs_next::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~"))
        .join(".tmux.conf");

    let source_line = format!("source-file {}", bindings_path.display());

    let existing = if tmux_conf.exists() {
        std::fs::read_to_string(&tmux_conf)?
    } else {
        String::new()
    };

    // Already present — don't duplicate
    if existing.contains(&source_line) {
        return Ok(());
    }

    // Remove any old ta source-file line (in case path changed)
    let cleaned: Vec<&str> = existing
        .lines()
        .filter(|line| {
            !(line.contains(TMUX_CONF_MARKER)
                || (line.starts_with("source-file") && line.contains("/ta/")))
        })
        .collect();

    let mut content = cleaned.join("\n");
    // Ensure trailing newline before appending
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!("{}\n{}\n", TMUX_CONF_MARKER, source_line));

    std::fs::write(&tmux_conf, content)?;
    Ok(())
}

/// Remove the ta source-file line from ~/.tmux.conf.
fn remove_source_from_tmux_conf() -> Result<(), TaError> {
    let tmux_conf = dirs_next::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~"))
        .join(".tmux.conf");

    if !tmux_conf.exists() {
        return Ok(());
    }

    let existing = std::fs::read_to_string(&tmux_conf)?;
    let cleaned: Vec<&str> = existing
        .lines()
        .filter(|line| {
            !(line.contains(TMUX_CONF_MARKER)
                || (line.starts_with("source-file") && line.contains("/ta/")))
        })
        .collect();

    let content = cleaned.join("\n") + "\n";
    std::fs::write(&tmux_conf, content)?;
    Ok(())
}

fn run_shell(shell: cli::ShellType) {
    let ta_bin = std::env::current_exe()
        .unwrap_or_else(|_| "ta".into())
        .to_string_lossy()
        .to_string();

    match shell {
        cli::ShellType::Zsh => print_zsh_integration(&ta_bin),
        cli::ShellType::Bash => print_bash_integration(&ta_bin),
    }
}

fn print_zsh_integration(bin: &str) {
    println!(
        r#"# ta shell integration (zsh)
# Add to your .zshrc: eval "$({bin} shell zsh)"

# Switcher aliases
if [[ -n "$TMUX" ]]; then
  ts()  {{ tmux display-popup -E -w 80% -h 60% "{bin} switch"; }}
  tss() {{ tmux display-popup -E -w 80% -h 60% "{bin} switch session"; }}
  tw()  {{ tmux display-popup -E -w 80% -h 60% "{bin} switch window"; }}
  tp()  {{ tmux display-popup -E -w 80% -h 60% "{bin} switch pane"; }}
  twt() {{ tmux display-popup -E -w 80% -h 60% "{bin} switch worktree"; }}
else
  ts()  {{ {bin} switch; }}
  tss() {{ {bin} switch session; }}
  tw()  {{ {bin} switch window; }}
  tp()  {{ {bin} switch pane; }}
  twt() {{
    local dir
    dir=$({bin} switch worktree 2>/dev/null) && [[ -n "$dir" ]] && cd "$dir"
  }}
fi
"#,
        bin = bin
    );
}

fn print_bash_integration(bin: &str) {
    println!(
        r#"# ta shell integration (bash)
# Add to your .bashrc: eval "$({bin} shell bash)"

# Switcher aliases
if [[ -n "$TMUX" ]]; then
  ts()  {{ tmux display-popup -E -w 80% -h 60% "{bin} switch"; }}
  tss() {{ tmux display-popup -E -w 80% -h 60% "{bin} switch session"; }}
  tw()  {{ tmux display-popup -E -w 80% -h 60% "{bin} switch window"; }}
  tp()  {{ tmux display-popup -E -w 80% -h 60% "{bin} switch pane"; }}
  twt() {{ tmux display-popup -E -w 80% -h 60% "{bin} switch worktree"; }}
else
  ts()  {{ {bin} switch; }}
  tss() {{ {bin} switch session; }}
  tw()  {{ {bin} switch window; }}
  tp()  {{ {bin} switch pane; }}
  twt() {{
    local dir
    dir=$({bin} switch worktree 2>/dev/null) && [[ -n "$dir" ]] && cd "$dir"
  }}
fi
"#,
        bin = bin
    );
}
