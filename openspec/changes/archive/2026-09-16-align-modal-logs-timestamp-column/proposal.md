## Why

The recently-shipped fix for the details modal's Logs pane moved the timestamp after the status text, but only as inline text separated by two spaces - the timestamp still sits directly next to each entry's text, at a position that shifts left or right depending on how long that entry's status text is. The user asked for the timestamp to sit in its own column at the far right of the pane, aligned the same way on every row, mirroring how the Agents list's UPDATED column is a fixed-width column at a consistent horizontal position regardless of the other cells' content.

## What Changes

- Change the details modal's Logs pane so every entry's timestamp renders at the same fixed horizontal position - the far right of the pane - regardless of that entry's status/category text length, instead of trailing immediately after variable-length text.
- No table headers are added to this pane (unchanged from the prior change - the pane still isn't meant to gain a header row).
- No change to the Logs tab table (already a real table with a fixed-width trailing TIME column - this only affects the modal's Logs pane, which is a plain text list).
- No change to the data shown, sort order, or any other pane.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: The "Project details modal" requirement's Logs pane behavior is refined - the timestamp SHALL occupy a fixed-width column at the far right of the pane, not merely trail the entry's text at a variable position.

## Impact

- Affected code: `crates/agentmon/src/ui.rs` - `render_details_modal`'s Logs pane construction, currently a `Paragraph` of `Line`s with the timestamp appended as trailing text. Achieving fixed-position alignment on every row likely means either rendering this pane as a `Table` (like the Logs tab and Agents tab, with no header) or right-padding the timestamp to the pane's inner width - the concrete approach is a design decision.
- Affected tests: existing details-modal Logs pane tests in `crates/agentmon/src/ui.rs` that assert on entry text, which may need updated assertions to check for a fixed-position timestamp rather than adjacent text.
- No changes to wire format, daemon behavior, the Logs tab, or other panes.
