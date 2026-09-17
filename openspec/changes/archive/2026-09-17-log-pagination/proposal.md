## Why

The Logs tab's `d`/`u` paging currently just jumps the selection cursor by a fixed 10 rows and leans on Ratatui's default minimal auto-scroll to keep it in view - there's no real page-window concept, no scrollbar, and no on-screen hint that paging exists. The details modal's Logs pane is worse off: it renders every entry for a project in one unscrollable table with no selection at all, so a project with more activity than fits in the pane simply has entries cut off with no way to reach them. Both need genuine, discoverable pagination.

## What Changes

- Logs tab and the details modal's Logs pane both gain real page-windowed scrolling: a page is however many rows currently fit in the pane (dynamic, not the fixed `LOGS_PAGE_SIZE` constant), and `d`/`PageDown`/`u`/`PageUp` move by a full page that overlaps the previous page by 1 row, snapping the selection to the new page's top row.
- The details modal's Logs pane gains its own row selection and highlight, matching the Logs tab's, plus its own `j`/`k`/`d`/`u`/`PageDown`/`PageUp` handling scoped to the modal - which the input layer already isolates from the tab underneath, but today the modal has no list keys at all to isolate.
- Both panes show a Ratatui vertical `Scrollbar` on the right edge whenever their entries exceed one page.
- Both panes' headings show `d`/`u` pagination hints whenever their entries exceed one page - alongside the Logs tab's existing sort/filter hints, and alone in the modal's Logs pane (which has no sort/filter controls).
- `App`'s paging logic takes `page_size` as a parameter (computed by the UI layer from the pane's actual rendered height) instead of assuming a fixed constant, so it stays testable without a real terminal while reflecting what's really on screen.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: the "Logs tab shows an aggregated, paginated activity list", "Project details modal", and "Paginated lists support keyboard navigation" requirements change - real page-windowed scrolling with 1-row overlap, a scrollbar, pagination hints in both the Logs tab and the details modal's Logs pane, and modal-scoped keyboard handling for the modal's own Logs pane.

## Impact

- `crates/agentmon/src/app.rs`: paging/selection state and logic for both the Logs tab and the modal's Logs pane; `page_size` becomes a caller-supplied parameter rather than the fixed `LOGS_PAGE_SIZE` constant.
- `crates/agentmon/src/input.rs`: modal key handling gains `j`/`k`/`d`/`u`/`PageDown`/`PageUp` for the modal's own Logs pane, still isolated from the tab underneath.
- `crates/agentmon/src/ui.rs`: `render_logs_tab` and `render_details_modal`'s Logs pane gain scrollbar rendering, pagination hints in their headings, and page-height-aware windowing.
- `README.md`: the state diagram is unaffected, but any TUI keyboard-shortcut documentation should note the modal's own paging shortcuts.
