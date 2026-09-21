## Why

Paginated lists in the TUI (the Logs tab and the details modal's Logs pane) only support line-at-a-time (`j`/`k`) and page-at-a-time (`d`/`u`) movement - jumping to either end of a long list still takes many keypresses. Separately, their shared scrollbar helper computes `ScrollbarState`'s `content_length` from the raw entry count rather than the number of valid scroll positions, so the thumb can never reach the track's bottom edge on the last page, misleading the user about how much list remains below.

## What Changes

- Add vim-style `g` (jump to first entry) and `G` (jump to last entry) keybindings to every paginated list (Logs tab, details modal's Logs pane), consistent with the existing `j`/`k`/`d`/`u` bindings.
- Fix `render_pagination_scrollbar` (`crates/agentmon/src/ui.rs`) so the scrollbar thumb reaches the track's bottom when the list's last page is showing, by deriving `ScrollbarState`'s `content_length` from the number of valid scroll-window positions instead of the raw entry count.
- Update the keyboard-shortcut hints and help modal text for both paginated lists to mention `g`/`G`.

## Capabilities

### Modified Capabilities
- `agent-monitor-tui`: the "Paginated lists support keyboard navigation" requirement gains `g`/`G` jump-to-start/end behavior; the Logs tab and details modal Logs pane scrollbar scenarios are tightened to require the thumb reach the track's end on the last page.

## Impact

- `crates/agentmon/src/input.rs`: add `g`/`G` match arms to `handle_logs_tab_key` and `handle_modal_key`.
- `crates/agentmon/src/app.rs`: add jump-to-start/end methods to (or reuse existing methods on) `Paginator`, invoked for both `App::logs_pagination` and `App::modal_logs_pagination`.
- `crates/agentmon/src/ui.rs`: fix `render_pagination_scrollbar`'s `ScrollbarState` construction; update pagination hint text and the help modal's shortcut list.
- `openspec/specs/agent-monitor-tui/spec.md`: delta to the "Paginated lists support keyboard navigation", "Logs tab shows an aggregated, paginated activity list", and "Project details modal" requirements.
