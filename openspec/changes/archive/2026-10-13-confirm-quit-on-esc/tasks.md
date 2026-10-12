## 1. App state

- [x] 1.1 Add `Modal::QuitConfirm` to the `Modal` enum in `crates/agentmon/src/app.rs` and verify `cargo build -p agentmon` compiles
- [x] 1.2 Add `App::open_quit_confirm(&mut self)` (sets `self.modal = Some(Modal::QuitConfirm)`) next to the existing `open_help_modal`/`open_details_modal`, and verify a unit test asserts `app.modal == Some(Modal::QuitConfirm)` after calling it
- [x] 1.3 Verify `App::close_modal` already closing `Modal::QuitConfirm` back to `None` needs no changes (it already sets `self.modal = None` unconditionally) by adding a unit test that opens the quit-confirm modal, calls `close_modal`, and asserts `app.modal == None`

## 2. Input handling

- [x] 2.1 In `crates/agentmon/src/input.rs::handle_key`, change the top-level `KeyCode::Esc => return InputAction::Quit` arm (reached only when `app.modal.is_none()`) to call `app.open_quit_confirm()` and return `InputAction::Continue` instead, and verify a unit test presses `Esc` with no modal open and asserts the resulting `app.modal == Some(Modal::QuitConfirm)` and the returned action is `InputAction::Continue`
- [x] 2.2 In `handle_modal_key`, add an early branch for `app.modal == Some(Modal::QuitConfirm)` (checked alongside the existing `reminder_form`/`confirm_delete_reminder` early-return checks, before the generic `Esc => close_modal()` and `Modal::Details` branches) that returns `InputAction::Quit` on `Enter` and calls `app.close_modal()` (returning `InputAction::Continue`) on `Esc`, and verify unit tests for both keys
- [x] 2.3 Verify `q` still returns `InputAction::Quit` unconditionally while `Modal::QuitConfirm` is open, by adding a unit test that opens the quit-confirm modal, presses `q`, and asserts `InputAction::Quit`
- [x] 2.4 Verify existing `Esc` behavior is unchanged for every other overlay (details modal, help modal, reminder form, delete-reminder confirmation, Agents-tab search prompt) by running the existing `input.rs` test suite (`cargo test -p agentmon`) and confirming no regressions - add a new test only if an existing one doesn't already cover a case

## 3. Rendering

- [x] 3.1 Add `render_quit_confirm_modal(frame: &mut Frame)` in `crates/agentmon/src/ui.rs`, styled like `render_delete_reminder_confirm` (centered `Clear`'d rect, bold red title/border, bold red "Quit agentmon?" question line, dim "Enter: quit   Esc: cancel" hint line)
- [x] 3.2 Add a `Modal::QuitConfirm => { render_quit_confirm_modal(frame); (0, 0) }` arm to `render_modal`'s match, next to the existing `Modal::Help` arm, and verify `cargo build -p agentmon` compiles
- [x] 3.3 Add a `ui.rs` render test that opens the quit-confirm modal and asserts the rendered buffer contains the confirmation text (e.g. via the existing `find_text` test helper), verified by `cargo test -p agentmon`

## 4. Help text

- [x] 4.1 Update the `Esc` line in `render_help_modal`'s shortcut list (`crates/agentmon/src/ui.rs`) to mention that a top-level `Esc` opens a quit confirmation, and verify the existing help-modal render test (if any) still passes after the wording change

## 5. Spec and docs sync

- [x] 5.1 Run `openspec validate confirm-quit-on-esc --strict` and fix any reported issues
- [x] 5.2 Per CLAUDE.md's "Keeping the README's diagrams in sync": check whether the `agent-monitor-tui` behavior change affects README.md's data-model or agent-status state diagrams (expected: no, since `AgentStatus` and the wire types are untouched) and leave a note in the PR/commit if a diagram update is genuinely needed
