## Context

See proposal.md - Why. Two facts about the existing code shape this design:

- `LogEntry` (`crates/agentmon-proto/src/lib.rs`) is a point-in-time snapshot: it already captures `reminder_name` at the moment of the event rather than pointing back at the live `ReminderInfo`, specifically so a later-deleted or later-renamed reminder's past entries keep showing what was true when they were recorded.
- `crates/agentmon/src/ui.rs` already has `format_reminder_eta(run_started_ms, duration_minutes) -> String` (`HH:MM`, local time), used by the Reminders tab, the Agents tab's Reminders column, and the details modal's Reminders pane. The Logs tab and the details modal's Logs pane both format entries through the same `log_entry_status_line`/`log_status_cell_text_and_style` functions in that file, so one change to shared code reaches both places "logs appear."

## Goals / Non-Goals

**Goals:**
- Show a "started" reminder log entry's ETA in both places it renders (Logs tab, details modal's Logs pane) via the shared formatting path, so they can't drift apart.
- Keep the ETA a fixed, point-in-time fact matching the existing `reminder_name` snapshot pattern, not a value that changes if the reminder is edited later.

**Non-Goals:**
- Recomputing or live-updating a "started" entry's ETA after the fact (e.g. if the reminder's duration is edited while running). The log is a history, not a live view; the live ETA is already available in the Reminders tab.
- Adding ETA to any other log entry status (stopped/finished) or category (agent/test-run) - "finished"/"stopped" already show elapsed duration, which serves the same "how is this reminder doing" purpose for a completed event.
- Sending a notification when a reminder starts (out of scope - reminders don't notify on start today, only on natural completion).

## Decisions

**Store the due time on the LogEntry itself, computed once at start time, rather than deriving it at render time from a live `ReminderInfo`.**
Rendering needs the reminder's duration as of the moment it started, not its current duration - `ingest_start_reminder` already captures `reminder.name` this way for the same reason (a later edit or delete shouldn't rewrite history). Add `pub reminder_due_at_ms: Option<u64>` to `LogEntry`, alongside `reminder_name`: `Some` only for reminder-category "started" entries, `None` for every other entry (matching how `reminder_name` is `None` for non-reminder categories and `pid` is `None` for non-agent categories).

Alternative considered: store `duration_minutes` instead of a precomputed absolute timestamp, and derive `due_at_ms` at render time as `occurred_at_ms + duration_minutes * 60_000`. Rejected as a needless extra step - `ingest_start_reminder` already has both numbers on hand (it builds the entry right after `self.reminders.start(id)`, which returns the reminder with `run_started_ms` and `duration_minutes` already set for this run), so storing the already-computed absolute timestamp is simpler at both ends and matches how `run_started_ms` itself is stored as an absolute timestamp elsewhere in the wire types.

**Render the ETA as `HH:MM` local time via the existing `format_reminder_eta` computation, not by adding a new formatting helper.**
`format_reminder_eta` takes `(run_started_ms, duration_minutes)` and returns the due time; since the log entry now carries the due time directly as `reminder_due_at_ms`, the renderer needs only the existing local-time formatting logic, not the addition step. Extract that formatting into a small helper (e.g. take an already-computed `due_at_ms: u64` and return `HH:MM`), used both by `format_reminder_eta` (still needed for the live Reminders-tab views) and by the new "started" log-entry rendering, so the two never format an ETA differently.

**Put the ETA text where `log_entry_status_line` currently appends duration, guarded by status == "started" instead of the existing completion-only guard.**
`log_entry_status_line` already appends `: {reminder_name}` for any reminder entry and then a duration for completion statuses via `log_completion_duration_ms`. Add a third step: for a reminder entry whose `status == "started"` and `reminder_due_at_ms` is present, append `, ETA: {eta}` after the name - matching the `"Running {elapsed}, ETA: {eta}"` phrasing already used in `reminder_status_cell_text_and_style`, so the same word order and punctuation appear whether the reminder is being viewed live or in the log.

## Risks / Trade-offs

- [Wire format change] Adding a field to `LogEntry` changes what's serialized in `Snapshot`/`LogAppended` messages. → Additive `Option<T>` field, `serde`-compatible; no version negotiation exists in this protocol today (agentd and agentmon ship together), so this follows the same pattern as every prior field addition to these wire types.
- [Diagram drift] README.md's data model `classDiagram` for `LogEntry` will be stale the moment this field is added. → Tasks includes updating the diagram, per this repo's CLAUDE.md instructions.
