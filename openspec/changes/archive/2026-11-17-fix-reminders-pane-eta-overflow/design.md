## Context

`render_reminders_pane` (`crates/agentmon/src/ui.rs`, ~line 1213) currently builds a `ratatui` `Table` with three `Constraint`s - `Fill(2)` for name, `Length(9)` for the configured duration, `Fill(2)` for status - and truncates only the status `Cell` (via `truncate_line`, which truncates from the end of a line and appends `...`) to whatever width `Layout::horizontal` resolves for that column. `reminder_status_cell_text_and_style` (~line 1683) builds that status text, e.g. `⏳ Running 1m23s, ETA: 2:05 PM`. See proposal.md for why this overflows on narrow terminals.

## Goals / Non-Goals

**Goals:**
- Replace the three-`Cell` row with a single flowing line of text per reminder, so there are no padded column boundaries.
- Always keep the configured duration and status (including a running reminder's ETA) fully visible; truncate the name instead when the row doesn't fit.
- Drop "Running" from the running-status text.
- Right-align a reminder's status (ending in its ETA, when running) flush with the pane's right edge, expanding the gap after the duration to reach it, instead of a single fixed space.

**Non-Goals:**
- No change to the Reminders tab or the Agents tab's REMINDERS column - both already size themselves to content and weren't reported as overflowing.
- No change to `Done` status wording, color rules, emoji markers, or the pane's pagination/selection/title behavior.
- No attempt to guarantee full visibility when the pane is narrower than the duration+status text alone (e.g. a handful of columns) - truncating the name down to nothing still can't shrink the duration/status text itself; this is the same physical limit every other truncation rule in the file already accepts.

## Decisions

- **Single-column `Table`, not a `List`**: keep using `ratatui::Table` (with one `Constraint::Fill(1)` "column") rather than switching widgets, so the existing `TableState`-based selection, highlight styling (`row_highlight_style`), and pagination (`app.modal_reminders_pagination`) keep working unchanged. Only the `widths` array and the per-row `Row::new([...])` construction change.
- **Build `left` (name + duration) and `right` (status) separately, then pad or truncate between them**: for each reminder, compute `left = "{name} {duration_minutes}m"` and `right` = the existing status spans (now without "Running" for `Running`; empty for `NotYetStarted`). When `right` is non-empty, the padding between them is `max(1, inner_width - display_width(left) - display_width(right))` spaces, so `right`'s last character lands on the pane's rightmost column whenever there's slack width; when `right` is empty, `left` renders with nothing after it. This guarantees the status (and a running reminder's ETA at its end) is never the part that gets cut, regardless of how narrow the pane is, and sits flush right whenever there's room to spare.
- **Truncate the name only once padding can't reach even 1 space**: if `display_width(left) + 1 + display_width(right)` exceeds `inner_width`, shrink `left`'s name portion - via `truncate_line(Line::from(Span::raw(name)), name_budget)`, where `name_budget = inner_width - 1 - display_width(" {duration_minutes}m") - display_width(right)` - so the row still ends with the duration, a single space, then `right` flush against the pane's right edge. `truncate_line` already does the display-width-aware budgeting (reserving 3 columns for `...`) this needs; no new truncation logic required. Its existing `max_width == 0` → "no limit" fallback is left as-is (same behavior every other caller already relies on), accepted as the pathological-narrow-terminal edge case called out above.
- **Drop "Running" only for the modal's call site, via a new parameter**: `reminder_status_cell_text_and_style` turns out to be shared between `render_reminders_pane` (the modal) *and* `render_reminders_tab` (the top-level Reminders tab) - not separate functions, as this document originally assumed. Since the proposal scopes the word removal to the modal's Reminders pane only, add a `drop_running: bool` parameter to `reminder_status_cell_text_and_style`; its `Running` branch formats `"{emoji} {elapsed}, ETA: {eta}"` when `true`, `"{emoji} Running {elapsed}, ETA: {eta}"` when `false`. `render_reminders_pane` passes `true`; `render_reminders_tab` passes `false`, preserving its current text unchanged. (The Agents-tab column, `reminder_agents_column_segment`, remains a genuinely separate function and is unaffected either way.)
- **Styling stays span-based, not row-based**: the name and configured-duration spans keep the table's default (unstyled) look, matching current behavior; only the status portion keeps its existing `Style` (blue while running, green while done). This matches the "Non-Goals" above - no new color/emoji rules.

## Risks / Trade-offs

- [A reminder name that's always truncated to a stub on narrow terminals reads worse than before] → Accepted trade-off per the user's explicit request: the ETA is the information worth protecting; the name was already truncated some of the time before this change, just truncated at the wrong end of the row.
- [Existing tests assert the old three-`Cell` structure, the literal "Running" text, and status-side truncation] → These are expected to fail until updated; tasks.md calls out the specific tests to rewrite.
