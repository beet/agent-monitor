## 1. Convert the details modal's Logs pane to a fixed-column layout

- [x] 1.1 In `render_details_modal` (`crates/agentmon/src/ui.rs`), replace the Logs pane's `Paragraph`/`Vec<Line>` construction with a `ratatui::widgets::Table` of two columns - the existing styled status/category/duration/pid text (from `log_entry_line_with_pid`) in column 1, and `format_last_updated(entry.occurred_at_ms)` in column 2 - using widths `[Constraint::Fill(1), Constraint::Length(19)]`, matching the Logs tab's own TIME column width
- [x] 1.2 Do not call `.header(...)` on the new `Table`, so no header row is rendered, and confirm no `TableState`/selection is introduced (this pane has no selectable rows, unlike the Logs tab)
- [x] 1.3 For the empty-activity case, render a single row with the "No activity" placeholder in column 1 and an empty column 2, instead of a separate `Paragraph` fallback
- [x] 1.4 Keep the surrounding `Block` (title "Logs", rounded borders) unchanged

## 2. Verification

- [x] 2.1 Run `cargo test -p agentmon` and confirm all existing tests pass, updating any assertion that depended on the previous two-space-separated inline text if needed
- [x] 2.2 Add or extend a test that renders two details-modal Logs pane entries with differently-lengthed status text and asserts their timestamps start at the same column offset in the rendered buffer, so a regression back to variable-position trailing text is caught
- [x] 2.3 Visually confirm (via a rendered `TestBackend` buffer, as used previously for this same pane) that timestamps line up in a straight column at the far right of the pane across multiple entries of different text lengths
