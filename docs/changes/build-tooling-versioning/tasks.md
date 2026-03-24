# Build Tooling + Versioning Tasks

- [x] Add a `justfile` with common cargo workflows
- [x] Add a guarded `release` recipe that bumps `Cargo.toml`, verifies the project, commits, and tags `vX.Y.Z`
- [x] Replace the hard-coded envelope version with build-time git-derived versioning
- [x] Expose computed version information through `clap --version`
- [ ] Run end-to-end manual release validation against a disposable tag
