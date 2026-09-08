## Context

`AgentInfo.last_updated_ms` is bumped on every registry upsert, including repeated `PreToolUse`/`PostToolUse` events during a single running turn (see `crates/agentd/src/registry.rs`), so it cannot be used to compute "time spent running" - it resets many times per turn. The TUI's main loop (`crates/agentmon/src/main.rs`) blocks on `rx.recv()` with no timeout, so today it only redraws in response to a client event or a keypress; nothing currently drives a periodic redraw. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Track, per agent, the timestamp of the most recent actual status transition, independent of same-status "heartbeat" events.
- Surface that timestamp to clients so the TUI can compute elapsed running time.
- Keep the displayed duration counting up in real time even when no new daemon event arrives.

**Non-Goals:**
- Historical/aggregate duration tracking (e.g. total time spent running across a session's lifetime, or per-status duration for idle/needs-input/done/stale). Only the current running streak is shown.
- Persisting durations across a daemon restart - like the rest of the registry, this is in-memory state.

## Decisions

### Add `status_since_ms` to `AgentInfo`, set only on real transitions
`Registry::upsert` already computes `previous_status` before applying an update. Extend it: set `status_since_ms = now_ms()` when `event.status != previous_status` (or the agent is new); otherwise carry the existing `status_since_ms` forward unchanged. `Registry::mark_stale` does the same, since stale is itself a transition.

Alternative considered: derive duration client-side by having the TUI remember when it first saw an agent in "running" status. Rejected because the TUI's view is reconstructed from scratch on every reconnect (including after a daemon restart mid-run), which would zero out an in-progress duration for no real reason; computing it once in the daemon and sending it as part of `AgentInfo` keeps a single source of truth that survives TUI reconnects.

### Drive periodic redraws with a tick channel, not a `recv_timeout` poll
Add a small thread that sends an `AppEvent::Tick` roughly once per second, merged onto the same channel `main.rs` already uses for client and key events. The render function recomputes duration from `status_since_ms` and wall-clock time on every draw, so a tick is just "wake up and redraw" - no extra state.

Alternative considered: replace `rx.recv()` with `rx.recv_timeout(1s)` and redraw on timeout. Rejected only for locality: a dedicated tick thread keeps `main.rs`'s event enum symmetric with how client/key events already arrive, rather than special-casing the timeout branch; behavior is equivalent.

### Duration formatting
Compact, fixed-ish width, matching the terse style of the existing status column: `<60s` → `9s`; `<1h` → `2m14s`; `>=1h` → `1h03m` (drop seconds once hours are shown, since sub-minute precision stops being useful). This lives next to `format_last_updated` in `ui.rs`.

## Risks / Trade-offs

- [Redrawing every ~1s costs a bit of idle CPU/battery] → Ratatui's diffed rendering makes an unchanged frame cheap; a 1s cadence is far below anything noticeable.
- [Clock skew if the daemon and TUI ran on different machines] → Not applicable: the socket is local-only (per agent-daemon spec), so daemon and TUI always share a clock.
