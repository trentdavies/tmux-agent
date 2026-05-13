# ta — Tmux Agent

A fast Rust CLI for navigating tmux sessions, windows, panes, worktrees, and AI coding agents.

`ta` embeds a fuzzy picker ([skim](https://github.com/skim-rs/skim)) directly in the binary — no external fzf dependency. All switchers run inside `tmux display-popup` when invoked from tmux, or inline in the terminal otherwise.

## Install

Popup bindings require tmux 3.2+ for `display-popup`.

### Manual

```bash
cargo build --release
cp target/release/ta ~/.local/bin/
ta setup tmux --persist
tmux source-file ~/.tmux.conf
```

Then use `prefix-a` to open the agent switcher.

### TPM / tmux plugin manager

Add to `~/.tmux.conf`:

```tmux
set -g @plugin 'tmux-plugins/tpm'
set -g @plugin 'trentdavies/tmux-agent'
run '~/.tmux/plugins/tpm/tpm'
```

Reload tmux config, then install plugins:

```bash
tmux source-file ~/.tmux.conf
```

Then press `prefix-I`. After install, use `prefix-a` to open the agent switcher.

The plugin loads the default bindings directly. By default it runs `cargo build --release --locked` from the plugin checkout when the plugin-local `target/release/ta` is missing or older than the checkout. This requires Cargo/Rust 1.85+ unless `@tmux-agent-bin` is set. On a cold Cargo cache, the first build can download crates and run dependency build scripts.

Upgrade through TPM with `prefix-U`, then reload tmux config or restart tmux so the plugin runs against the updated checkout.

To use a specific binary:

```tmux
set -g @tmux-agent-bin "$HOME/.local/bin/ta"
```

When `@tmux-agent-bin` is set, the plugin does not build or upgrade that binary.

Build failures are written to `${XDG_CACHE_HOME:-$HOME/.cache}/tmux-agent/build.log`, with a plugin-local `target/build.log` fallback.

## Quick start

After installing, open the agent switcher with:

```text
prefix-a
```

Optional shell aliases:

```bash
eval "$(ta shell zsh)"
```

## Switchers

| Command | Key | Description |
|---------|-----|-------------|
| `ta switch` | `prefix-f` | All panes across sessions — directory, branch, agent type |
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
ta setup tmux              # Bind all defaults
ta setup tmux --persist    # Also add source-file to ~/.tmux.conf
ta setup tmux --show       # Show current bindings
ta setup tmux --unbind     # Remove bindings (restores prior keys)
```

Bindings are persisted to `~/.config/ta/tmux.conf`. Prior keybindings are saved and restored on `--unbind`.

With TPM, the plugin binds keys on tmux startup and does not edit `.tmux.conf`.

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
