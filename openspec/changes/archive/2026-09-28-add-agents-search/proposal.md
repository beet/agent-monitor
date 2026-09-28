## Why

With several projects in flight, the Agents tab's project list must be scanned by eye to find one project among many. The user wants to type a partial project name (e.g. `LAB-1234` or just `1234`) and have the list filter down to matching rows as they type, the same way `/` search works in vim-like tools.

## What Changes

- Add a `/`-triggered inline search editor to the Agents tab: pressing `/` turns the tab's title into a live text-entry prompt with a visible cursor.
- Each keypress while editing updates the search string and immediately re-filters the Agents tab's rows by substring match against each row's project name (derived from its working directory).
- `Enter` commits the in-progress search string as the tab's applied filter, closes the editor, and the Agents tab title shows the applied filter in a distinct bold color so an active filter is obvious at a glance.
- Re-entering edit mode (`/`) on a tab that already has an applied filter starts from that filter's text, not empty; `Escape` while editing discards in-progress edits and restores whatever filter (if any) was applied before editing began.
- Applying an empty search string (via `Enter` on an empty prompt) clears the filter.
- The Agents tab title shows a `[/]` keyboard shortcut hint, matching the hint convention already used by the Logs and Reminders tabs' own controls.
- The existing project-row sort order (most recent update first) continues to apply among the rows that match the filter; a newly-tracked agent whose project matches the current filter appears in the list like any other filtered update.
- This filter applies only to the Agents tab - the Logs and Reminders tabs are unaffected and keep their own independent filter state.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the Agents tab gains a `/`-driven, incremental, substring search filter over project name with its own inline title-editing UI, applied and cancelled per the interaction rules above.

## Impact

- `crates/agentmon/src/app.rs`: new `App` state for the in-progress edit buffer and the applied Agents-tab filter string, plus an editing-mode flag; a filtering step added to whatever currently produces the Agents tab's row list.
- `crates/agentmon/src/input.rs`: new key handling for `/`, character input, `Backspace`, `Enter`, and `Escape` while the Agents tab's search editor is active, gated so it does not clash with existing Agents-tab or global key bindings.
- `crates/agentmon/src/ui.rs`: Agents tab title rendering grows an edit-mode (prompt + cursor) and an applied-filter (bold, distinct color) presentation, alongside the new `[/]` hint.
- No daemon (`agentd`), wire protocol (`agentmon-proto`), or `agentmon-report` changes - this is TUI-local, client-side filtering of data the daemon already sends.
