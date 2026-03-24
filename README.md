# ta — Tmux Agent

A fast Rust CLI for navigating tmux sessions, windows, panes, worktrees, and AI coding agents.

`ta` embeds a fuzzy picker ([skim](https://github.com/skim-rs/skim)) directly in the binary — no external fzf dependency. All switchers run inside `tmux display-popup` when invoked from tmux, or inline in the terminal otherwise.

## Install

```bash
cargo build --release
cp target/release/ta ~/.local/bin/
```

## Quick start

```bash
# Set up tmux keybindings (prefix-s, prefix-w, prefix-p, prefix-t, prefix-a)
ta bind --persist

# Or add shell aliases (ts, tw, tp, twt)
eval "$(ta shell zsh)"
```

## Switchers

| Command | Key | Description |
|---------|-----|-------------|
| `ta switch` | `prefix-p` | All panes across sessions — directory, branch, agent type |
| `ta switch session` | `prefix-s` | Pick a session |
| `ta switch window` | `prefix-w` | Pick a window across all sessions |
| `ta switch worktree` | `prefix-t` | Git worktrees in the current repo — jumps to existing window or creates one |
| `ta switch agent` | `prefix-a` | Claude Code and Codex agents with live status |

All switchers show a live preview pane. Use **shift-up / shift-down** to scroll the preview.

### Agent detection

`ta switch agent` finds AI agents using multiple methods in priority order:

1. **Process command** — `pane_current_command` contains `codex`, `claude`, etc.
2. **Process tree** — walks child processes of the pane's shell via [sysinfo](https://docs.rs/sysinfo) (catches Claude Code running under `zsh → /bin/sh → claude`)
3. **Pane content** — regex patterns against captured output (`anthropic`, `codex>`, etc.)
4. **Pane title** — spinner characters (braille dots, `✳`) that Claude Code sets when working
5. **Title keywords** — title contains `claude`, `codex`, `gemini`, etc.

Status is detected from pane title spinners and output patterns:

| Status | Icon | Color | Meaning |
|--------|------|-------|---------|
| working | `~` | green | Actively producing output |
| idle | `>` | yellow | Waiting for input |
| rate-limited | `!` | red | Hit API rate limit |
| error | `x` | red bold | Error state |
| unknown | `?` | gray | Can't determine |

## Keybindings

```bash
ta bind              # Bind all defaults
ta bind --persist    # Also add source-file to ~/.tmux.conf
ta bind --show       # Show current bindings
ta bind --unbind     # Remove bindings (restores prior keys)
```

Bindings are persisted to `~/.config/ta/tmux.conf`. Prior keybindings are saved and restored on `--unbind`.

## Structured output

Query commands return JSON envelopes:

```bash
ta session list
ta session show <name>
ta pane list <session>
ta pane capture <session> --pane <n>
```

```json
{
  "success": true,
  "timestamp": "2026-03-24T15:30:45+00:00",
  "version": "1.0.0",
  "data": [...]
}
```

## How it works

`ta` is stateless — every invocation queries live tmux state. There is no daemon, registry, or persistent cache. Pane metadata is parsed from tmux format strings and an optional naming convention (`session__type_index_variant[tags]`). Git worktrees are discovered from pane working directories.

## License

MIT
