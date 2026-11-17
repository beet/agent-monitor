## MODIFIED Requirements

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) and test-run statuses (running, passed, failed) from one another so the user can scan the list and immediately identify agents or test runs needing attention. Each agent status SHALL be prefixed with an emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🤖, done with 🤖, stale with 👻, and declined with 🚫; needs input (Yellow, bold) and done (Green) share the 🤖 marker and are distinguished from each other, and from every other agent status, by color rather than by a unique emoji. Each test-run status SHALL be prefixed with a distinct emoji marker: running with ⏳, passed with ✅, and failed with ❌. A reminder's status SHALL reuse these existing emoji markers rather than introducing new ones: running reuses ⏳ (the same marker as a running test run) and done reuses ✅ (the same marker a passed test run uses), so a reminder's status is instantly recognizable by an emoji the user already associates with "in progress" or "finished". In the Agents tab's Reminders column, a reminder's entire status segment - its emoji, its name, and its trailing elapsed/ETA/outcome text - SHALL be colored as one unit: the same Blue used for a running test run while running, and the same Green used for a passed test run while done; this matches how the Agents and Tests columns already color their entire status text rather than only a leading emoji, and how the Reminders tab and the details modal's Reminders pane already color a reminder's full status text. In the Logs tab and the details modal's Logs pane, a reminder-category log entry SHALL instead use a single fixed marker, ⏰, regardless of whether its status is started, stopped, or finished - unlike agent and test-run entries, which use a different emoji per status - since a reminder log entry's value is in marking it as a reminder event at a glance, not in re-deriving its status from an already text-labeled entry. Its color SHALL still vary by status, though: "finished" (a natural completion) SHALL use the same Green used for a "done" agent or a "passed" test run; "stopped" (a manual interruption) SHALL use a color distinct from both "started" and "finished", since it is neither the beginning nor the successful end of a run. The Logs tab and the details modal's Logs pane each additionally render two log-only statuses with no live Agents-tab equivalent, and one log-only category with no status distinction at all: an agent-category "started" entry SHALL use the 🤖 marker in the same Blue used for a running test run, distinguished from "done" and "needs input" by that color, consistent with the Agents tab's agent markers sharing 🤖 and differing only by color; a test-run-category "started" entry SHALL use a dedicated 🧪 marker in that same Blue, distinct from the live "running" test-run status's ⏳ marker; and a prompt-category entry SHALL use a dedicated 💬 marker with no foreground color override of its own (unlike every other category, whose marker is always paired with a status-specific color). When a project's Agents cell reflects more than one tracked agent in different statuses, the cell SHALL show each distinct status present rather than collapsing them into a single "winning" status, so no status needing attention is hidden behind another. On the currently-selected row in the Agents tab, the Logs tab, or the Reminders tab, the TUI SHALL override every status's own foreground color with a single fixed foreground color chosen to stay legible against the row-highlight background, rather than requiring the row-highlight background to avoid every status's own color.

#### Scenario: An agent needs input
- **WHEN** an agent's status is "needs input"
- **THEN** that status's contribution to the row displays the 🤖 marker and is visually distinguished (e.g. color) from other statuses shown, using bold colored text rather than a solid background fill

#### Scenario: Each status has a distinct emoji marker
- **WHEN** the TUI renders an agent's status within a project row
- **THEN** that status is prefixed with the emoji for that status (🔧 running, 💤 idle, 🤖 needs input, 🤖 done, 👻 stale, 🚫 declined), with needs input (Yellow, bold) and done (Green) told apart by color since they share the 🤖 marker

#### Scenario: The stale marker renders full-width
- **WHEN** the TUI renders an agent's "stale" status
- **THEN** it uses the 👻 marker rather than 🕸️, since 👻 renders reliably full-width across terminals and does not corrupt the row-highlight background the way 🕸️ did

#### Scenario: A declined permission is visually distinguished
- **WHEN** an agent's status is "declined"
- **THEN** that status's contribution to the row displays the 🚫 marker and is visually distinguished (e.g. color) from other statuses shown

#### Scenario: A failing test run is visually distinguished
- **WHEN** a test run's status is "failed"
- **THEN** that status's contribution to the row displays the ❌ marker and is visually distinguished (e.g. color) from other statuses shown

#### Scenario: Each test-run status has a distinct emoji marker
- **WHEN** the TUI renders a test run's status within a project row
- **THEN** that status is prefixed with the emoji for that status (⏳ running, ✅ passed, ❌ failed)

#### Scenario: A project with multiple agent statuses shows each one
- **WHEN** a project has two or more tracked agents whose statuses differ from one another
- **THEN** the row's Agents cell lists each distinct status present among those agents (for example, both running and needs input), rather than showing only one

#### Scenario: An agent status and a test-run status are shown together
- **WHEN** a project has both a tracked agent and a test run whose statuses differ
- **THEN** the row's Agents cell shows the agent's status and the row's Tests cell shows the test run's status, each in its own column rather than sharing one

#### Scenario: A running status stays legible on the selected row
- **WHEN** a project row showing the "running" status is the currently-selected (highlighted) row in the Agents tab
- **THEN** the status's label and emoji remain visible, rendered in the row's fixed selected-row foreground color rather than the status's own color

#### Scenario: A selected row's text overrides every status's own color
- **WHEN** any row in the Agents tab, the Logs tab, or the Reminders tab is the currently-selected (highlighted) row, regardless of which status or statuses it shows
- **THEN** all of that row's status text renders in the same fixed selected-row foreground color, rather than in each status's own color

#### Scenario: A running reminder reuses the running test-run emoji
- **WHEN** the TUI renders a reminder whose status is running, in the Reminders tab, the Agents tab's Reminders column, or the details modal's Reminders pane
- **THEN** that status displays the ⏳ marker, the same marker a running test run uses

#### Scenario: A done reminder reuses the done/passed emoji
- **WHEN** the TUI renders a reminder whose status is done, in the Reminders tab, the Agents tab's Reminders column, or the details modal's Reminders pane
- **THEN** that status displays the ✅ marker, the same marker a passed test run uses

#### Scenario: A running reminder's full Agents-tab segment is colored blue
- **WHEN** the Agents tab's Reminders column renders a reminder whose status is running
- **THEN** the whole segment - the ⏳ marker, the reminder's name, and its elapsed/ETA text - is colored the same Blue used for a running test run, not only the marker

#### Scenario: A done reminder's full Agents-tab segment is colored green
- **WHEN** the Agents tab's Reminders column renders a reminder whose status is done
- **THEN** the whole segment - the ✅ marker, the reminder's name, and its elapsed/outcome text - is colored the same Green used for a passed test run, not only the marker

#### Scenario: A reminder log entry always shows the clock marker
- **WHEN** the Logs tab or the details modal's Logs pane renders a log entry whose category is reminder
- **THEN** that entry displays the ⏰ marker regardless of whether its status is started, stopped, or finished

#### Scenario: A finished reminder log entry is green
- **WHEN** the Logs tab or the details modal's Logs pane renders a reminder log entry whose status is finished
- **THEN** that entry's text is colored the same Green used for a "done" agent or a "passed" test run

#### Scenario: A stopped reminder log entry is visually distinct from started and finished
- **WHEN** the Logs tab or the details modal's Logs pane renders a reminder log entry whose status is stopped
- **THEN** that entry's text is colored differently from both a "started" and a "finished" reminder log entry

#### Scenario: An agent-started log entry shares the robot marker, told apart by color
- **WHEN** the Logs tab or the details modal's Logs pane renders an agent-category log entry whose status is "started"
- **THEN** that entry displays the 🤖 marker in Blue, the same marker (in a different color) used by that agent's later "done" (Green) or "needs input" (Yellow, bold) entry

#### Scenario: A test-run-started log entry uses its own marker
- **WHEN** the Logs tab or the details modal's Logs pane renders a test-run-category log entry whose status is "started"
- **THEN** that entry displays the 🧪 marker in Blue, distinct from the ⏳ marker a live "running" test-run status uses

#### Scenario: A prompt log entry's marker carries no color
- **WHEN** the Logs tab or the details modal's Logs pane renders a prompt-category log entry
- **THEN** that entry displays the 💬 marker and its text renders in the default (unstyled) foreground, unlike every other log-entry category

### Requirement: Project details modal
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in the Agents tab (a project row), the Logs tab (an activity log entry, using that entry's project), or the Reminders tab (a reminder row, using that reminder's project) - the same modal, reached from any of the three tabs, regardless of that entry's category or the truncation of its displayed snippet (a prompt-category entry included - see the "Prompt entry full-text modal" requirement for how that entry's full text is reached instead, from within this modal's own Logs pane). The modal SHALL show five panes: an Agents pane listing every agent registered for that project with its status and process id, a Tests pane showing that project's last test run if any, a Reminders pane listing that project's reminders with each row rendered as a single line of text - that reminder's name, a single space, its duration (in minutes), then enough space to right-align its status (when it has one) flush with the pane's right edge, and finally that status - truncating the reminder's name (with a trailing `...`) rather than its duration or status whenever the full line does not fit the pane's width even with that space collapsed to a single column, so the duration and status - including a running reminder's ETA - are always fully visible, a Last Prompt pane (described below), and a Logs pane showing that project's activity log entries, most recent first, with each entry's event time rendered in a fixed-width column at the far right of the pane - aligned at the same horizontal position on every row regardless of that row's category/status text length, consistent with how the Agents tab's UPDATED column stays fixed regardless of its other cells' content - and each agent-category entry additionally showing the reporting agent's process id, and - for a "needs input" or "done" entry - a truncated one-line snippet, when one is available: a "needs input" entry's snippet is drawn from its recorded last-prompt snapshot, when one was recorded for that entry; a "done" entry's snippet is drawn from its recorded response text when one was captured, falling back to its recorded last-prompt snapshot when no response was captured. A "started" entry shows no inline snippet - its last-prompt snapshot, when recorded, is instead reachable in full via `Enter`, per the "Prompt entry full-text modal" requirement. In the Logs pane, each test-run-category entry SHALL be prefixed with a tree branch marker (`├─ `) immediately before its status emoji, so it reads as nested beneath the agent activity it ran under, while agent-category and reminder-category entries SHALL render with no such prefix, forming the unindented trunk of the list; this prefix is purely visual and SHALL NOT change entry ordering, the existing category-word prefix, or the fixed-width right-aligned timestamp column. The Logs pane SHALL NOT display column headers.

The Agents, Tests, and Reminders panes SHALL be arranged side by side (by width) in a row above the Last Prompt pane. By default, each of the three collapses to a single line of content with one blank line beneath it - the same collapsed look the Last Prompt pane has always had. Whichever of the three currently needs the most vertical room to show its content, up to a cap of one third of the modal's height, SHALL set the height applied to all three: they always share the same height as each other on a given frame, growing or shrinking together as their content changes, rather than each sizing independently. A pane whose content exceeds that shared height, once it is capped at one third of the modal, SHALL paginate per the "Paginated lists support keyboard navigation" requirement instead of growing further. The Last Prompt pane SHALL continue to occupy a thin, fixed-height strip spanning the modal's full width immediately below that row, unchanged in height, wrapping, and truncation behavior. The Logs pane SHALL occupy the remaining space beneath that, shrinking or growing by exactly however much the Agents/Tests/Reminders row's height changes from its one-line default, so the modal's total height stays fixed regardless of how tall that row currently is.

Agent and test-run statuses in the Agents and Tests panes, reminder statuses in the Reminders pane, and log entry statuses in the Logs pane, SHALL use the same emoji markers, color styling, and duration as their counterparts in the Agents tab, Reminders tab, and Logs tab respectively, so status is visually consistent wherever it appears; the Agents, Tests, and Reminders panes SHALL omit the category-word prefix used in the Agents, Reminders, and Logs tabs, since each pane itself already establishes the category (e.g. "🤖 done", "❌ failed", "✅ done, 10m"); the modal's Logs pane SHALL include the category-word prefix, consistent with the top-level Logs tab it mirrors. The user SHALL be able to close the modal with `Esc` while it is in its windowed size, returning to whichever tab was active without altering the underlying agent, test-run, or reminder state; see below for `Esc`'s contextual behavior while the modal is maximised.

While the details modal is open, the user SHALL be able to press `m` to toggle it between its default windowed size (described above and in the following paragraphs) and a maximised size that takes over the entire screen, including the rows the tab bar normally occupies: since the modal's own keyboard shortcuts take priority over all input while it is open and the outer tab bar cannot actually be switched while it is showing, maximising reserves no space for it. This toggle SHALL work identically regardless of which tab the modal was opened from. Maximising or restoring the modal SHALL NOT change its five-pane layout proportions, its focus cycling, its pagination behavior, or any pane's content or interaction rules - it only changes how much height and width that existing layout has to work with. Whether the modal is currently maximised or windowed is a single, sticky choice that persists for as long as the TUI keeps running: closing the modal and later reopening it - from the same tab or a different one, including after navigating away via the outer tab bar to another tab and back - SHALL reopen it in whichever of the two sizes it was last left in, rather than always resetting to windowed, and switching tabs SHALL NOT by itself change this choice. Since `Esc` always restores a maximised modal to windowed before a further `Esc` closes it (per the contextual behavior described below), closing the modal in practice always leaves it windowed for the next open; the sticky choice is what prevents that next open from being forced to windowed for any other reason (e.g. by the act of closing and reopening, or by tab navigation, each independent of the size actually chosen). Toggling between maximised and windowed - by `m`, or by the `Esc` behavior described below - SHALL leave no character left over from the previous size's frame visible anywhere on screen after the toggle.

The modal's frame SHALL show a keyboard shortcut hint anchored at its top-right corner for as long as the modal is open, alongside its existing top-left title and its bottom-left `<tab> focus pane` hint. This hint SHALL read `Maximise [m]` while the modal is windowed and `Un-maximise [m]` while the modal is maximised, rendering in normal (non-bold) weight in the former case and in bold in the latter, so the modal's current size is visible from both the hint's text and its weight.

`Esc`'s effect on the details modal is contextual on its current size: while the modal is maximised, `Esc` SHALL restore it to its windowed size without closing it; while the modal is windowed, `Esc` SHALL close it, per the closing behavior described above. Pressing `m` while the modal is already maximised SHALL likewise restore it to windowed size, giving the maximised modal two equivalent ways back to windowed size.

The Agents, Tests, and Reminders panes SHALL each be paginated per the "Paginated lists support keyboard navigation" requirement once they hold more content than fits within their current (possibly capped) height. Unlike the Logs pane, none of these three SHALL ever show a pagination keyboard-shortcut hint in its title, whether or not they are currently paginated; a scrollbar is their only pagination indicator. The Reminders pane's title SHALL always show a hint for creating a new reminder (`New [R]`) instead; the Agents and Tests panes' titles show only their pane name and focus-shortcut hint.

Each pane's title SHALL additionally be prefixed with an emoji identifying its category, ahead of the pane name itself: 🤖 for Agents, 🧪 for Tests, ⏰ for Reminders, and 💬 for Last Prompt, each reusing the same marker already shown for that category's status elsewhere in the TUI (per the "Status is visually distinguishable" requirement's emoji markers and the 💬 marker used for prompt-category log entries); the Logs pane, which has no pre-existing status marker of its own, SHALL use 🪵. This prefix is purely visual and SHALL NOT change a title's existing focus-shortcut hint, pagination hint, or (for the Reminders pane) its `New [R]` hint, nor affect any keyboard behavior.

Each of the modal's five panes SHALL show a single-key focus shortcut hint in its title: `a` for Agents, `t` for Tests, `r` for Reminders, `p` for Last Prompt, and `l` for Logs. Pressing that key while the modal is open SHALL move keyboard focus directly to the corresponding pane, regardless of which pane currently holds it. Exactly one of the modal's five panes holds keyboard focus at a time; the `Tab` key cycles focus forward through all five, in the order Agents, Tests, Reminders, Last Prompt, Logs, wrapping back to Agents after Logs, while `Shift+Tab` cycles focus through that same order in reverse, wrapping from Agents back to Logs. Opening the modal from the Agents tab or the Logs tab SHALL default focus to the Logs pane, unchanged from before this requirement's focus model was extended to all five panes, while opening it from the Reminders tab SHALL default focus to the Reminders pane with the reminder that was selected in the Reminders tab highlighted.

The pane that currently holds keyboard focus SHALL render its border in the modal's default border color; every other pane SHALL render its border in a visually dimmer color, so which pane holds focus is visible without inspecting its content for a highlighted row or cursor. The modal's own border SHALL show a `<tab> focus pane` hint anchored at its bottom-left corner for as long as the modal is open.

Of the modal's five panes, the Agents, Tests, Reminders, and Logs panes support a highlighted selection and paginated-list keyboard interaction (`j`/`k`/`d`/`u`/`g`/`G`) while they hold focus; the Last Prompt pane remains a non-interactive, non-scrolling information display with no row selection, though - like the other four - it can hold keyboard focus and respond to `Enter`. While the Agents or Tests pane holds focus, pressing `Enter` on its highlighted row has no effect. While the Last Prompt pane holds focus, pressing `Enter` opens the full-prompt modal (per the "Prompt entry full-text modal" requirement) showing that pane's current text in full, unless the pane is showing its "no prompt captured" placeholder, in which case `Enter` has no effect. While the Reminders pane holds focus, pressing `Enter` or `s` on its highlighted reminder starts it (if not running) or stops it (if running); pressing `e` opens the reminder creation/editing form pre-populated with its name and duration; pressing `Delete` opens a bold, red confirmation dialog that deletes the reminder on `Enter` or dismisses it on `Escape` without deleting - since the key labeled "delete" on most Mac keyboards sends a Backspace control code rather than a true forward-delete (reaching `Delete` itself needs `Fn`+`Delete`, or a dedicated key on an external/PC keyboard), and `Backspace` has no other meaning in this pane, the TUI SHALL treat `Backspace` the same as `Delete` here. When the Reminders pane does not hold focus, these keys SHALL NOT act on it. Pressing `R` opens the same form empty, to create a new reminder for this project - unlike `Enter`/`s`/`e`/`Delete`, this SHALL work regardless of which pane currently holds focus, since creating a reminder does not depend on a highlighted selection the way those do, and its `New [R]` hint is shown in the Reminders pane's title at all times. `Enter` on the Logs pane's highlighted entry opens the full-prompt modal when that entry qualifies, per the "Prompt entry full-text modal" requirement, and has no effect otherwise.

The Last Prompt pane SHALL show the last-prompt text of whichever agent registered for that project most recently had a status update - the tracked agent with the greatest last-updated timestamp among that project's agents - so it tracks "the agent that just finished or needs input" as the user's attention moves between agents in a multi-agent project. The pane's text SHALL wrap to the pane's width; if the full prompt does not fit within the pane's fixed height, it SHALL be truncated with a trailing `...` rather than overflowing or being made scrollable, consistent with the pane having no selection or scrolling of its own. A project with no tracked agents, or whose most-recently-updated agent has no last-prompt text yet (e.g. first tracked via `PreToolUse` or `PostToolUse` before any `UserPromptSubmit` event was seen for it), SHALL show a placeholder indicating no prompt has been captured, rather than an empty pane.

#### Scenario: Opening a project's details from the Agents tab
- **WHEN** the user presses `Enter` on a selected project row in the Agents tab
- **THEN** the TUI displays a modal overlay with that project's Agents, Tests, Reminders, Last Prompt, and Logs panes, with the Logs pane focused

#### Scenario: Opening a project's details from the Logs tab
- **WHEN** the user presses `Enter` on a selected activity log entry in the Logs tab
- **THEN** the TUI displays the same modal overlay, scoped to that entry's project, with the Logs pane focused, regardless of that entry's category or truncation

#### Scenario: Opening a project's details from the Reminders tab
- **WHEN** the user presses `Enter` on a selected reminder row in the Reminders tab
- **THEN** the TUI displays the same modal overlay, scoped to that reminder's project, with the Reminders pane focused and that reminder highlighted

#### Scenario: Agents and Tests panes share the top third
- **WHEN** the details modal is open
- **THEN** the Agents, Tests, and Reminders panes render side by side across the modal's top row, always sharing the same height as each other - one line of content plus one blank line by default, or taller (up to one third of the modal) when any of them needs more room to show its content; the Last Prompt pane occupies its fixed-height strip below them; and the Logs pane occupies the remaining space beneath that

#### Scenario: The Agents, Tests, and Reminders panes collapse to one line by default
- **WHEN** the details modal opens, or is showing, a project whose Agents, Tests, and Reminders panes each have at most one line of content
- **THEN** all three panes render at the same height: one line of content plus one blank line beneath it

#### Scenario: The three top panes grow together to fit whichever needs the most room
- **WHEN** the details modal is open and one of the Agents, Tests, or Reminders panes needs more than one line to show its content, up to one third of the modal's height
- **THEN** all three panes render at that same, taller height, not just the one whose content grew

#### Scenario: The three top panes shrink together when their content no longer needs the extra room
- **WHEN** the details modal is open, the Agents/Tests/Reminders row is taller than its one-line default, and the pane that had required the extra height no longer does (e.g. a different, shorter-content pane now requires the most room, or none do)
- **THEN** all three panes shrink together to the height the tallest-still-required pane now needs

#### Scenario: A pane whose content exceeds the height cap paginates instead of growing further
- **WHEN** the details modal is open and the Agents, Tests, or Reminders pane's content would need more than one third of the modal's height to show in full
- **THEN** that pane's row stops growing at one third of the modal's height, and the pane paginates its content per the "Paginated lists support keyboard navigation" requirement

#### Scenario: The Agents and Tests panes never show a pagination hint in their titles
- **WHEN** the details modal's Agents or Tests pane holds more content than fits on a single page
- **THEN** that pane renders a scrollbar, but its title shows only its emoji prefix, pane name, and focus-shortcut hint, never a pagination keyboard-shortcut hint

#### Scenario: Each pane's title is prefixed with a category emoji
- **WHEN** the details modal is open
- **THEN** the Agents, Tests, Reminders, Last Prompt, and Logs panes' titles each begin with 🤖, 🧪, ⏰, 💬, and 🪵 respectively, ahead of the pane's name and its other title content

#### Scenario: Each pane's title shows its focus-shortcut hint
- **WHEN** the details modal is open
- **THEN** the Agents, Tests, Reminders, Last Prompt, and Logs panes' titles show the hints `[a]`, `[t]`, `[r]`, `[p]`, and `[l]` respectively, alongside each pane's existing title content

#### Scenario: A focus-shortcut key jumps directly to its pane
- **WHEN** the user presses `a`, `t`, `r`, `p`, or `l` while the details modal is open
- **THEN** keyboard focus moves directly to the Agents, Tests, Reminders, Last Prompt, or Logs pane respectively, regardless of which pane previously held it

#### Scenario: Tab toggles focus between the Logs and Reminders panes
- **WHEN** the user presses `Tab` while the details modal is open
- **THEN** keyboard focus moves to the next pane in the fixed order Agents, Tests, Reminders, Last Prompt, Logs, wrapping from Logs back to Agents - superseding the previous two-pane-only toggle now that all five panes can hold focus

#### Scenario: Shift+Tab cycles focus backward
- **WHEN** the user presses `Shift+Tab` while the details modal is open
- **THEN** keyboard focus moves to the previous pane in the fixed order Agents, Tests, Reminders, Last Prompt, Logs, wrapping from Agents back to Logs - the exact reverse of `Tab`'s cycle direction

#### Scenario: The focused pane's border is visually distinct from the other panes'
- **WHEN** the details modal is open
- **THEN** the pane that holds keyboard focus renders its border in the modal's default border color, and every other pane renders its border in a visually dimmer color

#### Scenario: The modal shows a hint for cycling pane focus
- **WHEN** the details modal is open
- **THEN** its border shows a `<tab> focus pane` hint anchored at its bottom-left corner

#### Scenario: Enter on the Agents or Tests pane's highlighted row has no effect
- **WHEN** the Agents or Tests pane holds focus and the user presses `Enter` on its highlighted row
- **THEN** nothing happens - the modal, its panes, and the underlying agent and test-run state are unaffected

#### Scenario: Enter on the Last Prompt pane opens the full-prompt modal
- **WHEN** the Last Prompt pane holds focus, showing an agent's last-prompt text, and the user presses `Enter`
- **THEN** the TUI opens the full-prompt modal, layered above the details modal, showing that text in full

#### Scenario: Enter on the Last Prompt pane's placeholder has no effect
- **WHEN** the Last Prompt pane holds focus, showing its "no prompt captured" placeholder, and the user presses `Enter`
- **THEN** nothing happens - no modal opens

#### Scenario: Statuses are visually consistent with their tab
- **WHEN** the details modal renders an agent's status, a test run's status, a reminder's status, or a log entry's status
- **THEN** that status uses the same emoji marker, color, and duration as it would in the Agents tab, Reminders tab, or Logs tab

#### Scenario: The Agents pane shows each agent's process id
- **WHEN** the details modal renders its Agents pane
- **THEN** each listed agent's row includes that agent's process id alongside its status

#### Scenario: The Logs pane shows the process id for agent activities
- **WHEN** the details modal's Logs pane renders a log entry whose category is agent and whose status is "needs input"
- **THEN** that entry's row includes the reporting agent's process id, and a truncated one-line snippet of its recorded last-prompt snapshot when one was recorded for that entry

#### Scenario: The Logs pane shows the process id for a started entry, with no inline snippet
- **WHEN** the details modal's Logs pane renders a log entry whose category is agent and whose status is "started"
- **THEN** that entry's row includes the reporting agent's process id, but no inline prompt snippet, regardless of whether a last-prompt snapshot was recorded for that entry

#### Scenario: A done entry's snippet in the Logs pane prefers its captured response
- **WHEN** the details modal's Logs pane renders a "done" agent-category entry that carries a captured response
- **THEN** that entry's row includes the reporting agent's process id and a truncated one-line snippet of the response text, not its last-prompt snapshot

#### Scenario: A done entry's snippet in the Logs pane falls back to its last prompt when no response was captured
- **WHEN** the details modal's Logs pane renders a "done" agent-category entry recorded with no captured response
- **THEN** that entry's row shows a truncated one-line snippet of its last-prompt snapshot when one was recorded, unchanged from before this requirement preferred the response

#### Scenario: A project with no test run
- **WHEN** the details modal opens for a project with no tracked test run
- **THEN** the Tests pane indicates there is no test run rather than showing an error

#### Scenario: A project with no reminders
- **WHEN** the details modal opens for a project with no reminders
- **THEN** the Reminders pane indicates there are no reminders rather than showing an error

#### Scenario: The Reminders pane shows a reminder's duration after its name
- **WHEN** the details modal's Reminders pane renders a reminder
- **THEN** that row shows the reminder's duration (e.g. "10m") immediately after its name, separated by a single space, ahead of its status

#### Scenario: A running reminder's status omits the word "Running"
- **WHEN** the details modal's Reminders pane renders a reminder whose status is running
- **THEN** its status text shows only the emoji, the live elapsed duration, and its ETA (e.g. "⏳ 1m23s, ETA: 2:05 PM"), without the word "Running"

#### Scenario: A reminder's status is right-aligned to the pane's right edge
- **WHEN** the details modal's Reminders pane renders a reminder that has a status (running or done) and the pane is wide enough that the name, duration, and status together leave unused width
- **THEN** that unused width appears as extra space inserted after the duration, so the status's last character - the ETA's closing character for a running reminder - lands on the pane's rightmost column, rather than the status merely trailing the duration by a single fixed space

#### Scenario: A running reminder's status is truncated to fit the Reminders pane
- **WHEN** the details modal's Reminders pane is narrow enough that a running reminder's full row - its name, duration, and status, including its ETA - would not otherwise fit the pane's width even with the gap after the duration collapsed to a single space
- **THEN** the TUI truncates the reminder's name with a trailing `...`, leaving its duration and status - including the ETA at the end of the status text, still flush with the pane's right edge - fully visible, rather than truncating the status/ETA text the way it previously did

#### Scenario: A project with no logged activity
- **WHEN** the details modal opens for a project with no activity log entries
- **THEN** the Logs pane indicates there is no activity rather than showing an error

#### Scenario: Closing the modal
- **WHEN** the user presses `Esc` while the details modal is open and in its windowed size
- **THEN** the TUI closes the modal and returns to whichever tab was active when it opened, and the daemon's tracked agents, test runs, and reminders are unaffected

#### Scenario: The Logs pane's timestamp trails each entry's text
- **WHEN** the details modal's Logs pane renders two or more entries whose category/status text differs in length
- **THEN** every entry's event time starts at the same fixed horizontal column position, at the far right of the pane, rather than immediately trailing each entry's own text at a position that varies with that text's length, and no column headers are shown above the entries

#### Scenario: Test-run entries render nested under the agent trunk
- **WHEN** the details modal's Logs pane renders a log entry whose category is test-run
- **THEN** that entry's row is prefixed with `├─ ` before its status emoji, and its event time still starts at the pane's fixed right-hand column

#### Scenario: Agent entries render unindented
- **WHEN** the details modal's Logs pane renders a log entry whose category is agent or reminder
- **THEN** that entry's row has no tree branch prefix

#### Scenario: Consecutive test-run entries each get their own branch marker
- **WHEN** the details modal's Logs pane renders two or more consecutive test-run entries between agent entries
- **THEN** each of those test-run entries is individually prefixed with `├─ `, regardless of its position among the consecutive entries

#### Scenario: The Logs pane gains a selection and scrollbar once its entries overflow a page
- **WHEN** the details modal's Logs pane holds more entries than fit on a single page
- **THEN** it renders a vertical scrollbar along its right edge, in addition to the highlighted selected row it already shows regardless of pagination

#### Scenario: The Logs pane's title gains a pagination hint once its entries overflow a page
- **WHEN** the details modal's Logs pane holds more entries than fit on a single page
- **THEN** its "🪵 Logs" pane title shows a hint for the pagination keys, alongside its `[l]` focus-shortcut hint

#### Scenario: No selection, scrollbar, or hint when the Logs pane's entries all fit on one page
- **WHEN** the details modal opens for a project whose activity log entries all fit within the Logs pane's single page
- **THEN** the pane shows no scrollbar and no pagination hint in its title, but still renders a highlighted selected row (defaulting to the most recent entry) - unlike before, when the pane showed no selection at all in this case

#### Scenario: A single-page Logs pane still allows selecting its one entry
- **WHEN** the details modal's Logs pane holds focus and has at least one entry but not enough to need pagination
- **THEN** a row is still highlighted and selectable, so pressing `Enter` can act on it (e.g. opening the full-prompt modal for a prompt entry, per the "Prompt entry full-text modal" requirement)

#### Scenario: The Logs pane's scrollbar thumb reaches the bottom of the track on the last page
- **WHEN** the details modal's Logs pane holds more entries than fit on a single page, and the pane is scrolled to its last page
- **THEN** the scrollbar's thumb is positioned flush against the bottom of its track, not stopping short of it

#### Scenario: The Reminders pane's title always shows the create hint
- **WHEN** the details modal's Reminders pane renders its title, whether or not its reminders overflow a single page
- **THEN** the title shows its emoji prefix, its `[r]` focus-shortcut hint, and the `New [R]` hint, and never a pagination keyboard-shortcut hint

#### Scenario: An unfocused Reminders pane ignores its selection-dependent action keys
- **WHEN** the Logs pane holds focus and the user presses `Enter`, `s`, `e`, `Delete`, or `Backspace`
- **THEN** the Reminders pane's selection and reminders are unaffected

#### Scenario: Creating a reminder works regardless of which pane holds focus
- **WHEN** the Logs pane holds focus and the user presses `R`
- **THEN** the TUI opens the reminder form empty, scoped to the modal's project, exactly as it would if the Reminders pane held focus

#### Scenario: Starting a reminder from the Reminders pane
- **WHEN** the Reminders pane holds focus and the user presses `Enter` or `s` on a highlighted reminder that is not running
- **THEN** the TUI sends a start request for that reminder and its status begins showing as running

#### Scenario: Stopping a reminder from the Reminders pane
- **WHEN** the Reminders pane holds focus and the user presses `Enter` or `s` on a highlighted reminder that is running
- **THEN** the TUI sends a stop request for that reminder and its status transitions to done, with no notification sent

#### Scenario: Editing a reminder from the Reminders pane
- **WHEN** the Reminders pane holds focus and the user presses `e` on a highlighted reminder
- **THEN** the TUI opens the reminder form pre-populated with that reminder's name and duration; `Enter` saves the changes and `Escape` discards them and closes the form

#### Scenario: Deleting a reminder from the Reminders pane
- **WHEN** the Reminders pane holds focus and the user presses `Delete` or `Backspace` on a highlighted reminder
- **THEN** the TUI opens a bold, red confirmation dialog; `Enter` deletes the reminder and `Escape` dismisses the dialog without deleting it

#### Scenario: Creating a reminder from the Reminders pane
- **WHEN** the Reminders pane holds focus and the user presses `R`
- **THEN** the TUI opens the reminder form empty, scoped to the modal's project; `Enter` creates the reminder and `Escape` cancels without creating one

#### Scenario: The Last Prompt pane shows the most recently active agent's prompt
- **WHEN** the details modal opens for a project tracking more than one agent, with different last-updated timestamps
- **THEN** the Last Prompt pane shows the last-prompt text of the agent among them with the greatest last-updated timestamp, not any other tracked agent's

#### Scenario: The Last Prompt pane updates as a different agent becomes the most recently active
- **WHEN** the modal is open and a different agent in the same project receives a status update, making it the project's most recently updated agent
- **THEN** the Last Prompt pane switches to showing that agent's last-prompt text

#### Scenario: The Last Prompt pane shows a placeholder when no prompt has been captured
- **WHEN** the details modal opens for a project whose tracked agents (if any) have no last-prompt text recorded yet
- **THEN** the Last Prompt pane shows a placeholder indicating no prompt has been captured, rather than rendering empty

#### Scenario: The Last Prompt pane truncates a prompt that does not fit
- **WHEN** the Last Prompt pane's text, wrapped to its width, exceeds its fixed height
- **THEN** the pane shows as much as fits followed by a trailing `...`, rather than overflowing its border or becoming scrollable

#### Scenario: The Last Prompt pane is not part of the modal's keyboard focus cycle
- **WHEN** the user presses `Tab` or the `p` focus-shortcut while the details modal is open
- **THEN** keyboard focus can land on the Last Prompt pane, unlike before this requirement extended focus to all five panes - `Tab` reaches it in its place in the Agents/Tests/Reminders/Last Prompt/Logs order, and `p` jumps to it directly

#### Scenario: Maximising the modal takes over the entire screen
- **WHEN** the details modal is open in its windowed size and the user presses `m`
- **THEN** the modal grows to fill the entire screen, including the rows the tab bar normally occupies

#### Scenario: Maximising works the same regardless of which tab the modal was opened from
- **WHEN** the details modal is open, having been reached from the Agents tab, the Logs tab, or the Reminders tab, and the user presses `m`
- **THEN** the modal maximises to fill the entire screen in exactly the same way, regardless of which tab it was opened from

#### Scenario: The tab bar is covered while the modal is maximised
- **WHEN** the details modal is maximised
- **THEN** the tab bar is no longer visible - the modal's own border and panes occupy the rows it would otherwise occupy

#### Scenario: Maximising does not change the modal's internal layout or interactions
- **WHEN** the details modal is maximised
- **THEN** its five panes keep the same relative layout, focus cycling, pagination behavior, and content as when windowed - only the amount of space available to that existing layout changes

#### Scenario: Pressing m again while maximised restores the windowed size
- **WHEN** the details modal is maximised and the user presses `m`
- **THEN** the modal returns to its windowed size

#### Scenario: Esc restores a maximised modal instead of closing it
- **WHEN** the details modal is maximised and the user presses `Esc`
- **THEN** the modal returns to its windowed size and remains open, and the daemon's tracked agents, test runs, and reminders are unaffected

#### Scenario: Esc closes the modal only once it is windowed
- **WHEN** the details modal was maximised, the user presses `Esc` to restore it to windowed size, and then presses `Esc` again
- **THEN** the first `Esc` restores the windowed size and the second `Esc` closes the modal, returning to whichever tab was active when it opened

#### Scenario: The maximise hint is shown at the modal's top-right corner
- **WHEN** the details modal is open, whether windowed or maximised
- **THEN** its frame shows a keyboard shortcut hint anchored at its top-right corner, alongside its existing top-left title and bottom-left `<tab> focus pane` hint, reading `Maximise [m]` while windowed or `Un-maximise [m]` while maximised

#### Scenario: The maximise hint is bold while maximised
- **WHEN** the details modal is maximised
- **THEN** its top-right hint reads `Un-maximise [m]` and renders in bold

#### Scenario: The maximise hint is normal weight while windowed
- **WHEN** the details modal is in its windowed size
- **THEN** its top-right hint reads `Maximise [m]` and renders in normal, non-bold weight

#### Scenario: Toggling the maximised state leaves no stale characters on screen
- **WHEN** the details modal is maximised, showing a full screen of panes and log entries, and the user presses `m` (or `Esc`) to restore it to its windowed size
- **THEN** no character left over from the maximised frame remains visible anywhere on screen, unlike before this requirement's marker set stopped including a VS16 emoji sequence - the TUI no longer needs to force a full-terminal redraw to guarantee this, since ratatui's normal incremental diffing between frames is correct once no marker exercises that cell-width disagreement class

#### Scenario: The windowed state persists across closing and reopening the modal
- **WHEN** the details modal is in its windowed size, the user closes it, and later reopens the details modal (for the same or a different project)
- **THEN** the modal reopens in its windowed size

#### Scenario: Switching tabs does not reset the maximised/windowed choice
- **WHEN** the details modal is closed, whatever its maximised/windowed choice currently is, and the user switches between the Agents, Logs, and Reminders tabs via the outer tab bar any number of times
- **THEN** that choice is unchanged by the tab switching itself, and the next time the details modal is opened it reflects that same, untouched choice

#### Scenario: Reopening the modal honors whatever the sticky choice currently is, regardless of which tab opens it
- **WHEN** the details modal's sticky maximised/windowed choice is currently maximised, and the user opens the details modal from the Agents tab, the Logs tab, or the Reminders tab
- **THEN** the modal opens already maximised, regardless of which of the three tabs it was opened from

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects (including prompt-category entries), as a paginated list ordered by recency (most recent first) by default; when a prompt-category entry and the agent-category "started" entry it precipitated share the same event-occurred time, the started entry SHALL sort ahead of (be shown as more recent than) that prompt entry, consistent with the order the daemon recorded them in. Each entry SHALL show at least its status, event category, project, and event time, in that left-to-right column order - STATUS leftmost, then CATEGORY, then PROJECT, then event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column - so the narrow margin still visible around the details modal's centered overlay shows status/prompt text rather than a project name. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. The PROJECT column's width SHALL be sized to the widest project display name among the currently displayed entries (never narrower than its own "PROJECT" header), taking a user-assigned custom project name into account the same as a directory-derived default name, rather than a fixed proportional share of the table width. Once PROJECT, CATEGORY, and TIME have what they need, the STATUS column SHALL receive the remaining available width, so STATUS - not PROJECT - gets priority for any extra room and is the last of the two to have its content truncated. To keep an unusually long project name from squeezing STATUS down to an unusable width, PROJECT SHALL be capped at half of the combined width available to PROJECT and STATUS; content that doesn't fit within PROJECT's resulting width is clipped without an ellipsis, consistent with how the Agents tab's PROJECT column already handles an oversized name. A test-run entry whose status is "passed" or "failed", an agent entry whose status is "done", or a reminder entry whose status is "finished" or "stopped", SHALL additionally show the elapsed duration of that run/task/reminder, computed from the most recent preceding "started" entry of the same category logged for the same project - for a reminder entry, additionally matched to the "started" entry recording the same reminder name, since a project can have more than one reminder interleaved in the log and an agent/test-run has no equivalent identity to disambiguate by. A reminder entry whose status is "started" SHALL additionally show its recorded ETA (the reminder's local-time due time, `h:MM AM/PM`, as recorded on that entry - see the activity-log capability), rather than the elapsed duration shown for other completed entries, since a just-started reminder has no elapsed run to show. An agent entry whose status is "needs input" that was recorded with a last-prompt snapshot SHALL additionally show a snippet of that prompt text after its existing content, as a single line truncated with a trailing `...` rather than overflowing the row; such an entry recorded with no last-prompt snapshot SHALL show no such snippet. An agent entry whose status is "started" SHALL show no inline prompt snippet at all, regardless of whether a last-prompt snapshot was recorded for it - that text is instead reachable in full via `Enter` on the entry within the details modal's Logs pane, per the "Prompt entry full-text modal" requirement. An agent entry whose status is "done" SHALL additionally show, in that same truncated single-line form, a snippet of its recorded response text when one was captured; a "done" entry recorded with no captured response SHALL instead show a snippet of its last-prompt snapshot when one was recorded, or no snippet at all when neither was recorded - so a "done" entry's snippet prefers the response but never regresses to showing nothing when it previously had a prompt snippet to show. A "done" entry's snippet - whether drawn from its response or falling back to its last-prompt snapshot - SHALL render in the same unstyled, default-foreground color already used for a prompt-category entry's own snippet and the Last Prompt pane, distinct from the status label's color (e.g. green for "done"), so the agent's own words are visually distinguishable from the status styling around them; the status label and any duration text SHALL keep the status color unchanged. A prompt-category entry's entire label, beyond its 💬 marker, SHALL be its recorded prompt text rendered as a single-line snippet, separated from the marker by a single space, truncated with a trailing `...` rather than overflowing the row - unlike an agent entry's snippet (appended after other status content), a prompt entry's snippet is its only content, and unlike an agent entry's snapshot, a prompt entry's snippet is never absent. All of these snippets SHALL be sized against the STATUS column's actual rendered width at the current terminal size, the same way the Agents tab's AGENTS and REMINDERS columns and the details modal's Reminders pane size their own truncation against their real rendered width, rather than against a fixed character count chosen independently of how much room the column actually has - so a wider terminal shows more of the snippet instead of leaving the column's unused width blank. Whenever the list holds more entries than fit on a single page, the Logs tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) entry already fits on a single page.

The Logs tab's heading SHALL additionally be prefixed with 🪵, the same marker used for the Logs pane in the project details modal and the Logs tab in the tab bar, ahead of its sort, filter, and pagination hints. This prefix is purely visual and SHALL NOT change the heading's existing hints or any keyboard behavior.

#### Scenario: Logs tab lists recent activity across projects
- **WHEN** the user switches to the Logs tab
- **THEN** the TUI displays activity log entries from every project, most recent first, split into pages

#### Scenario: A new entry arrives while viewing the Logs tab
- **WHEN** the daemon pushes a new activity log entry while the Logs tab is active
- **THEN** the TUI incorporates it into the list without requiring the user to restart or reconnect

#### Scenario: Log entry statuses are visually distinguishable
- **WHEN** the Logs tab renders an entry's status
- **THEN** that status is prefixed with the same emoji marker and styled with the same color it would use in the Agents tab

#### Scenario: A completed test run shows its duration
- **WHEN** the Logs tab renders a "passed" or "failed" test-run entry that has a preceding "test-run"/"started" entry logged for the same project
- **THEN** the row includes the elapsed duration between that "started" entry and this one

#### Scenario: A completed agent task shows its duration
- **WHEN** the Logs tab renders a "done" agent entry that has a preceding "agent"/"started" entry logged for the same project
- **THEN** the row includes the elapsed duration between that "started" entry and this one

#### Scenario: A started test run with no completion shows no duration
- **WHEN** the Logs tab renders a "started" test-run entry
- **THEN** the row shows no duration for it, since the run has not yet completed in the log

#### Scenario: STATUS is the leftmost column
- **WHEN** the Logs tab renders its header and rows
- **THEN** the STATUS column appears first (far left), followed by CATEGORY, then PROJECT, then TIME

#### Scenario: The Logs tab's timestamp column is the last column
- **WHEN** the Logs tab renders its header and rows
- **THEN** the event-time column appears after the status, category, and project columns, rather than before them

#### Scenario: A long project name widens the PROJECT column
- **WHEN** a currently displayed entry's project display name (including a user-assigned custom name) is wider than the PROJECT header and than any other currently displayed entry's project name
- **THEN** the PROJECT column widens to fit that name, up to its half-of-combined-width cap, rather than leaving it truncated while STATUS holds a fixed share of the width

#### Scenario: Short project names leave more room for STATUS
- **WHEN** every currently displayed entry's project display name is narrower than a fixed proportional share of the table would have reserved for it
- **THEN** the PROJECT column shrinks to only what those names actually need, and the STATUS column receives the width that frees up

#### Scenario: An unusually long project name does not starve STATUS
- **WHEN** a currently displayed entry's project display name is long enough that giving it everything it needs would leave STATUS with less than half of the two columns' combined available width
- **THEN** the PROJECT column is capped at half of that combined width, its content is clipped without an ellipsis, and the STATUS column receives the remaining half

#### Scenario: A scrollbar appears once entries overflow a page
- **WHEN** the Logs tab's (filtered) entries hold more rows than fit on a single page
- **THEN** the TUI renders a vertical scrollbar along the pane's right edge, reflecting the current page's position within the full list

#### Scenario: No scrollbar when everything fits on one page
- **WHEN** the Logs tab's (filtered) entries all fit within a single page
- **THEN** no scrollbar is rendered

#### Scenario: The heading gains a pagination hint once entries overflow a page
- **WHEN** the Logs tab's (filtered) entries hold more rows than fit on a single page
- **THEN** its heading shows a hint for the pagination keys alongside its existing sort and filter hints

#### Scenario: No pagination hint when everything fits on one page
- **WHEN** the Logs tab's (filtered) entries all fit within a single page
- **THEN** its heading shows no pagination hint, only the sort and filter hints

#### Scenario: The scrollbar thumb reaches the bottom of the track on the last page
- **WHEN** the Logs tab's (filtered) entries hold more rows than fit on a single page, and the list is scrolled to its last page
- **THEN** the scrollbar's thumb is positioned flush against the bottom of its track, not stopping short of it

#### Scenario: A finished reminder shows its elapsed duration
- **WHEN** the Logs tab renders a "finished" reminder entry that has a preceding "started" entry logged for the same project and the same reminder name
- **THEN** the row includes the elapsed duration between that "started" entry and this one

#### Scenario: A stopped reminder shows its elapsed duration
- **WHEN** the Logs tab renders a "stopped" reminder entry that has a preceding "started" entry logged for the same project and the same reminder name
- **THEN** the row includes the elapsed duration between that "started" entry and this one

#### Scenario: A started reminder with no completion shows no duration
- **WHEN** the Logs tab renders a "started" reminder entry
- **THEN** the row shows no duration for it, since it has not yet stopped or finished in the log

#### Scenario: A started reminder shows its ETA
- **WHEN** the Logs tab renders a "started" reminder entry
- **THEN** the row includes that entry's recorded ETA (`h:MM AM/PM`, local time) alongside its name

#### Scenario: Interleaved reminders in the same project do not cross-match durations
- **WHEN** the Logs tab renders a "finished" or "stopped" reminder entry, and a different reminder in the same project also has "started"/"finished"/"stopped" entries interleaved with it in the log
- **THEN** the displayed duration is computed only from that reminder's own preceding "started" entry (matched by reminder name), not another reminder's

#### Scenario: An agent entry shows its recorded prompt snippet
- **WHEN** the Logs tab renders a "needs input" agent-category entry that was recorded with a last-prompt snapshot
- **THEN** the row shows a single-line snippet of that prompt after its existing status/duration text, with a trailing `...` if it does not fit

#### Scenario: An agent entry with no recorded prompt shows no snippet
- **WHEN** the Logs tab renders a "needs input" agent-category entry recorded with no last-prompt snapshot
- **THEN** the row shows no prompt snippet, rather than empty quotes or placeholder text

#### Scenario: A started entry shows no inline snippet
- **WHEN** the Logs tab renders a "started" agent-category entry, whether or not it was recorded with a last-prompt snapshot
- **THEN** the row shows no inline prompt snippet at all, unlike before this requirement removed it from "started" entries - that text remains reachable in full via `Enter` on the entry within the details modal's Logs pane

#### Scenario: A done entry shows its captured response snippet instead of its prompt
- **WHEN** the Logs tab renders a "done" agent-category entry that carries a captured response
- **THEN** the row shows a single-line snippet of that response text after its existing status/duration text, with a trailing `...` if it does not fit, instead of a snippet of its last-prompt snapshot

#### Scenario: A done entry with no captured response falls back to its prompt snippet
- **WHEN** the Logs tab renders a "done" agent-category entry recorded with no captured response, but with a recorded last-prompt snapshot
- **THEN** the row shows a single-line snippet of that last-prompt snapshot, unchanged from before this requirement preferred the response

#### Scenario: A done entry with neither a response nor a prompt shows no snippet
- **WHEN** the Logs tab renders a "done" agent-category entry recorded with neither a captured response nor a last-prompt snapshot
- **THEN** the row shows no snippet, rather than empty quotes or placeholder text

#### Scenario: A done entry's response snippet renders in the user-prompt color
- **WHEN** the Logs tab renders a "done" agent-category entry that carries a captured response
- **THEN** the appended response snippet renders in the same unstyled, default-foreground color used for a prompt-category entry's own snippet and the Last Prompt pane, while the "🤖 done" label and any duration text keep the status color (green)

#### Scenario: A done entry's fallback prompt snippet also renders in the user-prompt color
- **WHEN** the Logs tab renders a "done" agent-category entry recorded with no captured response but with a recorded last-prompt snapshot
- **THEN** the appended last-prompt snippet renders in that same unstyled, default-foreground color, not the status color

#### Scenario: A prompt entry shows its submitted text
- **WHEN** the Logs tab renders a prompt-category entry
- **THEN** the row shows 💬, then a single space, then a single-line snippet of the submitted prompt text, with no "prompt submitted" wording and no surrounding quotes

#### Scenario: A prompt entry that starts a run sorts just ahead of that started entry
- **WHEN** the Logs tab renders a prompt-category entry and the agent-category "started" entry it precipitated, both logged with the same event-occurred time
- **THEN** the started entry appears immediately above (more recent than) the prompt entry, rather than the two appearing in an arbitrary relative order, since the daemon now records the started entry first

#### Scenario: A wider STATUS column shows more of a prompt snippet
- **WHEN** the terminal is wide enough that the Logs tab's STATUS column (or the details modal's Logs pane) renders wider than it would at a narrower terminal size, and a prompt-category or response-snapshot entry's full text exceeds what fit at the narrower size
- **THEN** the rendered snippet shows more of that entry's text than it did at the narrower size, up to the column's new width, rather than staying capped at the same fixed length regardless of the column's actual width

#### Scenario: The Logs tab's heading is prefixed with a category emoji
- **WHEN** the user switches to the Logs tab
- **THEN** its heading begins with 🪵, ahead of its sort and filter hints

### Requirement: Status labels are prefixed by category outside dedicated panes
In the Agents tab and the Logs tab, each status label SHALL be prefixed with a category word - "agent" for an agent status, "tests" for a test-run status, or "reminder" for a reminder-category log entry - immediately after its emoji marker and before the status word (e.g. "🤖 agent done", "🧪 tests started", "⏰ reminder finished"), so a label read in a cross-category view is unambiguous about which kind of status it names. A prompt-category log entry is the one exception to this cross-category rule in the Logs tab and the details modal's Logs pane: it SHALL NOT include a category word or a status word at all - its label is its 💬 marker, then a single space, then its prompt snippet, per the "Logs tab shows an aggregated, paginated activity list" requirement - since the entry's single, always-"submitted" status carries no information the category word or a status word could usefully add, and the Logs tab's own CATEGORY column (or, in the details modal's Logs pane, the marker itself) already identifies it. The Agents tab's Reminders column is the one exception to this cross-category rule: it SHALL NOT include the category-word prefix, since the column's narrow width prioritizes the reminder's own name (the primary content of that cell) and the REMINDERS column header already establishes the category. In the details modal, the Agents pane, Tests pane, and Reminders pane SHALL NOT include this category-word prefix, since the pane itself already establishes the category (e.g. "🤖 done", "❌ failed"); the modal's Logs pane SHALL include the category-word prefix for agent, test-run, and reminder entries, consistent with the top-level Logs tab it mirrors. The top-level Reminders tab's Status column SHALL likewise NOT include this category-word prefix, since the tab already dedicates its entire view to reminders, consistent with how the modal's Agents and Tests panes omit it.

#### Scenario: The Agents tab prefixes an agent status with its category word
- **WHEN** the Agents tab renders an agent's status in its Agents column
- **THEN** the label reads the emoji, then "agent", then the status word (e.g. "🤖 agent done"), rather than omitting the category word

#### Scenario: The Agents tab prefixes a test-run status with its category word
- **WHEN** the Agents tab renders a project's test-run status in its Tests column
- **THEN** the label reads the emoji, then "tests", then the status word (e.g. "⏳ tests running"), rather than omitting the category word

#### Scenario: The Agents tab's Reminders column omits the category-word prefix
- **WHEN** the Agents tab renders a project's reminder status in its Reminders column
- **THEN** the label omits the category word (e.g. an emoji followed directly by the reminder's bold name), unlike its Agents and Tests columns

#### Scenario: The Logs tab prefixes entries with their category word
- **WHEN** the Logs tab renders an agent-category, test-run-category, or reminder-category entry
- **THEN** the label includes the category word ("agent", "tests", or "reminder") alongside its emoji and status, consistent with the Agents tab

#### Scenario: The Logs tab prefixes a prompt entry with its category word
- **WHEN** the Logs tab or the details modal's Logs pane renders a prompt-category entry
- **THEN** the label reads 💬, then a single space, then the entry's prompt snippet, with no "prompt" or "submitted" word - unlike every other category, a prompt entry's label carries no category or status word at all

#### Scenario: The details modal's Agents and Tests panes omit the category-word prefix
- **WHEN** the details modal renders a status in its Agents pane, Tests pane, or Reminders pane
- **THEN** the label omits the category word (e.g. "🤖 done", not "🤖 agent done")

#### Scenario: The details modal's Logs pane keeps the category-word prefix
- **WHEN** the details modal renders an agent-category, test-run-category, or reminder-category entry in its Logs pane
- **THEN** the label includes the category word, consistent with the top-level Logs tab

#### Scenario: The Reminders tab omits the category-word prefix
- **WHEN** the Reminders tab renders a reminder's status in its Status column
- **THEN** the label omits the category word (e.g. "⏳ running"), since the tab already dedicates its view to reminders
