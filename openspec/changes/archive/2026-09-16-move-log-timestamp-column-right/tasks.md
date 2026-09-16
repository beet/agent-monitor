## 1. Logs tab table

- [x] 1.1 In `render_logs_tab` (`crates/agentmon/src/ui.rs`), reorder the header array from `["TIME", "PROJECT", "CATEGORY", "STATUS"]` to `["PROJECT", "CATEGORY", "STATUS", "TIME"]`
- [x] 1.2 Reorder the row `Cell`s to match (project, category, status, then `format_last_updated(entry.occurred_at_ms)`), and reorder the `widths` array so `Constraint::Length(19)` trails the `Fill`/`Length(10)` constraints for project/category/status, mirroring `render_agent_table`'s widths order
- [x] 1.3 Verify by running the crate's existing UI tests for the Logs tab and confirming they still pass (or are updated to match) with `cargo test -p agentmon`

## 2. Details modal Logs pane

- [x] 2.1 In `render_details_modal`'s Logs pane construction (`crates/agentmon/src/ui.rs`), swap the `Line` span order so the status/category text (from `log_entry_line_with_pid`) comes first and `format_last_updated(entry.occurred_at_ms)` trails it, with a separating space, instead of leading
- [x] 2.2 Confirm no column headers are introduced for this pane (it stays a `Paragraph`, not a `Table`)
- [x] 2.3 Verify by running `cargo test -p agentmon`, including tests that assert on the details modal's Logs pane text (e.g. `details_modal_logs_pane_renders_an_agent_started_entry`, `details_modal_logs_pane_shows_a_completed_test_runs_duration`, `details_modal_logs_pane_shows_pid_for_agent_activities_but_not_test_runs`), updating any assertions that check for a specific text order

## 3. Full verification

- [x] 3.1 Run the full test suite with `cargo test -p agentmon` and confirm all tests pass
- [x] 3.2 Manually run `agentmon` against an isolated test daemon (never the default production socket) and visually confirm the Logs tab and the details modal's Logs pane both show the timestamp as the last element in each row/line
