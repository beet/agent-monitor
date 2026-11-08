## Context

`crates/agentmon/src/ui.rs`'s `log_entry_status_line` (and its wrapper `log_entry_line_with_pid`, used by the details modal's Logs pane) currently returns a single `(String, Style)` pair per log entry: one style covers the whole rendered line, including any appended quoted snippet. Both the top-level Logs tab (`render_logs_tab`) and the details modal's Logs pane build a `Cell` directly from that one `(text, style)` pair via `Cell::from(Span::styled(text, style))`. See proposal.md for why this needs to change.

Separately, `crates/agentmon/src/input.rs`'s `handle_modal_key` already has an `Enter` branch (under `ModalFocus::Logs`) that opens `app.open_prompt_detail(text)` for a prompt-category entry's `prompt` or a "done" agent entry's `response`. This is the mechanism a "started" entry's `Enter` handling needs to join.

## Goals / Non-Goals

**Goals:**
- Stop rendering a "started" entry's prompt snippet inline, in both the Logs tab and the details modal's Logs pane.
- Let `Enter` on a "started" entry (carrying a last-prompt snapshot) in the details modal's Logs pane open the same full-text modal already used for prompt entries and "done" entries.
- Render a "done" entry's quoted snippet (response, or its last-prompt fallback) in a color distinct from the status label's color, matching the unstyled color already used for a prompt-category entry's snippet and the Last Prompt pane.

**Non-Goals:**
- No change to what the daemon records on a `LogEntry` (`agentmon-proto`, `agentd`) - the last-prompt snapshot a "started" entry needs is already recorded today.
- No change to "needs input" entries' inline-snippet behavior, or to any entry's underlying data.
- No change to how the Last Prompt pane or prompt-category entries are styled - they're the reference color this change points "done" entries' snippets at, not something this change itself alters.

## Decisions

**Split row text into styled spans instead of one `(String, Style)`.** `log_entry_status_line` changes to return the label/duration portion and the quoted-snippet portion (when present) as separate `(String, Style)` pieces - e.g. `(String, Style, Option<(String, Style)>)`, or equivalently a small struct - rather than a single flattened string. Both call sites (`render_logs_tab`'s row-building closure and `log_entry_line_with_pid`) build their `Cell` from a `Line` of multiple `Span`s instead of one `Span::styled`. `log_entry_line_with_pid`'s pid suffix stays appended to the label span, not the snippet span, since it's reporting metadata, not the agent's own words.

Alternative considered: keep returning one concatenated string and apply a second `Style` by re-locating the quoted substring at render time (e.g. by byte offset). Rejected - it would require the rendering code to re-parse text it just assembled, and is more fragile against future formatting changes than returning the pieces already separated.

**The "unstyled" snippet color is `Style::new()` (no `fg` override), reusing the existing convention.** Prompt-category entries and the Last Prompt pane already render with no explicit foreground color (inheriting the terminal's default). Rather than inventing a new named color, a "done" entry's snippet reuses that same bare `Style::new()`, so "default foreground" uniformly means "the agent's/user's own words" everywhere it's used. This also means no new constant or theme value is introduced.

**"Started" entries keep recording a last-prompt snapshot; only presentation changes.** The daemon-side `LogEntry.prompt` field and the registry's carry-forward behavior are untouched. The Logs tab and the modal's Logs pane simply stop reading it into the row's inline text for a "started" status; `handle_modal_key`'s `Enter` branch gains a case that reads the same field to open the full-text modal, mirroring the existing "done"/`response` case.

**"Needs input" is intentionally left out of scope.** The proposal only asked to change "started" entries' inline snippet and "done" entries' snippet color; "needs input" keeps showing its snippet inline, in the status color, unchanged. Scoping it in would be a bigger behavioral change than requested and isn't needed for consistency - "needs input" is a prompt the user is actively being asked to look at, not a label the eye should skip past the way "started" now is.

## Risks / Trade-offs

- [Splitting the return type touches every existing caller/test of `log_entry_status_line` and `log_entry_line_with_pid`] → Both functions are internal to `ui.rs` (not part of any public API), so the signature change is a mechanical, compiler-checked update; existing tests that destructure `(text, style)` need updating to read the new shape, but none of this is behavior at risk of silently regressing.
- [A reader could expect "started" entries to have lost their prompt data entirely, since the row no longer shows it] → Mitigated by the `Enter`-to-full-text affordance this same change adds, plus the existing `[l]`-focused Logs pane already showing `Enter`'s availability via its normal selection/interaction model - no new hint text is introduced beyond what prompt/done entries already rely on.
