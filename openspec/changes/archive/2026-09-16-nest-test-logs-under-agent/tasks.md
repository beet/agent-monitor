## 1. Tree prefix for the details modal's Logs pane

- [x] 1.1 In `crates/agentmon/src/ui.rs`, add a small helper (e.g. `log_entry_tree_prefix(category: agentmon_proto::LogCategory) -> &'static str`) that returns `"├─ "` for `LogCategory::TestRun` and `""` for `LogCategory::Agent`, and verify it compiles with a `cargo build -p agentmon`.
- [x] 1.2 In `render_details_modal`'s Logs pane row construction, prepend that prefix as an unstyled `Span` ahead of the existing styled status `Span` (turning each row's first cell into a `Line` of two spans instead of a single styled string), so the branch marker itself is not colored by the entry's status style, and verify the details modal still renders (`cargo build -p agentmon`).
- [x] 1.3 Confirm the prefix does not shift the Logs pane's fixed-width right-aligned timestamp column, since that column is a separate `Cell`/`Constraint::Length(19)` unaffected by the first cell's content.

## 2. Tests

- [x] 2.1 Add a test asserting a test-run-category entry in the details modal's Logs pane renders with a leading `├─ ` before its status emoji, and pass `cargo test -p agentmon`.
- [x] 2.2 Add a test asserting an agent-category entry in the details modal's Logs pane renders with no tree prefix, and pass `cargo test -p agentmon`.
- [x] 2.3 Add a test with two consecutive test-run entries (e.g. "started" then "failed") between two agent entries, asserting both test-run rows get their own `├─ ` prefix and the agent rows remain unprefixed, and pass `cargo test -p agentmon`.
- [x] 2.4 Extend (or add alongside) the existing `details_modal_logs_pane_shows_pid_for_agent_activities_but_not_test_runs`-style test to confirm the tree prefix and the pid suffix coexist correctly on an agent row, and pass `cargo test -p agentmon`.

## 3. Verification

- [x] 3.1 Run the full workspace test suite (`cargo test`) and confirm no regressions in the top-level Logs tab (which must remain unprefixed) or other Logs-pane snapshot-style tests.
