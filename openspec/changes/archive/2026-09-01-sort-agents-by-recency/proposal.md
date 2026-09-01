## Why

`agentmon`'s agent table currently shows rows in whatever order the daemon reports them (insertion order for new agents, snapshot order on reconnect), with no indication of when an agent last changed. A user with several agents running has to scan the whole table to find which one just finished or needs input, instead of the most recently active agent simply being at the top.

## What Changes

- The agent table sorts rows by `last_updated_ms` descending, so the most recently updated agent is always the top row.
- The table gains a column showing each agent's last-updated time in the system's local timezone, formatted `%Y-%m-%d %H:%M:%S` (e.g. `2026-09-01 16:32:07`), with no UTC offset or timezone code shown, derived from `last_updated_ms`.
- Sort order re-evaluates on every snapshot and incremental update, so a row moves to the top the moment its status changes.
- Row selection/highlighting (`App.selected`, the up/down/`j`/`k` keybindings, and the `TableState` highlight) is removed. Nothing in the TUI currently acts on a selected row, and keeping it would mean tracking which agent is "selected" through re-sorts that can move it anywhere in the list on any update - dead weight for a feature nothing uses yet. Quitting is unaffected.

## Capabilities

### Modified Capabilities
- `agent-monitor-tui`: the "Live agent list" requirement changes to specify that displayed agents are sorted by most recent update (descending) and that each row shows a last-updated timestamp in the system's local timezone, formatted `%Y-%m-%d %H:%M:%S`. The "Navigation and quit do not affect tracked agents" requirement narrows to drop row selection, since the TUI no longer has anything to select.

## Impact

- `crates/agentmon/src/app.rs`: `App` needs `ui.rs` to compute agents in recency order rather than raw insertion/snapshot order; `selected` field and `select_next`/`select_previous`/`clamp_selection` are removed.
- `crates/agentmon/src/input.rs`: up/down/`j`/`k` key handling is removed; only quit keys remain.
- `crates/agentmon/src/ui.rs`: `render_agent_table` gains a timestamp column, iterates agents in sorted order, and drops `TableState` selection/highlighting.
- Existing tests in `app.rs`, `input.rs`, and `ui.rs` that assert on row order, selection, or content need updates; new tests cover sort order and timestamp formatting.
