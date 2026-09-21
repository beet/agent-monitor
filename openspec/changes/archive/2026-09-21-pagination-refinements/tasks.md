## 1. Paginator jump-to-start/end

- [x] 1.1 Add `Paginator::jump_to_start(&mut self, len: usize, page_size: usize)` and `Paginator::jump_to_end(&mut self, len: usize, page_size: usize)` in `crates/agentmon/src/app.rs`, matching `move_by`/`page`'s existing clamping (`jump_to_end`'s `top` uses the same `len.saturating_sub(page_size)` clamp as `page`'s `max_top`); verify with unit tests covering: empty list (no-op), single-page list (no-op scroll, selects first/last row), multi-page list (selects first/last entry and scrolls to first/last page), and repeated `jump_to_end`/`jump_to_start` calls being idempotent.

## 2. Wire up `g`/`G` keybindings

- [x] 2.1 Add `KeyCode::Char('g')` / `KeyCode::Char('G')` arms to `handle_logs_tab_key` in `crates/agentmon/src/input.rs`, calling the new `Paginator` methods on `app.logs_pagination`; verify by running the TUI against a daemon with enough logged activity to overflow one page and confirming `g`/`G` jump the Logs tab's selection and scrollbar to the first/last entry.
- [x] 2.2 Add the same `g`/`G` arms to `handle_modal_key`'s details-modal branch in `crates/agentmon/src/input.rs`, calling the new `Paginator` methods on `app.modal_logs_pagination`; verify by opening the details modal for a project with enough activity to overflow the Logs pane and confirming `g`/`G` work there, and that pressing them while the modal is open does not move the Logs tab's own selection underneath.

## 3. Fix the scrollbar's last-page bug

- [x] 3.1 In `render_pagination_scrollbar` (`crates/agentmon/src/ui.rs`), change `ScrollbarState::new(len)` to `ScrollbarState::new(len.saturating_sub(page_size) + 1)`, keeping `.position(top)` and `.viewport_content_length(page_size)` as-is; verify with a unit or snapshot test (or manual check) that on a list long enough to require multiple pages, the rendered scrollbar's thumb reaches the track's last cell when `top` is at its maximum (last page) and the track's first cell when `top` is 0 (first page).
- [x] 3.2 Manually verify in the running TUI, for both the Logs tab and the details modal's Logs pane, that paging (`d`/`u`) or jumping (`g`/`G`) to the last page moves the scrollbar thumb flush to the bottom of its track, and to the first page moves it flush to the top.

## 4. Update hint and help text

- [x] 4.1 Update `logs_controls_hint` in `crates/agentmon/src/ui.rs` to append the `g`/`G` hint (e.g. `"Page [d/u]  Top/Bottom [g/G]"`) alongside the existing `Page [d/u]` text, only when `needs_pagination` is true; verify by checking the Logs tab's title text in a terminal wide enough not to truncate it.
- [x] 4.2 Update the details modal's inline Logs-pane title hint (`crates/agentmon/src/ui.rs`, near the `"Logs  |  Page [d/u]"` literal) the same way; verify by opening the details modal on a project with a multi-page Logs pane and checking its pane title.
- [x] 4.3 Add a `g / G` line to the help modal's shortcut list in `render_help_modal` (`crates/agentmon/src/ui.rs`), phrased like the existing `d / PgDn` line; verify by pressing `?` in the running TUI and checking the new line appears.

## 5. Spec conformance

- [x] 5.1 Run `openspec validate pagination-refinements --strict` and fix any reported issues.
- [x] 5.2 Run the crate's existing test suite (`cargo test -p agentmon`) and confirm it passes with the new `Paginator` unit tests included.
