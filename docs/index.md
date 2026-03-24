# ta — Tmux Agent TAsks

Rust CLI tool for managing and monitoring AI coding agents in tmux.

## Quick Links

- [Architecture](./architecture.md) — system design, module map, data flow
- [Spec](./spec.md) — CLI subcommands, flags, envelope format, naming convention
- [Tasks](./tasks.md) — all tracked work items
- [Lessons](./lessons.md) — things to avoid in future work

## Change Log

| Phase | Status | Description |
|-------|--------|-------------|
| [Phase 1: Core + Switching](./changes/phase-1-core-and-switching/overview.md) | In Progress | Scaffold, tmux abstraction, fzf switchers |
| [Phase 2: Communication](./changes/phase-2-communication/overview.md) | Planned | Send to panes, adopt sessions |
| [Phase 3: Attention](./changes/phase-3-attention/overview.md) | Planned | Event stream, cursor-based replay |
| [Phase 4: Alerts](./changes/phase-4-alerts/overview.md) | Planned | Alert management, health checks |
