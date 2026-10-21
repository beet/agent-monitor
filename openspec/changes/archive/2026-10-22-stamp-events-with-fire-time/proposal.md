## Why

The Logs tab still shows "agent started" entries before the "user prompt" entry that caused them, even after a prior fix (`fix-prompt-log-ordering-and-truncation`) narrowed the race. Root cause: the daemon has no ground truth for when a hook actually fired. `AgentEvent` carries no client-observed timestamp, so every "current time" the daemon records (an agent's `run_started_ms`/`status_since_ms`/`last_updated_ms`, and a `LogEntry`'s `occurred_at_ms`) is stamped at *daemon processing time*, which only matches true hook-fire order if the reporting processes happen to reach the daemon in that order. They don't reliably: each hook invocation spawns its own one-shot `agentmon-report` process, and `UserPromptSubmit` and the hooks that follow it (`PreToolUse`/`PostToolUse`) map to the same `Running` status, so whichever process's connection is accepted first gets credit for the "started" transition - and today that's decided by process-level jitter (notably, `agentmon-report` shells out to `ps` up to 12 times for host-context/parent-pid detection *before* it even connects to the daemon), not by which hook fired first.

## What Changes

- **BREAKING**: `AgentEvent` gains a required `fired_at_ms: u64` field, stamped by `agentmon-report` as the first instruction in its `main`, before any subprocess work (host-context detection, parent-pid lookup) or socket I/O. Daemon and reporter must be upgraded together.
- The daemon stops stamping event-caused timestamps with its own processing-time `now_ms()` and instead uses the reported `fired_at_ms` for: `AgentInfo.run_started_ms`, `status_since_ms`, `last_updated_ms`, and the `occurred_at_ms` of any `LogEntry` produced by that event (agent and prompt categories). Server-side `now_ms()` stays in use only where there is no originating hook event (e.g. the liveness sweep marking an agent stale).
- The `agent-daemon` spec's "Local socket ingestion API" requirement is corrected: its accept-order guarantee is replaced with a stronger one based on the reported fire time, since accept order was never a reliable proxy for hook-fire order once `agentmon-report`'s pre-connect work (host-context/parent-pid `ps` calls) is accounted for.
- Test-run events and reminder events are unaffected - they aren't produced by one-shot per-hook processes racing each other, so they keep using daemon-side `now_ms()`.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: `AgentEvent`'s wire shape gains `fired_at_ms`; the registry's event-caused timestamps and the activity log's `occurred_at_ms` are sourced from it instead of daemon processing time; the "Local socket ingestion API" requirement's ordering guarantee is restated in terms of reported fire time instead of connection accept order.

## Impact

- `crates/agentmon-proto/src/lib.rs`: `AgentEvent` gains `fired_at_ms: u64`.
- `crates/agentmon-report/src/main.rs` / `report.rs`: capture `fired_at_ms` as the first instruction in `main`, before `detect_host_context()`/`parent_pid()`/`connect()`; include it when building the event.
- `crates/agentd/src/registry.rs`: `upsert` uses `event.fired_at_ms` instead of `now_ms()` wherever it stamps `last_updated_ms`, `status_since_ms`, or `run_started_ms` for an agent event.
- `crates/agentd/src/ingest.rs`: `ingest_event`'s `record_log` calls use `event.fired_at_ms` (via the registry outcome) instead of `now_ms()` for agent/prompt log entries.
- `openspec/specs/agent-daemon/spec.md`: "Local socket ingestion API" requirement and its scenarios rewritten around reported fire time.
- All existing tests/fixtures that construct `AgentEvent` literals need a `fired_at_ms` field added.
- README.md's data-model diagram (per CLAUDE.md's sync instructions) needs `AgentEvent` updated with the new field.
