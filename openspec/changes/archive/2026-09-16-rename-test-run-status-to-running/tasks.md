## 1. Wire type

- [x] 1.1 In `crates/agentmon-proto/src/lib.rs`, rename `TestRunStatus::Started` to `TestRunStatus::Running` and add `#[serde(rename = "started")]` on that variant (with a doc comment explaining the intentional wire/display split, per design.md), then verify the crate's existing `TestRunStatus`-related serde tests still pass and add one asserting `TestRunStatus::Running` serializes to `"started"` and deserializes back from it

## 2. Daemon (agentd)

- [x] 2.1 Update `crates/agentd/src/registry.rs`'s `Started` references (code and tests) to `Running`, and verify `cargo test -p agentd registry::` passes with no behavior change (assertions on the enum variant, not on strings)
- [x] 2.2 Update `crates/agentd/src/ingest.rs`'s `test_run_log_status` match arm from `TestRunStatus::Started => "started"` to `TestRunStatus::Running => "started"` (string unchanged) plus its `Started` test references, and verify `cargo test -p agentd ingest::` passes, confirming logged entries still carry the literal status `"started"`
- [x] 2.3 Update `crates/agentd/src/notify.rs`'s `test_run_notification_script` match arm from `TestRunStatus::Started => ("tests started", "Pop")` to `TestRunStatus::Running => ("tests started", "Pop")` (text unchanged) plus its `Started` test references, and verify `cargo test -p agentd notify::` passes
- [x] 2.4 Update `crates/agentd/src/server.rs`'s `Started` test references to `Running`, and verify `cargo test -p agentd server::` passes

## 3. TUI client (agentmon)

- [x] 3.1 Update `crates/agentmon/src/app.rs` and `crates/agentmon/src/client.rs`'s `Started` test references to `Running`, and verify `cargo test -p agentmon app:: client::` passes
- [x] 3.2 In `crates/agentmon/src/ui.rs`, rename `test_run_status_cell_text_and_style`'s `TestRunStatus::Started` match arm to `TestRunStatus::Running` and change its label from `"⏳ started"` to `"⏳ running"`, and update `format_test_run_duration`'s matching arm (the `TestRunStatus::Started =>` case computing the live/counting-up duration) to `TestRunStatus::Running =>`
- [x] 3.3 Add a `test_run_started_text_and_style` function to `crates/agentmon/src/ui.rs`, mirroring `agent_started_text_and_style`, returning `("⏳ tests started", Style::new().fg(Color::Blue))`; change `log_status_cell_text_and_style`'s `LogCategory::TestRun` `"started"` arm to call it instead of `test_run_status_cell_text_and_style(TestRunStatus::Running, true)`
- [x] 3.4 Update `crates/agentmon/src/ui.rs`'s existing `Started`-named test fixtures/assertions to `Running`, and update any assertion text expecting `"started"` from the live-status renderer to expect `"running"` (the Agents tab's Tests column and the details modal's Tests pane), while assertions covering the Logs tab/pane's test-run "started" entry keep expecting `"started"`; verify `cargo test -p agentmon ui::` passes

## 4. Verification

- [x] 4.1 Run `cargo test --workspace` and confirm it passes
- [x] 4.2 Manually run the TUI against a real daemon (or the existing test harness pattern from `crates/agentmon/tests/`), start a tracked test run, and confirm the Agents tab's Tests column and the details modal's Tests pane show "⏳ tests running" / "⏳ running" while it's in progress, while the Logs tab still shows "⏳ tests started" for the run's start entry
