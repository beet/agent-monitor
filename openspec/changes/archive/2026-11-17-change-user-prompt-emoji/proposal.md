## Why

The prompt-category marker `🗣️` (U+1F5E3 SPEAKING HEAD + U+FE0F variation selector) is the only VS16 emoji sequence in the TUI, and the variation selector is the confirmed root cause of a ratatui cell-width disagreement with real terminals (see the archived `fix-details-modal-maximise-glitches` change's design notes). That change could only work around the symptom - forcing a full terminal redraw after the details modal's maximise toggle - because fixing ratatui's own diffing is out of this codebase's reach. The user is still occasionally seeing leftover-character artifacts. Switching the marker to `💬` (U+1F4AC SPEECH BALLOON), which is emoji-presentation by default and needs no variation selector, removes the width disagreement at its source, so the maximise-toggle workaround can be removed rather than kept as a now-pointless no-op.

## What Changes

- Replace the `🗣️` prompt-category marker with `💬` everywhere it appears: the Logs tab/details-modal marker, the Last Prompt pane's title, and every related comment and test assertion.
- **BREAKING** (internal API only): remove the maximise-toggle full-redraw workaround, now unnecessary once no VS16 sequence remains in the TUI's emoji set:
  - `App::needs_full_redraw` field, `App::take_needs_full_redraw()`, and the flag-setting inside `toggle_modal_maximised()`.
  - The `take_needs_full_redraw()` / `terminal.resize(...)` call in `main.rs`'s draw loop.
  - The associated unit tests in `app.rs` and `input.rs`.
- Update the "Project details modal" requirement: the user-facing "no stale characters after toggling maximised state" guarantee stays, but the mandate that the TUI force a full-terminal redraw to achieve it is dropped, since that's no longer something the TUI does or needs to do.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the prompt-category marker changes from `🗣️` to `💬` everywhere it's specified (status marker, pane title, label-prefix exception scenarios); the "Project details modal" requirement's mandate that the maximise toggle force a full-terminal redraw is removed (the observable "no stale characters" outcome stays), since the bug that mandate worked around no longer exists once no VS16 emoji sequence remains.

## Impact

- `crates/agentmon/src/ui.rs`: marker literal, pane title, comments explaining the VS16/double-width subtraction, and two test assertions.
- `crates/agentmon/src/app.rs`: remove `needs_full_redraw` field, `toggle_modal_maximised()`'s flag-set, `take_needs_full_redraw()`, and their doc comments; remove two unit tests.
- `crates/agentmon/src/main.rs`: remove the `take_needs_full_redraw()` / `terminal.resize(...)` call and its surrounding comment in the draw loop.
- `crates/agentmon/src/input.rs`: remove the test that asserts `take_needs_full_redraw()` fires on `Esc`-while-maximised; the `Esc` handler itself keeps calling `toggle_modal_maximised()` (still correct for flipping `modal_maximised`, independent of the redraw signal).
- `openspec/specs/agent-monitor-tui/spec.md`: update marker references and remove the full-redraw scenario/requirement text.
- No wire-format, persisted-state, or user-facing keybinding changes; `m`/`Esc` maximise-toggle behavior is otherwise unchanged.
