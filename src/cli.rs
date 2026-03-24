use clap::{Args, Parser, Subcommand};

use crate::version;

#[derive(Parser)]
#[command(
    name = "ta",
    version = version::VERSION,
    long_version = version::LONG_VERSION,
    about = "Tmux Agent TAsks — manage and monitor AI agents in tmux"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Remote host (user@host) for remote tmux
    #[arg(long, global = true)]
    pub remote: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Session operations
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Pane operations
    Pane {
        #[command(subcommand)]
        action: PaneAction,
    },
    /// Fuzzy switch between sessions, windows, panes, or worktrees
    Switch {
        #[command(subcommand)]
        target: Option<SwitchTarget>,
    },
    /// Set up tmux keybindings for popup switchers
    Bind(BindArgs),
    /// Generate shell integration (aliases, completions)
    Shell {
        /// Shell type
        shell: ShellType,
    },
}

#[derive(Subcommand)]
pub enum SessionAction {
    /// List all tmux sessions
    List,
    /// Show session details
    Show {
        /// Session name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum PaneAction {
    /// List panes in a session
    List {
        /// Session name
        session: String,
    },
    /// Capture pane output
    Capture {
        /// Session name
        session: String,
        /// Pane index
        #[arg(long)]
        pane: u32,
        /// Number of lines to capture
        #[arg(long, default_value_t = 50)]
        lines: u32,
    },
}

#[derive(Subcommand)]
pub enum SwitchTarget {
    /// Switch session
    Session,
    /// Switch window
    Window,
    /// Switch pane
    Pane,
    /// Switch to a git worktree
    Worktree,
    /// Switch to an agent pane (Claude Code, Codex)
    Agent,
}

#[derive(Args)]
pub struct BindArgs {
    /// Key to bind (default: varies by target)
    #[arg(long, short)]
    pub key: Option<String>,

    /// Bind session switcher
    #[arg(long)]
    pub session: bool,

    /// Bind window switcher
    #[arg(long)]
    pub window: bool,

    /// Bind pane switcher
    #[arg(long)]
    pub pane: bool,

    /// Bind worktree switcher
    #[arg(long)]
    pub worktree: bool,

    /// Bind agent switcher
    #[arg(long)]
    pub agent: bool,

    /// Remove all ta bindings
    #[arg(long)]
    pub unbind: bool,

    /// Show current bindings
    #[arg(long)]
    pub show: bool,

    /// Also add source-file line to ~/.tmux.conf
    #[arg(long)]
    pub persist: bool,
}

#[derive(Clone, clap::ValueEnum)]
pub enum ShellType {
    Zsh,
    Bash,
}
