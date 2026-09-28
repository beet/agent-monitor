## 1. Wire type

- [x] 1.1 Add `pub reminder_due_at_ms: Option<u64>` to `LogEntry` in `crates/agentmon-proto/src/lib.rs`, documented as populated only for reminder-category "started" entries; update `sample_log_entry`/other test fixtures that construct `LogEntry` so the crate still compiles, and verify `cargo test -p agentmon-proto` passes
- [x] 1.2 Add/extend a round-trip JSON test for a "started" reminder `LogEntry` asserting `reminder_due_at_ms` serializes and deserializes correctly, and verify it passes

## 2. Daemon

- [x] 2.1 In `crates/agentd/src/ingest.rs`, populate `reminder_due_at_ms` in `ingest_start_reminder` as `reminder.run_started_ms.unwrap_or_else(now_ms) + reminder.duration_minutes as u64 * 60_000`, leaving it `None` on the "stopped" and "finished" entries built elsewhere in that file
- [x] 2.2 Add/extend a unit test in `crates/agentd/src/ingest.rs` asserting a freshly-started reminder's log entry carries the expected `reminder_due_at_ms`, and verify `cargo test -p agentd` passes
- [x] 2.3 Add a unit test asserting a "stopped" or "finished" reminder log entry's `reminder_due_at_ms` is `None`, and verify it passes

## 3. TUI rendering

- [x] 3.1 In `crates/agentmon/src/ui.rs`, extract the `due_at_ms -> "HH:MM"` local-time formatting out of `format_reminder_eta` into a small helper, and have `format_reminder_eta` call it with `run_started_ms + duration_minutes * 60_000`; verify existing ETA-related tests (e.g. the ones asserting `"ETA:"` text) still pass
- [x] 3.2 In `log_entry_status_line`, append `, ETA: {eta}` (using the new helper) after the reminder name when `entry.category == Reminder`, `entry.status == "started"`, and `entry.reminder_due_at_ms` is `Some`
- [x] 3.3 Add a unit test asserting the Logs tab's rendered text for a "started" reminder entry includes its ETA in `HH:MM` form
- [x] 3.4 Add a unit test asserting a "stopped" or "finished" reminder entry's rendered text is unchanged (still shows elapsed duration, not an ETA)
- [x] 3.5 Manually verify in the running TUI (per `agentmon-manual-testing` practice: isolated `HOME`, run `agentd`/`agentmon`) that starting a reminder shows its ETA in both the Logs tab and the details modal's Logs pane

## 4. Docs

- [x] 4.1 Update README.md's `LogEntry` entry in the "Data model" `classDiagram` to add the new `reminder_due_at_ms` field, per this repo's CLAUDE.md instructions on keeping that diagram in sync
