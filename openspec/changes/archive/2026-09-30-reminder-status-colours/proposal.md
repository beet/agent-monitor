## Why

In the Agents tab's REMINDERS column, only a reminder's leading emoji is colored (blue while running, green while done); the reminder's name and its duration/ETA/outcome text render in the terminal's default foreground color. Every other status cell in the TUI (agent statuses, test-run statuses, and reminder statuses in the Logs tab, the details modal's Logs pane, the Reminders tab, and the details modal's Reminders pane) colors its *entire* status text as one styled span, so a runing/done reminder in the Agents tab reads visually inconsistent with the same reminder shown elsewhere, and doesn't visually match the blue/green a scanning user already associates with "running" and "done/passed" elsewhere in the same tab.

## What Changes

- In the Agents tab's REMINDERS column, style a running reminder's entire segment (emoji, name, and elapsed/ETA text) in the same Blue used for a running test run, instead of coloring only the emoji.
- In the Agents tab's REMINDERS column, style a done reminder's entire segment (emoji, name, and elapsed/outcome text) in the same Green used for a done agent or a passed test run, instead of coloring only the emoji.
- No change to emoji markers (⏳/✅ are already correct), to the Reminders tab, to the details modal's Reminders pane, or to the Logs tab/Logs pane - those already color their full reminder text and are out of scope.

## Capabilities

### Modified Capabilities
- `agent-monitor-tui`: the "Status is visually distinguishable" requirement gains an explicit scenario for the Agents tab's REMINDERS column: a reminder's full status segment (not just its emoji) SHALL be colored to match the corresponding agent/test-run color (Blue for running, Green for done).

## Impact

- `crates/agentmon/src/ui.rs`: `reminder_agents_column_segment` (builds the Agents tab's per-reminder segment) currently colors only its emoji `Span`; the name and trailing text spans carry no color. This is the only code path affected - `reminder_status_cell_text_and_style` (used by the Reminders tab and the modal's Reminders pane) and the Logs-entry coloring already style their full text and need no change.
