## Why

The daemon's agent registry is keyed only by session id (`crates/agentd/src/registry.rs`). Claude Code assigns a new session id to the same running process when the user runs `/clear`, but keeps the same pid. The next hook event carries that new session id, which the registry treats as a brand-new agent and inserts as a second entry. The old entry is never updated again (no more hooks reference its session id) and is never swept as stale (its pid is still alive, since it's the same process), so it lingers in the TUI forever as a frozen duplicate row sharing the live row's pid.

## What Changes

- Registry `upsert` retires any existing entry that shares the new event's pid but has a different session id, before inserting the new session's entry, so a pid never has more than one live registry entry at a time.
- Retiring an old entry removes it outright rather than marking it stale, since the same pid is provably still active under its new session id, not disconnected.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: the agent registry requirement gains a same-pid dedup rule so a new session id for a pid that already has a tracked entry replaces that entry instead of coexisting with it.

## Impact

- `crates/agentd/src/registry.rs`: `Registry::upsert` gains pid-based dedup logic.
- `crates/agentd/src/liveness.rs`: no change, but this is the mechanism the bug currently falls through (a live pid is never swept), which is why the dedup has to happen in `upsert` instead.
- No proto or wire-format changes; no TUI changes (the TUI just renders what the daemon reports).
