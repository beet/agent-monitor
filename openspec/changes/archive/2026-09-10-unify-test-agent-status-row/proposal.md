## Why

The TUI's live agent list already groups agents and test runs by working directory, but renders that grouping as a flat table with one row per agent and a separate row per test run, repeating the project name on every row. In the user's actual workflow - one project directory, opened as several Zellij tabs/panes (nvim, gitui, a terminal), with an agent and an RSpec run each started independently and neither reliably preceding the other - that flat layout makes the table taller than it needs to be and splits a single project's status across multiple rows the user has to visually re-associate. It also gives no sense of how long the last test run took, even though the agent row already shows a live "running" duration.

## What Changes

- **BREAKING**: The TUI collapses each project (working directory) down to exactly one row instead of one row per tracked agent plus a separate row per test run.
- The STATUS column becomes a combined cell listing every distinct status present in the project - each of its agents' statuses and its test run's status if any - joined together (e.g. `🔧 running · 🔔 needs input · ❌ tests failed 45s`), rather than picking a single "winning" status.
- The HOST and PID columns are dropped from the table. The daemon continues tracking host context and pid for every agent as before; this data is just not rendered in the collapsed row (a future per-project "details" view could surface it later).
- Test runs gain a duration, shown the same way a running agent's duration is: live and counting up while the test run's status is "started"; once it passes or fails, the row keeps showing the total elapsed time the run took, computed from when it started to its last update.
- The daemon's `TestRunInfo` gains a run-start timestamp, preserved across the started → passed/failed lifecycle of the same test process (same pid) and reset only when a genuinely new run (a different pid) is reported for that directory.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the "Live agent list" and "Status is visually distinguishable" requirements change from one-row-per-agent/test-run to one row per project with a combined status cell; HOST/PID columns are removed; test-run rows gain a duration.
- `agent-daemon`: `TestRunInfo` (and the "Agent list query and live updates" / "Agents and test runs are grouped by working directory" requirements) gain a run-start timestamp that the registry preserves across same-pid lifecycle events and resets on a new pid.

## Impact

- `crates/agentmon-proto/src/lib.rs`: `TestRunInfo` gains a `run_started_ms` field; serialization tests updated.
- `crates/agentd/src/registry.rs`: `upsert_test_run` computes and preserves `run_started_ms` per the same-pid rule above.
- `crates/agentmon/src/app.rs`: `DirectoryGroup` (or its replacement) becomes the single source for one row per project; test-run tracking already keyed by `(cwd, pid)` is unaffected.
- `crates/agentmon/src/ui.rs`: table rendering rewritten around one row per project - combined STATUS cell, dropped HOST/PID columns, test-run duration formatting reusing the existing `format_running_duration` logic.
- Existing tests in `app.rs` and `ui.rs` asserting one-row-per-agent/test-run layout and the HOST/PID columns need updating to match the new row shape.
