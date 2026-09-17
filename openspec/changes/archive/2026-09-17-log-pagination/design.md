## Context

Two lists need real pagination: the Logs tab (`render_logs_tab` in `crates/agentmon/src/ui.rs`, state in `App`) and the details modal's Logs pane (`render_details_modal`, currently a plain, non-stateful `Table` with no selection or scroll at all). Today the Logs tab's `d`/`u` only jump `logs_selected` by the fixed `LOGS_PAGE_SIZE` (10) and rely on Ratatui's `TableState` auto-scroll, which scrolls the minimum amount to keep the selected row visible - not a page-aligned window. `App` is deliberately terminal-independent ("Pure application state... decoupled from the terminal... so it can be tested without either", `app.rs:67-68`), so page geometry cannot live inside `App` as a fixed constant if it's going to reflect the real rendered height. See proposal.md for motivation; see specs/agent-monitor-tui/spec.md for the target behavior.

## Goals / Non-Goals

**Goals:**
- One shared pagination model (state + transition logic) used by both the Logs tab and the modal's Logs pane, so the overlap-by-1 paging, scrollbar, and heading-hint behavior stay identical by construction rather than by convention.
- Page size computed from the pane's actual rendered inner height at draw/input time, not a fixed constant.
- Keep `App`'s paging logic unit-testable without a terminal, per its existing design.

**Non-Goals:**
- No change to sort/filter behavior, tree-branch prefixes, timestamp alignment, or any other Logs-pane rendering already covered by existing requirements.
- No drill-down interaction from the modal's Logs pane (no `Enter` on a log row); its selection exists only to anchor pagination and the highlight, not to trigger an action.
- No persistence of scroll position across tab switches or modal open/close - both reset to the top, matching current `logs_selected`/modal-open reset behavior.

## Decisions

**A shared `Paginator` (or similarly named) helper type, not two parallel implementations.** Both lists need identical semantics (line move, page move with 1-row overlap, clamping, scrollbar state derivation, hint visibility). Implementing it once in `app.rs` as a small struct - e.g. `{ selected: usize, len }` with methods `move_by(delta)`, `page(direction, page_size)`, `needs_pagination(page_size)` - and using one instance for `logs_selected`'s state and a second instance for the modal's own Logs-pane selection avoids the two lists drifting apart the way the modal's Logs pane already drifted from the Logs tab (no scroll support at all). Alternative considered: duplicate the logic inline in each call site, matching today's ad hoc `move_logs_selection`/`page_logs` free functions - rejected because it's exactly the kind of duplication that let the modal's pane fall behind.

**`page_size` is a parameter, computed by the UI layer, not a constant.** `render_logs_tab` and the modal's Logs-pane renderer each know their own inner `Rect` height at draw time; they pass `inner.height` (minus 1 if a header row is rendered) into the paginator's `page()` call and into `App`'s input-handling entry points. `input.rs`'s key handlers (`page_logs`, and the new modal equivalent) take `page_size: usize` as an argument supplied by the caller (`main`'s event loop, which has the last-rendered layout) rather than reading a constant. `LOGS_PAGE_SIZE` is removed; tests that need a page size construct one explicitly (e.g. `10`), preserving `App`'s terminal-independent testability. Alternative considered: keep `LOGS_PAGE_SIZE` fixed and just add the scrollbar/hint - rejected per the user's explicit choice that page size should reflect the rendered height, and because a fixed page size makes the scrollbar's position lie about how much is really on screen once the terminal is resized.

**Overlap-by-1 semantics: `new_top = old_top ± (page_size - 1)`, selection snaps to the new top row.** This matches conventional pager behavior (`less`, vim's `Ctrl-F`/`Ctrl-B`) where one line of continuity carries across the page boundary so the user doesn't lose their place. Clamping: `new_top` is bounded to `[0, len.saturating_sub(page_size)]` so the last page always shows a full page's worth of rows ending at the last entry (or all remaining rows if fewer than a page remain, matching the existing "clamps instead of overshooting" scenario).

**Modal's Logs pane gets its own `Paginator` instance, scoped to the modal.** Stored on `App` alongside `modal: Option<Modal>` (e.g. as a field reset whenever `open_details_modal` runs), so switching projects or reopening the modal starts at the top, consistent with today's non-scrolled Logs pane always showing from the top.

**Input routing: extend the modal branch in `input.rs::handle_key`, don't touch the tab branch.** Today `if app.modal.is_some() { only Esc/q }` - this already guarantees no leakage to the tab underneath (verified by the existing `non_esc_keys_are_ignored_while_a_modal_is_open` test). The change adds `j`/`k`/`d`/`u`/`PageDown`/`PageUp` handling inside that branch, dispatching to the modal's own paginator. No change to the "modal always wins" short-circuit structure itself - it already provides the precedence the spec calls for.

**Scrollbar: Ratatui's built-in `Scrollbar`/`ScrollbarState`, default symbols.** Rendered only when `paginator.needs_pagination(page_size)` (i.e., `len > page_size`), positioned via `Scrollbar::new(ScrollbarOrientation::VerticalRight)` over the pane's own area so it overlays the right border, matching how Ratatui examples typically place it. `ScrollbarState` is derived fresh each frame from `(len, page_size, selected)` - no persistent widget state beyond what's already in the paginator.

**Heading hint: appended to the existing title string, only when paginated.** The Logs tab's title-building (`logs_controls_hint`) gains a `d/u page` segment appended when `entries.len() > page_size`; the modal's "Logs" pane title gains the same segment under the same condition, and only that segment (it has no sort/filter hints to sit alongside).

## Risks / Trade-offs

- **Page size depends on render order.** Because `page_size` comes from the last-rendered inner height rather than being known before the first frame, the very first keypress before any draw has occurred could use a stale or default value. Mitigation: seed the event loop's known page size from a first `frame.area()` computed before entering the input loop (the app already draws once before reading input in a typical Ratatui main loop), and reuse the size from the immediately preceding frame otherwise - a one-frame lag on resize is already how the rest of the TUI behaves (e.g. column widths).
- **Shared `Paginator` risks becoming a leaky abstraction if the two lists' needs diverge later** (e.g. if the modal's Logs pane ever needs `Enter`-to-drill-down). Mitigation: keep the type small and focused on index/page math only; rendering (scrollbar, highlight style, hint text) stays in `ui.rs`, not baked into the shared type.
- **Existing tests hardcode `LOGS_PAGE_SIZE`.** Removing the constant breaks `app.rs` and `input.rs` tests that reference it directly (`page_logs_moves_by_a_full_page_clamped`, `d_and_u_page_the_logs_list`). Mitigation: update those tests to pass an explicit page size, which also documents the new parameterized signature.
