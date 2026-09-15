## 1. Protocol changes

- [x] 1.1 In `crates/agentmon-proto/src/lib.rs`, add `run_started_ms: u64` to `AgentInfo` and add `pid: Option<u32>` to `LogEntry`; update the sample fixtures/round-trip tests (`sample_agent`, `sample_log_entry`, and their JSON assertions) to cover the new fields, and verify `cargo test -p agentmon-proto` passes.

## 2. Daemon: run-started timestamp

- [x] 2.1 In `crates/agentd/src/registry.rs`, set `run_started_ms` to the current time when `Registry::upsert` creates a brand-new entry (no existing session id or pid match).
- [x] 2.2 In the same function, reset `run_started_ms` to the current time whenever an existing entry's status transitions to `Running` from a different status - **including from `Done`** (an agent's pid persists across many started→running→done turns, unlike a test run's pid-scoped run-start timestamp, so this reset must fire on every re-entry into running, not just once per entry). Leave `run_started_ms` unchanged on: a same-status update, a transition away from running, and a transition between two non-running statuses.
- [x] 2.3 When `upsert` replaces an existing pid's entry with a new session id (the `/clear` case), set the new entry's `run_started_ms` to the current time, the same as any other newly created entry.
- [x] 2.4 Add/extend unit tests in `registry.rs` asserting: a new entry's `run_started_ms` equals its creation time; it is unchanged across a status-same update; it resets on every transition into running, including done -> running; it is left unchanged across a transition out of running or between two non-running statuses; a pid-replacement sets it to the new entry's creation time.
- [x] 2.5 In `crates/agentd/src/server.rs`, include `run_started_ms` in the agent snapshot/update payloads sent to clients; extend the existing snapshot/update round-trip tests to assert its presence and that it refreshes exactly on transitions into running.

## 3. Daemon: "agent started" activity-log capture

- [x] 3.1 In `crates/agentd/src/ingest.rs`, add a check in `ingest_event` (independent of `should_notify`) that appends an `agent`/`started` `LogEntry` (with `pid` set) whenever the transition moves status to `Running` from a different previous status - **including from `Done`** - or the event registers a brand-new entry whose initial status is `Running`. This should reuse the same transition check that resets `run_started_ms` in task 2.2, since both fire on exactly the same trigger.
- [x] 3.2 Verify no `started` entry is appended when an already-`Running` agent receives another `Running`-producing event (`PreToolUse`/`PostToolUse` no-op case) - add a unit test asserting the activity log gains no new entry in that case.
- [x] 3.3 Add unit tests covering each resuming transition into running (from idle, needs input, done, declined, stale) appending exactly one `agent`/`started` entry each, and confirm a session that cycles through several started→running→done turns under the same pid produces one `started` entry per turn, not just the first.
- [x] 3.4 Populate `LogEntry.pid` for the existing "done" and "needs input" log-append call sites in `ingest.rs`, using the pid already available on the triggering event/registry entry; extend existing activity-log tests to assert the pid is present and correct.
- [x] 3.5 Run `cargo test -p agentd` and confirm all existing and new tests pass, including the full activity-log capture requirement's scenarios (started/done/needs-input/declined-not-logged/stale-not-logged).

## 4. TUI: Agents/Tests column split

- [x] 4.1 In `crates/agentmon/src/ui.rs`, change `render_agent_table`'s header from `["PROJECT", "STATUS", "UPDATED"]` to `["PROJECT", "AGENTS", "TESTS", "UPDATED"]` and split row construction so agent status segments go into the Agents cell and the test-run segment goes into the Tests cell (each project row now has one `Cell` per column instead of one combined status `Cell`).
- [x] 4.2 Update the column-width constraint logic so the combined space previously given to STATUS is split between the new Agents and Tests columns, preserving the existing rule that status columns receive the majority of space beyond PROJECT/UPDATED; update the wide/narrow-terminal width tests for the new column count.
- [x] 4.3 Update/extend existing tests that locate the "STATUS" header or assert on the combined status cell (e.g. `seeded_agents_are_rendered_in_the_table`, `two_agents_with_the_same_status_produce_one_status_segment`, `an_agent_status_and_test_run_status_are_shown_together`, `a_directory_with_no_tracked_agent_still_shows_its_test_run`) to look for "AGENTS"/"TESTS" headers and cells independently; verify `cargo test -p agentmon` passes for these.

## 5. TUI: stale emoji fix

- [x] 5.1 In `crates/agentmon/src/ui.rs`, change the stale status's emoji constant from `🕸️` to `👻` everywhere it's defined (`status_label_and_style` and any duplicated literal in tests/help text).
- [x] 5.2 Update existing tests referencing `🕸️` (e.g. `each_test_run_status_has_a_distinct_emoji_marker`-style stale-specific assertions, help-modal text if it lists the marker) to expect `👻`; verify no test still asserts the old marker.

## 6. TUI: "done" duration

- [x] 6.1 In `crates/agentmon/src/ui.rs`, extend `status_cell_text_and_style` (or its caller) so an agent whose status is "done" computes a fixed duration from `run_started_ms` to `status_since_ms`, formatted with the existing compact-duration formatter, and appends it to the cell the same way the "running" duration is appended today.
- [x] 6.2 Confirm idle/needs-input/stale/declined continue to render with no duration (should already follow from only handling "running" and "done" explicitly); add/update a test enumerating all six agent statuses and asserting exactly running and done carry a duration.
- [x] 6.3 Add a test asserting a "done" agent's duration is fixed (computed once from `run_started_ms`/`status_since_ms`) and does not change across repeated redraws with different "now" values, mirroring `format_test_run_duration_for_a_finished_run_is_fixed_regardless_of_now`.
- [x] 6.4 Add a test simulating a session that cycles through two full started→running→done turns under one pid (done -> running -> done again) and asserts the second "done" duration is computed from the second turn's `run_started_ms`, not accumulated from the first turn's start.

## 7. TUI: process id display in the details modal

- [x] 7.1 In `render_details_modal`, render each agent row in the Agents pane with its process id alongside its status text; add a rendering test asserting a known pid value appears in the Agents pane's rendered buffer.
- [x] 7.2 In the same modal's Logs pane rendering, show the reporting agent's process id for entries whose category is agent (using the new `LogEntry.pid`), and omit it for test-run entries; add a test asserting an agent-category entry's pid appears in the Logs pane and a test-run entry's row does not show one.

## 8. TUI: category-word prefix in top-level views

- [x] 8.1 In `crates/agentmon/src/ui.rs`, add a `with_category_prefix: bool` parameter (or equivalent) to the shared label-building helpers (`status_label_and_style`, `test_run_status_cell_text_and_style`, `log_entry_status_line`/`log_status_cell_text_and_style`) so the emitted text is `"{emoji} agent {status}"` / `"{emoji} tests {status}"` when `true`, and `"{emoji} {status}"` when `false`, with duration appended after the status word in both cases.
- [x] 8.2 Pass `true` from `render_agent_table` (Agents tab) and `render_logs_tab` (Logs tab); pass `false` from `render_details_modal`'s Agents-pane and Tests-pane rendering; pass `true` from `render_details_modal`'s Logs-pane rendering.
- [x] 8.3 Update existing tests asserting exact label text (e.g. tests searching for `"running"`, `"done"`, `"passed"`, `"failed"` substrings in the Agents/Logs tabs) to expect the `"agent "`/`"tests "` prefix; add new tests asserting the details modal's Agents/Tests panes render without the prefix while its Logs pane renders with it.

## 9. TUI: rendering the new "agent started" activity

- [x] 9.1 Confirm (or extend if needed) that `render_logs_tab` and the details modal's Logs pane render an `agent`/`started` entry using the same emoji/color scheme as other agent statuses (⏳-equivalent or a dedicated started marker consistent with existing agent status markers - reuse whatever marker convention `status_label_and_style` already applies, extended to cover a log-only "started" agent status label) and the category prefix rules from task 8; add a rendering test asserting an `agent`/`started` log entry appears correctly in both the Logs tab and the modal's Logs pane.
- [x] 9.2 Confirm the Agents tab's Agents column and the details modal's Agents pane never render a "started" label (since it is a one-time log event, not a live status) - add a test asserting the Agents column only ever shows the six existing `AgentStatus` labels.
- [x] 9.3 Extend `log_test_run_duration_ms`'s pattern (or add an analogous agent-scoped helper) so a "done" agent Logs-tab/Logs-pane entry with a preceding same-project `agent`/`started` entry shows the elapsed duration between them; add a test mirroring `logs_tab_shows_a_completed_test_runs_duration` for the agent case, and one mirroring `logs_tab_shows_no_duration_for_a_started_run_with_no_completion_yet` for a `started` entry with no following `done`.

## 10. Verification

- [x] 10.1 Run `cargo test --workspace` and confirm all tests pass; run `cargo clippy --workspace --all-targets` and confirm no new warnings. (All 257 tests across the workspace pass; clippy is clean except one pre-existing doc-formatting warning in `registry.rs`, confirmed present on `main` before these edits.)
- [x] 10.2 Manually verify against an isolated `HOME`-scoped `agentd`/`agentmon` pair (never the default socket path, per this project's manual-testing convention). (Ran the real `agentd`/`agentmon-report` binaries under `HOME=/tmp/amon-verify-home-<ts>`, socket separate from the two live production `agentmon` TUIs and the real `agentd` found already running - confirmed those three processes' pids and the real socket's mtime were unchanged throughout and after teardown. Drove a real two-turn session through the daemon via `agentmon-report` piped hook JSON - PreToolUse->Stop, then PreToolUse->Notification(permission_prompt)->PreToolUse->Stop - and dumped the daemon's live Snapshot over its real Unix socket (newline-JSON `Subscribe`, per `framing.rs`): the activity log showed exactly `started, done, started, needs_input, started, done` (one `started` per turn plus the resume-after-needs-input case, matching design.md's documented behavior), each agent-category entry carried the correct `pid`, and after the liveness sweep marked the agent `stale`, `run_started_ms` remained pinned to the last turn's start (matching the last `started` entry's timestamp) while `status_since_ms` advanced - confirming the "transition away from running leaves run_started_ms unchanged" rule live. Also sent a real `ReportTestRun` message and confirmed it was tracked with its own `pid`, both on `TestRunInfo` and its log entry. Did not additionally drive the interactive `agentmon` TUI in this sandboxed, non-tty environment - the 123 `ui.rs` rendering tests (AGENTS/TESTS columns, the 👻 stale marker, category-word prefixes, done duration, and pid display in both the Agents pane and agent-category Logs-pane rows) already exercise the exact same rendering functions against real `AgentInfo`/`LogEntry` data with buffer-level assertions, so this manual pass focused on the daemon-side, real-process, real-socket behavior that those unit tests don't cover.)
