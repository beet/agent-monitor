## Why

When a submitted prompt also starts a new run (the common case: the previous turn already ended, so the next `UserPromptSubmit` both carries the prompt and flips status into "running"), the daemon logs a "prompt"/"submitted" entry and an "agent"/"started" entry sharing the same timestamp, and the Logs tab's recency sort was deliberately built to break that tie by showing the prompt entry above the started entry. Checking directly against a live daemon confirms this works exactly as built - the data and the sort are both correct, and there is no race or ordering bug.

But that's not the order the user actually wants. Scanning a reverse-chronological feed top-to-bottom, "agent started" reads as the headline (it already carries a snapshot of the triggering prompt inline, e.g. `agent started — "Fix the flaky test"`), and the standalone prompt entry beneath it as supporting detail - the opposite of today's deliberate tie-break, which puts the prompt first. This flips a specific, previously-deliberate convention (see `openspec/specs/activity-log/spec.md` and `openspec/specs/agent-monitor-tui/spec.md`), not a bug fix.

## What Changes

- When a `UserPromptSubmit` event both carries a prompt and starts a new run, the daemon now records the "agent"/"started" entry *before* the "prompt"/"submitted" entry (both still sharing the same event-occurred time) - the reverse of today's recording order.
- The Logs tab and the details modal's Logs pane (both driven by the same stable, insertion-order-preserving recency sort) consequently show the "started" entry immediately above its prompt, rather than below it, with no sort-logic changes required on the client - only the daemon's recording order changes.
- A follow-up prompt that does *not* start a new run (status already "running") is unaffected: there's no started entry to pair with, so it continues to log and sort purely by its own timestamp, same as today.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `activity-log`: the "Activity log captures notification-worthy events" requirement's ordering clause changes from "record the prompt entry before the started entry, so the prompt entry takes precedence in recency order" to "record the started entry before the prompt entry, so the started entry takes precedence in recency order."
- `agent-monitor-tui`: the "Logs tab shows an aggregated, paginated activity list" requirement's tie-break clause, and the "A prompt entry that starts a run sorts just ahead of that started entry" scenario, change to describe the started entry sorting ahead of (above) the prompt entry it precipitated, consistent with the new recording order.

## Impact

- `crates/agentd/src/ingest.rs`: `Ingestor::ingest_event` swaps the order of its two `record_log` calls when a prompt-bearing event also starts a run.
- `crates/agentd/src/ingest.rs` tests: the recording-order test is updated to assert started-before-prompt.
- `crates/agentmon/src/app.rs` tests: the tie-break test is updated to assert the started entry sorts ahead of the prompt entry (`visible_logs`/`project_logs` themselves need no code change - they already just stably sort by `occurred_at_ms`, and now receive entries in the new order).
- No wire-format, proto, or daemon-ordering-mechanism change - this is purely which of two already-co-timestamped entries gets appended to the activity log first.
