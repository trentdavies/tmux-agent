# Fuzzy Switching Design

## Approach
Uses the `skim` crate (v0.10) embedded in the binary. No external fzf dependency.

## Switchers

### General (`ta switch`)
Replaces `~/.local/bin/tmux-pane-finder`. Shows all panes with:
- Target (session:window.pane)
- Agent label (cc_1_opus, user, etc.)
- Tags
- Working directory
- Git branch

Batch-resolves git branches to avoid N sequential git calls.

### Session (`ta switch session`)
Lists sessions with window count, attached status, directory.
Preview shows `ta session show` output.

### Window (`ta switch window`)
Lists windows across all sessions with pane labels.
Preview shows capture-pane of active pane.

### Worktree (`ta switch worktree`)
Replaces `wt()` from `~/.zshrc`. Improvements:
- Discovers worktrees from all pane working directories (not just current repo)
- Cross-references with existing panes
- Preview: capture-pane if pane exists, git log otherwise
- Selection: jump to existing pane or create new window

## Popup Integration
`ta bind` generates `tmux bind-key <key> display-popup -E -w 80% -h 60% "ta switch <target>"`.

## Shell Integration
`ta shell zsh` generates aliases (ts, tss, tw, tp, twt) that auto-detect tmux and use display-popup.
