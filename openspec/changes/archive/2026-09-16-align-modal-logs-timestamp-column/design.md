## Context

`render_details_modal`'s Logs pane (`crates/agentmon/src/ui.rs`) currently builds a `ratatui::widgets::Paragraph` from a `Vec<Line>`, where each `Line` is `[Span::styled(status_text, style), Span::raw("  " + timestamp)]`. Because `Paragraph`/`Line` lay out spans back-to-back with no independent column tracking, the timestamp's horizontal position varies with the length of `status_text` (which itself varies: entry category, status label, optional duration, optional `pid N` suffix). See proposal.md for why this doesn't satisfy "far right column, like the Agents list."

The Logs tab and Agents tab already solve exactly this alignment problem using `ratatui::widgets::Table`, whose `Constraint`-based column widths guarantee every row's Nth cell starts at the same horizontal offset regardless of the other cells' content.

## Goals / Non-Goals

**Goals:**
- Every rendered entry's timestamp starts at the same horizontal position, at the far right of the Logs pane's inner area.
- No header row is introduced (unchanged constraint from the prior change).
- Keep the existing per-entry text (status label, emoji, duration, pid suffix) unchanged - only its column position changes.

**Non-Goals:**
- No change to the Logs tab table (already correct).
- No change to sort order, filtering, or which entries are shown.
- No change to the Agents/Tests panes.

## Decisions

**Render the Logs pane as a `Table` with two columns, header omitted.** Column 1 (`Constraint::Fill(1)`) holds the existing styled status/category/duration/pid text; column 2 (`Constraint::Length(19)`, matching the Logs tab's and Agents tab's own TIME/UPDATED width) holds the timestamp. Omitting the `.header(...)` call (rather than passing an empty header) keeps the pane exactly as header-less as it is today - `Table` only reserves a header row when one is supplied.

Alternative considered: keep `Paragraph`/`Line` and manually right-pad each line with spaces so the timestamp lands at a fixed offset from the pane's rendered width. Rejected because it requires re-implementing the width-aware column math (including truncation when the pane is too narrow, and recomputing on every resize) that `Table`'s constraint solver already provides, and it would diverge from the pattern the Logs tab and Agents tab already use for the same problem - using `Table` here instead keeps all three "timestamp column" implementations consistent.

**Empty-state row.** When a project has no logged activity, render a single `Table` row with the placeholder text ("No activity") in column 1 and an empty column 2, rather than falling back to a separate `Paragraph`. This keeps one rendering path for the pane instead of two, and the existing scenario ("Logs pane indicates there is no activity") only requires the text to appear, not a specific widget.

## Risks / Trade-offs

- [Very long status/pid text could now be clipped by the `Fill(1)` column instead of wrapping onto a new line, since `Table` cells don't wrap the way `Paragraph` did] → Existing entries are short single-line labels (emoji + a few words + optional `pid N`); this matches how the Logs tab already renders the same text in a fixed-width `Fill` column today, so no new truncation risk is introduced.
- [Existing tests assert on the exact text produced by `log_entry_line_with_pid`, not on `Table` vs `Paragraph` structure] → these should keep passing unchanged since the assertions check for substrings in the rendered buffer, not widget type; verify during implementation.
