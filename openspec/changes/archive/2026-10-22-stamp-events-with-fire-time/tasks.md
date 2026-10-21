## 1. Wire protocol

- [x] 1.1 Add `fired_at_ms: u64` to `AgentEvent` in `crates/agentmon-proto/src/lib.rs`, with a doc comment explaining it's the reporting client's own captured time, not the daemon's - verify by updating every `AgentEvent { ... }` literal in that file's `#[cfg(test)]` module (e.g. `agent_event_round_trips_through_json`, `client_message_report_event_round_trips_through_json`) to supply it, and running `cargo test -p agentmon-proto`.
- [x] 1.2 Update the README's `classDiagram` data-model section (per `CLAUDE.md`'s sync instructions) to add `fired_at_ms` to `AgentEvent` - verify by diffing the diagram's `AgentEvent` fields against `crates/agentmon-proto/src/lib.rs` by hand.

## 2. Reporter: capture fire time before any other work

- [x] 2.1 In `crates/agentmon-report/src/main.rs`, capture `fired_at_ms` (milliseconds since `UNIX_EPOCH`) as the first instruction of `run_report`/`main`, before `detect_host_context()` and `parent_pid()` - verify by reading the function top-to-bottom and confirming no other statement precedes the capture.
- [x] 2.2 Thread `fired_at_ms` into `read_and_build_event` (or wherever `AgentEvent` is constructed in `crates/agentmon-report/src/report.rs`) so the built event carries it - verify with a unit test asserting the constructed `AgentEvent.fired_at_ms` equals the value passed in, not a value derived from `SystemTime::now()` inside that function.
- [x] 2.3 Update any existing `agentmon-report` tests/fixtures that build an `AgentEvent` or call `read_and_build_event` to supply a `fired_at_ms` - verify with `cargo test -p agentmon-report`.

## 3. Daemon: use reported fire time instead of processing time

- [x] 3.1 In `crates/agentd/src/registry.rs`, change `Registry::upsert` so `last_updated_ms`, `status_since_ms`, and `run_started_ms` are set from `event.fired_at_ms` everywhere they currently call `now_ms()` for an agent event - verify by grepping `registry.rs` for `now_ms()` and confirming only non-event call sites (if any) remain.
- [x] 3.2 Update `registry.rs`'s existing tests (the scenarios for first-event registration, same-status updates, status transitions, running-transition resets, same-pid replacement, etc.) to set distinct `fired_at_ms` values per event and assert the registry's timestamps match the triggering event's `fired_at_ms`, not wall-clock time at test-run time - verify with `cargo test -p agentd registry`.
- [x] 3.3 In `crates/agentd/src/ingest.rs`'s `ingest_event`, replace the `now_ms()` calls used for the prompt log entry's `occurred_at_ms` (the non-run-starting branch) and the notify-triggered log entry's `occurred_at_ms` with the triggering event's `fired_at_ms` - verify with `cargo test -p agentd ingest`.
- [x] 3.4 Add a regression test reproducing the reported bug: feed the ingestor a `PreToolUse`-derived event (causing `is_run_start`) with a *later* `fired_at_ms`, followed by a `UserPromptSubmit`-derived event for the same session with an *earlier* `fired_at_ms` and prompt text (simulating the race - later-fired event applied first) - verify the resulting "started" log entry's `occurred_at_ms` is greater than the "submitted" log entry's `occurred_at_ms`, matching true fire order rather than apply order.

## 4. Workspace-wide verification

- [x] 4.1 Run `cargo build --workspace` and `cargo test --workspace` and fix every remaining `AgentEvent` literal or call site missing `fired_at_ms` - verify both commands exit 0.
- [x] 4.2 Manually reproduce the original bug end-to-end against a locally built daemon + reporter: submit a prompt, let the agent start, and confirm the Logs tab (or a `Subscribe` client snapshot) shows the "submitted" entry before the "started" entry - verify by visual inspection of the Logs tab after the fix, contrasted with the pre-fix build.
