## Why

The agentmon TUI's PROJECT column is a fixed 20 characters wide, so project names longer than that are cut off mid-word with no indication of truncation (`claude-labmaster-14s`, `lab-1002-a-21-sectio`). The fixed width was likely chosen to leave room for long status lines, but on typical terminal widths there is plenty of unused space and truncation is now the more common case than overflow.

## What Changes

- Replace the PROJECT column's fixed `Constraint::Length(20)` with a width that scales with the available terminal width, so project names get more room on wider terminals instead of always capping at 20 characters.
- Keep the STATUS column able to claim the majority of extra space, since status segments (multiple agent statuses plus a test-run segment) are typically the widest content in a row.
- Preserve the UPDATED column's fixed width (unchanged).
- On narrow terminals where full names still don't fit, names continue to be clipped by the table renderer (existing behavior, not a new truncation indicator).

## Capabilities

### Modified Capabilities
- `agent-monitor-tui`: the PROJECT column's width is no longer a fixed 20-character cap; it scales with available terminal width instead.

## Impact

- Affected code: `crates/agentmon/src/ui.rs` (`render_agent_table`'s `widths` array).
- No protocol, daemon, or data model changes. Purely a TUI layout adjustment.
