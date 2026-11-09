## Context

The details modal (`Modal::Details(PathBuf)`) is one of three `Modal` variants in `crates/agentmon/src/app.rs`. Today `render_details_modal` (`crates/agentmon/src/ui.rs:874`) always sizes itself with `centered_rect(80, 80, frame.area())` - 80% of the *whole* terminal, computed independent of the tab bar. `render_body` (`ui.rs:74`) splits the frame into `chunks[0]` (the 2-row tab bar) and `chunks[1]` (everything below it, where the active tab's content - the agents table, Logs tab, or Reminders tab - renders); the modal is drawn afterward, on top, via `render_modal`. `m` is not bound to anything today. See `proposal.md` for why this change is being made.

## Goals / Non-Goals

**Goals:**
- Toggle the details modal between its existing windowed size and a maximised size that takes over the entire terminal, including the rows the tab bar normally occupies - the modal intercepts all keyboard input while open, so the tab bar can't be interacted with regardless of whether it's visible, and there's no reason to reserve space for it.
- Make the maximised/windowed choice sticky: closing and reopening the modal, or switching tabs while it's closed, never resets it by itself.
- Give `Esc` a one-extra-step behavior while maximised (restore to windowed) before it falls through to the existing close-the-modal behavior.

**Non-Goals:**
- No change to the five-pane internal layout, focus cycling, pagination, or pane content rules - maximising only changes how much height/width `render_details_modal`'s existing proportional layout math has to divide up.
- No maximise support for `Modal::Help` or `Modal::QuitConfirm` - the proposal and the `m` shortcut are scoped to the details modal only.
- No persistence across process restarts - "sticky" means for the lifetime of the running TUI process, same as every other piece of `App` state.

## Decisions

**Store the flag on `App`, not inside the `Modal::Details` variant.** `App` gains `pub modal_maximised: bool`. `close_modal` (`app.rs:713`) must *not* reset it - that's what makes it survive a close/reopen cycle. Storing it instead as `Modal::Details(PathBuf, bool)` was considered and rejected: `open_details_modal` constructs a fresh `Modal::Details(cwd)` on every open, so any flag carried inside the variant would reset to its default each time the modal is reopened, defeating the "retains state across tab navigation and reopening" requirement.

**Toggle key lives in the existing `Modal::Details`-only key block.** `input.rs:281`'s `if matches!(app.modal, Some(Modal::Details(_)))` block already carries other focus-independent shortcuts (`Tab`/`BackTab`/`R`/`n`/pane-jump letters). Add `KeyCode::Char('m') => app.toggle_modal_maximised()` there, so it works regardless of which pane holds focus, consistent with how `n` and `R` already behave.

**`Esc` branches on the flag before the generic close.** Today `input.rs:276` unconditionally closes on `Esc` for any open modal. Add a check immediately before it: if `Modal::Details` is open and `app.modal_maximised` is true, `Esc` flips the flag to `false` and returns `Continue` without closing; otherwise, fall through to the existing `app.close_modal()` path unchanged. This leaves `Modal::Help` and `Modal::QuitConfirm` unaffected.

**Rendering picks the modal's area based on the flag - no new parameter needs to be threaded anywhere.** `render_details_modal` already computes `frame.area()` for its windowed `centered_rect(80, 80, frame.area())` call. When `app.modal_maximised` is true, use `frame.area()` directly as the modal's area instead - the full terminal, including the rows `render_tab_bar` already painted. The modal's own `Clear` widget, already rendered across its area before its border and panes draw, blanks that painted tab bar along with everything else beneath it; no coordination with `render_body`'s tab-bar rendering is needed beyond that, and no `chunks[1]`-style content-area parameter needs to pass through `render_modal`/`render_details_modal` at all. `Modal::Help` and `Modal::QuitConfirm` keep calling `centered_rect` against `frame.area()` regardless of the flag, since maximise is scoped to the details modal.

**The existing proportional layout inside `render_details_modal` needs no changes.** The Agents/Tests/Reminders row height, the Last Prompt strip, and the Logs pane's remaining space are already computed from `inner.height` (`ui.rs:899-916`), so handing the function a taller `inner` (because the outer area is now `chunks[1]`-sized instead of 80%-of-frame-sized) is already enough to give every pane more room, including raising the one-third-of-modal pagination cap. No sizing formula needs to change.

**Hint rendering: a right-aligned top title segment, bold exactly while maximised.** The block already carries a left-aligned top title (`"Details - {name}  Rename [n]"`) and a bottom-left title (`"<tab> focus pane"`). Add a second, right-aligned top title segment reading `Maximise [m]`, styled bold when `app.modal_maximised` else normal weight - ratatui supports multiple title segments per edge with independent alignment, so this doesn't disturb the existing two.

## Risks / Trade-offs

- [A very narrow terminal could make the left title (project name + Rename hint) collide with the new right-aligned `Maximise [m]` hint] → Accepted: every other title/hint in this modal already degrades the same way at extreme widths (no existing truncation guard for the left title either), so this isn't a new class of problem.
- [Maximising could make the Agents/Tests/Reminders row's one-third cap grow large enough to look sparse on a very tall terminal] → Accepted: the existing cap formula already scales with `inner.height` for the windowed modal; this is the same trade-off at a larger size, not a new one.
- [Since `Esc` always restores a maximised modal to windowed before a further `Esc` closes it, there is no key sequence that closes the modal while `modal_maximised` is still `true` - so in practice, closing the modal via `Esc` always leaves it windowed for the next open, and the "sticky across close/reopen" guarantee is only actually exercised by tab-switching not resetting the flag while the modal is closed, not by a maximised-then-closed round trip] → Accepted (confirmed with the user during implementation): a single `modal_maximised` flag, with `Esc`-while-maximised setting it `false` exactly like `m` does, matches the spec's literal `Esc` wording and keeps the state model simple. A two-field alternative (a separate sticky preference untouched by `Esc`'s restore step) was considered and rejected as unnecessary complexity for a consequence the user accepted.

## Migration Plan

None needed - this is a purely additive, opt-in keyboard interaction gated behind a previously-unbound key (`m`). No data model or wire-protocol changes.
