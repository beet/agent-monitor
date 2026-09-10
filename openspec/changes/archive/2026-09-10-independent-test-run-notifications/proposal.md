## Why

Today the daemon only plays a sound for a test run when it fails *and* its directory has no tracked agent - it's framed purely as a fallback for the case where nothing else would surface the failure. In practice, an agent session can run long enough for the user to start, stop, and re-run tests several times before the agent itself finishes, and the user wants to hear about each test-run outcome as it happens rather than only when no agent happens to be tracked in that directory.

## What Changes

- **BREAKING**: A failing test run now plays the `Basso` sound every time, regardless of whether its directory has a tracked agent - the notification is no longer a fallback gated on agent absence.
- A passing test run now also plays a sound (`Tink`), distinct from `Basso` (failed), `Glass` (agent done), and `Ping` (agent needs input), so the user can hear a suite recover as well as fail.
- A started test run now also plays a sound (`Pop`), so the user hears a run begin, not just how it ended.
- Every reported test-run event ("started", "passed", or "failed") notifies independently - repeated start/finish cycles in the same directory over the life of one long agent session each produce their own notification, mirroring how repeated "needs input" agent events already notify every time.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: the "Notification fallback for a test run with no tracked agent" requirement is replaced by an unconditional "Test-run lifecycle notifications" requirement covering "started", "passed", and "failed" statuses, independent of whether the directory has a tracked agent.

## Impact

- `crates/agentd/src/notify.rs`: `Notifier::notify_test_run_failure(&self, cwd: &Path)` is replaced by `Notifier::notify_test_run(&self, cwd: &Path, status: TestRunStatus)`, handling `Started` (new `Pop` sound), `Passed` (new `Tink` sound), and `Failed` (existing `Basso` sound).
- `crates/agentd/src/ingest.rs`: `ingest_test_run` drops the `has_tracked_agent` gate and calls the notifier for every reported status unconditionally.
- `crates/agentd/src/registry.rs`: `TestRunUpsertOutcome` and its `has_tracked_agent` field are removed - `upsert_test_run` returns `TestRunInfo` directly, since nothing consumes that field once the gate is gone.
- Existing tests in `notify.rs` and `ingest.rs` covering the old fallback-only behavior are updated or replaced to assert the new unconditional, per-status notification behavior.
