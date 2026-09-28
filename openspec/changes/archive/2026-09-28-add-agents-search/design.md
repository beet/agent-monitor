## Context

The Agents tab (`crates/agentmon/src/app.rs`, `ui.rs`, `input.rs`) currently has no filter or free-text input concept; `directory_groups()` produces its row list directly with no filtering step. The Logs and Reminders tabs already have a filter concept, but theirs cycles through discrete values (`p`/`f`/`s` cycle Project/Status) rather than accepting free text - there's no existing free-text input widget to reuse. The `reminder_form` (`Option<ReminderForm>`) is the only existing free-text entry point in the TUI, but it's a full-screen modal form with multiple fields, not an inline, single-line, title-embedded prompt. See proposal.md - Why.

Global single-key shortcuts (`q`, `?`, `A`/`a`, `L`/`l`, `R`/`r`, `Tab`) are checked in `input.rs::handle_key` before dispatch reaches any per-tab handling (`crates/agentmon/src/input.rs:28-60`), and `reminder_form.is_some()` is checked early to divert all key handling into the form while it's open (`crates/agentmon/src/input.rs:141`). Search-editing mode needs the same kind of early diversion so that typing e.g. `a` or `R` into the search string doesn't jump tabs or open the reminder form instead.

## Goals / Non-Goals

**Goals:**
- Define the state machine for search-editing vs. applied-filter, including the cancel-restores-prior-filter behavior.
- Decide where key dispatch diverts into search-editing mode, so it composes cleanly with existing global shortcuts and the reminder form.
- Decide the matching semantics precisely enough to write scenarios and tests against.

**Non-Goals:**
- Extending this search pattern to the Logs or Reminders tabs (proposal explicitly scopes this to Agents only).
- Any daemon-side or wire-protocol change - all filtering is client-side over data the TUI already holds.
- Fuzzy or ranked matching - substring-only, per the proposal.

## Decisions

**State shape**: add `agents_search_editing: bool`, `agents_search_buffer: String` (live edit contents), and `agents_search_applied: Option<String>` (committed filter) to `App`. Three fields rather than a single enum keeps `agents_search_applied` trivially readable by the row-filtering step regardless of edit state, matching the existing `logs_filter_project`/`logs_filter_status`-style plain fields rather than introducing a new enum-based pattern this codebase doesn't otherwise use for filter state.

**Entering edit mode seeds the buffer from the applied filter**: `open_agents_search()` sets `agents_search_buffer = agents_search_applied.clone().unwrap_or_default()` and `agents_search_editing = true`. This gives "start from the current filter's text" (proposal) for free, and makes `Escape`'s job simply "discard the buffer, don't touch `agents_search_applied`".

**Commit/cancel**: `Enter` sets `agents_search_applied = if buffer.is_empty() { None } else { Some(buffer) }` and clears `agents_search_editing`. `Escape` clears `agents_search_editing` only, leaving `agents_search_applied` exactly as it was (whether `None` or a prior `Some`) - it never touches the buffer's owning field, so there's nothing to "restore" beyond not committing.

**Key dispatch order**: in `input.rs::handle_key`, check `app.agents_search_editing` immediately after the existing `reminder_form.is_some()` early-return (`input.rs:141`) and before the global `q`/`?`/`A`/`L`/`R`/`Tab` block, mirroring how the form diverts all input. While editing: `Char(c)` appends, `Backspace` pops, `Enter` commits, `Escape` cancels; every other key (including arrows, `Tab`) is swallowed rather than falling through, consistent with how the reminder form already swallows keys it doesn't recognize. `/` itself is only handled to *enter* edit mode, from the Agents-tab-specific key block alongside `j`/`k`, and only takes effect when `app.active_tab == Tab::Agents` and not already editing.

**Filtering step**: `visible_agent_groups()` applies a case-insensitive substring filter over `project_name(&group.cwd)` at the same point `directory_groups()` is consumed for rendering (`ui.rs::render_agent_table`) and for selection clamping (`agents_selected`), the same pattern `visible_logs`/`visible_reminders` already use for their own tabs - keeping `directory_groups()`'s unfiltered output available for anything that still needs full membership (there's currently nothing that does, but this matches the Logs/Reminders precedent of keeping the raw source separate from the filtered view). Which string it filters by depends on edit state: while `agents_search_editing` is true it filters by the live `agents_search_buffer` (so every keystroke narrows the list immediately, per the "updates immediately after every such change" requirement); otherwise it filters by the committed `agents_search_applied`. An empty filter string (a fresh, not-yet-typed-into edit, or a cleared applied filter) needs no special case - every project name contains the empty substring, so the check alone already yields every row.

**Selection clamping**: reuse the existing `clamp_selection`-style helper already used elsewhere (`app.rs:48`) so `agents_selected` stays in range whenever the filtered set shrinks - same mechanism the Logs/Reminders filters already rely on.

**Title rendering**: `render_agent_table`'s title gains a third state alongside today's `banner`-driven `"Agents"` / `"Agents - {banner}"`: while editing, render `"Agents /{buffer}█"` (or a styled cursor block) as the whole title; otherwise render `"Agents  Filter [/]"`, matching the `<Word> [<key>]` convention `logs_controls_hint`'s other bracketed hints (`Project [p]`, `Status [s]`, `Clear [c]`) already use - and, per that same convention, the hint stays present even once a filter is applied rather than being replaced by it: when `agents_search_applied` is `Some`, append `": {filter}"` after the hint, with the filter span styled bold in a distinct color (mirroring `tab_style`'s existing yellow/bold convention in `render_tab_bar`).

## Risks / Trade-offs

[Typing a character that's also a global shortcut (e.g. `a`, `R`, `q`) while editing could leak through if the dispatch-order change is done incorrectly] → covered by the "key dispatch order" decision above, verified with a test that types each of `q`/`?`/`A`/`L`/`R`/`Tab` into the search buffer and asserts none of them fire their global effect while `agents_search_editing` is true.

[Substring match against `project_name()` rather than the raw working-directory path could surprise a user searching for a path segment that got stripped by name-derivation] → accepted; the proposal is explicit that matching is against "the project name (working directory, currently)" as already displayed in the PROJECT column, so what's typed matches what's shown.
