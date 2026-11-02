## 1. Daemon-side validation

- [x] 1.1 In `crates/agentd/src/reminders.rs`, make `ReminderRegistry::create` return `None` without inserting anything when `duration_minutes == 0`, and update its doc comment to mention the new rejection case
- [x] 1.2 In the same file, make `ReminderRegistry::update` return `None` without mutating the reminder when `duration_minutes == 0`
- [x] 1.3 Update `sweep_transitions_a_reminder_whose_duration_has_elapsed` (and any other test constructing a 0-minute reminder to exercise instant completion) to use a small positive duration together with a `run_started_ms` far enough in the past that the elapsed time still reaches the duration, and verify `cargo test -p agentd reminders::` passes
- [x] 1.4 Add unit tests: `create` rejects a 0-minute duration and leaves the project's existing reminders unchanged; `update` rejects setting an existing reminder's duration to 0 and leaves that reminder unchanged

## 2. Client/server wiring

- [x] 2.1 Confirm (via `crates/agentd/src/server.rs` and `ingest.rs`, wherever `CreateReminder`/`UpdateReminder` are handled) that a `None` from the registry is already treated as "request dropped, no state change, nothing pushed to clients" consistent with how the per-project cap rejection and unknown-id edits are already handled; add a test if that path isn't already covered
- [x] 2.2 Run `cargo test -p agentd` and confirm all tests pass

## 3. TUI form validation

- [x] 3.1 In `crates/agentmon/src/app.rs`, add an `error: Option<String>` field to `ReminderForm`, mirroring `RenameForm::error`, initialized to `None` everywhere a `ReminderForm` is constructed
- [x] 3.2 Change `submit_reminder_form` to borrow the form instead of taking it (`self.reminder_form.as_ref()?`), parse the Duration field, and if it is empty or parses to `0`, set `self.reminder_form.as_mut().unwrap().error` to an explanatory message (e.g. "Duration must be greater than 0") and return `None` without closing the form or sending a message - mirroring `submit_rename_form`'s blank-name rejection
- [x] 3.3 Only on successful validation, take the form (`self.reminder_form = None`), clearing any previous error, and build the `CreateReminder`/`UpdateReminder` message as today
- [x] 3.4 In `crates/agentmon/src/ui.rs`, update `render_reminder_form` to render `form.error` the same way `render_rename_form` renders `RenameForm::error`
- [x] 3.5 Update the existing test that currently asserts `Enter` on an empty Duration field sends `CreateReminder { duration_minutes: 0, .. }` so it instead asserts the form stays open with an inline error and no `InputAction::SendReminderCommand` is produced
- [x] 3.6 Add a test asserting that typing a positive duration after a rejected empty submission, then pressing `Enter` again, submits successfully and clears the error
- [x] 3.7 Run `cargo test -p agentmon` and confirm all tests pass, including the reminder-form tests

## 4. Manual verification

- [x] 4.1 Run the TUI, open the reminder creation form, press `Enter` with the Duration field empty, and confirm an inline error appears and no reminder is created; then type a positive duration and confirm `Enter` creates it normally
