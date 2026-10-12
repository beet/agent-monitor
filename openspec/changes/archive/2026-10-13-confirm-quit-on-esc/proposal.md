## Why

`Esc` currently quits the TUI outright whenever no modal, form, or dialog is open - the same action as the explicit `q` quit key. Because `Esc` is also the key that closes every modal, form, search prompt, and confirmation dialog, a user repeatedly tapping it to dismiss a stack of overlays can tap one time too many and exit the whole app by accident, losing their place with no way back. `q` is already an unambiguous, deliberate quit gesture and needs no extra friction, but a bare `Esc` at the top level is easy to over-press and should ask for confirmation before it ends the process.

## What Changes

- When `Esc` is pressed at the top level (no modal, form, dialog, or search prompt open), the TUI opens a quit-confirmation dialog instead of quitting immediately.
- While the quit-confirmation dialog is open: `Enter` confirms and quits the TUI; `Esc` dismisses the dialog and returns to the active tab, leaving the app running.
- `q` continues to quit immediately with no confirmation, from anywhere it already does today (including while the new quit-confirmation dialog is open) - its behavior is unchanged.
- `Esc`'s existing behavior inside every other context (closing the details modal, the help modal, the reminder form, the delete-reminder confirmation, and the Agents-tab search prompt) is unchanged - only the top-level, no-overlay-open case changes.
- The `?` help modal's shortcut list is updated to describe the new `Esc` behavior at the top level.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: adds a quit-confirmation dialog that top-level `Esc` opens instead of quitting directly, with `Enter`/`Esc` to confirm/cancel; clarifies that `q` quits unconditionally and without confirmation.

## Impact

- `crates/agentmon/src/app.rs`: new `Modal::QuitConfirm` variant (or equivalent state) and `open`/`close` helpers.
- `crates/agentmon/src/input.rs`: top-level `Esc` handling changes from returning `InputAction::Quit` to opening the quit-confirmation dialog; new key handling for the dialog's `Enter`/`Esc`.
- `crates/agentmon/src/ui.rs`: new rendering for the quit-confirmation dialog (styled consistently with the existing delete-reminder confirmation); updated help-modal shortcut text.
- `openspec/specs/agent-monitor-tui/spec.md`: "Navigation and quit do not affect tracked agents" requirement gains a quit-confirmation scenario; a new requirement (or amendment) documents the confirmation dialog's keys.
- No daemon, protocol, or persisted-state changes - this is TUI-local input/render behavior only.
