## 1. Fix connection ordering in `agentmon-report`

- [x] 1.1 Split `report::send_event` into a `connect` step (opens the `UnixStream`, sets write timeout) and a `send_on`-style step (writes the already-built `ClientMessage::ReportEvent`), and update `main.rs::run_report` to call `connect` immediately (before reading stdin), then read/parse stdin and build the event, then write it on the already-open stream - dropping the connection without writing if `build_event`/parsing yields nothing. Verify with a unit test asserting the connection is opened before stdin is read (e.g. a slow/blocking stdin reader paired with a mock listener that records accept time vs. a channel signal from the read step), and that existing `report.rs` tests (`sent_event_is_received_by_a_mock_daemon_listener`, `sending_with_no_daemon_listening_fails_fast`) still pass unmodified in behavior.
- [x] 1.2 Add/update a test confirming that when stdin parsing fails or produces no trackable event after connecting, the connection closes without sending a message and without panicking or leaking the socket.
- [x] 1.3 Add a daemon-side test in `crates/agentd/src/server.rs` (or `ingest.rs` if more appropriate) confirming a connection that is accepted but closes before sending any message still releases its `EventSequencer` ticket (reuses/extends existing malformed-payload coverage - no new behavior expected here, just confirming the existing path covers "connected then closed early" the same as "sent malformed bytes").

## 2. Add the missing space between a prompt entry's marker and its snippet

- [x] 2.1 Update `log_entry_status_line` (`crates/agentmon/src/ui.rs`) so the prompt-category branch inserts a single space between the 🗣️ marker and the prompt snippet. Verify with a unit test asserting the built text matches `"🗣️ <snippet>"` (not `"🗣️<snippet>"`).

## 3. Make prompt snippets use the real column/pane width

- [x] 3.1 Change `prompt_snippet` (`crates/agentmon/src/ui.rs`) to take a `max_width: usize` parameter instead of using the fixed `PROMPT_SNIPPET_MAX_CHARS` constant, truncating by display width (reusing `truncate_line`'s width-measurement approach) rather than raw char count, and remove `PROMPT_SNIPPET_MAX_CHARS`. Update all call sites (`log_entry_status_line`'s prompt-category and agent-category branches) and existing tests to pass an explicit width.
- [x] 3.2 In `render_logs_tab`, resolve the STATUS column's actual rendered width the same way the details modal's Logs pane already does (`Layout::horizontal(widths).split(inner)` or equivalent, accounting for the table's selection-symbol reservation if applicable), and thread that width into `log_entry_status_line`/`prompt_snippet` when building each row's status text, replacing today's width-blind fixed-constant truncation.
- [x] 3.3 In the details modal's Logs pane rendering, pass its already-resolved `status_width` into `log_entry_status_line`/`prompt_snippet` (currently computed but not reaching the snippet step), so its snippet truncation is driven by the same real width `truncate_line` already uses there.
- [x] 3.4 Add/update unit tests asserting that a prompt-category entry's (and an agent entry's last-prompt snippet's) rendered text grows to use more of a wider resolved column width and still gets an ellipsis only when it overflows that width, for both the Logs tab and the details modal's Logs pane.

## 4. Spec validation

- [x] 4.1 Run `openspec validate --strict --change fix-prompt-log-ordering-and-truncation` and resolve any reported issues.
- [x] 4.2 Run the full workspace test suite (`cargo test --workspace`) and confirm it passes.
