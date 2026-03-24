# Phase 1: Core + Switching

## What
Scaffold the ta Rust project with tmux abstraction and embedded skim-powered fuzzy switchers for sessions, windows, panes, and git worktrees.

## Why
Immediate daily-driver value: fast navigation between tmux sessions, windows, and panes — especially when managing multiple AI agents across worktrees.

## Acceptance Criteria
- `cargo build` + `cargo clippy` clean
- `ta session list` returns JSON envelope
- `ta pane list <session>` shows panes with parsed agent metadata
- `ta switch` opens skim picker with all panes, directories, branches
- `ta switch session/window/pane/worktree` each work
- `ta switch worktree` shows worktrees cross-referenced with existing panes
- `ta bind` sets tmux keybindings for popup switchers
- `ta shell zsh` outputs valid shell integration
