## Why

The Logs tab currently orders its columns PROJECT, CATEGORY, STATUS, TIME, and gives PROJECT a 2x width-weight over STATUS's 1x. Project names are typically short (e.g. "PR-1234 Change requests"), so PROJECT often reserves more width than it needs while STATUS - which carries the longer status/duration text and, for agent and prompt entries, a trailing prompt snippet - gets squeezed and truncated first. Having PROJECT as the leftmost column also means that when the details modal opens (a centered overlay covering 80% of the terminal), the narrow strip still visible to its left shows project names rather than the status/prompt text the user more often wants a peek at while the modal is open.

## What Changes

- Swap the Logs tab's column order so STATUS is leftmost, followed by CATEGORY, then PROJECT, then TIME (CATEGORY and TIME keep their relative position; only PROJECT and STATUS trade places).
- Size the PROJECT column to the widest project display name among the currently displayed entries (never narrower than its own "PROJECT" header), taking a user-assigned custom project name into account the same as a directory-derived default name - rather than a fixed proportional share of the table width.
- Give the STATUS column the width left over once PROJECT, CATEGORY, and TIME have what they need, so STATUS - not PROJECT - gets priority for extra room and truncates last, not first.
- Cap PROJECT at half of the width available to PROJECT and STATUS combined, so one unusually long custom project name can't squeeze STATUS down to nothing; content that doesn't fit is clipped without an ellipsis, consistent with how the Agents tab's PROJECT column already handles an oversized name.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: the "Logs tab shows an aggregated, paginated activity list" requirement changes its column order (STATUS, CATEGORY, PROJECT, TIME) and its width-allocation behavior (PROJECT sized to content, STATUS receiving the leftover space, capped to protect STATUS from an oversized project name).

## Impact

- `crates/agentmon/src/ui.rs`: `render_logs_tab`'s header `Row`, its `widths` array, and the per-entry `Row` construction (currently `PROJECT, CATEGORY, STATUS, TIME` with `Fill(2)`/`Fill(10)`/`Fill(1)`/`Length(22)`) need their column order and width computation reworked, following the content-based sizing pattern (`content_need`, `str_width`, `truncate_line`) that `render_agent_table` already uses for its own PROJECT column.
- Existing tests in `crates/agentmon/src/ui.rs` that assert the Logs tab's column order, header text, or STATUS-column-width-dependent truncation (e.g. the narrow/wide STATUS-column snippet tests) will need their expectations updated to match the new order and width rules.
- Scoped to the top-level Logs tab's table only - the details modal's Logs pane (which has no PROJECT column, since it's already scoped to one project) is unaffected.
