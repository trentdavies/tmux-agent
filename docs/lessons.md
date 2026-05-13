# Lessons

Things to avoid in future work on ta.

- Do not ship a hard-coded CLI or envelope version string when the repository already has git state available at build time.
- Do not let TPM auto-build failures surface as raw dependency errors; check the Rust/Cargo version first and point users to the build log.
- Do not source a TPM `.tmux` shell entrypoint with `tmux source-file`; test it by executing the script with a tmux wrapper or through TPM.
- Do not parse tmux versions as strictly dotted numbers; releases can include suffixes like `3.6a`.
- Do not let TPM updates keep using a stale plugin-local binary; rebuild when the checkout is newer than the binary unless the user configured `@tmux-agent-bin`.
- Do not confuse the Bash-based `smoke-tmux-plugin` repo harness with plugin runtime requirements; `tmux-agent.tmux` stays POSIX sh for TPM portability.
