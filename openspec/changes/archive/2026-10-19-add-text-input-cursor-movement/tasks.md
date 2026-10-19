## 1. Add the dependency

- [x] 1.1 Add `tui-input` to `crates/agentmon/Cargo.toml` with its `crossterm` feature (matching the project's pinned `crossterm = "0.29.0"`), run `cargo build -p agentmon`, and verify it compiles with no version conflicts in `Cargo.lock`.

## 2. Agents-tab search cursor

- [x] 2.1 Change `App.agents_search_buffer` from `String` to `tui_input::Input`; update every place it's constructed or re-seeded (`open_agents_search`, committing/cancelling the search, re-editing an applied filter) to build an `Input` from the existing text, which defaults its cursor to the end - verify with a unit test in `app.rs` that re-entering edit mode on an applied filter seeds both the text and an end-of-string cursor.
- [x] 2.2 Update every read of `agents_search_buffer` as a `String` (filtering logic, tests) to go through `.value()` instead; verify `cargo test -p agentmon` still passes for the existing filtering tests with no behavior change.
- [x] 2.3 In `handle_agents_search_editing_key` (`crates/agentmon/src/input.rs`), keep the existing `Enter`/`Esc` arms as-is and forward every other key as `app.agents_search_buffer.handle_event(&crossterm::event::Event::Key(key))` (bringing in `tui_input::backend::crossterm::EventHandler`); verify with unit tests covering `Left`/`Right`/`Home`/`End`/`Ctrl+Left`/`Ctrl+Right`/`Delete`/`Ctrl+W`/`Ctrl+K`/`Ctrl+U` against a multi-word search string, asserting both the resulting text and that the displayed rows update to match.
- [x] 2.4 Update `agents_tab_title` in `crates/agentmon/src/ui.rs` to render the search buffer as `(before_cursor, char_at_cursor_or_space, after_cursor)` using `agents_search_buffer.value()` split at `agents_search_buffer.visual_cursor()`, with the middle span carrying the existing reversed-style cursor marker, instead of always appending a reversed `"_"` after the full text; verify with a `ui.rs` test asserting the reversed span's content and position when the cursor sits mid-string versus at the end.

## 3. Reminder form cursor

- [x] 3.1 Change `ReminderForm.name` and `ReminderForm.duration_minutes` from `String` to `tui_input::Input`; update every place the form is opened (`open_reminder_create_form`, `open_reminder_edit_form`) and every place `toggle_reminder_form_field` runs to (re)construct the newly-focused field's `Input` from its current text, so its cursor lands at the end - verify with a unit test that opening the edit form and tabbing between fields each put the cursor at the end of that field's pre-populated text.
- [x] 3.2 Update every read of the form's fields as `String` (submit, validation, existing tests) to go through `.value()`.
- [x] 3.3 In `handle_reminder_form_key` (`input.rs`), keep the existing `Enter`/`Esc`/`Tab` arms as-is. For a `Char(c)` key when the Duration field is active, forward it only if `c.is_ascii_digit()` (ignore otherwise); forward every other key, on whichever field is active, via `handle_event(&Event::Key(key))`; verify with unit tests that a non-digit character leaves the Duration field unchanged, and that `Left`/`Right`/`Home`/`End`/`Delete`/`Ctrl+W`/`Ctrl+K`/`Ctrl+U` work on both the Name and Duration fields.
- [x] 3.4 Update `render_reminder_form` in `ui.rs` to call `frame.set_cursor_position` using the active field's `visual_cursor()` instead of its full length; verify with a `ui.rs` test asserting the terminal cursor's column when the cursor is mid-string on each field.

## 4. Rename form cursor

- [x] 4.1 Change `RenameForm.name` from `String` to `tui_input::Input`; update every place the form is opened (`open_rename_form_from_agents_tab`, `open_rename_form_from_details_modal`) to build it from the project's current display name, cursor at the end.
- [x] 4.2 Update every read of `RenameForm.name` as a `String` (submit/validation, existing tests) to go through `.value()`.
- [x] 4.3 In `handle_rename_form_key` (`input.rs`), keep the existing `Enter`/`Esc` arms as-is and forward every other key via `handle_event(&Event::Key(key))`; verify with unit tests covering `Left`/`Right`/`Home`/`End`/`Ctrl+Left`/`Ctrl+Right`/`Delete`/`Ctrl+W`/`Ctrl+K`/`Ctrl+U`, and that typing `q` still inserts into the text (rather than quitting) at both an end-of-string and a mid-string cursor position.
- [x] 4.4 Update `render_rename_form` in `ui.rs` to call `frame.set_cursor_position` using `name.visual_cursor()` instead of `name.chars().count()`; verify with a `ui.rs` test asserting the terminal cursor's column when the cursor is mid-string.

## 5. Full-suite verification

- [x] 5.1 Run the full `agentmon` test suite (`cargo test -p agentmon`) and confirm it passes, including every existing cursor-at-end test (e.g. `reminder_form_shows_the_cursor_at_the_end_of_the_active_fields_text`, `rename_form_shows_the_cursor_at_the_end_of_the_typed_text`) still passing with only the `.value()`/construction updates from tasks 2-4, since the default cursor position (end of text) is unchanged.
- [x] 5.2 Exercise all three inputs end-to-end through the real daemon+client+input+ui pipeline (per CLAUDE.md's isolated-`HOME` testing note and the project's own `agentmon-manual-testing` convention of a throwaway `crates/agentmon/tests/` integration test over a pty - a real in-process `agentd`, a real `agentmon::client::spawn_client`, real `agentmon::input::handle_key`, and real `agentmon::ui::render` to a `TestBackend`, deleted after use): typed text, then `Left`/`Right`, `Home`/`End`, `Delete`, `Ctrl+W`, `Ctrl+K`, `Ctrl+U` in the Agents-tab search, the rename form, and the reminder form's Name field, confirming the rendered frames and resulting text matched expectations in each case.
