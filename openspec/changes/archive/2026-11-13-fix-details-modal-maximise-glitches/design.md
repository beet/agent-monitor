## Context

See proposal.md for the root cause of the stale-characters bug. The relevant mechanics:

- `crates/agentmon/src/main.rs`'s loop owns the `ratatui::DefaultTerminal` and is the only place with `&mut Terminal`; it calls `terminal.draw(|frame| render(frame, &app))` every iteration, then blocks on `rx.recv()` for the next event.
- `crates/agentmon/src/input.rs::handle_key` mutates `App` and returns an `InputAction` (`Quit`, `SendReminderCommand`, `Continue`) that `main.rs` matches on. It has no way today to tell `main.rs` anything about the terminal itself.
- `App::toggle_modal_maximised()` (`app.rs:732`) flips `modal_maximised` and is called from the `m` key handler in `input.rs`. The other site that changes maximised state - `Esc` while maximised, which restores instead of closing (`input.rs:281-283`) - currently sets `app.modal_maximised = false` directly, bypassing `toggle_modal_maximised()`, since `modal_maximised` itself is a `pub` field.
- Ratatui's `Terminal::clear()` (re-exported through `ratatui::DefaultTerminal`) clears the screen for the active viewport and - critically - resets the terminal's internal "previous frame" buffer, so the *next* `terminal.draw()` call diffs against an all-blank buffer and therefore emits an explicit absolute cursor move before every non-blank cell, for every cell in the frame. That's the existing, intended API for "I know the screen doesn't match what I think I last drew; redraw everything" - but it is unsafe here: `clear()` calls `get_cursor_position()`, which (per crossterm's own doc comment on `cursor::position()`) "will block and possibly time out while `crossterm::event::read` or `crossterm::event::poll` are being called", and `main.rs` always has a dedicated background thread blocked in exactly that (the `key_tx` thread, spawned a few lines above the draw loop, polling every 150ms for the next key event). `Terminal::resize(area)` clears the screen and resets the same "previous frame" buffer, but - for a `Viewport::Fullscreen` terminal (what `ratatui::init()` sets up) - never touches cursor position, so it doesn't race the key-reading thread.

## Goals / Non-Goals

**Goals:**
- Guarantee that the frame immediately following a maximise/un-maximise toggle is drawn with no reliance on the terminal's actual on-screen state matching ratatui's internal model.
- Keep the fix scoped to the maximise toggle - the one interaction that reliably exercises this class of bug - without changing how any other frame is drawn.

**Non-Goals:**
- Fixing the underlying width disagreement between ratatui's cell-width model and real terminals for VS16 emoji sequences (`🗣️`). That lives in the vendored `ratatui-core` diffing code, not in this codebase, and isn't something this change can or should patch.
- Forcing a full redraw on modal open/close, tab switches, or any other transition - those aren't reported as buggy and a blanket "always clear" would add needless flicker on the TUI's most common interactions.

## Decisions

**Decision: `App` tracks a `needs_full_redraw` flag, set inside `toggle_modal_maximised()`, consumed by `main.rs` right before the next `terminal.draw()`; the `Esc`-while-maximised handler in `input.rs` is changed to call `toggle_modal_maximised()` instead of setting `modal_maximised` directly.**

- `App` gains a private `needs_full_redraw: bool` field (defaulting to `false`) and `toggle_modal_maximised()` sets it to `true` whenever it flips `modal_maximised`.
- `App` exposes `take_needs_full_redraw(&mut self) -> bool`, which returns the current value and resets it to `false` (same take-and-clear shape the codebase already uses elsewhere for one-shot signals).
- `main.rs`'s loop calls `take_needs_full_redraw()` immediately before `terminal.draw(...)` each iteration; if it returns `true`, it calls `terminal.resize(terminal.size()?.into())?` first (see the next decision for why not `terminal.clear()`).
- `input.rs:282`'s `Esc`-while-maximised branch changes from `app.modal_maximised = false;` to `app.toggle_modal_maximised();` - behaviorally identical (the branch's guard already established `modal_maximised` is `true`, so toggling it is the same as setting it to `false`), but now routes through the one function that sets the redraw flag, instead of being a second, easy-to-miss place that changes maximised state.

Alternative considered: add a new `InputAction` variant (e.g. `ForceRedraw`) returned from the `m`/`Esc` handlers in `input.rs`, matched in `main.rs` alongside `Quit`/`SendReminderCommand`/`Continue`. Rejected for the same reason the `Esc` handler is being changed to call `toggle_modal_maximised()` rather than left alone: either way, every call site that changes `modal_maximised` has to remember to also signal the redraw, and an `InputAction` variant doesn't remove that duplication, it just moves it to the `return` statement instead of the field assignment. Centralizing on `toggle_modal_maximised()` removes the duplication instead of relocating it.

**Decision: use `Terminal::resize(terminal.size()?.into())`, not `Terminal::clear()` or a hand-rolled full-repaint.**

`Terminal::clear()` was the first instinct (it's the API whose name matches the intent), but it calls `get_cursor_position()` internally, which performs a query/response round-trip against the terminal over stdout/stdin. `main.rs` runs a dedicated thread that is continuously blocked in `crossterm::event::poll`/`event::read` reading keyboard input - exactly the condition crossterm's own docs call out as able to block or time out a concurrent `cursor::position()` call. Calling `clear()` here risked the `m` key hanging for up to a 2-second timeout, or the `?` propagating an `Err` that would exit the whole app, depending on which read "won" the race for the terminal's response bytes.

`Terminal::resize(area)`, called with the terminal's own current size (so nothing about the actual layout changes), achieves the same result via a different path: for a `Viewport::Fullscreen` terminal it clears the backend (`clear_region(ClearType::All)`) and resets the internal "previous frame" buffer - the same two effects `clear()` provides - but only sets/restores cursor position for `Viewport::Inline`, never for `Viewport::Fullscreen`. No cursor query, so no race with the key-reading thread.

**Decision: the hint's label text is computed in `ui.rs` alongside its existing bold/non-bold logic, not stored on `App`.**

`render_details_modal` already reads `app.modal_maximised` to choose `maximise_hint_style`; it reads the same field to choose between `"Maximise [m]"` and `"Un-maximise [m]"`. No new state needed - this is a pure rendering change.

## Risks / Trade-offs

- [Risk] Calling `terminal.resize(...)` issues a terminal-level clear command, which could itself produce a visible flash on some terminal emulators. → Mitigation: it only fires on an explicit `m`/`Esc` maximise toggle, an infrequent, intentional user action (not, e.g., every tick or every log update), so an occasional clear is an acceptable trade for guaranteed-correct redraws.
- [Risk] `needs_full_redraw` is a one-shot flag; `modal_maximised` is a `pub` field, so a future production code path could set it directly (as the `Esc` handler did before this change) and silently skip setting the flag, resurfacing the bug. → Mitigation: this change removes the one existing direct-mutation site in production code (`input.rs`'s `Esc` handler); the test-only direct assignments in `ui.rs` (e.g. `app.modal_maximised = true;` to set up a rendering fixture) don't go through `main.rs`'s draw loop at all, so they're unaffected either way.
