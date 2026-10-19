## Why

Typing `q` into the reminder creation/editing form's Name field quits the whole TUI instead of inserting the character, because the global `q`-quits-immediately check in `handle_key` (`crates/agentmon/src/input.rs:46-48`) runs before the reminder form gets a chance to consume the keystroke. The Agents-tab search prompt and the project rename form are both already shielded from this (checked earlier in `handle_key`, at lines 35-37 and 42-44 respectively) - the reminder form is the one free-text input that was missed when that precedence was established, and the fix is to bring it in line with the existing pattern rather than invent a new one.

## What Changes

- In `handle_key` (`crates/agentmon/src/input.rs`), check `app.reminder_form.is_some()` and route to `handle_reminder_form_key` before the global `q`-quits-immediately check, the same way `agents_search_editing` and `rename_form` already are.
- Update the "Navigation and quit do not affect tracked agents" requirement in the `agent-monitor-tui` spec so its list of `q`-exempt contexts also names the reminder creation/editing form, alongside the search prompt and rename form it already names.
- No change to the Duration field's digit-only filtering, to any other key binding, or to the quit-confirmation dialog's behavior.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the "Navigation and quit do not affect tracked agents" requirement's description of which contexts exempt `q` from quitting immediately is extended to include the reminder creation/editing form's free-text fields.

## Impact

- `crates/agentmon/src/input.rs`: `handle_key`'s dispatch order.
- `openspec/specs/agent-monitor-tui/spec.md`: the quit/navigation requirement's text and scenarios.
- No daemon, protocol, or persisted-state changes; no other key binding is affected.
