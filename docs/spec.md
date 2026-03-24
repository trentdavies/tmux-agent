# CLI Spec

## Commands

### `ta session list`
List all tmux sessions. Output: `Envelope<Vec<Session>>`.

### `ta session show <name>`
Show session details with panes. Output: `Envelope<Session>`.

### `ta pane list <session>`
List panes in a session with parsed agent metadata. Output: `Envelope<Vec<Pane>>`.

### `ta pane capture <session> --pane <n> --lines <n>`
Capture pane output. Output: `Envelope<String>`.

### `ta switch [target]`
Open fuzzy picker. Targets: `session`, `window`, `pane`, `worktree`.
Default (no target): general pane switcher.

### `ta bind [flags]`
Set up tmux keybindings for popup switchers.
- `--session --key s` — bind prefix-s to session switcher
- `--window --key w` — bind prefix-w to window switcher
- `--pane --key p` — bind prefix-p to pane switcher
- `--worktree --key t` — bind prefix-t to worktree switcher
- No flags: bind all defaults
- `--show` — list current bindings
- `--unbind` — remove all ta bindings

### `ta shell <zsh|bash>`
Generate shell integration script with aliases.

## Response Envelope

All structured output uses this format:

```json
{
  "success": true,
  "timestamp": "2026-03-24T15:30:45+00:00",
  "version": "1.0.0",
  "data": { ... }
}
```

Error:
```json
{
  "success": false,
  "timestamp": "2026-03-24T15:30:45+00:00",
  "version": "1.0.0",
  "error": "session not found: foo",
  "error_code": "SESSION_NOT_FOUND",
  "hint": "Use 'ta session list' to see available sessions"
}
```

## Error Codes

| Code | Meaning |
|------|---------|
| `SESSION_NOT_FOUND` | Named session doesn't exist |
| `PANE_NOT_FOUND` | Pane index out of range |
| `INVALID_FLAG` | Bad CLI argument |
| `TIMEOUT` | Tmux command timed out |
| `TMUX_NOT_INSTALLED` | Can't find tmux binary |
| `NOT_IN_TMUX` | Command requires tmux session |
| `INTERNAL_ERROR` | Unexpected failure |
| `CURSOR_EXPIRED` | Attention cursor outside retention window |

## Global Flags

| Flag | Description |
|------|-------------|
| `--remote <user@host>` | Execute against remote tmux via SSH |
