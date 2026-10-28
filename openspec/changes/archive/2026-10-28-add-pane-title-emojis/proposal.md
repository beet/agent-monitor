## Why

The details modal's five panes (Agents, Tests, Reminders, Last Prompt, Logs) are told apart only by their text titles. Each pane's category already has an established emoji marker used elsewhere in the TUI (🤖 for agent status, 🧪 for test status, ⏰ for reminder status, 🗣️ for prompt text) - prefixing each pane's title with its own marker makes the panes easier to scan at a glance and reuses markers the user already associates with that category, instead of introducing new iconography.

The same categories also appear outside the modal - the top-level tab bar, each tab's own table title, and some of the Agents tab's column headers - and are just as easy to tell apart at a glance with the same treatment, using the exact same markers so a category reads the same emoji everywhere it appears in the TUI.

## What Changes

- Prefix each details-modal pane title with an emoji, reusing the same marker already shown elsewhere in the TUI for that category:
  - Agents pane: `🤖 Agents [a]`
  - Tests pane: `🧪 Tests [t]`
  - Reminders pane: `⏰ Reminders [r]  |  New [R]`
  - Last Prompt pane: `🗣️ Last Prompt [p]`
  - Logs pane: `🪵 Logs [l]` (and `🪵 Logs [l]  |  Page [d/u]  Top/Bottom [g/G]` when paginated) - logs have no pre-existing marker of their own, so this introduces a new one (a literal "log")
- Prefix the top-level tab bar's three labels with the same markers: `🤖 Agents [a]`, `🪵 Logs [l]`, `⏰ Reminders [r]`.
- Prefix each tab's own table title with the same markers: the Agents tab's title starts with `🤖 Agents` (in all three of its states - idle, live search-editing, and filtered), the Logs tab's heading starts with `🪵 Logs - ...`, and the Reminders tab's heading starts with `⏰ Reminders - ...`.
- Prefix the Agents tab's `AGENTS`, `TESTS`, and `REMINDERS` column headers with 🤖, 🧪, and ⏰ respectively. `PROJECT` and `UPDATED` keep no prefix (no established category marker of their own), and the Logs tab's and Reminders tab's own column headers (`STATUS`, `CATEGORY`, `PROJECT`, `TIME`, `NAME`, `DURATION`) are unchanged - none of them names a category with an existing marker.
- Keep every title's and header's existing keyboard-shortcut hints, filter/search state, and content unchanged; the emoji is a prefix only.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: the "Project details modal" requirement's pane titles, the "Tab navigation between Agents and Logs" requirement's tab-bar labels, the "Agents tab title reflects search state" requirement's title, the "Logs tab shows an aggregated, paginated activity list" and "Reminders tab shows an aggregated, paginated reminder list" requirements' headings, and the "Agents tab splits agent and test-run status into separate columns" and "Agents tab shows a Reminders column" requirements' column headers all gain emoji prefixes.

## Impact

- `crates/agentmon/src/ui.rs`:
  - The five `render_*_pane` functions that set each details-modal pane's `Block` title (`render_agents_pane`, `render_tests_pane`, `render_reminders_pane`, `render_last_prompt_pane`, and the Logs title built in `render_logs_pane`/wherever `logs_title` is assembled).
  - `render_tab_bar`, where the top-level tab labels are built.
  - `agents_tab_title`, where all three of the Agents tab's title variants are built.
  - `render_logs_tab` and `render_reminders_tab`, where each tab's own heading and column header `Row` are built.
  - `render_agent_table`, where the Agents tab's column header `Row` is built.
- Existing tests in `crates/agentmon/src/ui.rs` that assert on exact title/header text (e.g. `find_text_from(buffer, "Agents", ...)`, `text.contains("AGENTS")`) will need their expected strings checked against the new emoji prefixes.
