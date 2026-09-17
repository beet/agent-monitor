## 1. Shared pagination model

- [x] 1.1 Add a `Paginator`-style type in `crates/agentmon/src/app.rs` with line-move, overlap-by-1 page-move, and clamping logic parameterized by an explicit `page_size`, and verify with unit tests covering: single-line move at both ends, a full page-down/page-up with the 1-row overlap, and clamping at the first/last page when fewer than a page's worth of rows remain
- [x] 1.2 Replace `logs_selected`'s ad hoc `move_logs_selection`/`page_logs` free functions with the shared type, remove the fixed `LOGS_PAGE_SIZE` constant, and update existing tests (`page_logs_moves_by_a_full_page_clamped`, `d_and_u_page_the_logs_list`, and any other `LOGS_PAGE_SIZE` references) to pass an explicit page size, verifying `cargo test -p agentmon` passes
- [x] 1.3 Add a second paginator instance scoped to the details modal's Logs pane, reset whenever `open_details_modal` runs, and verify with a unit test that opening the modal for a different project resets its selection to the top

## 2. Modal keyboard routing

- [x] 2.1 In `crates/agentmon/src/input.rs`, extend the modal branch of `handle_key` to route `j`/`k`/`d`/`u`/`PageDown`/`PageUp` to the modal's Logs-pane paginator when a `Modal::Details` is open, leaving `Modal::Help` unaffected, and verify with a test that these keys move the modal's own selection
- [x] 2.2 Verify with a test that pressing these keys while the modal is open does not change the Logs tab's `logs_selected`/page state underneath (extending the existing `non_esc_keys_are_ignored_while_a_modal_is_open`-style coverage)

## 3. Rendering: Logs tab

- [x] 3.1 In `crates/agentmon/src/ui.rs`'s `render_logs_tab`, compute `page_size` from the rendered inner area's height and pass it into the shared paginator for `TableState`'s selection instead of relying on `TableState`'s own auto-scroll, verified by a snapshot/text-buffer test showing the visible rows match the current page
- [x] 3.2 Render a Ratatui vertical `Scrollbar` along the pane's right edge when entries exceed one page, and verify with a test that it's present when `entries.len() > page_size` and absent otherwise
- [x] 3.3 Append a `d`/`u` pagination hint to the Logs tab's heading (via `logs_controls_hint` or alongside it) only when entries exceed one page, and verify with a text-buffer test asserting the hint's presence/absence in both cases

## 4. Rendering: details modal's Logs pane

- [x] 4.1 In `render_details_modal`, switch the Logs pane from a plain `Table` to a stateful one driven by the modal's paginator (computing `page_size` from the pane's rendered inner height), rendering the same row-highlight style used by the Logs tab, verified by a text-buffer test asserting the selected row is highlighted
- [x] 4.2 Render a Ratatui vertical `Scrollbar` along the Logs pane's right edge when its entries exceed one page, and verify with a test that it's present/absent matching entry count vs. page size
- [x] 4.3 Append a `d`/`u` pagination hint to the Logs pane's "Logs" title only when its entries exceed one page, and verify with a text-buffer test asserting the hint's presence/absence in both cases
- [x] 4.4 Verify with a test that a project whose logs all fit on one page shows no selection highlight, no scrollbar, and no pagination hint (matching current no-scroll appearance)

## 5. Help modal and docs

- [x] 5.1 Update `render_help_modal`'s shortcut list in `crates/agentmon/src/ui.rs` to note that `d`/`u`/PgDn/PgUp and `j`/`k` also apply within the details modal's Logs pane, and verify by reading the rendered help text in a test
- [x] 5.2 Re-read `README.md`'s "How it works" section for any TUI keyboard-shortcut documentation referencing the old fixed-page-size behavior and update it to describe the new page-size-follows-terminal-height and 1-row-overlap behavior, if such documentation exists — none exists (the section covers only the agent-status state diagram), so nothing needed updating

## 6. Full verification

- [x] 6.1 Run `cargo test -p agentmon` and `cargo clippy -p agentmon` and confirm both pass clean
- [x] 6.2 Manually run the TUI against a non-default, isolated `agentd` per the project's manual-testing convention, generate enough log activity to overflow a page in both the Logs tab and a project's details modal, and confirm pagination, the scrollbar, the heading hint, and modal keyboard precedence all behave as specced — verified via an isolated `agentd`/`agentmon` (HOME override) driven inside a scratch Zellij session; found and fixed a real title-truncation bug along the way (see commit/summary)
