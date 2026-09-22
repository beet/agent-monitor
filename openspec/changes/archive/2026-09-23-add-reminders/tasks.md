## 1. Wire protocol (`agentmon-proto`)

- [x] 1.1 Add `ReminderId(String)` and `ReminderStatus` (`NotYetStarted | Running | Done`, snake_case serde) to `crates/agentmon-proto/src/lib.rs`, and verify with a round-trip JSON test for each new type
- [x] 1.2 Add `ReminderInfo` (`id`, `cwd`, `name`, `duration_minutes`, `status`, `created_at_ms`, `run_started_ms: Option<u64>`, `last_updated_ms`) and verify with a round-trip JSON test
- [x] 1.3 Add `LogCategory::Reminder` and a `reminder_name: Option<String>` field on `LogEntry`, and verify `log_category_serializes_as_snake_case` and the existing `LogEntry` round-trip tests are extended to cover it
- [x] 1.4 Add `ClientMessage` variants `CreateReminder { cwd, name, duration_minutes }`, `UpdateReminder { id, name, duration_minutes }`, `DeleteReminder { id }`, `StartReminder { id }`, `StopReminder { id }`, each with a round-trip JSON test
- [x] 1.5 Add `reminders: Vec<ReminderInfo>` to `ServerMessage::Snapshot`, plus `ServerMessage::ReminderUpdate { reminder }` and `ServerMessage::ReminderRemoved { id }`, each covered by a round-trip JSON test; update the existing `server_message_snapshot_round_trips_through_json` test to include reminders

## 2. Daemon: reminder registry (`agentd`)

- [x] 2.1 Create `crates/agentd/src/reminders.rs` with a `ReminderRegistry` (`Clone`, `Arc<Mutex<..>>`-backed like `Registry`) supporting create/update/delete/start/stop/snapshot, and a unit test that creating a reminder registers it as `NotYetStarted` (spec: "Creating a reminder registers it as not-yet-started")
- [x] 2.2 Enforce the 100-reminders-per-project cap in `create`, returning `None`/an error the caller drops rather than panicking, with a unit test asserting the 101st creation is rejected and the existing 100 are unchanged (spec: "Cap reached")
- [x] 2.3 Implement start (fresh or re-run) and stop, with unit tests for: starting a never-run reminder, re-running a done reminder (fresh start time, no accumulation from the prior run), and stopping a running reminder (status -> Done, elapsed computed, no notification side effect at this layer)
- [x] 2.4 Implement edit (name/duration, any status) and delete (any status, including running, removed immediately), with unit tests for both, including that editing a running reminder's duration changes its due time without resetting `run_started_ms`
- [x] 2.5 Wire `ReminderRegistry` into `Ingestor` (or equivalent) alongside the existing `Registry`, and verify with a test that an `Ingestor`-level create/start/stop round-trips through the registry

## 3. Daemon: completion sweep

- [x] 3.1 Implement `sweep_once`/`spawn_reminder_sweep` in `reminders.rs`, structurally matching `liveness.rs`: find every `Running` reminder where `now - run_started_ms >= duration_minutes`, transition to `Done`, and return the changed reminders
- [x] 3.2 Unit test: a running reminder whose duration has elapsed is swept to `Done` with elapsed time equal to its duration (spec: "A reminder finishes with no client connected")
- [x] 3.3 Unit test: sweeping twice does not re-report an already-`Done` reminder (mirrors `sweep_does_not_repeat_already_stale_agents`)

## 4. Daemon: notifications and activity log integration

- [x] 4.1 Add a reminder-completion notification to `notify.rs` using the `Hero` system sound, identifying the project and reminder name, and verify with a `RecordingNotifier`-style unit test that it fires exactly once per natural completion and not on manual stop
- [x] 4.2 Append activity log entries for reminder `started` (fresh or re-run), `stopped`, and `finished`, each recording `category: Reminder` and the reminder's current name in `reminder_name`; verify with unit tests matching the activity-log spec's new scenarios, including that editing a reminder appends no entry
- [x] 4.3 Verify a deleted reminder's previously-logged entries are unaffected (their `reminder_name` stays populated) with a unit test that deletes a reminder after it has logged activity and re-reads the log

## 5. Daemon: server wiring

- [x] 5.1 Add `Update::Reminder(ReminderInfo)` and `Update::ReminderRemoved(ReminderId)` to the `Broadcaster`'s `Update` enum in `server.rs`, mapped to `ServerMessage::ReminderUpdate`/`ReminderRemoved` in the subscriber loop
- [x] 5.2 Handle the five new `ClientMessage` reminder variants in `handle_connection`, applying each to the `ReminderRegistry` via `Ingestor` and broadcasting the result, following the existing `ReportEvent`/`ReportTestRun` fire-and-forget pattern
- [x] 5.3 Include the current reminders in the `Subscribe` snapshot response
- [x] 5.4 Spawn the reminder completion sweep in `serve()` alongside the existing liveness sweep, reusing the same `liveness_interval`
- [x] 5.5 Integration test (matching the style of `server.rs`'s existing end-to-end tests): create a reminder over one connection, start it, stop it, and assert a subscriber receives the expected sequence of `ReminderUpdate`/`LogAppended` messages
- [x] 5.6 Integration test: a reminder started with a very short duration (e.g. via a test-only short interval) completes on its own, and the subscriber receives a `ReminderUpdate` to `Done` plus a notification call, without any client sending a stop

## 6. TUI: daemon client (`agentmon`)

- [x] 6.1 Add `ClientEvent::ReminderUpdate(ReminderInfo)` and `ClientEvent::ReminderRemoved(ReminderId)` in `client.rs`, forwarded from the corresponding `ServerMessage` variants, and extend `ClientEvent::Snapshot` to carry reminders; update existing snapshot tests and add new ones for the reminder push messages
- [x] 6.2 Add a helper that opens a short-lived connection and sends one of the five reminder `ClientMessage`s (mirroring how a hook event or test-run report would be sent), with a test asserting the message reaches a mock daemon

## 7. TUI: application state (`app.rs`)

- [x] 7.1 Add `Reminders` as a third top-level tab with its own selection/pagination/sort/filter state, and apply incoming `ReminderUpdate`/`ReminderRemoved`/snapshot events to an in-memory reminder list, keyed by `ReminderId`
- [x] 7.2 Implement Reminders tab sorting (recency by last completed run, falling back to creation time when none - see design.md; Project; Status) and filtering (Project, Status), with unit tests for each sort/filter mode
- [x] 7.3 Compute each project's Reminders-column content for the Agents tab (running reminder's live duration/ETA, or most recent done reminder's outcome/duration; bullet-joined when multiple), with unit tests for zero/one/multiple-reminder projects
- [x] 7.4 Add details-modal state: a `Reminders` pane reminder list scoped to the modal's project, a `ModalFocus` (Logs | Reminders) toggled by `Tab`, defaulting to Logs when opened from the Agents/Logs tab and to Reminders (with the invoking reminder pre-selected) when opened from the Reminders tab
- [x] 7.5 Add reminder form state (create vs. edit mode, Name/Duration fields, active field) and delete-confirmation-dialog state, with unit tests for opening pre-populated in edit mode and empty in create mode

## 8. TUI: input handling (`input.rs`)

- [x] 8.1 Add `R`/`r` as a top-level tab-jump key for Reminders, alongside the existing `A`/`a` and `L`/`l`, and extend `Tab` to cycle through all three tabs; update/extend existing tab-cycling tests
- [x] 8.2 Add Reminders tab key handling: `j`/`k`/`down`/`up`, `d`/`u`/page-down/page-up, `g`/`G`, `o` (sort), `p` (project filter), `f` (status filter), `c` (clear filters), `s` (start/stop), `Enter` (open modal, Reminders pane focused); test that `s` does not cycle the status filter there (unlike the Logs tab)
- [x] 8.3 Add modal `Tab` handling to toggle focus between the Logs pane and Reminders pane, and gate the Reminders pane's `j`/`k`/`d`/`u`/`g`/`G`/`Enter`/`s`/`e`/`d`/`Delete`/`R` keys on it currently holding focus; test that these keys are inert when the Logs pane holds focus
- [x] 8.4 Wire the reminder form's `Tab` (switch field), `Enter` (save), and `Escape` (discard/cancel) keys, and the delete confirmation dialog's `Enter` (confirm)/`Escape` (dismiss) keys

## 9. TUI: rendering (`ui.rs`)

- [x] 9.1 Render the Reminders tab table (Project, Name, Duration, Status, Updated columns) with blank/running/done status formatting per spec, its scrollbar and pagination hint once it overflows a page, and the `f`-filter hint in its heading
- [x] 9.2 Render the Agents tab's new Reminders column, including ellipsis truncation for both it and the existing Agents column when content overflows the available width
- [x] 9.3 Update the details modal layout to a three-way Agents/Tests/Reminders top split (each one third of the top third) and render the Reminders pane (reminder rows, `New [R]` hint instead of a pagination hint, no category-word prefix)
- [x] 9.4 Render focus-aware highlighting so exactly one of the Logs pane and Reminders pane shows a selected row at a time, matching the currently-focused pane
- [x] 9.5 Render reminder log entries in the Logs tab and the modal's Logs pane with the fixed ⏰ marker (regardless of started/stopped/finished) and no tree-branch prefix, with the "reminder" category word in the Logs tab/pane and omitted in the Reminders tab/pane
- [x] 9.6 Render the reminder creation/editing form modal (Name/Duration fields, active-field highlight) and the bold red delete-confirmation dialog
- [x] 9.7 Extend the keyboard shortcuts help modal (`?`) to list the new Reminders-related shortcuts

## 10. Documentation

- [x] 10.1 Update the README's data model `classDiagram` for the new/changed `agentmon-proto` types (`ReminderInfo`, `ReminderStatus`, `ReminderId`, the new `ClientMessage`/`ServerMessage` variants, `LogCategory::Reminder`, `LogEntry.reminder_name`), per `CLAUDE.md`'s diagram-sync requirement

## 11. End-to-end verification

- [x] 11.1 Manually run `agentd` and `agentmon` together (per the project's isolated-`HOME` manual testing convention) and walk the full flow: create a reminder, start it, watch its ETA count up on the Reminders tab and the Agents tab, let it finish naturally and confirm the notification and log entry, then create and manually stop another one and confirm no notification fires
- [x] 11.2 Manually verify editing a running reminder's duration changes its ETA immediately, and that deleting a reminder leaves its past log entries showing its name
