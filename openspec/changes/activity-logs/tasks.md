## 1. Protocol: activity log types

- [x] 1.1 Add `LogEntry { working_dir, category, status, occurred_at_ms }` and `LogCategory { Agent, TestRun }` to `agentmon-proto`, with serde derives matching existing message types, and verify the crate builds
- [x] 1.2 Add `logs: Vec<LogEntry>` to `ServerMessage::Snapshot` and a new `ServerMessage::LogAppended { entry: LogEntry }` variant, and verify `agentmon-proto`'s existing (de)serialization tests still pass alongside new ones covering the new field/variant

## 2. Daemon: capture and retain activity log entries

- [x] 2.1 Add a `VecDeque<LogEntry>` field (capped at `MAX_LOG_ENTRIES = 500`, FIFO eviction from the front) to `Ingestor` in `crates/agentd/src/ingest.rs` (implemented as a dedicated `ActivityLog` type in `crates/agentd/src/activity_log.rs`, owned by `Ingestor`)
- [x] 2.2 In `Ingestor::ingest_event`, append a `LogEntry` (category `Agent`) whenever `should_notify()` is true for "done" or "needs input", using the same condition that triggers `notifier.notify(...)`; verify with a unit test asserting a `RecordingNotifier`-style log double receives entries exactly when the notifier does, including the repeated-needs-input case, and not for declined/stale
- [x] 2.3 In `Ingestor::ingest_test_run`, append a `LogEntry` (category `TestRun`) for every started/passed/failed event, matching `notifier.notify_test_run(...)`; verify with a unit test
- [x] 2.4 Implement global FIFO eviction once the log exceeds 500 entries; verify with a unit test that appends 501 events across multiple working directories and asserts the oldest (by occurrence, regardless of directory) is gone and the log length stays at 500

## 3. Daemon: serve the log over the socket

- [x] 3.1 Include the current log (chronological order) in the `Snapshot` sent on `Subscribe`, in `crates/agentd/src/server.rs`; verify with an integration test (following the existing `unique_socket_path`/real-`UnixStream` pattern) that a subscribing client's snapshot contains previously-appended entries
- [x] 3.2 Add a `Log` arm to the `Update`/`Broadcaster` enum and broadcast `ServerMessage::LogAppended` to connected clients whenever `Ingestor` appends an entry; verify with an integration test that a connected client receives the push after a new notification-worthy event

## 4. TUI: tab and selection state

- [x] 4.1 Add `Tab { Agents, Logs }`, `active_tab`, `agents_selected: usize`, and a `logs: Vec<LogEntry>` buffer to `App` in `crates/agentmon/src/app.rs`, populated from `Snapshot.logs` and appended to on `LogAppended`; verify with unit tests covering initial population and incremental append
- [x] 4.2 Handle `Tab`, `A`, and `L` in `crates/agentmon/src/input.rs` to switch `active_tab`; verify with unit tests for cycling and direct-jump behavior
- [x] 4.3 Handle `j`/`down`/`k`/`up` on the Agents tab to move `agents_selected` within the current project row count, clamped at both ends; verify with unit tests

## 5. TUI: project details modal

- [x] 5.1 Add `Modal { Details(String), Help }` and `modal: Option<Modal>` to `App`; opening on `Enter` (Agents tab, selected row's working dir) and closing on `Esc`, verified with unit tests
- [x] 5.2 Render the details modal with Agents/Tests/Logs panes, reusing `directory_groups()` for the Agents/Tests panes and filtering `App.logs` by working directory (most recent first) for the Logs pane, in `crates/agentmon/src/ui.rs`
- [x] 5.3 Render "no test run" / "no activity" placeholders in the Tests/Logs panes when empty, and verify by starting the app against a project with no test run and no logged activity (see plan for manual verification in section 7)

## 6. TUI: Logs tab list, sort, filter, pagination

- [x] 6.1 Render the Logs tab as a table of all entries (time, project, category, status), default-sorted most-recent-first, in `ui.rs`; verify with a unit test on the sort/render data transform
- [x] 6.2 Add sort-mode state (Recency/Project/Status) and a keybinding to cycle it, applying the corresponding ordering to the rendered list; verify with unit tests for each sort mode
- [x] 6.3 Add filter state (by Project, by Status) and keybindings to set/clear them, restricting the rendered list; verify with unit tests for each filter
- [x] 6.4 Add pagination state (page size derived from terminal height) and handle `j`/`down`, `k`/`up` (one line) and `d`/PageDown, `u`/PageUp (one page) on the Logs tab list; verify with unit tests covering line movement, page movement, and clamping at list boundaries (implemented as a fixed `LOGS_PAGE_SIZE` constant rather than a terminal-height-derived size - see design.md)

## 7. TUI: help modal and manual verification

- [x] 7.1 Open the help modal on `?` listing all shortcuts (Tab/A/L, j/k/d/u, Enter, Esc, ?, q); verify with a unit test that `?` sets `modal = Some(Help)` and `Esc` clears it
- [x] 7.2 Manually run the TUI against a live `agentd` (per the run skill / manual-testing memory: never bind to the default production socket path) and confirm: tab switching, row selection, opening/closing the details modal, Logs tab pagination/sort/filter, and the help modal all behave as specced
