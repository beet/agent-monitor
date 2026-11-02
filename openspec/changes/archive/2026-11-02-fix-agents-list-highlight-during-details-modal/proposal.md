## Why

The Agents tab's selection cursor is a bare index into a list that gets re-sorted by recency on every update. When an agent or test run for a *different* project updates while the details modal is open, that re-sort can shift which project sits at the previously-selected index. The modal itself stays correctly pinned to the project it was opened for (it's keyed by working directory, not index), but the Agents-tab highlight silently drifts to a different row underneath it - so closing the modal leaves the wrong project highlighted, with no indication to the user that this happened.

## What Changes

- While the details modal is open, the Agents tab's selection (`agents_selected`) stays pinned to the same project the modal is showing, even as background updates re-sort the underlying list.
- Each time the list is re-sorted (on snapshot, per-agent update, agent removal, or test-run update) while the modal is open, the selection index is re-resolved to wherever that project's row now sits, rather than merely clamped to the new list length.
- If the modal's project is ever filtered out of the visible list entirely (e.g. search applied oddly, or the project has no more tracked members) the selection falls back to length-based clamping as before - there's no row to pin to.
- No change to behavior when the modal is closed: `agents_selected` continues to clamp by length only, as today.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: the "Agents tab supports row selection" requirement gains a constraint that the selected row must continue to identify the project shown in an open details modal, even if a background list re-sort would otherwise move a bare index to a different project.

## Impact

- `crates/agentmon/src/app.rs`: `clamp_agents_selected` (currently length-only) and its callers (`apply_snapshot`, `apply_update`, `remove_agent`, `apply_test_run_update`) need to re-resolve `agents_selected` against the modal's pinned `cwd` (from `Modal::Details(cwd)`) when a details modal is open.
- No change to `Modal::Details`'s own id-based lookups (`project_logs`, `project_reminders`, etc.) - those are already correct.
- No wire-protocol, daemon, or spec changes outside `agent-monitor-tui`.
