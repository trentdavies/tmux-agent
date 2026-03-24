# Tmux Abstraction Design

## TmuxClient
- Wraps all tmux interaction via `tokio::process::Command`
- Resolves binary via `which` crate at construction
- Supports remote execution via SSH (`--remote user@host`)
- 30-second timeout on all commands
- Detects common tmux errors (no server, session not found) and maps to typed errors

## Pane Naming Convention
Regex: `^.+__([\w-]+)_(\d+)(?:_([A-Za-z0-9._/@:+-]+))?(?:\[([^\]]*)\])?$`

Parsing: `parse_pane_title()` extracts agent type, index, variant, tags.
Fallback: `detect_agent_from_command()` checks `pane_current_command` for known agent names.

## Session/Pane Queries
Uses `tmux list-panes -a -F <format>` with a custom field separator (`_TA_SEP_`).
Includes `pane_current_path` for worktree cross-referencing.
All queries are live — no caching or state persistence.
