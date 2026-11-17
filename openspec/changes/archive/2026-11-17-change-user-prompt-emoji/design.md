## Context

See proposal.md for why: `💬` replaces `🗣️` because it's the only emoji in the TUI's marker set using a VS16 variation-selector sequence, and that sequence is the confirmed root cause of the ratatui/terminal cell-width disagreement the archived `fix-details-modal-maximise-glitches` change worked around rather than fixed (its design.md's Non-Goals explicitly named this as out of that change's reach). Removing the marker removes the disagreement; this design covers what to do with the workaround that disagreement necessitated.

## Goals / Non-Goals

**Goals:**
- Swap the marker everywhere it's rendered or asserted on, with no change in column layout, truncation, or color behavior (💬 is emoji-presentation by default, so it occupies the same 2-column display width `🗣️` did - the width-budget-subtraction logic in `ui.rs` stays correct unchanged).
- Remove the maximise-toggle full-redraw workaround (`App::needs_full_redraw`/`take_needs_full_redraw`, `toggle_modal_maximised()`'s flag-set, and the `terminal.resize(...)` call in `main.rs`), since its only reason to exist no longer applies once no VS16 sequence remains anywhere in the TUI's emoji set.

**Non-Goals:**
- Auditing ratatui or other terminal libraries for *other*, unrelated width-disagreement classes - this change only removes the workaround whose documented cause (a VS16 sequence) is being eliminated, not every conceivable redraw-correctness issue.
- Changing `m`/`Esc` maximise-toggle keybindings or the sticky maximised/windowed choice - only the extra forced redraw on toggle goes away.

## Decisions

**Decision: `input.rs`'s `Esc`-while-maximised handler keeps calling `App::toggle_modal_maximised()` rather than reverting to a direct `app.modal_maximised = false` assignment.**

The maximise-glitches change routed this call through `toggle_modal_maximised()` specifically so every state-changing site would also set the redraw flag. Once this change deletes the flag entirely, `toggle_modal_maximised()`'s only remaining job is flipping `modal_maximised` - which is exactly what the `Esc` handler needs. Calling the one function that owns this state transition, instead of mutating the `pub` field directly from a second call site, remains the better practice independent of the redraw signal, so there's nothing to revert here.

**Decision: delete `needs_full_redraw`/`take_needs_full_redraw`/the flag-set in `toggle_modal_maximised()`, and the `terminal.resize(...)` call in `main.rs`'s draw loop, rather than leaving them in place as a defensive no-op.**

Alternative considered: keep the mechanism in place in case a future marker reintroduces a VS16 sequence. Rejected per the user's explicit choice - dead state and an unexplained `terminal.resize()` call in the hot draw-loop path is itself a maintenance liability (the exact "formatting hacks/bodges" category this change exists to reduce), and any future emoji choice that reintroduces this class of bug can reintroduce a targeted fix at that time, informed by whatever the actual new cause is.

**Decision: comments that explain the width-subtraction arithmetic in `ui.rs` (e.g. "the already-emitted marker's own display width... is subtracted first") are kept, only updated to drop the now-inaccurate "VS16 emoji" framing.**

The arithmetic itself (subtract the marker's rendered width from the snippet budget) is still necessary and correct for `💬`, which is still a double-width glyph - just not via a variation selector. Only the explanatory wording that specifically attributes the need to VS16 needs to change; the underlying logic and tests are unaffected.

## Risks / Trade-offs

- [Risk] Removing `needs_full_redraw` removes a safety net: if ratatui's incremental diffing has some *other*, undiscovered misalignment case on the maximise toggle, this change would let it become visible again where it was previously masked. → Mitigation: the archived change's own design notes identify the VS16 sequence as the specific, confirmed cause exercised by this toggle; no other cause was ever observed or reported. Manual testing of the maximise/un-maximise toggle (per this repo's TUI testing norms) after the change verifies no stale characters reappear.
- [Risk] Deleting test coverage (`toggle_modal_maximised_sets_a_one_shot_full_redraw_signal` and the `input.rs` assertion on `take_needs_full_redraw()`) removes regression protection for the behavior itself. → Mitigation: that's intentional - the behavior is being removed, not just its test; `toggle_modal_maximised_flips_back_and_forth` (unaffected) continues covering the state flip that actually remains.
