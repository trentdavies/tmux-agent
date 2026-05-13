set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

build:
    cargo build

test:
    cargo test

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

clippy:
    cargo clippy --all-targets --all-features

plugin-test:
    sh tests/tmux-agent-plugin-test.sh

# This is a repo test harness, not plugin runtime. It can use Bash.
# The TPM runtime entrypoint stays POSIX sh for macOS/Linux portability.
smoke-tmux-plugin:
    #!/usr/bin/env bash
    if ! command -v tmux >/dev/null 2>&1; then
      echo "tmux is required for smoke-tmux-plugin" >&2
      exit 1
    fi

    cargo build --release --locked

    tmpdir="$(mktemp -d)"
    socket_path="$tmpdir/ta-smoke.sock"
    tmux_bin="$(command -v tmux)"
    cleanup() {
      "$tmux_bin" -S "$socket_path" kill-server >/dev/null 2>&1 || true
      rm -rf "$tmpdir"
    }
    trap cleanup EXIT

    cat >"$tmpdir/tmux" <<EOF
    #!/usr/bin/env bash
    exec "$tmux_bin" -S "$socket_path" "\$@"
    EOF
    chmod +x "$tmpdir/tmux"

    "$tmux_bin" -S "$socket_path" -f /dev/null new-session -d -s ta-smoke
    PATH="$tmpdir:$PATH" sh "$PWD/tmux-agent.tmux"

    keys="$("$tmux_bin" -S "$socket_path" list-keys -T prefix)"
    assert_key() {
      if ! grep -F "$1" <<<"$keys" >/dev/null; then
        echo "missing tmux-agent binding containing: $1" >&2
        "$tmux_bin" -S "$socket_path" show-messages 2>/dev/null >&2 || true
        exit 1
      fi
    }

    assert_key "switch agent"
    assert_key "switch session"
    assert_key "switch window"
    assert_key "switch pane"
    assert_key "switch worktree"
    assert_key "display-popup"

check:
    sh tests/tmux-agent-plugin-test.sh
    cargo fmt --all --check
    cargo test
    cargo clippy --all-targets --all-features

install:
    cargo install --offline --path . --locked

install-dev:
    cargo build && ln -sf $(pwd)/target/debug/ta ~/.cargo/bin/ta

run *args:
    cargo run -- {{args}}

version:
    cargo run -- --version

release version:
    #!/usr/bin/env bash
    if [[ ! "{{version}}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      echo "release version must match X.Y.Z" >&2
      exit 1
    fi
    if [[ -n "$(git status --short)" ]]; then
      echo "release requires a clean worktree" >&2
      exit 1
    fi
    if git rev-parse --verify "refs/tags/v{{version}}" >/dev/null 2>&1; then
      echo "tag v{{version}} already exists" >&2
      exit 1
    fi
    perl -0pi -e 's/^version = "\K[^"]+(?=")/{{version}}/m' Cargo.toml
    cargo generate-lockfile
    cargo fmt --all
    cargo test
    cargo clippy --all-targets --all-features
    git add Cargo.toml Cargo.lock
    git commit -m "release: v{{version}}"
    git tag -a "v{{version}}" -m "v{{version}}"
