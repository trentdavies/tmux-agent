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

        /// Filter to current session only
        #[arg(long, short = 'l')]
        local: bool,
    },
    /// Generate shell integration (aliases, completions)
    Shell {
        /// Shell type
        shell: ShellType,
    },
    /// Set the window status icon for the current pane
    SetWindowStatus(SetWindowStatusArgs),
    /// Set up tmux keybindings, agent hooks, and integrations
    Setup {
        #[command(subcommand)]
        action: SetupAction,
    },
}

#[derive(Args)]
pub struct SetWindowStatusArgs {
    /// Status to set
    pub status: WindowStatus,

    /// Override command to run instead of the built-in tmux logic.
    /// Useful for delegating to another tool (e.g. "workmux set-window-status").
    /// The status name (working/waiting/done/clear) is appended as the last argument.
    #[arg(long)]
    pub command: Option<String>,
}

#[derive(Clone, clap::ValueEnum)]
pub enum WindowStatus {
    /// Agent is actively working
    Working,
    /// Agent needs user input (auto-clears on window focus)
    Waiting,
    /// Agent has finished (auto-clears on window focus)
    Done,
    /// Clear the status
    Clear,
}

#[derive(Subcommand)]
pub enum SetupAction {
    /// Set up tmux keybindings for popup switchers
    Tmux(BindArgs),
    /// Install agent hooks for automatic status tracking
    Hooks,
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
    /// Jump to a named window (e.g. a dashboard), launching it if absent
    Base {
        /// Window name to find or create (default: "base")
        #[arg(long, default_value = "base")]
        name: String,

        /// Command to run when creating the window
        #[arg(long)]
        command: Option<String>,
    },
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

    /// Command to run in the base window (enables the base binding)
    #[arg(long)]
    pub base_command: Option<String>,

    /// Window name for the base binding (default: "base")
    #[arg(long, default_value = "base")]
    pub base_name: String,

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
