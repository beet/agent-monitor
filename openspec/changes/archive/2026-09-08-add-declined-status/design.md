## Context

`AgentStatus` (`crates/agentmon-proto/src/lib.rs`) is a closed, exhaustively-matched enum with 5 variants today (Running, Idle, NeedsInput, Done, Stale). Every consumer that matches on it (`status_label_and_style` in `ui.rs`, the notification logic in `notify.rs`) matches exhaustively, so the compiler will flag every site that needs updating once a 6th variant is added - there's no risk of silently missing a call site. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Give a declined permission prompt its own terminal-ish status instead of leaving "needs input" stuck.
- Keep the change mechanical: reuse all the generic status-transition machinery (`status_since_ms`, sorting, registry dedup) already built for the 5 existing statuses rather than special-casing "declined" anywhere it doesn't need to be.

**Non-Goals:**
- Distinguishing *who* declined (`denied_by: "user"` vs `"auto_mode"` in the hook payload) - both map to the same "declined" status. Not needed for the stated problem (status getting stuck), and the daemon doesn't currently thread any extra per-event metadata through to `AgentInfo` beyond status/timestamps.
- Any handling of user-initiated cancel/interrupt - explicitly out of scope, see proposal.md's Non-goals.

## Decisions

### Reuse the generic registry transition logic as-is
`Registry::upsert`'s status-since-timestamp logic (`crates/agentd/src/registry.rs`) already compares `event.status != previous_status` generically, with no hardcoded list of "known" statuses. Adding `Declined` to the `AgentStatus` enum and mapping `PermissionDenied` to it in `hook_payload.rs` is sufficient - no registry code changes needed beyond what the enum addition itself requires (the `match` in `status_label_and_style` and `notify.rs`'s attention-state check, both exhaustive).

### No "declined-after-done" guard, unlike the existing needs-input-after-done guard
The registry has a specific guard: a "needs input" event is dropped when status is already "done" (handles a known hook-ordering quirk where a stale `Notification` can arrive after `Stop`). `PermissionDenied` doesn't have the same quirk potential - per Claude Code's hook lifecycle, `PermissionDenied` fires synchronously as part of resolving a specific tool call's permission check, strictly before that turn can reach `Stop`. No evidence of a similar late-arrival race, so no equivalent guard is added. If this turns out to be wrong in practice, it's a small follow-up to the registry, not a design change.

### Emoji and style: 🚫, red, not bold
Follows the existing pattern in `status_label_and_style` (one emoji + one color per status). Red fits "denied/blocked" semantically. Not bold/attention-grabbing like "needs input" (yellow+bold), since by the time the TUI shows "declined" the user has already acted - there's nothing left for them to do, unlike "needs input" which is actively asking for action.

## Risks / Trade-offs

- [A future Claude Code hook-lifecycle change reintroduces a late/out-of-order `PermissionDenied` event] → Mitigation deferred (see "No declined-after-done guard" above); low likelihood, cheap to fix later if observed.
- [`AgentStatus` gaining a 6th variant is a wire-format-compatible but semantically new value for any external consumer of the JSON protocol] → None known today (the TUI is the only real consumer, and it's built from the same repo); no migration concern.
