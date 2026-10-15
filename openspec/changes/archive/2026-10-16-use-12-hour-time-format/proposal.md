## Why

The TUI currently renders every clock time (the Agents/Logs/Reminders tabs' "UPDATED" columns, and reminder ETAs) in 24-hour format (`%H:%M:%S` / `%H:%M`). The user wants these presented in 12-hour format with an AM/PM marker instead, which is the more familiar convention for everyday reading.

## What Changes

- The "UPDATED" column timestamp (Agents tab, Logs tab, Reminders tab, and the details modal's Logs pane) changes from `YYYY-MM-DD HH:MM:SS` (24-hour) to `YYYY-MM-DD h:MM:SS AM/PM` (12-hour, no leading zero on the hour, uppercase AM/PM) - e.g. `2026-10-16 2:05:30 PM` instead of `2026-10-16 14:05:30`.
- Every reminder ETA display (the Reminders tab's Status column, the Agents tab's Reminders column, the details modal's Reminders pane, and a "started" log entry's recorded ETA as shown in the Logs tab/pane) changes from `HH:MM` (24-hour) to `h:MM AM/PM` (12-hour) - e.g. `2:05 PM` instead of `14:05`.
- This is a display-only change: no stored timestamp, wire format, or daemon-side data changes - only how `agentmon`'s TUI formats a time for on-screen display.
- There is no new user-facing setting to switch between 12-hour and 24-hour display; this change makes 12-hour the only display format, consistent with there being no existing config/preferences mechanism in the project.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the "UPDATED" column's timestamp format and every reminder ETA format change from 24-hour to 12-hour with an AM/PM marker, across the Agents tab, Logs tab, Reminders tab, and the details modal's Logs and Reminders panes.

## Impact

- `crates/agentmon/src/ui.rs`: `format_last_updated` (currently `%Y-%m-%d %H:%M:%S`) and `format_eta_ms`/`format_reminder_eta` (currently `%H:%M`) change their `chrono` format strings to 12-hour equivalents with AM/PM.
- Tests in `crates/agentmon/src/ui.rs` that independently hardcode `"%Y-%m-%d %H:%M:%S"` to build expected strings need updating to the new format (or de-duplicated to call the real formatter instead of re-deriving the pattern).
- No daemon, wire-protocol (`agentmon-proto`), or persisted-data changes - times are stored as epoch milliseconds throughout and only the final display formatting changes.
