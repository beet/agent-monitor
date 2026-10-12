## Context

See proposal.md - Why. The relevant code lives entirely in `crates/agentmon/src`:

- `input.rs::handle_key` checks `KeyCode::Char('q')` first (returns `InputAction::Quit` unconditionally, before even checking whether a modal is open), then - only if no modal is open - matches top-level `KeyCode::Esc` to also return `InputAction::Quit` directly (`crates/agentmon/src/input.rs:37-46`).
- `app.rs` already has a `Modal` enum (`Details`, `Help`) drawn as a centered overlay, plus a sibling `confirm_delete_reminder: Option<ReminderId>` field that layers the bold-red delete-confirmation dialog on top of the details modal and is checked first inside `handle_modal_key` (`crates/agentmon/src/input.rs:161-178`).
- `ui.rs::render_delete_reminder_confirm` (`crates/agentmon/src/ui.rs:1445-1483`) is the existing confirmation-dialog look: a centered `Clear`'d rect, bold red title/border, bold red question line, and a dim "Enter: ... Esc: cancel" hint line.

## Goals / Non-Goals

**Goals:**
- Reuse the existing `Modal`/overlay plumbing rather than inventing a parallel state machine, so the new dialog gets `Clear`-on-top rendering and centered-rect layout for free.
- Keep `q`'s key handling exactly where it is today (checked before anything else in `handle_key`) so it keeps quitting unconditionally, including while the new dialog is open.

**Non-Goals:**
- No change to how any *other* modal/form/dialog's `Esc` behaves - only the no-overlay-open, top-level case.
- No change to the help modal's mechanics, only its listed shortcut text (cosmetic, not spec-level).

## Decisions

**Represent the dialog as `Modal::QuitConfirm`, not a separate `Option<bool>` field.**
The top-level `Esc` branch in `handle_key` only reaches the `KeyCode::Esc => return InputAction::Quit` arm when `app.modal.is_none()` (the `if app.modal.is_some() { return handle_modal_key(...) }` guard above it already routes anything else away). Setting `app.modal = Some(Modal::QuitConfirm)` means that guard now sends the *next* keypress into `handle_modal_key` for free, and `app.close_modal()` (`modal = None`, plus clearing `reminder_form`/`confirm_delete_reminder`, which will already be `None` here) is exactly "return to the active tab, nothing else changed." This matches the project's existing pattern for `Modal::Help` more closely than it matches `confirm_delete_reminder`, which is a sibling flag layered *on top of* `Modal::Details` rather than a replacement for "no modal" - here there is no modal underneath to preserve, so folding it into `Modal` itself is simpler than adding a fourth top-level `Option` field.
Alternative considered: a new `app.confirm_quit: bool` field, checked in `handle_key` alongside `app.modal.is_some()`. Rejected because it duplicates the routing `handle_modal_key` already provides and would need its own `Esc`/`Enter` branch wired in at the top of `handle_key` rather than reusing `handle_modal_key`'s existing structure.

**Check `q` before the `Modal::QuitConfirm` dispatch, unchanged.**
`handle_key`'s existing `if let KeyCode::Char('q') = key.code { return InputAction::Quit }` already runs before `if app.modal.is_some() { ... }`, so it fires whether `app.modal` is `None`, `Some(Modal::QuitConfirm)`, or any other variant, with no new code needed to make `q` bypass the new dialog.

**Render via a new `render_quit_confirm_modal` matched in `render_modal`, styled like `render_delete_reminder_confirm`.**
Add a `Modal::QuitConfirm => { render_quit_confirm_modal(frame); (0, 0) }` arm (next to the existing `Modal::Help` arm) in `render_modal`. The dialog itself: centered rect (the delete-confirm dialog uses `centered_rect(50, 20, ...)` - small enough that reusing the same dimensions reads fine for a short "Quit?" prompt too), `Clear`'d, bold-red title/border/question text ("Quit agentmon?"), and a dim "Enter: quit   Esc: cancel" hint line - the same visual language the delete-reminder dialog already established, so the user recognizes it as the same kind of dialog rather than a new idiom.
Alternative considered: a plain (non-red) style, since quitting isn't destructive to data the way deleting a reminder is. Rejected for consistency - the project has exactly one confirmation-dialog visual pattern today, and introducing a second one for a dialog that does the same *shape* of job (confirm a one-way action) would read as inconsistent rather than intentionally different.

**Update the `?` help modal's static text to describe the new `Esc` split.**
`render_help_modal`'s shortcut list currently has one line each for `Esc` ("close modal / form / dialog") and `q` ("quit"). Reword the `Esc` line to note it opens a quit confirmation at the top level (e.g. `Esc close modal / form / dialog (or ask to quit)`) so the help text doesn't go stale. This is UI text, not spec-level behavior - no scenario governs its exact wording - but proposal.md calls it out under "What Changes" and it should ship in the same commit.

## Risks / Trade-offs

- [Risk] A user who genuinely wants to quit via `Esc` now needs two keypresses instead of one. → Accepted: this is the entire point of the change (per proposal.md - Why), and `q` remains a true one-press quit for anyone who wants that.
- [Risk] Forgetting to exempt `Modal::QuitConfirm` from any future logic that assumes "a modal is open" implies the details/help modal's own keybindings apply (e.g. `Tab` to toggle focus, pagination keys) could misroute keys. → Mitigated: `handle_modal_key` already special-cases `confirm_delete_reminder` and the reminder form *before* falling into the `Modal::Details`-specific key matching; `Modal::QuitConfirm` gets the same early-return treatment (checked right after the `reminder_form`/`confirm_delete_reminder` checks, before the `Esc => close_modal()` and `Modal::Details` branches), so it never reaches that code.

## Migration Plan

Not applicable - purely additive, in-process TUI behavior with no persisted state, protocol messages, or data migration involved. Ships as a normal `agentmon` release.
