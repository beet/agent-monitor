## 1. App state

- [x] 1.1 Add `pub modal_maximised: bool` to `App` (default `false`); add `App::toggle_modal_maximised(&mut self)` that flips it. Verify `close_modal` does NOT reset it (grep `close_modal`'s body and confirm the field isn't touched) and add a unit test asserting the flag survives a `open_details_modal` → `close_modal` → `open_details_modal` cycle.
- [x] 1.2 Add a unit test asserting `toggle_modal_maximised` flips `false` → `true` → `false`.

## 2. Key handling

- [x] 2.1 In `input.rs`'s `Modal::Details`-only key block (`crates/agentmon/src/input.rs:281`), add `KeyCode::Char('m') => { app.toggle_modal_maximised(); return InputAction::Continue; }`, alongside the existing `n`/`R`/pane-shortcut cases. Verify with a test that pressing `m` while the details modal is open (any focused pane) sets `modal_maximised` to `true`, and pressing it again sets it back to `false`.
- [x] 2.2 Change the unconditional `KeyCode::Esc => app.close_modal()` branch (`input.rs:276`) so that when `Modal::Details` is open and `app.modal_maximised` is `true`, `Esc` instead sets `modal_maximised` to `false` and returns `Continue` without closing the modal; otherwise fall through to the existing close behavior unchanged. Verify with tests: (a) `Esc` while maximised restores windowed and leaves `app.modal` set; (b) `Esc` while windowed closes the modal exactly as before; (c) `Esc` with `Modal::Help` or `Modal::QuitConfirm` open is unaffected by `modal_maximised`'s value.
- [x] 2.3 Verify `m` has no effect on `Modal::Help` or `Modal::QuitConfirm` (it isn't bound outside the `Modal::Details` block) with a test that presses `m` while one of those is open and asserts `modal_maximised` is unchanged and the modal is unchanged.

## 3. Rendering

- [x] 3.1 In `render_details_modal` (`ui.rs:874`), pick the modal's area based on `app.modal_maximised`: `centered_rect(80, 80, frame.area())` when `false` (unchanged), `frame.area()` directly when `true` - no new parameter needed, since `frame.area()` is already computed in this function for the windowed branch. If a `chunks[1]`-style content-area parameter was already threaded through `render_modal`/`render_details_modal` for the earlier "fill the content area beneath the tab bar" design, remove it - it's no longer used for anything.
- [x] 3.2 Update the existing maximised-geometry rendering tests to match full-screen takeover instead of content-area-only: the modal's border now starts at the terminal's actual top-left corner (row 0, column 0), not row 2, and the tab bar is no longer expected to remain visible while maximised - invert any assertion that previously checked the tab bar's emoji/label text was still present when maximised to instead confirm it's covered. Verify with a rendering test (using ratatui's `TestBackend`) that the maximised modal's rendered area matches `frame.area()` exactly, including the rows the tab bar would otherwise occupy.
- [x] 3.3 Confirm no changes are needed to the Agents/Tests/Reminders row height, Last Prompt strip, or Logs pane sizing math (`ui.rs:899-916`) now that `inner` can be taller - run the existing pane-layout tests under a taller `TestBackend` size and confirm they still pass unmodified.
- [x] 3.4 Add a right-aligned top title segment to the details modal's block reading `Maximise [m]`, bold when `app.modal_maximised` else normal weight, without disturbing the existing left-aligned top title (`"Details - {name}  Rename [n]"`) or the bottom-left `<tab> focus pane` title. Verify with a rendering test asserting the hint's text, position, and style (bold vs. not) for both maximised and windowed states.
- [x] 3.5 Confirm `Modal::Help` and `Modal::QuitConfirm` rendering is unaffected (still always `centered_rect` against `frame.area()`, no new title segment) with a rendering test.

## 4. Cross-tab persistence

- [x] 4.1 Add an integration-style test confirming switching tabs (via the outer tab keys) does not reset `modal_maximised` while the modal is closed - set the flag, switch through Logs and Reminders and back to Agents, and assert it's unchanged. (Note: resolved a design ambiguity with the user here - since `Esc` always restores a maximised modal to windowed before a further `Esc` closes it, there is no real key sequence that closes the modal while `modal_maximised` is still `true`, so "maximise, close, navigate tabs, reopen, assert still maximised" is not a reachable flow. The user confirmed the single-flag design as-is; see design.md's Risks/Trade-offs.)
- [x] 4.2 Add an integration-style test: open the details modal from the Agents tab (windowed by default), press `Esc` to close, switch to the Logs tab and then the Reminders tab via the outer tab keys, switch back to the Agents tab, reopen the details modal, and assert it reopens windowed.
- [x] 4.3 Add a test opening the details modal from the Logs tab (or Reminders tab) while `modal_maximised` is already `true` and asserting it opens already maximised, confirming the toggle's effect is independent of which tab the modal was opened from.

## 5. Spec and docs

- [x] 5.1 Run `openspec validate maximise-details-modal --strict` and confirm it passes.
- [x] 5.2 Run the full `agentmon` test suite (`cargo test -p agentmon`) and confirm it passes.
