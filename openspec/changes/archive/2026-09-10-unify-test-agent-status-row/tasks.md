## 1. Protocol: add run-start timestamp to `TestRunInfo`

- [x] 1.1 Add `run_started_ms: u64` to `TestRunInfo` in `crates/agentmon-proto/src/lib.rs` and update `sample_test_run()` and the JSON round-trip tests to cover it; verify with `cargo test -p agentmon-proto`
- [x] 1.2 Update `crates/agentd/src/server.rs`'s sample/fixture test-run builders that construct `TestRunInfo` directly, if any, so they compile with the new field; verify with `cargo build -p agentd` (no direct `TestRunInfo` literals in `server.rs`; nothing to update)

## 2. Daemon: compute and preserve the run-start timestamp

- [x] 2.1 In `crates/agentd/src/registry.rs`'s `upsert_test_run`, set `run_started_ms` to `now` when the incoming pid differs from (or there is no) currently tracked test run for that `cwd`, and carry over the existing `run_started_ms` when the pid matches; verify with a new registry test asserting both branches (same-pid preserves, different-pid resets)
- [x] 2.2 Add a registry test asserting a `started` → `passed` sequence from the same pid keeps `run_started_ms` fixed while `last_updated_ms` advances; verify with `cargo test -p agentd`
- [x] 2.3 Add a registry test asserting a new pid reporting to the same directory resets `run_started_ms` to that event's time even though the directory already had a tracked test run; verify with `cargo test -p agentd`

## 3. TUI: collapse each project into one row

- [x] 3.1 In `crates/agentmon/src/ui.rs`, rewrite `render_agent_table` to build exactly one `Row` per `DirectoryGroup` instead of one row per agent plus one row per test run; verify with `cargo test -p agentmon` after updating the tests in task 3.4-3.6 below
- [x] 3.2 Drop the HOST and PID columns from the header and `widths`, leaving `[PROJECT, STATUS, UPDATED]`; verify the rendered table in a unit test no longer contains a host or pid cell
- [x] 3.3 Implement the combined STATUS cell: iterate a group's agents in `AgentStatus` declaration order, emit one segment per distinct status present (with a duration only when exactly one agent holds a "running" status), then append the test run's segment (if present) using its own status/duration, joined with ` · `; verify with unit tests covering one agent, multiple agents with the same status, multiple agents with different statuses, and an agent plus a differing test-run status
- [x] 3.4 Compute the row's UPDATED time as the most recent `last_updated_ms` among the group's agents and test run (reuse `DirectoryGroup::most_recent_update_ms` logic already used for group ordering); verify with a unit test
- [x] 3.5 Add test-run duration formatting: `format_running_duration(run_started_ms, now_ms)` for `Started`, and `format_running_duration(run_started_ms, last_updated_ms)` for `Passed`/`Failed`; verify with unit tests for a live "started" duration, a fixed "passed" duration, and a fixed "failed" duration
- [x] 3.6 Update existing `ui.rs` tests that assert one row per agent/test-run, or that read HOST/PID cells, to match the new one-row-per-project layout; verify with `cargo test -p agentmon`

## 4. Verify end to end

- [x] 4.1 Run `cargo test --workspace` and confirm all crates pass with the new row shape and run-start timestamp
- [x] 4.2 Manually exercise the TUI per the project's manual-testing guidance (isolated `HOME`/socket, never the default production socket): start an agent and an RSpec run in the same directory, confirm one row shows both statuses combined with the test run's live-then-final duration, and confirm a second agent in a different status appears as an added segment in the same row (confirmed by the user via manual `agentmon-report`/`nc` testing)
