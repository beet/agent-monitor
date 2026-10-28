## Context

See proposal.md for motivation. Today `render_details_modal` (`crates/agentmon/src/ui.rs:814-958`) lays out the modal as `[Ratio(1,3), Length(LAST_PROMPT_PANE_HEIGHT=4), Min(0)]`, with the top third split into three equal-width columns (Agents, Tests, Reminders). `ModalFocus` (`crates/agentmon/src/app.rs:183-190`) has exactly two variants, `Logs` and `Reminders`, toggled by `toggle_modal_focus`. The Logs and Reminders panes already use a shared `Paginator` (`crates/agentmon/src/app.rs:20-105`) for selection/scroll state, a `needs_pagination` helper, and `render_pagination_scrollbar` for the scrollbar widget; this change reuses all three for the Agents and Tests panes rather than inventing a second pattern. No pane currently sets `Block::border_style`, except the two confirmation dialogs (bold red).

## Goals / Non-Goals

**Goals:**
- Give the Agents, Tests, and Reminders panes a shared, content-driven height, capped at one third of the modal, reusing the existing `Paginator`/scrollbar machinery once a pane's content exceeds that cap.
- Extend the modal's focus model from 2 panes to all 5, via `Tab` cycling and direct single-key shortcuts, with a visible border-color cue for whichever pane holds focus.
- Give the Last Prompt pane an `Enter` action (open the existing full-prompt modal) without changing its own layout or content behavior.

**Non-Goals:**
- Changing the Last Prompt pane's height, wrapping, or truncation behavior (it stays a fixed 4-row strip - confirmed with the user).
- Adding `Enter` actions to the Agents or Tests panes' highlighted row (deferred; this change only wires the no-op).
- Changing anything about the Logs or Reminders panes' existing pagination/selection/action behavior beyond adding the focus border and a `[l]`/`[r]` title hint.

## Decisions

### Row height formula
Each of the three top panes has a `content_lines` count known before layout (its row/placeholder count - e.g. `group.agents.len().max(1)`, always `1` for the Tests pane today since it shows only the latest test run, `app.project_reminders(cwd).len().max(1)`). Define that pane's desired inner height as:
```
desired(content_lines, cap) = if content_lines <= 1 { 2 } else { min(content_lines, cap) }
```
`2` covers the "one content line + one blank line beneath" default (matching the Last Prompt pane's existing collapsed look); above that, height tracks content up to `cap`. The row's actual shared inner height is `max` of the three panes' `desired(...)`, which by construction is already `<= cap`. `cap = modal_inner_height / 3`, computed the same way `Constraint::Ratio(1,3)` already carves out the top third today, so the cap doesn't change even though the row is usually shorter than it.

Alternative considered: give each pane an independent height (no shared-row constraint). Rejected - the request explicitly asks for the three to "retain the same height" so the row reads as one aligned strip rather than three independently jittering panes.

### Layout mechanics
Replace `Constraint::Ratio(1, 3)` for the top row with `Constraint::Length(row_inner_height + 2)` (the `+2` is the row's own top/bottom border), computed from the content counts above before the vertical `Layout::split` call - mirroring how `logs_page_size` is already computed from `Min(0)`'s resolved rect today, just inverted (here the height is computed first and fed into the constraint, rather than read back after). `Last Prompt` keeps `Constraint::Length(LAST_PROMPT_PANE_HEIGHT)` unchanged; `Logs` keeps `Constraint::Min(0)` and absorbs whatever the top row's height change takes from or gives back to it, which is already its existing behavior (its share already varies with the Last Prompt strip's fixed height).

### Pagination state and page size
Add `modal_agents_pagination` and `modal_tests_pagination` fields to `App` (same `Paginator` type as `modal_logs_pagination`/`modal_reminders_pagination`), and `modal_agents`/`modal_tests` fields to `PageSizes`. The page size passed for both is the shared row's resolved inner height (not each pane's own `desired(...)` - they're identical by construction, but the row's actual rendered height is the source of truth, same principle as `design.md`'s existing "`page_size` is a parameter" decision for Logs/Reminders). `needs_pagination(content_lines, row_height)` gates the scrollbar exactly as it does today for Logs/Reminders; per the proposal, neither pane's title ever shows a keyboard hint regardless of this result - only Reminders' existing `New [R]` hint and each pane's new focus-shortcut hint appear in the title.

### Focus model
Extend `ModalFocus` to `{ Agents, Tests, Reminders, LastPrompt, Logs }`. Replace `toggle_modal_focus`'s two-arm match with a 5-arm cycle in the fixed order the panes are laid out (Agents → Tests → Reminders → LastPrompt → Logs → Agents), and add `set_modal_focus(ModalFocus)` for the direct shortcut keys. In `handle_modal_key`, add `a`/`t`/`r`/`p`/`l` arms alongside the existing `R`/`n` top-level bindings (confirmed collision-free against every existing modal binding), each calling `set_modal_focus`. The existing `tab_never_focuses_the_non_interactive_last_prompt_pane` test (`crates/agentmon/src/input.rs:1613-1647`) asserts the old 2-variant premise and will be replaced by a test asserting the opposite - that `Tab` does reach all five panes in order.

Default focus on open (Logs for Agents-tab/Logs-tab entry, Reminders for Reminders-tab entry) is unchanged - the five-way cycle and shortcuts are additive to how focus starts, not how it starts.

### Reverse cycling (`Shift+Tab`)
Crossterm delivers `Shift+Tab` as its own distinct keycode, `KeyCode::BackTab`, not as `KeyCode::Tab` plus a `SHIFT` modifier flag - so this is a separate match arm, not a modifier check on the existing one. Add `toggle_modal_focus_reverse(&mut self)` to `App`, mirroring `toggle_modal_focus`'s 5-arm match but walking the same fixed order backward (Agents ← Tests ← Reminders ← LastPrompt ← Logs ← Agents, i.e. `Agents -> Logs`, `Tests -> Agents`, `Reminders -> Tests`, `LastPrompt -> Reminders`, `Logs -> LastPrompt`). In `handle_modal_key`, add a `KeyCode::BackTab` arm alongside the existing `KeyCode::Tab` arm, calling `app.toggle_modal_focus_reverse()`. No change to `set_modal_focus` or the shortcut keys - this only adds a second direction to the existing cycle.

### Per-pane keyboard interaction
`handle_modal_key`'s per-focus match grows from 2 arms to 5:
- `Agents`/`Tests`: `j`/`k`/`d`/`u`/`g`/`G` drive the new paginators; `Enter` is a no-op (explicit `_ => {}`-style arm, not simply unhandled, so it reads as a deliberate decision rather than an oversight).
- `Reminders`/`Logs`: unchanged.
- `LastPrompt`: no navigation keys (nothing to navigate - it's still non-interactive content). `Enter` calls `app.open_prompt_detail(text)` with the pane's current text, skipped when that text is the `NO_PROMPT_CAPTURED_PLACEHOLDER` sentinel (`crates/agentmon/src/ui.rs:965`) - the same text the render path already computes, so the input-handling side needs the same "most recently updated agent's last prompt" lookup `render_last_prompt_pane` does; this is the one place this change duplicates a lookup across `ui.rs`/`app.rs` rather than threading pre-rendered state through `InputAction`, consistent with how the Logs pane's `Enter` handler (`input.rs:317-329`) already re-derives its entry from `app.project_logs(&cwd)` rather than reading anything the last render produced.

### Border color
Add a small helper, e.g. `fn pane_border_style(focused: bool) -> Style`, returning `Style::default()` when focused and `Style::new().fg(Color::DarkGray)` when not - reusing the `Color::DarkGray` already used elsewhere in `ui.rs` for dimmed/placeholder text, rather than introducing a new color. Applied via `.border_style(...)` on all five pane `Block`s. This requires threading `app.modal_focus` into `render_last_prompt_pane` (which doesn't take it today) and the new Agents/Tests pane renderers, alongside the existing Reminders/Logs renderers which already read `app.modal_focus` for selection and now also use it for border color.

### Title hints and bottom-left modal hint
Title strings gain a bracketed shortcut hint: `"Agents [a]"`, `"Tests [t]"`, `"Reminders [r]  |  New [R]"`, `"Last Prompt [p]"`, and `"Logs [l]"` (or `"Logs [l]  |  Page [d/u]  Top/Bottom [g/G]"` when paginated, per the existing pattern). The modal's own `Block` gains a bottom title (ratatui 0.30's `Block::title_bottom`, left-aligned) reading `"<tab> focus pane"`, alongside its existing top title (`"Details - <project>  Rename [n]"`).

## Risks / Trade-offs

- **Row height recomputation happens before the frame's `Layout::split`, same as today's `Ratio(1,3)`, but now depends on live content counts** → no new risk versus today: `logs_page_size`/`reminders_page_size` are already computed this way (content informs a constraint, which is then split and read back for rendering); this just adds a data dependency (content counts) that wasn't there when the constraint was a flat ratio. Mitigated by computing all three content counts up front, before any `Layout` call, exactly mirroring the existing `let group = ...` / `let reminders = ...` lookups already done early in `render_details_modal`.
- **A one-frame visual "jump" when the row's height changes** (e.g. an agent is added, pushing `content_lines` past the collapsed default) → acceptable; this is the explicit request ("expanding/collapsing dynamically"), not a bug, and matches how the Logs pane's own content length already varies frame to frame today.
- **Five-arm focus match duplicates structure across `ui.rs` (rendering) and `input.rs` (key handling)** → mitigated by keeping the pane order (`Agents, Tests, Reminders, LastPrompt, Logs`) as a single source of truth comment near `ModalFocus`'s definition, so `toggle_modal_focus`'s cycle, the title-hint order, and the layout order all visibly derive from the same list.

## Migration Plan

None - this is a TUI-only behavior change with no persisted state, wire format, or data migration involved. It ships as a normal `agentmon` release.
