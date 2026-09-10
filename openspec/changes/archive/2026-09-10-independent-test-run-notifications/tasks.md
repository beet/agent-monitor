## 1. Notifier: generalize the test-run notification method

- [x] 1.1 In `crates/agentd/src/notify.rs`, replace `Notifier::notify_test_run_failure(&self, cwd: &Path)` with `notify_test_run(&self, cwd: &Path, status: TestRunStatus)` (default no-op), and update `OsaScriptNotifier`'s implementation to build a script for `Failed` (`Basso`, unchanged) and `Passed` (new `Tink` sound), returning no script for `Started`; verify with `cargo build -p agentd`
- [x] 1.2 Update `notify.rs`'s tests: rename/extend the existing `test_run_failure_script_*` tests for the new function signature, and add a test asserting the `Passed` script ends with `sound name "Tink"` and a test asserting `Started` produces no script; verify with `cargo test -p agentd`
- [x] 1.3 Extend `OsaScriptNotifier`'s test-run script builder to also cover `Started`, playing the new `Pop` sound instead of producing no script; update the "Started produces no script" test into one asserting it ends with `sound name "Pop"`; verify with `cargo test -p agentd`

## 2. Ingest: notify independent of tracked-agent presence

- [x] 2.1 In `crates/agentd/src/ingest.rs`'s `ingest_test_run`, drop the `!outcome.has_tracked_agent` condition and call `self.notifier.notify_test_run(cwd, status)` whenever `status` is `Passed` or `Failed` (never for `Started`); verify with `cargo build -p agentd`
- [x] 2.2 Update `ingest.rs`'s `RecordingNotifier` test double to implement `notify_test_run` (recording `(PathBuf, TestRunStatus)` pairs) instead of `notify_test_run_failure`; verify with `cargo build -p agentd --tests`
- [x] 2.3 Update the existing fallback-only tests to match the new unconditional behavior: a failing run with a tracked agent must now notify (rename `a_failing_run_with_a_tracked_agent_does_not_notify_via_the_fallback` to assert it *does* notify), and a passed run must notify while a started run still must not; verify with `cargo test -p agentd`
- [x] 2.4 Add a test asserting two "failed" events for the same directory (each preceded by its own "started" event, so each is a distinct run) each produce their own notification, not just the first; verify with `cargo test -p agentd`
- [x] 2.5 In `ingest_test_run`, call the notifier for `Started` too (drop the `matches!(status, Passed | Failed)` restriction to include `Started`); replace `a_started_run_never_notifies` with a test asserting it *does* notify, and extend `repeated_failures_in_the_same_directory_each_notify`'s scenario (or add a sibling) to confirm each `started` event in a start/fail/start/fail sequence also notifies independently; verify with `cargo test -p agentd`

## 3. Registry: drop the now-unused `has_tracked_agent` field

- [x] 3.1 In `crates/agentd/src/registry.rs`, remove `TestRunUpsertOutcome` and change `upsert_test_run` to return `TestRunInfo` directly, since `has_tracked_agent` has no remaining consumer once the ingest-side gate is gone; update `ingest_test_run` in `ingest.rs` to match the new return type; verify with `cargo build -p agentd`
- [x] 3.2 Update or remove `registry.rs`'s tests that assert `has_tracked_agent` (`first_started_event_creates_a_test_run_entry`'s `!outcome.has_tracked_agent` assertion, and `upsert_test_run_reports_a_tracked_agent_in_the_same_directory`, which no longer has anything to assert); verify with `cargo test -p agentd`

## 4. Verify end to end

- [x] 4.1 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets` and confirm no failures or new warnings
- [x] 4.2 Manually exercise the daemon per the project's manual-testing guidance (isolated `HOME`/socket): report a "started" event and confirm `Pop` plays, a "failed" event for a directory with a tracked agent and confirm `Basso` plays, then a "passed" event for the same directory and confirm `Tink` plays (confirmed by the user manually via `nc -U`)
