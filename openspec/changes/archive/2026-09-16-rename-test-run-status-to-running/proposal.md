## Why

A tracked test run's ongoing/live status is currently named `TestRunStatus::Started` and displayed as "started" wherever the TUI shows a test run in progress (the Agents tab's Tests column, the details modal's Tests pane). This conflates two distinct things that agents already keep separate: a one-time "the run began" event (which the RSpec formatter reports and which the activity log records, both correctly called "started") and the ongoing state of a run that is currently executing, which for agents is named "running" (`AgentStatus::Running`), never "started". Displaying "started" for a test run that has been running for the last two minutes reads as stale/wrong, and the naming mismatch with agents is confusing when the two appear side by side in the same row.

## What Changes

- Rename the live/ongoing test-run status from `Started` to `Running`, matching `AgentStatus::Running`'s naming. The Agents tab's Tests column and the details modal's Tests pane now display "running" (e.g. "⏳ tests running") instead of "started" for an in-progress test run.
- Decouple the Logs tab/pane's one-time "a test run began" log entry from this live status, the same way agents already decouple their "started" log entry from `AgentStatus` (see `agent_started_text_and_style` in `crates/agentmon/src/ui.rs`, which never routes through `AgentStatus::Running`'s own renderer). A dedicated renderer produces that log entry's "⏳ tests started" label directly, independent of `TestRunStatus`.
- No wire-protocol, RSpec-formatter, or activity-log behavior changes: the formatter continues to report a "started" event (per the rspec-test-reporting spec, unchanged), and the daemon continues to log and notify on that event using the word "started" (per the activity-log and agent-daemon specs, unchanged). Only the Rust-level identifier for the live status and its TUI display text change. The wire/JSON value for this status is preserved as `"started"` via an explicit serde rename on the renamed variant, so no client/formatter compatibility work is needed.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the "Live agent list" and "Status is visually distinguishable" requirements are updated so a test run's ongoing status is named and displayed as "running" rather than "started"; the "Status labels are prefixed by category outside dedicated panes" requirement's Agents-tab example is updated to match.

## Impact

- `crates/agentmon-proto/src/lib.rs`: `TestRunStatus::Started` renamed to `TestRunStatus::Running`, with `#[serde(rename = "started")]` added to that variant so its wire JSON representation is unchanged.
- `crates/agentd/src/registry.rs`, `crates/agentd/src/ingest.rs`, `crates/agentd/src/notify.rs`, `crates/agentd/src/server.rs`: match arms and test fixtures updated for the renamed variant; the literal strings they produce (log entry status "started", notification text "tests started") are unchanged.
- `crates/agentmon/src/ui.rs`: `test_run_status_cell_text_and_style`'s `Running` arm displays "running" instead of "started"; a new `test_run_started_text_and_style` function (mirroring `agent_started_text_and_style`) renders the Logs tab/pane's "started" log entry independently of `TestRunStatus`.
- No changes to `crates/agentmon-report` (the RSpec formatter) or the rspec-test-reporting/activity-log/agent-daemon specs - the reported event and logged/notified text remain "started".
