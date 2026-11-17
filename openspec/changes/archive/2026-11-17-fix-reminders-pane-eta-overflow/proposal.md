## Why

The details modal's Reminders pane lays out each row as three fixed-width columns (name, configured duration, status). On anything narrower than a large display, a running reminder's status text - `⏳ Running <elapsed>, ETA: h:MM AM/PM` - doesn't fit the status column's share of the width, so it gets truncated with a trailing `...`. Because the ETA sits at the end of that text, it's exactly what gets cut off, defeating the reason the pane shows an ETA at all.

## What Changes

- Remove the Reminders pane's three-column, even-width table layout; render each row as one flowing line instead.
- Drop the word "Running" from a running reminder's status text (e.g. `⏳ 1m23s, ETA: 2:05 PM` instead of `⏳ Running 1m23s, ETA: 2:05 PM`).
- Keep showing the reminder's configured duration (e.g. `10m`) immediately after its name, separated by a single space, for every status - unchanged in content, just no longer padded into its own column.
- Right-align each row's status text (and, for a running reminder, its ETA) to the pane's right edge, expanding the gap after the duration as needed to reach it, instead of a single fixed space there - so the status doesn't just trail loosely after the duration on a wide pane.
- When a row's full content (name + configured duration + status) doesn't fit the pane's width even with that gap collapsed to a single space, truncate the reminder's *name* (with a trailing `...`) instead of the trailing status/ETA text, so the duration and status - including a running reminder's ETA - are always fully visible.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: the "Project details modal" requirement's Reminders-pane row formatting changes - single-line-per-row instead of three even-width columns, "Running" dropped from the running-status text, the status right-aligned to the pane's right edge via an expanding gap after the duration, and truncation now applies to the reminder's name instead of its trailing status/ETA text.

## Impact

- `crates/agentmon/src/ui.rs`: `render_reminders_pane` (currently ~line 1213) drops its `Table`/`widths`/`Layout::horizontal` column-width split in favor of building one `Line` per row from a `name + duration` left part and a `status` right part, padding between them to right-align the status to the pane's inner width; `reminder_status_cell_text_and_style`'s `Running` branch (currently ~line 1693) drops the `Running ` word from its formatted text; a new or adapted truncation helper budgets the name's width against the rest of the row's already-known width, reusing the same display-width-aware truncation `truncate_line` already does, but applied to the name instead of the row's tail.
- Existing tests in `crates/agentmon/src/ui.rs` asserting the pane's three-column layout, the literal `"Running"` text, or status-text truncation (e.g. around `details_modal_reminders_pane_shows_the_duration_after_the_name` and `details_modal_reminders_pane_truncates_a_running_reminders_status_instead_of_overflowing`) need updating to match the new row text and truncation target.
- Scoped to the details modal's Reminders pane only - the top-level Reminders tab and the Agents tab's REMINDERS column keep their own existing status text (including the word "Running") and layout, since neither was reported to have this overflow problem.
