# Build Tooling + Versioning

## What
Add a repository `justfile` for common development and release flows, plus build-time version derivation from git tags and repository state.

## Why
The project needs a single, repeatable workflow for local verification and releasing, and the CLI should report versions that reflect the actual tagged source state rather than a hard-coded string.

## Acceptance Criteria
- `just` exposes the standard local workflows (`build`, `test`, `fmt`, `clippy`, `check`, `release`)
- build-time version output uses `vX.Y.Z` tags when present
- non-release builds include commit distance, short hash, and dirty state
- `ta --version` and the JSON envelope report the same computed version string
