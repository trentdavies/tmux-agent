#!/bin/sh

set -eu

repo_dir="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/tmux-agent-plugin-test.XXXXXX")"

trap 'rm -rf "$tmp_root"' EXIT HUP INT TERM

fail() {
  printf 'not ok - %s\n' "$1" >&2
  exit 1
}

dump() {
  printf '%s\n' "--- $1 ---" >&2
  cat "$1" >&2 2>/dev/null || true
}

has() {
  grep -F -- "$2" "$1" >/dev/null 2>&1 || {
    dump "$1"
    fail "$3"
  }
}

lacks() {
  if grep -F -- "$2" "$1" >/dev/null 2>&1; then
    dump "$1"
    fail "$3"
  fi
}

make_exe() {
  mkdir -p "$(dirname "$1")"
  : >"$1"
  chmod +x "$1"
}

make_case() {
  case_dir="$tmp_root/$1"
  mkdir -p "$case_dir/bin" "$case_dir/home"
  cp "$repo_dir/tmux-agent.tmux" "$repo_dir/Cargo.toml" "$case_dir/"

  cat >"$case_dir/bin/tmux" <<'EOF'
#!/bin/sh
log_call() {
  printf '%s' "$1" >>"$CALLS"
  shift
  for arg do printf ' <%s>' "$arg" >>"$CALLS"; done
  printf '\n' >>"$CALLS"
}

case "$1" in
  -V) printf 'tmux %s\n' "${TMUX_VERSION:-3.6a}" ;;
  show-option)
    case "$3" in
      @tmux-agent-bin) printf '%s' "${OPT_BIN-}" ;;
      @tmux-agent-build) printf '%s' "${OPT_BUILD-}" ;;
      @tmux-agent-bindings) printf '%s' "${OPT_BINDINGS-}" ;;
    esac
    ;;
  display-message) printf '%s\n' "$2" >>"$MESSAGES" ;;
  *) log_call "$@" ;;
esac
EOF
  chmod +x "$case_dir/bin/tmux"

  cat >"$case_dir/bin/cargo" <<'EOF'
#!/bin/sh
case "$1" in
  --version)
    printf 'cargo %s (test)\n' "${CARGO_VERSION:-1.85.0}"
    ;;
  build)
    printf 'cargo build' >>"$CALLS"
    shift
    for arg do printf ' <%s>' "$arg" >>"$CALLS"; done
    printf '\n' >>"$CALLS"
    mkdir -p "$PLUGIN_DIR/target/release"
    : >"$PLUGIN_DIR/target/release/ta"
    chmod +x "$PLUGIN_DIR/target/release/ta"
    ;;
esac
EOF
  chmod +x "$case_dir/bin/cargo"

  : >"$case_dir/calls"
  : >"$case_dir/messages"
  printf '%s\n' "$case_dir"
}

run_plugin() {
  case_dir="$1"
  shift

  (
    cd "$tmp_root"
    env -i \
      PATH="$case_dir/bin:$PATH" \
      HOME="$case_dir/home" \
      TMPDIR="$case_dir" \
      PLUGIN_DIR="$case_dir" \
      CALLS="$case_dir/calls" \
      MESSAGES="$case_dir/messages" \
      "$@" \
      sh "$case_dir/tmux-agent.tmux"
  )
}

sh -n "$repo_dir/tmux-agent.tmux"
build_call="cargo build <--release> <--locked>"
manifest_arg="<--manifest-path>"
plugin_agent="/target/release/ta' switch agent"
build_log="home/.cache/tmux-agent/build.log"

case_dir="$(make_case configured-bin)"
make_exe "$case_dir/home/bin/ta path"
run_plugin "$case_dir" OPT_BIN='$HOME/bin/ta path'
lacks "$case_dir/calls" "cargo build" "configured binary should skip build"
has "$case_dir/calls" "bind-key <a> <display-popup>" "agent binding should be created"
has "$case_dir/calls" "TA_POPUP=1 '$case_dir/home/bin/ta path' switch agent" "configured binary should be quoted"

case_dir="$(make_case relative-bin)"
make_exe "$case_dir/relative/ta"
run_plugin "$case_dir" OPT_BIN="relative/ta"
has "$case_dir/messages" "must be an absolute path or command on PATH" "relative configured binary should be rejected"
lacks "$case_dir/calls" "bind-key" "relative configured binary should not bind"

case_dir="$(make_case old-cargo)"
run_plugin "$case_dir" CARGO_VERSION="1.60.0"
has "$case_dir/messages" "Cargo/Rust 1.85+ required" "old Cargo should be rejected"
has "$case_dir/$build_log" "rustup update stable" "old Cargo should write remediation"

case_dir="$(make_case build-success)"
run_plugin "$case_dir"
has "$case_dir/calls" "$build_call" "default path should build checkout"
has "$case_dir/calls" "$manifest_arg" "build should use explicit manifest path"
has "$case_dir/calls" "/Cargo.toml>" "manifest path should point at plugin checkout"
has "$case_dir/calls" "$plugin_agent" "default path should bind built binary"

case_dir="$(make_case upgrade-refresh)"
make_exe "$case_dir/target/release/ta"
touch -t 200001010000 "$case_dir/target/release/ta"
run_plugin "$case_dir"
has "$case_dir/calls" "$build_call" "existing plugin binary should be refreshed"
has "$case_dir/calls" "$plugin_agent" "refreshed plugin binary should be used"

printf 'ok - tmux-agent TPM plugin contract\n'
