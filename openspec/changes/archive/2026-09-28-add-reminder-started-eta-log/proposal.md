## Why

A reminder's "started" log entry currently shows only its name, with no indication of when it's due. Everywhere else a running reminder's status is shown (the Reminders tab, the Agents tab's Reminders column, the details modal), an ETA is already displayed alongside it. The "started" log entry is the odd one out - a user scanning the Logs tab or the details modal's Logs pane has to cross-reference the Reminders tab to know when a just-started reminder will fire.

## What Changes

- A reminder's "started" log entry additionally records the reminder's due time (its ETA), captured at the moment it starts - a point-in-time fact, consistent with how the entry already captures the reminder's name as of that event.
- The Logs tab and the details modal's Logs pane, which both render log entries through the same shared formatting, display that ETA (in the system's local timezone, `HH:MM`, matching the existing ETA format used elsewhere for reminders) as part of a "started" reminder entry's text.
- No other log entry category or status gains an ETA; a "started" entry's ETA is not recomputed or updated later, even if the reminder's duration is subsequently edited while running.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities
- `activity-log`: a reminder-category "started" log entry additionally records the reminder's due time (ETA) as of that event.
- `agent-monitor-tui`: the Logs tab and the details modal's Logs pane render a "started" reminder entry's ETA alongside its name.

## Impact

- `crates/agentmon-proto/src/lib.rs`: `LogEntry` gains a field carrying the reminder's due time, populated only for reminder-category "started" entries.
- `crates/agentd/src/ingest.rs`: `ingest_start_reminder` populates the new field when building the "started" log entry.
- `crates/agentmon/src/ui.rs`: `log_entry_status_line` (shared by the Logs tab and the details modal's Logs pane) renders the ETA for a "started" reminder entry.
- README.md's data model diagram needs updating for the `LogEntry` field addition, per this repo's `CLAUDE.md`.
