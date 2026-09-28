## 1. App state and filtering logic

- [x] 1.1 Add `agents_search_editing: bool`, `agents_search_buffer: String`, and `agents_search_applied: Option<String>` to `App` (with `App::new()` defaults) and verify the crate builds
- [x] 1.2 Add `App::open_agents_search()` (seeds the buffer from `agents_search_applied`, sets editing true), `push_agents_search_char(c)`, `pop_agents_search_char()`, `commit_agents_search()` (empty buffer clears `agents_search_applied`), and `cancel_agents_search()` (clears editing only, leaves `agents_search_applied` untouched), each covered by a unit test
- [x] 1.3 Add `App::visible_agent_groups()` returning `directory_groups()` filtered by a case-insensitive substring match of `agents_search_applied` against each group's `project_name(&cwd)`, preserving existing relative order; verify with a unit test covering a matching filter, a non-matching filter (empty result), and no filter applied
- [x] 1.4 Clamp `agents_selected` (using the existing `clamp_selection`-style helper) whenever `visible_agent_groups()`'s length shrinks below the current selection, verified by a unit test that applies a filter which drops the selected row

## 2. Key handling

- [x] 2.1 In `input.rs::handle_key`, add an `app.agents_search_editing` early-diversion branch (mirroring the existing `reminder_form.is_some()` branch) that handles `Char(c)` (append), `Backspace` (pop), `Enter` (commit), and `Escape` (cancel), swallowing every other key, placed before the global `q`/`?`/`A`/`L`/`R`/`Tab` block
- [x] 2.2 Add `/` handling in the Agents-tab key block to call `open_agents_search()` when `app.active_tab == Tab::Agents` and not already editing
- [x] 2.3 Add a test that, while `agents_search_editing` is true, types `q`, `?`, `A`, `L`, `R`, and `Tab` and asserts none of their global effects fire (no quit, no help modal, no tab switch, no reminder form) and each becomes part of the search buffer instead
- [x] 2.4 Add a test for `/` -> type -> `Enter` committing a filter, and a separate test for `/` -> type -> `Escape` restoring the prior applied filter (both empty-prior and non-empty-prior cases)

## 3. Rendering

- [x] 3.1 Update `render_agent_table` (and its title logic) in `ui.rs` to use `visible_agent_groups()` for rows instead of `directory_groups()` directly
- [x] 3.2 Render the search-editing title state (`"Agents /{buffer}"` plus a visible cursor) when `agents_search_editing` is true
- [x] 3.3 Render the applied-filter title state (filter text bold, in a color distinct from the title's normal text) when `agents_search_applied.is_some()` and not editing
- [x] 3.4 Render the default title state with a `[/]` keyboard shortcut hint (styled like the existing Logs/Reminders bracketed hints) when neither editing nor an applied filter is present
- [x] 3.5 Add rendering tests (buffer-based, matching this crate's existing `find_text`/buffer-assertion style) for each of the three title states, plus a test that a filter hides non-matching rows and keeps matching rows in their existing relative order

## 4. Verification

- [x] 4.1 Run `cargo test -p agentmon` and confirm all new and existing tests pass
- [x] 4.2 Manually run the TUI against a live daemon with multiple tracked projects (per the project's manual-testing memory: isolate `HOME`), confirm typing `/`, filtering, `Enter` to commit, `/` again to re-edit, and `Escape` to cancel all behave as specified, and that Logs/Reminders tabs are unaffected
