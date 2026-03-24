# Tasks

## Phase 1: Core + Switching

See [phase-1 tasks](./changes/phase-1-core-and-switching/tasks.md) for details.

- [x] Project scaffold (Cargo.toml, CLI, error handling, envelope)
- [x] Tmux abstraction (client, pane, session, capture, keys)
- [x] Fuzzy switchers (session, window, pane, worktree) with embedded skim
- [x] Bind and shell commands
- [x] Documentation
- [ ] Build verification and manual testing in live tmux

## Phase 2: Communication

- [ ] Send to panes (targeting by index, agent type, broadcast)
- [ ] Adopt pre-existing sessions

## Build Tooling + Versioning

- [x] Add a repository `justfile`
- [x] Add build-time git-derived version strings for CLI and envelopes
- [x] Add a release workflow that creates `vX.Y.Z` tags
- [ ] Manual release validation against a disposable version bump

## Phase 3: Attention

- [ ] Event types and envelope
- [ ] Attention journal (ring buffer, cursor replay)
- [ ] CLI commands (poll, watch, stats)

## Phase 4: Alerts

- [ ] Alert model and tracker
- [ ] Health checks (velocity, stall detection)
- [ ] CLI commands (list, ack, mute, resolve, summary)
