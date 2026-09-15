## Why

The Agents and Logs tables' selected-row background was changed from blue to magenta so that no status's own foreground color could ever collide with it (most notably "running"'s blue, which the default-selected row hit as soon as a single agent was tracked). Magenta reads as an alarm color and doesn't match the rest of the TUI's palette. Forcing the selected row's foreground to a single readable color instead of leaving each status's own color in place removes the need to hand-pick a background that avoids every current and future status color, so blue can come back.

## What Changes

- Restore the Agents and Logs tables' selected-row background from magenta to blue.
- Force the selected row's text to a single white foreground, overriding every status's own color (agent statuses: running, idle, needs input, done, stale, declined; test-run statuses: started, passed, failed) rather than letting each status's color show through selection.
- Drop the "no status color may match the row-highlight background" constraint, since the selected row no longer renders any status's own foreground color at all.

## Capabilities

### Modified Capabilities
- `agent-monitor-tui`: the selected-row requirement changes from "preserve each status's own color, so the highlight background must avoid all of them" to "force a single foreground color on the selected row, overriding every status's color."

## Impact

- `crates/agentmon/src/ui.rs`: `SELECTED_ROW_BG` constant reverts to `Color::Blue`; a new `SELECTED_ROW_FG` (`Color::White`) is added and applied via `.fg(...)` on both tables' `row_highlight_style`, alongside the existing `.bg(...)`.
- Tests in the same file that asserted a status's own color survives selection, or that no status color equals the highlight background, are updated to assert the selected row is forced to white instead.
- No protocol, daemon, or CLI changes.
