## 1. Selected-row styling

- [x] 1.1 Revert `SELECTED_ROW_BG` in `crates/agentmon/src/ui.rs` from `Color::Magenta` to `Color::Blue`, and add a `SELECTED_ROW_FG` constant set to `Color::White`
- [x] 1.2 Apply `SELECTED_ROW_FG` via `.fg(...)` alongside `.bg(SELECTED_ROW_BG)` on both `row_highlight_style` call sites (the Agents table and the Logs table), and verify by running the app and confirming every status renders in white text on a blue background when its row is selected

## 2. Tests

- [x] 2.1 Update `a_running_status_stays_legible_on_the_default_selected_row` to assert the selected row's foreground equals `SELECTED_ROW_FG`, and verify with `cargo test -p agentmon --lib ui::`
- [x] 2.2 Update `selected_row_uses_a_background_fill_rather_than_reversed_video` to assert the selected cell's foreground equals `SELECTED_ROW_FG` instead of the status's own color, and verify with `cargo test -p agentmon --lib ui::`
- [x] 2.3 Remove `no_status_foreground_color_matches_the_row_highlight_background`, since the selected row no longer renders any status's own foreground color for that invariant to protect
- [x] 2.4 Fix `needs_input_is_visually_distinguished_from_other_statuses`, `declined_is_visually_distinguished_from_other_statuses`, and `logs_tab_status_column_is_colored_by_status` so they compare colors on rows that aren't the default selection (adding a more-recently-updated anchor row/entry), and match status text by whole-word substring rather than a single ambiguous character, and verify with `cargo test -p agentmon --lib ui::`
- [x] 2.5 Run `cargo test --workspace` and `cargo clippy -p agentmon --all-targets` and confirm no failures or new warnings
