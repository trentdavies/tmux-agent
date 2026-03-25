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

check:
    cargo fmt --all --check
    cargo test
    cargo clippy --all-targets --all-features

install:
    cargo install --offline --path . --locked

install-dev:
    cargo build && ln -sf $(pwd)/target/debug/workmux ~/.cargo/bin/workmux

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
