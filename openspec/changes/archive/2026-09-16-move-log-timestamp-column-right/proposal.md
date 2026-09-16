## Why

The Logs tab's table and the details modal's Logs pane both show their timestamp first (leftmost), while the Agents tab shows its equivalent timestamp column (UPDATED) last (rightmost). This inconsistency makes the Logs views harder to scan against the Agents tab's established pattern, where the more identifying/actionable columns (project, category, status) lead and the timestamp trails.

## What Changes

- Reorder the Logs tab table's columns from `TIME, PROJECT, CATEGORY, STATUS` to `PROJECT, CATEGORY, STATUS, TIME`, matching the Agents tab's convention of trailing timestamp.
- Reorder the details modal's Logs pane text lines so the timestamp trails the status/category text instead of leading it. This pane has no table headers and none are being added.
- No change to the data shown, formatting of the timestamp itself, sorting, or filtering behavior - only column/text position.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: The "Logs tab shows an aggregated, paginated activity list" requirement's column order changes so the timestamp is the last column rather than the first.

## Impact

- Affected code: `crates/agentmon/src/ui.rs` - `render_logs_tab` (header array, row `Cell` order, `widths` array) and `render_details_modal`'s Logs pane construction (the `Line` built from `format_last_updated` and the status text).
- Affected tests: any existing UI tests in `crates/agentmon/src/ui.rs` that assert on column order or text position for the Logs tab or the details modal's Logs pane.
- No changes to wire format, daemon behavior, or other tabs/panes.
