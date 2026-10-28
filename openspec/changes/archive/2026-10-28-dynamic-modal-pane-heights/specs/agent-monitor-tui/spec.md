## MODIFIED Requirements

### Requirement: Project details modal
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in the Agents tab (a project row), the Logs tab (an activity log entry, using that entry's project), or the Reminders tab (a reminder row, using that reminder's project) - the same modal, reached from any of the three tabs, regardless of that entry's category or the truncation of its displayed snippet (a prompt-category entry included - see the "Prompt entry full-text modal" requirement for how that entry's full text is reached instead, from within this modal's own Logs pane). The modal SHALL show five panes: an Agents pane listing every agent registered for that project with its status and process id, a Tests pane showing that project's last test run if any, a Reminders pane listing that project's reminders with each row showing that reminder's name, its duration (in minutes) immediately after the name, and its status - truncated with a trailing `...` rather than overflowing the pane's right border if it doesn't fit, the same ellipsis-truncation behavior the Agents tab's AGENTS and REMINDERS columns use - a Last Prompt pane (described below), and a Logs pane showing that project's activity log entries, most recent first, with each entry's event time rendered in a fixed-width column at the far right of the pane - aligned at the same horizontal position on every row regardless of that row's category/status text length, consistent with how the Agents tab's UPDATED column stays fixed regardless of its other cells' content - and each agent-category entry additionally showing the reporting agent's process id and, when one was recorded for that entry, a truncated one-line snippet of its last-prompt snapshot. In the Logs pane, each test-run-category entry SHALL be prefixed with a tree branch marker (`├─ `) immediately before its status emoji, so it reads as nested beneath the agent activity it ran under, while agent-category and reminder-category entries SHALL render with no such prefix, forming the unindented trunk of the list; this prefix is purely visual and SHALL NOT change entry ordering, the existing category-word prefix, or the fixed-width right-aligned timestamp column. The Logs pane SHALL NOT display column headers.

The Agents, Tests, and Reminders panes SHALL be arranged side by side (by width) in a row above the Last Prompt pane. By default, each of the three collapses to a single line of content with one blank line beneath it - the same collapsed look the Last Prompt pane has always had. Whichever of the three currently needs the most vertical room to show its content, up to a cap of one third of the modal's height, SHALL set the height applied to all three: they always share the same height as each other on a given frame, growing or shrinking together as their content changes, rather than each sizing independently. A pane whose content exceeds that shared height, once it is capped at one third of the modal, SHALL paginate per the "Paginated lists support keyboard navigation" requirement instead of growing further. The Last Prompt pane SHALL continue to occupy a thin, fixed-height strip spanning the modal's full width immediately below that row, unchanged in height, wrapping, and truncation behavior. The Logs pane SHALL occupy the remaining space beneath that, shrinking or growing by exactly however much the Agents/Tests/Reminders row's height changes from its one-line default, so the modal's total height stays fixed regardless of how tall that row currently is.

Agent and test-run statuses in the Agents and Tests panes, reminder statuses in the Reminders pane, and log entry statuses in the Logs pane, SHALL use the same emoji markers, color styling, and duration as their counterparts in the Agents tab, Reminders tab, and Logs tab respectively, so status is visually consistent wherever it appears; the Agents, Tests, and Reminders panes SHALL omit the category-word prefix used in the Agents, Reminders, and Logs tabs, since each pane itself already establishes the category (e.g. "🤖 done", "❌ failed", "✅ done, 10m"); the modal's Logs pane SHALL include the category-word prefix, consistent with the top-level Logs tab it mirrors. The user SHALL be able to close the modal with `Esc`, returning to whichever tab was active without altering the underlying agent, test-run, or reminder state.

The Agents, Tests, and Reminders panes SHALL each be paginated per the "Paginated lists support keyboard navigation" requirement once they hold more content than fits within their current (possibly capped) height. Unlike the Logs pane, none of these three SHALL ever show a pagination keyboard-shortcut hint in its title, whether or not they are currently paginated; a scrollbar is their only pagination indicator. The Reminders pane's title SHALL always show a hint for creating a new reminder (`New [R]`) instead; the Agents and Tests panes' titles show only their pane name and focus-shortcut hint.

Each of the modal's five panes SHALL show a single-key focus shortcut hint in its title: `a` for Agents, `t` for Tests, `r` for Reminders, `p` for Last Prompt, and `l` for Logs. Pressing that key while the modal is open SHALL move keyboard focus directly to the corresponding pane, regardless of which pane currently holds it. Exactly one of the modal's five panes holds keyboard focus at a time; the `Tab` key cycles focus forward through all five, in the order Agents, Tests, Reminders, Last Prompt, Logs, wrapping back to Agents after Logs, while `Shift+Tab` cycles focus through that same order in reverse, wrapping from Agents back to Logs. Opening the modal from the Agents tab or the Logs tab SHALL default focus to the Logs pane, unchanged from before this requirement's focus model was extended to all five panes, while opening it from the Reminders tab SHALL default focus to the Reminders pane with the reminder that was selected in the Reminders tab highlighted.

The pane that currently holds keyboard focus SHALL render its border in the modal's default border color; every other pane SHALL render its border in a visually dimmer color, so which pane holds focus is visible without inspecting its content for a highlighted row or cursor. The modal's own border SHALL show a `<tab> focus pane` hint anchored at its bottom-left corner for as long as the modal is open.

Of the modal's five panes, the Agents, Tests, Reminders, and Logs panes support a highlighted selection and paginated-list keyboard interaction (`j`/`k`/`d`/`u`/`g`/`G`) while they hold focus; the Last Prompt pane remains a non-interactive, non-scrolling information display with no row selection, though - like the other four - it can hold keyboard focus and respond to `Enter`. While the Agents or Tests pane holds focus, pressing `Enter` on its highlighted row has no effect. While the Last Prompt pane holds focus, pressing `Enter` opens the full-prompt modal (per the "Prompt entry full-text modal" requirement) showing that pane's current text in full, unless the pane is showing its "no prompt captured" placeholder, in which case `Enter` has no effect. While the Reminders pane holds focus, pressing `Enter` or `s` on its highlighted reminder starts it (if not running) or stops it (if running); pressing `e` opens the reminder creation/editing form pre-populated with its name and duration; pressing `Delete` opens a bold, red confirmation dialog that deletes the reminder on `Enter` or dismisses it on `Escape` without deleting - since the key labeled "delete" on most Mac keyboards sends a Backspace control code rather than a true forward-delete (reaching `Delete` itself needs `Fn`+`Delete`, or a dedicated key on an external/PC keyboard), and `Backspace` has no other meaning in this pane, the TUI SHALL treat `Backspace` the same as `Delete` here. When the Reminders pane does not hold focus, these keys SHALL NOT act on it. Pressing `R` opens the same form empty, to create a new reminder for this project - unlike `Enter`/`s`/`e`/`Delete`, this SHALL work regardless of which pane currently holds focus, since creating a reminder does not depend on a highlighted selection the way those do, and its `New [R]` hint is shown in the Reminders pane's title at all times. `Enter` on the Logs pane's highlighted entry opens the full-prompt modal when that entry is prompt-category, per the "Prompt entry full-text modal" requirement, and has no effect otherwise.

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
- **THEN** that pane renders a scrollbar, but its title shows only its pane name and focus-shortcut hint, never a pagination keyboard-shortcut hint

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
- **WHEN** the details modal's Logs pane renders a log entry whose category is agent
- **THEN** that entry's row includes the reporting agent's process id, and a truncated one-line snippet of its recorded last-prompt snapshot when one was recorded for that entry

#### Scenario: A project with no test run
- **WHEN** the details modal opens for a project with no tracked test run
- **THEN** the Tests pane indicates there is no test run rather than showing an error

#### Scenario: A project with no reminders
- **WHEN** the details modal opens for a project with no reminders
- **THEN** the Reminders pane indicates there are no reminders rather than showing an error

#### Scenario: The Reminders pane shows a reminder's duration after its name
- **WHEN** the details modal's Reminders pane renders a reminder
- **THEN** that row shows the reminder's duration (e.g. "10m") immediately after its name, ahead of its status

#### Scenario: A running reminder's status is truncated to fit the Reminders pane
- **WHEN** the details modal's Reminders pane is narrow enough that a running reminder's "Running <duration>, ETA: <time>" status would not otherwise fit its Status column
- **THEN** the TUI truncates that status with a trailing `...` rather than letting it overflow past the pane's right border, the same ellipsis-truncation behavior the Agents tab's AGENTS and REMINDERS columns already use

#### Scenario: A project with no logged activity
- **WHEN** the details modal opens for a project with no activity log entries
- **THEN** the Logs pane indicates there is no activity rather than showing an error

#### Scenario: Closing the modal
- **WHEN** the user presses `Esc` while the details modal is open
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
- **THEN** its "Logs" pane title shows a hint for the pagination keys, alongside its `[l]` focus-shortcut hint

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
- **THEN** the title shows its `[r]` focus-shortcut hint and the `New [R]` hint, and never a pagination keyboard-shortcut hint

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

### Requirement: Paginated lists support keyboard navigation
Any paginated list in the TUI (the Logs tab's activity list, the Reminders tab's reminder list, and the details modal's Agents pane, Tests pane, Reminders pane, and Logs pane) SHALL support `j`/`down` and `k`/`up` to move the selection one line at a time, `d`/page-down and `u`/page-up to move by a full page, and `g`/`G` to jump directly to the list's first or last entry respectively. A page is however many entries currently fit in the list's rendered area at the current terminal size - for the details modal's Agents, Tests, and Reminders panes, that area's height is whatever the "Project details modal" requirement's dynamic-height rule currently gives that row, capped at one third of the modal's height. Paging SHALL overlap the previous page by exactly 1 row: paging down SHALL advance the page's top row by (page size - 1) rows, and paging up SHALL move the page's top row back by (page size - 1) rows, in both cases selecting the new page's top row, clamped so the page never scrolls past the list's first or last entry. Pressing `g` SHALL select the list's first entry and scroll the page so that entry is the page's top row; pressing `G` SHALL select the list's last entry and scroll the page so the last page is showing, consistent with how `d`/page-down already clamps at the list's last page. While the details modal is open, its Agents, Tests, Reminders, and Logs panes are each paginated lists, but only the one currently holding keyboard focus (per the "Project details modal" requirement's `Tab`-cycled and shortcut-key focus) reacts to `j`/`k`/`d`/`u`/`g`/`G`/page-down/page-up; the other three panes, and the Agents, Logs, and Reminders tabs underneath, SHALL NOT receive or react to those keys while the modal is open, consistent with the details modal taking precedence for keyboard shortcuts over whatever page is open beneath it.

#### Scenario: Moving one line at a time
- **WHEN** the user presses `j`, `down`, `k`, or `up` on a paginated list
- **THEN** the selection moves by exactly one line in the corresponding direction, if a line is available in that direction

#### Scenario: Paging
- **WHEN** the user presses `d`, page-down, `u`, or page-up on a paginated list showing more entries than fit on one page
- **THEN** the page moves by (page size - 1) rows in the corresponding direction and selects the new page's top row, so exactly one row of the previous page remains visible on the new page

#### Scenario: Paging at the start or end of the list clamps instead of overshooting
- **WHEN** the user presses `u`/page-up while already on the list's first page, or `d`/page-down while already on the list's last page
- **THEN** the selection and page stay clamped at the first or last entry rather than scrolling past it

#### Scenario: The details modal's Logs pane keys do not leak to the Logs tab underneath
- **WHEN** the details modal is open and the user presses `j`, `k`, `d`, `u`, `g`, `G`, page-down, or page-up
- **THEN** only the pane currently holding focus (Agents, Tests, Reminders, or Logs) updates its selection and page; the other three panes, and the Agents, Logs, and Reminders tabs underneath, remain unchanged

#### Scenario: Jumping to the first entry
- **WHEN** the user presses `g` on a paginated list that is not already showing its first page
- **THEN** the selection moves to the list's first entry and the page scrolls so that entry is the top row shown

#### Scenario: Jumping to the last entry
- **WHEN** the user presses `G` on a paginated list that is not already showing its last page
- **THEN** the selection moves to the list's last entry and the page scrolls so the list's final page is showing, with the last entry visible

#### Scenario: Jumping to the first or last entry when already there is a no-op
- **WHEN** the user presses `g` while the list's first entry is already selected, or `G` while the list's last entry is already selected
- **THEN** the selection and page remain unchanged
