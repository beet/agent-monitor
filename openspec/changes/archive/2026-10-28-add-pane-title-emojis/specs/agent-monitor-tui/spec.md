## MODIFIED Requirements

### Requirement: Project details modal
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in the Agents tab (a project row), the Logs tab (an activity log entry, using that entry's project), or the Reminders tab (a reminder row, using that reminder's project) - the same modal, reached from any of the three tabs, regardless of that entry's category or the truncation of its displayed snippet (a prompt-category entry included - see the "Prompt entry full-text modal" requirement for how that entry's full text is reached instead, from within this modal's own Logs pane). The modal SHALL show five panes: an Agents pane listing every agent registered for that project with its status and process id, a Tests pane showing that project's last test run if any, a Reminders pane listing that project's reminders with each row showing that reminder's name, its duration (in minutes) immediately after the name, and its status - truncated with a trailing `...` rather than overflowing the pane's right border if it doesn't fit, the same ellipsis-truncation behavior the Agents tab's AGENTS and REMINDERS columns use - a Last Prompt pane (described below), and a Logs pane showing that project's activity log entries, most recent first, with each entry's event time rendered in a fixed-width column at the far right of the pane - aligned at the same horizontal position on every row regardless of that row's category/status text length, consistent with how the Agents tab's UPDATED column stays fixed regardless of its other cells' content - and each agent-category entry additionally showing the reporting agent's process id and, when one was recorded for that entry, a truncated one-line snippet of its last-prompt snapshot. In the Logs pane, each test-run-category entry SHALL be prefixed with a tree branch marker (`├─ `) immediately before its status emoji, so it reads as nested beneath the agent activity it ran under, while agent-category and reminder-category entries SHALL render with no such prefix, forming the unindented trunk of the list; this prefix is purely visual and SHALL NOT change entry ordering, the existing category-word prefix, or the fixed-width right-aligned timestamp column. The Logs pane SHALL NOT display column headers.

The Agents, Tests, and Reminders panes SHALL be arranged side by side (by width) in a row above the Last Prompt pane. By default, each of the three collapses to a single line of content with one blank line beneath it - the same collapsed look the Last Prompt pane has always had. Whichever of the three currently needs the most vertical room to show its content, up to a cap of one third of the modal's height, SHALL set the height applied to all three: they always share the same height as each other on a given frame, growing or shrinking together as their content changes, rather than each sizing independently. A pane whose content exceeds that shared height, once it is capped at one third of the modal, SHALL paginate per the "Paginated lists support keyboard navigation" requirement instead of growing further. The Last Prompt pane SHALL continue to occupy a thin, fixed-height strip spanning the modal's full width immediately below that row, unchanged in height, wrapping, and truncation behavior. The Logs pane SHALL occupy the remaining space beneath that, shrinking or growing by exactly however much the Agents/Tests/Reminders row's height changes from its one-line default, so the modal's total height stays fixed regardless of how tall that row currently is.

Agent and test-run statuses in the Agents and Tests panes, reminder statuses in the Reminders pane, and log entry statuses in the Logs pane, SHALL use the same emoji markers, color styling, and duration as their counterparts in the Agents tab, Reminders tab, and Logs tab respectively, so status is visually consistent wherever it appears; the Agents, Tests, and Reminders panes SHALL omit the category-word prefix used in the Agents, Reminders, and Logs tabs, since each pane itself already establishes the category (e.g. "🤖 done", "❌ failed", "✅ done, 10m"); the modal's Logs pane SHALL include the category-word prefix, consistent with the top-level Logs tab it mirrors. The user SHALL be able to close the modal with `Esc`, returning to whichever tab was active without altering the underlying agent, test-run, or reminder state.

The Agents, Tests, and Reminders panes SHALL each be paginated per the "Paginated lists support keyboard navigation" requirement once they hold more content than fits within their current (possibly capped) height. Unlike the Logs pane, none of these three SHALL ever show a pagination keyboard-shortcut hint in its title, whether or not they are currently paginated; a scrollbar is their only pagination indicator. The Reminders pane's title SHALL always show a hint for creating a new reminder (`New [R]`) instead; the Agents and Tests panes' titles show only their pane name and focus-shortcut hint.

Each pane's title SHALL additionally be prefixed with an emoji identifying its category, ahead of the pane name itself: 🤖 for Agents, 🧪 for Tests, ⏰ for Reminders, and 🗣️ for Last Prompt, each reusing the same marker already shown for that category's status elsewhere in the TUI (per the "Status is visually distinguishable" requirement's emoji markers and the 🗣️ marker used for prompt-category log entries); the Logs pane, which has no pre-existing status marker of its own, SHALL use 🪵. This prefix is purely visual and SHALL NOT change a title's existing focus-shortcut hint, pagination hint, or (for the Reminders pane) its `New [R]` hint, nor affect any keyboard behavior.

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
- **THEN** that pane renders a scrollbar, but its title shows only its emoji prefix, pane name, and focus-shortcut hint, never a pagination keyboard-shortcut hint

#### Scenario: Each pane's title is prefixed with a category emoji
- **WHEN** the details modal is open
- **THEN** the Agents, Tests, Reminders, Last Prompt, and Logs panes' titles each begin with 🤖, 🧪, ⏰, 🗣️, and 🪵 respectively, ahead of the pane's name and its other title content

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

### Requirement: Tab navigation between Agents and Logs
The TUI SHALL organize its display into three tabs: **Agents** (the existing project table), **Logs** (an aggregated activity log view), and **Reminders** (an aggregated, cross-project reminder list). The user SHALL be able to cycle between tabs with the `Tab` key, and jump directly to a tab with `A`/`a` (Agents), `L`/`l` (Logs), or `R`/`r` (Reminders).

The tab bar SHALL show each tab's label prefixed with the same emoji marker used for that category elsewhere in the TUI: 🤖 for Agents, 🪵 for Logs, and ⏰ for Reminders - the same markers the project details modal's Agents and Logs panes and Reminders pane use. This prefix is purely visual and SHALL NOT change a tab's keyboard shortcut, its position in the cycling order, or any other tab-bar content.

#### Scenario: Cycling tabs
- **WHEN** the user presses `Tab`
- **THEN** the TUI switches to the next tab in the Agents -> Logs -> Reminders -> Agents cycle

#### Scenario: Jumping directly to a tab
- **WHEN** the user presses `A`/`a`, `L`/`l`, or `R`/`r`
- **THEN** the TUI switches to the Agents, Logs, or Reminders tab respectively, even if that tab is already active

#### Scenario: Each tab's label is prefixed with a category emoji
- **WHEN** the TUI renders its tab bar
- **THEN** the Agents, Logs, and Reminders tab labels each begin with 🤖, 🪵, and ⏰ respectively, ahead of the tab's name and its shortcut hint

### Requirement: Agents tab title reflects search state
The Agents tab's title SHALL show a `Filter [/]` keyboard shortcut hint whenever search-editing mode is not active, matching the hint convention already used by the Logs and Reminders tabs' own sort/filter controls, which stay visible whether or not a filter is currently applied. When a search filter is applied (and search-editing mode is not active), the title SHALL display the applied filter text immediately after that hint, bold and in a color distinct from the title's normal text, so an active filter remains obvious at a glance without hiding how to change it.

The Agents tab's title SHALL additionally be prefixed with 🤖, the same marker used for the Agents pane in the project details modal and the Agents tab in the tab bar, ahead of its name and every other title content - the live search prompt while editing, the `Filter [/]` hint and any applied filter text otherwise, and the `Rename [n]` hint. This prefix is purely visual and SHALL NOT change the title's existing hints, its live search-editing behavior, or any keyboard behavior.

#### Scenario: Hint shown with no applied filter
- **WHEN** the Agents tab is active, not in search-editing mode, and no filter is applied
- **THEN** the tab's title includes a `Filter [/]` keyboard shortcut hint

#### Scenario: Hint remains visible alongside an applied filter
- **WHEN** the Agents tab has an applied search filter and is not in search-editing mode
- **THEN** the tab's title still includes the `Filter [/]` keyboard shortcut hint, immediately followed by the applied filter text

#### Scenario: Applied filter is visually distinguished
- **WHEN** the Agents tab has an applied search filter and is not in search-editing mode
- **THEN** the filter text shown in the title is bold and a distinct color from the title's normal text

#### Scenario: The Agents tab's title is prefixed with a category emoji
- **WHEN** the Agents tab renders its title, whether idle, search-editing, or showing an applied filter
- **THEN** the title begins with 🤖, ahead of its name and any other title content

### Requirement: Agents tab splits agent and test-run status into separate columns
The Agents tab SHALL display a project's agent status(es) and its test-run status in two separate columns, **Agents** and **Tests**, instead of one combined STATUS column. The Agents column SHALL show that project's tracked agent(s)' status(es); the Tests column SHALL show that project's test run status, if any, or remain empty if the project has no tracked test run.

The `AGENTS` and `TESTS` column headers SHALL each be prefixed with the same emoji marker used for that category elsewhere in the TUI: 🤖 for `AGENTS` and 🧪 for `TESTS`.

#### Scenario: The Agents tab header shows separate columns
- **WHEN** the Agents tab renders its header row
- **THEN** it shows "🤖 AGENTS" and "🧪 TESTS" as separate column headers rather than a single "STATUS" header

#### Scenario: A project with only agents leaves the Tests column empty
- **WHEN** a project has tracked agent(s) but no tracked test run
- **THEN** its row's Agents column shows the agent status(es) and its Tests column is empty

#### Scenario: A project with only a test run leaves the Agents column empty
- **WHEN** a project has a tracked test run but no tracked agent
- **THEN** its row's Tests column shows the test run's status and its Agents column is empty

### Requirement: Agents tab shows a Reminders column
The Agents tab SHALL display a fourth column, **Reminders**, alongside PROJECT, Agents, Tests, and UPDATED. A project with a running reminder SHALL show a status emoji per the "Status is visually distinguishable" requirement, then that reminder's name (bold), then its live elapsed duration and ETA, formatted like "⏳ **Check the thing** 1m23s, ETA 12:34 PM" - without the category-word prefix the Agents and Tests columns carry, per the "Status labels are prefixed by category outside dedicated panes" requirement's exception for this column. A project whose most recent reminder activity is a stop or a natural finish, with no reminder currently running, SHALL show its emoji, name, and outcome and duration, formatted like "✅ **Check the thing** completed/stopped, 10m". A project with two or more reminders with activity to show SHALL present them separated by a bullet, ordered by recency, the same convention the Agents column already uses for multiple agents. A project with no reminders SHALL leave the Reminders column empty. If either the Agents column's or the Reminders column's content exceeds its available column width, the TUI SHALL truncate it with a trailing `...` rather than wrapping or overflowing into adjacent columns.

The `REMINDERS` column header SHALL be prefixed with ⏰, the same marker used for that category elsewhere in the TUI.

#### Scenario: A project with a running reminder
- **WHEN** the Agents tab renders a project row with a running reminder
- **THEN** its Reminders column shows the reminder's name, its live elapsed duration, and its ETA, updating as the TUI redraws

#### Scenario: A project with a completed reminder and none running
- **WHEN** the Agents tab renders a project row whose most recent reminder activity is a stop or finish, with none currently running
- **THEN** its Reminders column shows that reminder's name, its outcome (completed or stopped), and its duration

#### Scenario: A project with multiple reminders
- **WHEN** a project has two or more reminders with activity to show
- **THEN** its Reminders column lists them bullet-separated, ordered by recency

#### Scenario: A project with no reminders leaves the column empty
- **WHEN** a project has no reminders
- **THEN** its Reminders column is empty

#### Scenario: Overflowing content is truncated with an ellipsis
- **WHEN** the Agents column's or the Reminders column's content would exceed its available column width
- **THEN** the TUI truncates that cell's content with a trailing `...` instead of wrapping or overflowing

#### Scenario: The Reminders column header is prefixed with a category emoji
- **WHEN** the Agents tab renders its header row
- **THEN** the `REMINDERS` column header begins with ⏰

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects (including prompt-category entries), as a paginated list ordered by recency (most recent first) by default; when a prompt-category entry and the agent-category "started" entry it precipitated share the same event-occurred time, the prompt entry SHALL sort ahead of (be shown as more recent than) that started entry, consistent with the order the daemon recorded them in. Each entry SHALL show at least its status, event category, project, and event time, in that left-to-right column order - STATUS leftmost, then CATEGORY, then PROJECT, then event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column - so the narrow margin still visible around the details modal's centered overlay shows status/prompt text rather than a project name. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. The PROJECT column's width SHALL be sized to the widest project display name among the currently displayed entries (never narrower than its own "PROJECT" header), taking a user-assigned custom project name into account the same as a directory-derived default name, rather than a fixed proportional share of the table width. Once PROJECT, CATEGORY, and TIME have what they need, the STATUS column SHALL receive the remaining available width, so STATUS - not PROJECT - gets priority for any extra room and is the last of the two to have its content truncated. To keep an unusually long project name from squeezing STATUS down to an unusable width, PROJECT SHALL be capped at half of the combined width available to PROJECT and STATUS; content that doesn't fit within PROJECT's resulting width is clipped without an ellipsis, consistent with how the Agents tab's PROJECT column already handles an oversized name. A test-run entry whose status is "passed" or "failed", an agent entry whose status is "done", or a reminder entry whose status is "finished" or "stopped", SHALL additionally show the elapsed duration of that run/task/reminder, computed from the most recent preceding "started" entry of the same category logged for the same project - for a reminder entry, additionally matched to the "started" entry recording the same reminder name, since a project can have more than one reminder interleaved in the log and an agent/test-run has no equivalent identity to disambiguate by. A reminder entry whose status is "started" SHALL additionally show its recorded ETA (the reminder's local-time due time, `h:MM AM/PM`, as recorded on that entry - see the activity-log capability), rather than the elapsed duration shown for other completed entries, since a just-started reminder has no elapsed run to show. An agent entry ("started", "done", or "needs input") that was recorded with a last-prompt snapshot SHALL additionally show a snippet of that prompt text after its existing content, as a single line truncated with a trailing `...` rather than overflowing the row; an agent entry recorded with no last-prompt snapshot SHALL show no such snippet. A prompt-category entry's entire label, beyond its 🗣️ marker, SHALL be its recorded prompt text rendered as a single-line snippet, separated from the marker by a single space, truncated with a trailing `...` rather than overflowing the row - unlike an agent entry's snippet (appended after other status content), a prompt entry's snippet is its only content, and unlike an agent entry's snapshot, a prompt entry's snippet is never absent. Both kinds of snippet SHALL be sized against the STATUS column's actual rendered width at the current terminal size, the same way the Agents tab's AGENTS and REMINDERS columns and the details modal's Reminders pane size their own truncation against their real rendered width, rather than against a fixed character count chosen independently of how much room the column actually has - so a wider terminal shows more of the prompt instead of leaving the column's unused width blank. Whenever the list holds more entries than fit on a single page, the Logs tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) entry already fits on a single page.

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
- **WHEN** the Logs tab renders an agent-category entry that was recorded with a last-prompt snapshot
- **THEN** the row shows a single-line snippet of that prompt after its existing status/duration text, with a trailing `...` if it does not fit

#### Scenario: An agent entry with no recorded prompt shows no snippet
- **WHEN** the Logs tab renders an agent-category entry recorded with no last-prompt snapshot
- **THEN** the row shows no prompt snippet, rather than empty quotes or placeholder text

#### Scenario: A prompt entry shows its submitted text
- **WHEN** the Logs tab renders a prompt-category entry
- **THEN** the row shows 🗣️, then a single space, then a single-line snippet of the submitted prompt text, with no "prompt submitted" wording and no surrounding quotes

#### Scenario: A prompt entry that starts a run sorts just ahead of that started entry
- **WHEN** the Logs tab renders a prompt-category entry and the agent-category "started" entry it precipitated, both logged with the same event-occurred time
- **THEN** the prompt entry appears immediately above (more recent than) the started entry, rather than the two appearing in an arbitrary relative order

#### Scenario: A wider STATUS column shows more of a prompt snippet
- **WHEN** the terminal is wide enough that the Logs tab's STATUS column (or the details modal's Logs pane) renders wider than it would at a narrower terminal size, and a prompt-category or last-prompt-snapshot entry's full text exceeds what fit at the narrower size
- **THEN** the rendered snippet shows more of that entry's text than it did at the narrower size, up to the column's new width, rather than staying capped at the same fixed length regardless of the column's actual width

#### Scenario: The Logs tab's heading is prefixed with a category emoji
- **WHEN** the user switches to the Logs tab
- **THEN** its heading begins with 🪵, ahead of its sort and filter hints

### Requirement: Reminders tab shows an aggregated, paginated reminder list
The Reminders tab SHALL display every project's reminders, aggregated across all projects, as a paginated list ordered by recency (the last time each reminder completed a run - stopped or naturally finished - most recent first) by default. Each row SHALL show the reminder's Project, Name, Duration (in minutes), Status, and Updated columns. The Status column SHALL be blank when the reminder has no started/stopped/finished log event yet (it is still not-yet-started). While a reminder is running, the Status column SHALL show its live elapsed duration and its estimated time of completion, formatted like "Running 1m23s, ETA: 12:34 PM", prefixed with the ⏳ marker, and kept current by the TUI's own periodic redraw, consistent with a running agent's or test run's duration. Once a reminder is done (stopped or finished), the Status column SHALL show its elapsed duration formatted like "Done 10m", prefixed with the ✅ marker. The Updated column SHALL show the time of the reminder's most recent started, stopped, or finished log event, or, if it has none, the time the reminder was created. Whenever the list holds more reminders than fit on a single page, the Reminders tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) reminder already fits on a single page.

The Reminders tab's heading SHALL additionally be prefixed with ⏰, the same marker used for the Reminders pane in the project details modal and the Reminders tab in the tab bar, ahead of its sort, filter, and pagination hints. This prefix is purely visual and SHALL NOT change the heading's existing hints or any keyboard behavior.

#### Scenario: Reminders tab lists reminders across projects
- **WHEN** the user switches to the Reminders tab
- **THEN** the TUI displays reminders from every project, most recent first by their last completed run, split into pages

#### Scenario: A reminder with no runs shows a blank status
- **WHEN** the Reminders tab renders a reminder that is still not-yet-started
- **THEN** its Status column is blank

#### Scenario: A running reminder shows its live duration and ETA
- **WHEN** the Reminders tab renders a reminder whose status is running
- **THEN** its Status column shows the ⏳ marker, the current elapsed duration, and the estimated completion time (e.g. "⏳ Running 1m23s, ETA: 12:34 PM"), updating as the TUI redraws

#### Scenario: A done reminder shows its total duration
- **WHEN** the Reminders tab renders a reminder whose status is done
- **THEN** its Status column shows the ✅ marker and its elapsed duration (e.g. "✅ Done 10m")

#### Scenario: The Updated column falls back to creation time
- **WHEN** the Reminders tab renders a reminder with no started/stopped/finished log event
- **THEN** its Updated column shows the time the reminder was created

#### Scenario: A scrollbar appears once reminders overflow a page
- **WHEN** the Reminders tab's (filtered) reminders hold more rows than fit on a single page
- **THEN** the TUI renders a vertical scrollbar along the pane's right edge, reflecting the current page's position within the full list

#### Scenario: No scrollbar when everything fits on one page
- **WHEN** the Reminders tab's (filtered) reminders all fit within a single page
- **THEN** no scrollbar is rendered

#### Scenario: The Reminders tab's heading is prefixed with a category emoji
- **WHEN** the user switches to the Reminders tab
- **THEN** its heading begins with ⏰, ahead of its sort and filter hints
