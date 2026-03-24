# AGENTS.md — ta (Tmux Agent TAsks)

> Guidelines for AI coding agents working in this Rust codebase.

---

## Documentation Conventions

All plans, architecture, and specs live in `docs/`. Follow this workflow:

1. **Read `docs/index.md` first** for orientation on the project
2. **Read `docs/tasks.md`** for all main tasks — it can link to `./docs/changes/<topic>/tasks.md`
3. **Check `docs/changes/`** for the current phase of work
4. **Add new work** in `docs/changes/<topic>/` with its own `overview.md` and `tasks.md`
5. **Update `docs/index.md`** when adding new docs
6. **Keep `docs/tasks.md` updated** when completing or adding tasks
7. **Update `docs/lessons.md`** with things to avoid in future work

---

## Project Structure

```
src/
  main.rs           # CLI entry point, command dispatch
  lib.rs            # Module re-exports
  cli.rs            # clap derive definitions
  error.rs          # TaError enum (thiserror)
  envelope.rs       # JSON response envelope
  tmux/             # Tmux abstraction layer
    client.rs       # Command execution (local + SSH)
    pane.rs         # Pane types, naming convention parser
    session.rs      # Session/pane queries from live tmux
    capture.rs      # capture-pane wrapper
    keys.rs         # send-keys wrapper
  switch/           # Fuzzy switchers (embedded skim)
    mod.rs          # Shared picker logic
    session.rs      # Session switcher
    window.rs       # Window switcher
    pane.rs         # General pane switcher
    worktree.rs     # Git worktree switcher
```

## Build & Test

```bash
cargo build          # Build
cargo test           # Run tests
cargo clippy         # Lint
```

## Key Design Decisions

- **Zero-registration model**: all state queried live from tmux, no persistent registry
- **Pane naming convention**: `{session}__{type}_{index}[_{variant}][tags]` (from ntm)
- **Embedded skim**: fuzzy picker runs in-process, no fzf dependency
- **JSON envelope**: all structured output wrapped in `Envelope<T>` with error codes + hints
- **Feature-gated agent detection**: `--features agent-detect` for per-agent-type state inference

## Git

- **One branch: `main`** — all work happens here
- Do not create side branches without explicit instruction
- Do not delete files without explicit permission
