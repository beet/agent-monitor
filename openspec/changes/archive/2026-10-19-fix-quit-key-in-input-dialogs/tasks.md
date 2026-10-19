## 1. Fix key dispatch order

- [x] 1.1 In `handle_key` (`crates/agentmon/src/input.rs`), add an `app.reminder_form.is_some()` check that routes to `handle_reminder_form_key` before the global `KeyCode::Char('q')` quit check at line 46-48, matching the existing `agents_search_editing` and `rename_form` checks immediately above it. Verify by inspection that the reminder form is now checked ahead of the quit guard, the same way the other two free-text inputs are.
- [x] 1.2 Update the doc comments on `handle_key` (lines 24-29) and the per-check comments (lines 31-34, 39-41) to mention the reminder form alongside the search prompt and rename form, so the "checked before the global `q` quit shortcut" rationale stays accurate for all three. Verify by reading the updated comments against the new code order.

## 2. Tests

- [x] 2.1 Add a unit test in `input.rs`'s existing `mod tests` block that opens the reminder creation form, sends a `KeyEvent` for `KeyCode::Char('q')` while the Name field has focus, and asserts the result is `InputAction::Continue` (not `InputAction::Quit`) and that `q` was appended to the form's name. Verify with `cargo test -p agentmon`.
- [x] 2.2 Add a unit test asserting that pressing `q` with no reminder form, rename form, or search prompt open still returns `InputAction::Quit`, guarding against the fix over-widening the exemption. Verify with `cargo test -p agentmon`.

## 3. Validate

- [x] 3.1 Run `cargo test -p agentmon` and confirm the full suite passes, including the new tests from section 2.
- [x] 3.2 Run `cargo clippy -p agentmon` and confirm no new warnings were introduced by the reordering.
