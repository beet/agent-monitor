## Why

Pressing `Enter` in the reminder creation form before typing a duration submits the empty duration field as `0`. The daemon treats a 0-minute reminder as already elapsed, so it immediately transitions to done and fires the completion notification the instant the reminder is created - surprising behavior for what the user experienced as an accidental, not-yet-configured submission. A reminder's duration should always be a positive number of minutes, whether entered in the TUI or sent by any client.

## What Changes

- The daemon rejects `CreateReminder` and `UpdateReminder` requests whose `duration_minutes` is `0`, leaving the project's reminders unchanged, instead of creating/updating a reminder that completes instantly.
- The TUI's reminder creation/editing form validates that the Duration field is a positive number before submitting: pressing `Enter` with the field empty or `0` shows an inline error and keeps the form open, the same way the project-rename form already rejects a blank name in place rather than sending it to the daemon.

## Capabilities

### Modified Capabilities
- `reminders`: the "Reminder identity and lifecycle" requirement (which governs creation) and the "Editing a reminder" requirement both gain the constraint that `duration_minutes` must be positive, with creation/edit requests carrying a non-positive duration rejected rather than applied.

## Impact

- `crates/agentd/src/reminders.rs`: `ReminderRegistry::create` and `ReminderRegistry::update` currently accept any `u32` duration, including `0`; both need a positive-duration check.
- `crates/agentmon/src/app.rs`: `submit_reminder_form` currently parses the Duration field with `.unwrap_or(0)` and always returns a message; it needs the same validate-and-keep-form-open pattern `submit_rename_form` uses for a blank name, which means `ReminderForm` gains an `error` field (mirroring `RenameForm::error`).
- `crates/agentmon/src/ui.rs`: the reminder form's rendering needs to display that new inline error, the same way the rename form's error is already rendered.
- Existing test fixtures/tests that construct a 0-minute reminder purely to exercise "elapsed time reaches duration instantly" (e.g. `sweep_transitions_a_reminder_whose_duration_has_elapsed` in `crates/agentd/src/reminders.rs`) need to use a small positive duration instead, now that `0` is rejected at creation.
