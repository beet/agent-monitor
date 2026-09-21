## Context

Both paginated lists (Logs tab, details modal's Logs pane) already share one `Paginator` struct (`crates/agentmon/src/app.rs:16-78`, fields `selected`/`top`) and one scrollbar-rendering helper, `render_pagination_scrollbar` (`crates/agentmon/src/ui.rs:290-295`). `Paginator` already has `move_by` (line movement) and `page` (page movement, with 1-row overlap and clamping via `max_top = len.saturating_sub(page_size)`); there is no jump-to-start/end method yet. See proposal.md for motivation.

The pagination hint shown in each pane's title comes from two call sites that don't share a helper: `logs_controls_hint` (`ui.rs:403-419`, produces `"Page [d/u]"`, prepended for the Logs tab) and an inline conditional in the details modal's Logs pane title (`ui.rs:559-561`, produces `"Logs  |  Page [d/u]"`). The `?` help modal's shortcut list is a flat `&[&str]` array in `render_help_modal` (`ui.rs:588-619`).

The scrollbar bug: `render_pagination_scrollbar` builds `ScrollbarState::new(len).position(top)`. Ratatui's thumb-position math (confirmed by reading `ratatui-widgets-0.3.2/src/scrollbar.rs`) expects `position` to range over `0..content_length`, with the thumb flush against the track's end only when `position == content_length - 1`. But `top`'s maximum value is `len.saturating_sub(page_size)` (`Paginator`'s `max_top`/`synced_top` clamp), which is strictly less than `len - 1` whenever `page_size > 1`. Passing raw `len` as `content_length` therefore makes the thumb stop short of the track's bottom on the last page for every realistic terminal size.

## Goals / Non-Goals

**Goals:**
- `g`/`G` behave identically on both paginated lists, implemented once on `Paginator` so the two call sites (Logs tab, modal Logs pane) can't drift.
- Fix the scrollbar's `content_length` computation once, in the shared `render_pagination_scrollbar` helper, so both lists get the fix for free.
- Keep hint/help text accurate without over-engineering a shared "hint builder" the codebase doesn't otherwise have.

**Non-Goals:**
- Not adding `g`/`G` (or any pagination controls) to the Agents tab's row selection - it uses a plain `TableState` with no `Paginator`/page-size windowing and is out of scope per the proposal.
- Not changing paging (`d`/`u`) semantics or the 1-row-overlap behavior - only adding a new jump operation and fixing scrollbar math.
- Not introducing a generic vim-motion layer (e.g. count-prefixed `dd`-style bindings) - just the two literal keys the user asked for.

## Decisions

**`g`/`G` as a `Paginator` method, not inline key-handling logic.** Add `pub fn jump_to_start(&mut self, len: usize, page_size: usize)` and `pub fn jump_to_end(&mut self, len: usize, page_size: usize)` to `Paginator` (`app.rs`), mirroring the existing `move_by`/`page` pattern: `jump_to_start` sets `selected = 0, top = 0`; `jump_to_end` sets `selected = len.saturating_sub(1)`, `top = len.saturating_sub(page_size)` (i.e. `max_top`, the same clamp `page` already uses). Both are no-ops when `len == 0`. Alternative considered: computing the jump inline in `input.rs`'s key handlers - rejected because it would duplicate the `max_top` clamp math that already lives in `Paginator` and risks the two call sites (Logs tab, modal) drifting the way the codebase's existing `Paginator` consolidation was specifically designed to prevent (see the archived `log-pagination` change's design rationale, still referenced in `app.rs`'s doc comment).

**Bind `g`/`G` alongside the existing keys in both `input.rs` match arms**, not through a new abstraction - `handle_logs_tab_key` and `handle_modal_key` already match `KeyCode::Char(...)` per-key, so this is two added arms per function (`Char('g') => app.<...>_jump_to_start(...)`, `Char('G') => app.<...>_jump_to_end(...)`), consistent with how `d`/`u` were added.

**Scrollbar fix: change what `render_pagination_scrollbar` passes as `content_length`.** Instead of `ScrollbarState::new(len)`, compute the number of valid scroll-window positions - `let scroll_positions = len.saturating_sub(page_size) + 1;` (this equals `max_top + 1`, i.e. `top`'s actual value range) - and use `ScrollbarState::new(scroll_positions).position(top)`. `position` stays `top` unchanged; it already correctly reports the window's current top row, which is exactly what needs to range over `0..scroll_positions` for the thumb to reach both ends. `viewport_content_length(page_size)` is left in place since it only affects thumb *length* (proportional to page_size/len), not the position bug. Alternative considered: keep `content_length = len` and instead scale `position` up to `len - 1` (e.g. `position * (len - 1) / max_top`) - rejected as more convoluted and harder to reason about than fixing the input that's actually wrong; the chosen fix also matches how ratatui's own examples use `ScrollbarState` (`content_length` = number of valid positions, not raw item count) when a viewport is involved.

**Hint text: extend the existing literal strings, no new shared builder.** `logs_controls_hint`'s `"Page [d/u]"` and the modal's inline `"Logs  |  Page [d/u]"` both become `"Page [d/u]  Top/Bottom [g/G]"` (or equivalent short form) at their existing call sites - two small, independent edits, matching how these two strings are already independent today rather than introducing a shared hint-formatting function neither call site currently needs. The help modal's shortcut array gains one new line, e.g. `"g / G     jump to first / last entry (Logs tab, or the details modal's Logs pane)"`, phrased like the existing `d / PgDn` line it sits next to.

## Risks / Trade-offs

- [Widening the scrollbar's `content_length` formula could shift thumb *length*, not just position, since `viewport_content_length` interacts with `content_length` in ratatui's internal math] → Verified against ratatui's `part_lengths` formula: `thumb_length` depends on `viewport_length` and `max_viewport_position = content_length - 1 + viewport_length`, so shrinking `content_length` from `len` to `len - page_size + 1` changes `max_viewport_position` too; manually check the worked example from proposal research (`len=20, page_size=10`) renders correctly at both ends before merging, and spot-check a page_size=1 case (scrollbar still usable when nearly nothing fits).
- [Two independent hint-text edit sites could drift in wording if a future change touches one but not the other] → Accepted as consistent with the existing pattern (they're already two independent literals, not a shared function); low risk since both are one-line string edits reviewed together in this change's diff.

