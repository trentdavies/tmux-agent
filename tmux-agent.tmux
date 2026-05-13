#!/bin/sh

# TPM entrypoint for tmux-agent.
# Keep this script small and POSIX-ish: TPM runs it during tmux startup.

set -u

current_dir="$(CDPATH= cd "$(dirname "$0")" && pwd)"
ta_release="$current_dir/target/release/ta"
required_tmux_version="3.2"

cache_root="${XDG_CACHE_HOME:-}"
if [ -z "$cache_root" ] && [ -n "${HOME:-}" ]; then
  cache_root="$HOME/.cache"
fi

if [ -n "$cache_root" ]; then
  build_log_dir="$cache_root/tmux-agent"
else
  build_log_dir="$current_dir/target"
fi

if ! mkdir -p "$build_log_dir" 2>/dev/null; then
  build_log_dir="$current_dir/target"
  mkdir -p "$build_log_dir" 2>/dev/null || build_log_dir=""
fi

if [ -n "$build_log_dir" ]; then
  chmod 700 "$build_log_dir" 2>/dev/null || true
  build_log="$build_log_dir/build.log"
else
  build_log="/dev/null"
fi

tmux_option() {
  value="$(tmux show-option -gqv "$1" 2>/dev/null || true)"
  if [ -n "$value" ]; then
    printf '%s' "$value"
  else
    printf '%s' "$2"
  fi
}

enabled() {
  case "$1" in
    1 | [Yy][Ee][Ss] | [Oo][Nn] | [Tt][Rr][Uu][Ee] | [Ee][Nn][Aa][Bb][Ll][Ee][Dd])
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

say() {
  tmux display-message "$1" >/dev/null 2>&1 || printf '%s\n' "$1" >&2
}

quote() {
  printf '%s\n' "$1" | sed "s/'/'\\\\''/g; 1s/^/'/; \$s/\$/'/"
}

expand_path() {
  home="${HOME:-}"

  case "$1" in
    '~')
      [ -n "$home" ] && printf '%s' "$home" || printf '%s' "$1"
      ;;
    '~/'*)
      [ -n "$home" ] && printf '%s/%s' "$home" "${1#\~/}" || printf '%s' "$1"
      ;;
    '$HOME'/*)
      [ -n "$home" ] && printf '%s/%s' "$home" "${1#\$HOME/}" || printf '%s' "$1"
      ;;
    '${HOME}'/*)
      [ -n "$home" ] && printf '%s/%s' "$home" "${1#\$\{HOME\}/}" || printf '%s' "$1"
      ;;
    *)
      printf '%s' "$1"
      ;;
  esac
}

rust_version() {
  awk -F\" '/^[[:space:]]*rust-version[[:space:]]*=/ { print $2; exit }' \
    "$current_dir/Cargo.toml"
}

cargo_version() {
  cargo --version 2>/dev/null | awk '{ print $2; exit }'
}

tmux_version() {
  tmux -V 2>/dev/null | awk '{ print $2; exit }'
}

version_at_least() {
  awk -v have="$1" -v need="$2" '
    function split_version(raw, out) {
      sub(/-.*/, "", raw)
      split(raw, out, ".")
      for (i = 1; i <= 3; i++) {
        sub(/[^0-9].*/, "", out[i])
        if (out[i] == "") out[i] = 0
      }
    }
    BEGIN {
      split_version(have, h)
      split_version(need, n)
      for (i = 1; i <= 3; i++) {
        if (h[i] + 0 > n[i] + 0) exit 0
        if (h[i] + 0 < n[i] + 0) exit 1
      }
      exit 0
    }
  '
}

write_log() {
  printf '%s\n' "$@" >"$build_log"
}

check_tmux() {
  found_tmux="$(tmux_version)"
  [ -n "$found_tmux" ] || found_tmux="unknown"
  if ! version_at_least "$found_tmux" "$required_tmux_version"; then
    say "tmux-agent: tmux $required_tmux_version+ required for display-popup bindings; found tmux $found_tmux"
    return 1
  fi
}

check_cargo() {
  required_rust="$(rust_version)"
  [ -n "$required_rust" ] || required_rust="1.85"

  if ! command -v cargo >/dev/null 2>&1; then
    write_log \
      "tmux-agent could not build ta." \
      "" \
      "cargo was not found on PATH." \
      "" \
      "Install Rust with rustup, or set @tmux-agent-bin to an existing ta binary."
    say "tmux-agent: cargo not found; see $build_log"
    return 1
  fi

  found_cargo="$(cargo_version)"
  if ! version_at_least "$found_cargo" "$required_rust"; then
    write_log \
      "tmux-agent could not build ta." \
      "" \
      "Cargo/Rust $required_rust or newer is required; found cargo $found_cargo." \
      "" \
      "Fix:" \
      "  rustup update stable" \
      "" \
      "Or set @tmux-agent-bin to an existing ta binary."
    say "tmux-agent: Cargo/Rust $required_rust+ required; found cargo $found_cargo; see $build_log"
    return 1
  fi
}

resolve_configured_ta() {
  configured="$(tmux_option "@tmux-agent-bin" "")"
  if [ -n "$configured" ]; then
    configured="$(expand_path "$configured")"

    case "$configured" in
      */*)
        case "$configured" in
          /*) ;;
          *)
            say "tmux-agent: @tmux-agent-bin must be an absolute path or command on PATH: $configured"
            return 2
            ;;
        esac
        ;;
      *)
        resolved="$(command -v "$configured" 2>/dev/null || true)"
        [ -n "$resolved" ] && configured="$resolved"
        ;;
    esac

    case "$configured" in
      /*) ;;
      *)
        say "tmux-agent: @tmux-agent-bin did not resolve to an absolute path: $configured"
        return 2
        ;;
    esac

    if [ -x "$configured" ]; then
      printf '%s' "$configured"
      return 0
    fi

    say "tmux-agent: @tmux-agent-bin is not executable: $configured"
    return 2
  fi

  return 1
}

resolve_existing_ta() {
  if [ -x "$ta_release" ]; then
    printf '%s' "$ta_release"
    return 0
  fi

  if command -v ta >/dev/null 2>&1; then
    command -v ta
    return 0
  fi

  return 1
}

has_newer_source() {
  find "$@" -type f -newer "$ta_release" -print 2>/dev/null |
    awk 'NR == 1 { found = 1; exit } END { exit found ? 0 : 1 }'
}

needs_build() {
  [ ! -x "$ta_release" ] && return 0

  for path in "$current_dir/Cargo.toml" "$current_dir/Cargo.lock" "$current_dir/build.rs" "$current_dir/src"; do
    [ -e "$path" ] || continue
    has_newer_source "$path" && return 0
  done

  return 1
}

build_ta() {
  check_cargo || return 1

  say "tmux-agent: building ta..."

  if {
    printf 'tmux-agent build\n\n'
    printf 'cargo: %s\n' "$(cargo --version)"
    if command -v rustc >/dev/null 2>&1; then
      printf 'rustc: %s\n' "$(rustc --version)"
    fi
    printf '\n$ cargo build --release --locked --manifest-path %s\n' \
      "$(quote "$current_dir/Cargo.toml")"
    cargo build --release --locked --manifest-path "$current_dir/Cargo.toml"
  } >"$build_log" 2>&1; then
    if [ -x "$ta_release" ]; then
      printf '%s' "$ta_release"
      return 0
    fi

    say "tmux-agent: build finished but target/release/ta is missing; see $build_log"
    return 1
  fi

  say "tmux-agent: cargo build failed; see $build_log"
  return 1
}

bind_popup() {
  name="$1"
  default_key="$2"
  subcommand="$3"

  enabled "$(tmux_option "@tmux-agent-bind-$name" "on")" || return 0

  key="$(tmux_option "@tmux-agent-$name-key" "$default_key")"
  [ -n "$key" ] || return 0

  command="TA_POPUP=1 $quoted_ta switch"
  if [ -n "$subcommand" ]; then
    command="$command $subcommand"
  fi

  tmux bind-key "$key" display-popup -E -w "$popup_width" -h "$popup_height" "$command"
}

bind_base() {
  base_command="$(tmux_option "@tmux-agent-base-command" "")"
  [ -n "$base_command" ] || return 0
  enabled "$(tmux_option "@tmux-agent-bind-base" "on")" || return 0

  key="$(tmux_option "@tmux-agent-base-key" "b")"
  [ -n "$key" ] || return 0

  base_name="$(tmux_option "@tmux-agent-base-name" "base")"
  command="$quoted_ta switch base --name $(quote "$base_name") --command $(quote "$base_command")"

  tmux bind-key "$key" run-shell -b "$command"
}

main() {
  enabled "$(tmux_option "@tmux-agent-bindings" "on")" || return 0
  check_tmux || return 0

  ta_bin="$(resolve_configured_ta)"
  status=$?

  [ "$status" -eq 2 ] && return 0

  if [ "$status" -ne 0 ]; then
    if enabled "$(tmux_option "@tmux-agent-build" "on")"; then
      if needs_build; then
        ta_bin="$(build_ta)" || return 0
      else
        ta_bin="$ta_release"
      fi
    else
      ta_bin="$(resolve_existing_ta)"
      status=$?
      if [ "$status" -ne 0 ]; then
        say "tmux-agent: ta not found; set @tmux-agent-bin or enable @tmux-agent-build"
        return 0
      fi
    fi
  fi

  quoted_ta="$(quote "$ta_bin")"
  popup_width="$(tmux_option "@tmux-agent-popup-width" "80%")"
  popup_height="$(tmux_option "@tmux-agent-popup-height" "60%")"

  bind_popup "session" "s" "session"
  bind_popup "window" "w" "window"
  bind_popup "pane" "f" "pane"
  bind_popup "worktree" "t" "worktree"
  bind_popup "agent" "a" "agent"
  bind_base
}

main
