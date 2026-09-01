## 1. Registry guard against late needs-input after done

- [x] 1.1 In `crates/agentd/src/registry.rs`, change `Registry::upsert` (or add a check `Ingestor::ingest_event` applies before calling it) so a `NeedsInput` event is dropped when the session's currently stored status is `Done`, leaving the stored `AgentInfo` and `last_updated_ms` unchanged; `Running` and other statuses continue to overwrite `Done` as before
- [x] 1.2 Add a unit test in `registry.rs` asserting a `NeedsInput` event for a session already `Done` leaves the registry entry at `Done` with its original `last_updated_ms`
- [x] 1.3 Add a unit test in `registry.rs` asserting a `Running` event for a session already `Done` still updates the entry to `Running`, so the guard doesn't block a genuine new turn

## 2. Notify on every needs-input transition

- [x] 2.1 In `crates/agentd/src/ingest.rs`, change `should_notify` so it returns `true` for any `NeedsInput` event regardless of `previous`, and keeps the `previous != Some(current)` check only for `Done`
- [x] 2.2 Update the existing `repeated_identical_status_does_not_renotify` test (or split it) so it covers `Done` repeats not notifying while a repeated `NeedsInput` still does
- [x] 2.3 Add a unit test asserting two consecutive `NeedsInput` events for the same session (no `Running` in between) each produce a notification

## 3. End-to-end coverage for the race

- [x] 3.1 Add a test in `crates/agentd/src/server.rs` that drives a session through `Running` → `Done` → `NeedsInput` over separate connections (mirroring the existing `wait_for_status`-based tests) and asserts the final status stays `Done` with exactly one notification recorded
- [x] 3.2 Run `cargo test -p agentd -p agentmon-report` and confirm the full suite passes, including the new and updated tests from sections 1 and 2
