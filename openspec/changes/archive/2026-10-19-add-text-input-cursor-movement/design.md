## Context

Three free-text inputs exist in `crates/agentmon`, none with a cursor concept today - see proposal.md's "Why". All three follow the same shape: a `String` buffer, an "append char" method, and a "pop last char" method, invoked from `crates/agentmon/src/input.rs`:

- `App.agents_search_buffer` (Agents-tab `/`-search), rendered inline in the tab title (`ui.rs::agents_tab_title`) with a fake cursor - a trailing reversed `"_"` span always appended after the full text, since the title is a `Line` inside a `Block::title`, not an area `frame.set_cursor_position` can address.
- `RenameForm.name`, rendered in its own modal area with the real terminal cursor placed via `frame.set_cursor_position` at `label_width + name.chars().count()` - always the end.
- `ReminderForm.{name, duration_minutes}`, same real-cursor technique, computed per active field.

Project names and search text are free-form and may contain multi-byte UTF-8, which rules out naive byte-indexed cursor math.

Ratatui itself provides no text-editing widget - only `Frame::set_cursor_position`, which places the terminal's own cursor but carries no logic about moving through text, inserting, or deleting. The `tui-input` crate (0.15.5, `crossterm` feature) fills that gap: it owns a `String` plus a codepoint-counted cursor, exposes `value()`/`cursor()`/`visual_cursor()`/`visual_scroll()`, and its `backend::crossterm::EventHandler::handle_event(&Event) -> Option<StateChanged>` maps a crossterm event straight to the right internal edit (insert, delete, cursor move) using `unicode-segmentation`/`unicode-width`, so char-boundary handling is the crate's problem, not this app's. It depends on `crossterm ^0.29`, matching this project's pinned `crossterm = "0.29.0"` exactly, and lists `ratatui ^0.30.2` compatibility, matching this project's `ratatui = "0.30.2"`.

## Goals / Non-Goals

**Goals:**
- Replace all three inputs' hand-rolled append/pop-last logic with `tui_input::Input`, gaining cursor-aware editing without hand-written byte-offset math.
- Adopt `Input`'s full crossterm key mapping via `handle_event` rather than cherry-picking individual `InputRequest` variants, since that mapping (Left/Right, Home/End, word jumps, forward/backward delete, kill-word, kill-to-end) is exactly the line-editing behavior a terminal user already expects from any readline-style prompt, and reimplementing a subset of it by hand would be duplicate work for no behavioral benefit.

**Non-Goals:**
- Multi-line editing, text selection, or clipboard integration - `tui-input` doesn't provide these either, and this app has no field that needs them (every field here is single-line).
- Per-field cursor memory across a `Tab` switch in the reminder form. Per proposal.md, switching fields (or reopening a form) always resets the cursor to the end of that field's text, so each field's `Input` is freshly constructed (or re-seeded) on focus change rather than suspended and resumed.

## Decisions

**Each field's `String` becomes a `tui_input::Input`.** `App.agents_search_buffer: String` becomes `agents_search_buffer: Input`; `RenameForm.name: String` and both `ReminderForm` fields (`name`, `duration_minutes`) likewise. `Input::value()` returns `&str` for every place that read the old `String` (filtering, submit, validation), so call sites outside the key-handling and rendering paths are largely unaffected.

**Key handling forwards to `Input::handle_event`, with `Enter`/`Esc`/`Tab` and the Duration field's digit filter intercepted first.** Each of the three free-text key-handling functions in `input.rs` keeps its existing `Enter` (submit/commit), `Esc` (cancel), and - for the reminder form - `Tab` (switch field) arms exactly as today, since those are this app's own actions, not text editing. Everything else is forwarded as `field.handle_event(&Event::Key(key))`. For the reminder form's Duration field specifically, a `KeyCode::Char(c)` is only forwarded when `c.is_ascii_digit()`; any other character key is swallowed (ignored), preserving the existing "digits only" rule since `tui-input` itself has no concept of per-field character restrictions.

**Rendering reads `Input::value()` and `Input::visual_cursor()` for the real terminal cursor; the Agents-tab title's hand-drawn cursor uses `Input::cursor()` instead.** `render_reminder_form`/`render_rename_form` replace `field.chars().count()` with `field.visual_cursor()` (a display-column count) when computing the terminal cursor's column via `frame.set_cursor_position`, since that's the unit `set_cursor_position` itself needs. The Agents-tab title keeps its existing "fake cursor" technique (a reversed-style span, since a `Block` title isn't an addressable `frame.set_cursor_position` area) - but that code builds the before/at/after spans by splitting `agents_search_buffer.value().chars()`, so it indexes with `agents_search_buffer.cursor()` (a codepoint count) rather than `visual_cursor()`: `chars().take(n)` must be indexed in codepoints, and the two counts only diverge for wide characters, which this title has never needed to handle correctly either way.

**Scope is whatever `tui-input`'s crossterm mapping provides, not a hand-picked subset.** Accepting `handle_event` wholesale means `Home`/`End`/`Ctrl+Left`/`Ctrl+Right`/`Delete`/`Ctrl+W`/`Ctrl+K`/`Ctrl+U` all work as a direct consequence of the dependency, not as separately-specified features. The delta spec enumerates each as observable behavior so it's covered by scenarios and tests, rather than being an undocumented side effect of the library choice.

**`App`, `ReminderForm`, and `RenameForm` drop their `PartialEq`/`Eq` derives.** Discovered during implementation: `tui_input::Input` derives only `Default, Debug, Clone` - no equality impl - so the three structs holding one could no longer derive `PartialEq`/`Eq` either. Nothing in the crate compared a whole `App`/`ReminderForm`/`RenameForm` by equality (verified by grepping for `assert_eq!(app,` and any trait-bound usage before removing the derive) - every existing test already compared individual fields - so this has no observed behavioral effect; it's recorded here because it's a real, if small, surface change the proposal/design didn't anticipate.

## Risks / Trade-offs

- [New dependency adds a crate with its own release cadence and bug surface] → `tui-input` is small, has no transitive dependencies beyond `unicode-segmentation`/`unicode-width` (already pulled in by ratatui's own text-wrapping) and its optional backend crates, and its crossterm/ratatui version constraints already match this project's pinned versions exactly - no upgrade forcing function.
- [`tui-input`'s key mapping includes bindings (e.g. `Ctrl+U`, `Ctrl+K`) this project never asked for and hasn't tested for conflicts] → None of those chords are bound to anything elsewhere in `input.rs` within these three editing contexts (verified by reading the existing match arms in `handle_agents_search_editing_key`/`handle_rename_form_key`/`handle_reminder_form_key`), so nothing is shadowed; task 5 adds coverage for the newly-available bindings so a future change that wants to claim one of these chords for something else will see a failing test rather than silently breaking text editing.
- [The Agents-tab title's custom "fake cursor" rendering still needs hand-written span-splitting since `tui-input` has no opinion on rendering] → Scoped narrowly to one function (`agents_tab_title`); the delta spec's scenarios pin its expected behavior.
