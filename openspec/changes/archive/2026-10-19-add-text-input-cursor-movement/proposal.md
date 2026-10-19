## Why

Every free-text field in the TUI (the Agents-tab search prompt, the project rename form, and the reminder creation/editing form's Name and Duration fields) only supports appending at the end and backspacing from the end. There is no cursor concept, so a user who types a typo partway through a name, or wants to fix a mid-string character, cannot move there with the arrow keys - the only way to fix it today is to backspace all the way to the mistake and retype everything after it.

## What Changes

- Adopt the `tui-input` crate (crossterm backend) as the text-editing engine behind every free-text input (search buffer, rename form, reminder form's Name and Duration fields), replacing each field's bare `String` with a `tui_input::Input`.
- Each field gains the full set of line-editing keys `tui-input`'s crossterm integration maps out of the box: `Left`/`Right` move the cursor one character; `Home`/`End` jump to the start/end; `Ctrl+Left`/`Ctrl+Right` move by word; `Delete` removes the character after the cursor; `Backspace` removes the character before it; `Ctrl+W` deletes the previous word; `Ctrl+K` deletes from the cursor to the end; `Ctrl+U` clears the field. All are clamped to the field's own text - no wraparound, no crossing into another field.
- Typing a printable character inserts it at the cursor position (not always at the end) and advances the cursor past it.
- Switching fields (`Tab` in the reminder form) or reopening a form resets that field's cursor to the end of its text, consistent with today's "seed from existing text" behavior.
- The reminder form's Duration field keeps its existing digits-only restriction: a non-digit character key is ignored rather than forwarded to the input engine.

## Capabilities

### Modified Capabilities
- `agent-monitor-tui`: free-text input fields (Agents-tab search, rename form, reminder form) gain a movable cursor and the standard line-editing key set listed above; insertion and deletion act at the cursor position instead of always at the end of the string.

## Impact

- `crates/agentmon/Cargo.toml`: new dependency on `tui-input` (crossterm feature), matching the project's existing `crossterm = "0.29.0"`.
- `crates/agentmon/src/app.rs`: `App`'s search/rename/reminder-form state replaces its plain `String` fields with `tui_input::Input`; the custom char-insert/backspace helper methods for each of the three inputs are replaced by delegating to `Input`.
- `crates/agentmon/src/input.rs`: the three free-text key-handling functions (`handle_agents_search_editing_key`, `handle_rename_form_key`, `handle_reminder_form_key`) forward whichever key isn't already claimed by this app (`Enter`/`Esc`/`Tab`/the Duration field's digit filter) to the active `Input` via its crossterm `EventHandler::handle_event`.
- `crates/agentmon/src/ui.rs`: wherever these fields are rendered, the text comes from `Input::value()` and the cursor's screen position from `Input::visual_cursor()` rather than the field's raw length.
