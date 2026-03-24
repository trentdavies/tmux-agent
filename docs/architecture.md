# Architecture

## Overview

ta is a stateless CLI tool that queries live tmux state on every invocation. There is no daemon, no registry, and no persistent cache. All pane/session metadata is derived from tmux format strings and the pane naming convention.

## Module Map

```
┌─────────────────────────────────────────────┐
│ main.rs / cli.rs                            │
│   CLI parsing (clap) + command dispatch     │
├─────────────────────────────────────────────┤
│ envelope.rs / error.rs                      │
│   JSON response envelope, typed errors      │
├──────────────┬──────────────────────────────┤
│ switch/      │ tmux/                        │
│  session     │  client    (cmd execution)   │
│  window      │  session   (queries)         │
│  pane        │  pane      (types + naming)  │
│  worktree    │  capture   (output capture)  │
│  (skim TUI)  │                              │
└──────────────┴──────────────────────────────┘
```

## Data Flow

### Query commands (`session list`, `pane list`)
```
CLI → tmux client → tmux server → parse format string → Envelope<T> → stdout
```

### Switch commands (`switch session`, `switch worktree`)
```
CLI → tmux client → list panes/sessions → git branch resolution
    → build PickerItems → skim TUI → user selects
    → tmux switch-client/select-window/select-pane
```

### Bind command
```
CLI → tmux bind-key → display-popup → ta switch <target>
```

## Pane Naming Convention

Format: `{session}__{type}_{index}[_{variant}][tags]`

| Component | Example | Description |
|-----------|---------|-------------|
| session | `myproject` | Tmux session name |
| type | `cc` | Agent type tag (cc, cod, gmi, user, ...) |
| index | `1` | Sequential index per agent type |
| variant | `opus` | Model variant or persona (optional) |
| tags | `[frontend,api]` | User-defined metadata (optional) |

Fallback: if title doesn't match, detect agent type from `pane_current_command`.

## Zero-Registration Model

- No persistent state file or database
- All pane inventory comes from `tmux list-panes`
- Pre-existing sessions can be adopted by renaming pane titles (planned: `ta adopt`)
- Git worktrees discovered from pane working directories
