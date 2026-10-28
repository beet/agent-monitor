## Why

The details modal's Agents, Tests, and Reminders panes are each fixed at one third of the modal's top third, even though they usually hold only a line or two of content - wasting space that the Logs pane, typically the longest-running list, could use instead. At the same time, only the Logs and Reminders panes can be reached with `Tab` or acted on with `Enter`; the Agents and Tests panes are inert, and there's no way to jump straight to any specific pane without cycling `Tab`, nor any visual cue for which pane currently holds focus beyond the selected-row highlight inside it.

## What Changes

- The Agents, Tests, and Reminders panes collapse to a single content line (plus one blank line beneath, matching the Last Prompt pane's existing look) by default, and grow dynamically up to a cap of one third of the modal's height when their content needs more room.
- All three of those panes always share the same height: whichever one needs the most room (up to the cap) sets the height for all three on that frame.
- Once a pane's content exceeds its current height, it paginates using the same `Paginator` pattern the Logs and Reminders panes already use (scrollbar, `j`/`k`/`d`/`u`/`g`/`G`), but - like the Reminders pane's existing exception - never shows a pagination keyboard-hint in its title; only a scrollbar indicates more content.
- **BREAKING**: `Tab` now cycles keyboard focus through all five panes (Agents, Tests, Reminders, Last Prompt, Logs) instead of only toggling between Logs and Reminders. `ModalFocus` gains three variants.
- Holding `Shift` while pressing `Tab` (`Shift+Tab`, delivered as `BackTab`) cycles focus through the same five panes in the opposite direction.
- Each pane gains a single-key shortcut to focus it directly while the modal is open: `a` (Agents), `t` (Tests), `r` (Reminders), `p` (Last Prompt), `l` (Logs) - shown as a hint in that pane's title.
- `Enter` on the Agents and Tests panes' highlighted row has no effect (reserved for future use). `Enter` on the Last Prompt pane opens the existing full-prompt modal with that pane's text (new entry point into the "Prompt entry full-text modal" requirement; a no-op when the pane shows its "no prompt captured" placeholder). `Enter` on the Reminders and Logs panes is unchanged.
- The focused pane's border renders in the current (default) border color; every other pane's border renders in a dimmer color, so focus is visible without looking inside the pane for a highlighted row.
- The modal's bottom-left border shows a `<tab> focus pane` hint.
- Last Prompt's own layout and content behavior (fixed-height strip, word-wrap-and-truncate, no pagination) is unchanged - it only gains Tab-reachability, a focus shortcut/hint, the border-color treatment, and the new `Enter` action.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the "Project details modal" requirement's pane layout (dynamic height for Agents/Tests/Reminders, focus cycling across all five panes, per-pane focus shortcuts, border-color focus indicator, bottom-left hint, Last Prompt's new `Enter` action) and the "Paginated lists support keyboard navigation" requirement (extending it to the Agents and Tests panes, and to the Reminders-pane-style no-title-hint exception).

## Impact

- `crates/agentmon/src/ui.rs`: `render_details_modal` and its pane-rendering helpers (height computation/layout, new `render_agents_pane`/`render_tests_pane` paginated renderers, border-style-by-focus, title hints, bottom-left modal hint).
- `crates/agentmon/src/app.rs`: `ModalFocus` enum (5 variants), `toggle_modal_focus`, new `toggle_modal_focus_reverse`, new `Paginator` state for the Agents and Tests panes, new direct-focus setters, Last Prompt `Enter` handling plumbed into `open_prompt_detail`.
- `crates/agentmon/src/input.rs`: `handle_modal_key` - per-pane key routing extended from 2 arms to 5, new `a`/`t`/`r`/`p`/`l` shortcut handling, new `KeyCode::BackTab` arm, Last Prompt `Enter` handler, removal/rework of the `tab_never_focuses_the_non_interactive_last_prompt_pane` test (its premise no longer holds).
- `openspec/specs/agent-monitor-tui/spec.md`: delta to the two requirements named above.
