## MODIFIED Requirements

### Requirement: Status labels are prefixed by category outside dedicated panes
In the Agents tab and the Logs tab, each status label SHALL be prefixed with a category word - "agent" for an agent status, "tests" for a test-run status, or "reminder" for a reminder-category log entry - immediately after its emoji marker and before the status word (e.g. "🤖 agent done", "🧪 tests started", "⏰ reminder finished"), so a label read in a cross-category view is unambiguous about which kind of status it names. A prompt-category log entry is the one exception to this cross-category rule in the Logs tab and the details modal's Logs pane: it SHALL NOT include a category word or a status word at all - its label is its 🗣️ marker followed directly by its prompt snippet, per the "Logs tab shows an aggregated, paginated activity list" requirement - since the entry's single, always-"submitted" status carries no information the category word or a status word could usefully add, and the Logs tab's own CATEGORY column (or, in the details modal's Logs pane, the marker itself) already identifies it. The Agents tab's Reminders column is the one exception to this cross-category rule: it SHALL NOT include the category-word prefix, since the column's narrow width prioritizes the reminder's own name (the primary content of that cell) and the REMINDERS column header already establishes the category. In the details modal, the Agents pane, Tests pane, and Reminders pane SHALL NOT include this category-word prefix, since the pane itself already establishes the category (e.g. "🤖 done", "❌ failed"); the modal's Logs pane SHALL include the category-word prefix for agent, test-run, and reminder entries, consistent with the top-level Logs tab it mirrors. The top-level Reminders tab's Status column SHALL likewise NOT include this category-word prefix, since the tab already dedicates its entire view to reminders, consistent with how the modal's Agents and Tests panes omit it.

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
- **THEN** the label reads 🗣️ followed directly by the entry's prompt snippet, with no "prompt" or "submitted" word - unlike every other category, a prompt entry's label carries no category or status word at all

#### Scenario: The details modal's Agents and Tests panes omit the category-word prefix
- **WHEN** the details modal renders a status in its Agents pane, Tests pane, or Reminders pane
- **THEN** the label omits the category word (e.g. "🤖 done", not "🤖 agent done")

#### Scenario: The details modal's Logs pane keeps the category-word prefix
- **WHEN** the details modal renders an agent-category, test-run-category, or reminder-category entry in its Logs pane
- **THEN** the label includes the category word, consistent with the top-level Logs tab

#### Scenario: The Reminders tab omits the category-word prefix
- **WHEN** the Reminders tab renders a reminder's status in its Status column
- **THEN** the label omits the category word (e.g. "⏳ running"), since the tab already dedicates its view to reminders

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects (including prompt-category entries), as a paginated list ordered by recency (most recent first) by default; when a prompt-category entry and the agent-category "started" entry it precipitated share the same event-occurred time, the prompt entry SHALL sort ahead of (be shown as more recent than) that started entry, consistent with the order the daemon recorded them in. Each entry SHALL show at least its project, event category, status, and event time, with event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. A test-run entry whose status is "passed" or "failed", an agent entry whose status is "done", or a reminder entry whose status is "finished" or "stopped", SHALL additionally show the elapsed duration of that run/task/reminder, computed from the most recent preceding "started" entry of the same category logged for the same project - for a reminder entry, additionally matched to the "started" entry recording the same reminder name, since a project can have more than one reminder interleaved in the log and an agent/test-run has no equivalent identity to disambiguate by. A reminder entry whose status is "started" SHALL additionally show its recorded ETA (the reminder's local-time due time, `h:MM AM/PM`, as recorded on that entry - see the activity-log capability), rather than the elapsed duration shown for other completed entries, since a just-started reminder has no elapsed run to show. An agent entry ("started", "done", or "needs input") that was recorded with a last-prompt snapshot SHALL additionally show a truncated, single-line snippet of that prompt text after its existing content, truncated with a trailing `...` rather than overflowing the row; an agent entry recorded with no last-prompt snapshot SHALL show no such snippet. A prompt-category entry's entire label, beyond its 🗣️ marker, SHALL be a truncated, single-line snippet of its recorded prompt text, truncated with a trailing `...` rather than overflowing the row - unlike an agent entry's snippet (appended after other status content), a prompt entry's snippet is its only content, and unlike an agent entry's snapshot, a prompt entry's snippet is never absent. Whenever the list holds more entries than fit on a single page, the Logs tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) entry already fits on a single page.

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

#### Scenario: The Logs tab's timestamp column is the last column
- **WHEN** the Logs tab renders its header and rows
- **THEN** the event-time column appears after the project, category, and status columns, rather than before them

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
- **THEN** the row shows a truncated, single-line snippet of that prompt after its existing status/duration text, with a trailing `...` if it does not fit

#### Scenario: An agent entry with no recorded prompt shows no snippet
- **WHEN** the Logs tab renders an agent-category entry recorded with no last-prompt snapshot
- **THEN** the row shows no prompt snippet, rather than empty quotes or placeholder text

#### Scenario: A prompt entry shows its submitted text
- **WHEN** the Logs tab renders a prompt-category entry
- **THEN** the row shows 🗣️ followed directly by a truncated, single-line snippet of the submitted prompt text, with no "prompt submitted" wording and no surrounding quotes

#### Scenario: A prompt entry that starts a run sorts just ahead of that started entry
- **WHEN** the Logs tab renders a prompt-category entry and the agent-category "started" entry it precipitated, both logged with the same event-occurred time
- **THEN** the prompt entry appears immediately above (more recent than) the started entry, rather than the two appearing in an arbitrary relative order

### Requirement: Project details modal
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in the Agents tab (a project row), the Logs tab (an activity log entry, using that entry's project), or the Reminders tab (a reminder row, using that reminder's project) - the same modal, reached from any of the three tabs, regardless of that entry's category or the truncation of its displayed snippet (a prompt-category entry included - see the "Prompt entry full-text modal" requirement for how that entry's full text is reached instead, from within this modal's own Logs pane). The modal SHALL show five panes: an Agents pane listing every agent registered for that project with its status and process id, a Tests pane showing that project's last test run if any, a Reminders pane listing that project's reminders with each row showing that reminder's name, its duration (in minutes) immediately after the name, and its status - truncated with a trailing `...` rather than overflowing the pane's right border if it doesn't fit, the same ellipsis-truncation behavior the Agents tab's AGENTS and REMINDERS columns use - a Last Prompt pane (described below), and a Logs pane showing that project's activity log entries, most recent first, with each entry's event time rendered in a fixed-width column at the far right of the pane - aligned at the same horizontal position on every row regardless of that row's category/status text length, consistent with how the Agents tab's UPDATED column stays fixed regardless of its other cells' content - and each agent-category entry additionally showing the reporting agent's process id and, when one was recorded for that entry, a truncated one-line snippet of its last-prompt snapshot. In the Logs pane, each test-run-category entry SHALL be prefixed with a tree branch marker (`├─ `) immediately before its status emoji, so it reads as nested beneath the agent activity it ran under, while agent-category and reminder-category entries SHALL render with no such prefix, forming the unindented trunk of the list; this prefix is purely visual and SHALL NOT change entry ordering, the existing category-word prefix, or the fixed-width right-aligned timestamp column. The Logs pane SHALL NOT display column headers. The Agents, Tests, and Reminders panes SHALL be arranged side by side, each occupying one third of the modal's top third (by width); the Last Prompt pane SHALL occupy a thin, fixed-height strip spanning the modal's full width immediately below them; and the Logs pane SHALL occupy the remaining space beneath that - its share reduced from the bottom two-thirds it held before the Last Prompt pane was added, by exactly the Last Prompt pane's fixed height, unchanged in every other respect. Agent and test-run statuses in the Agents and Tests panes, reminder statuses in the Reminders pane, and log entry statuses in the Logs pane, SHALL use the same emoji markers, color styling, and duration as their counterparts in the Agents tab, Reminders tab, and Logs tab respectively, so status is visually consistent wherever it appears; the Agents, Tests, and Reminders panes SHALL omit the category-word prefix used in the Agents, Reminders, and Logs tabs, since each pane itself already establishes the category (e.g. "🤖 done", "❌ failed", "✅ done, 10m"); the modal's Logs pane SHALL include the category-word prefix, consistent with the top-level Logs tab it mirrors. The user SHALL be able to close the modal with `Esc`, returning to whichever tab was active without altering the underlying agent, test-run, or reminder state. The Reminders pane SHALL be paginated per the "Paginated lists support keyboard navigation" requirement when it holds more reminders than fit on a single page, but unlike the Logs pane it SHALL NOT show a pagination keyboard-shortcut hint in its title; instead its title SHALL always show a hint for creating a new reminder (`New [R]`), regardless of whether its reminders overflow a page. Of the modal's five panes, only the Logs pane and the Reminders pane support a highlighted selection and keyboard interaction; the Agents, Tests, and Last Prompt panes remain non-interactive information displays. Exactly one of the Logs pane and Reminders pane holds keyboard focus at a time, and the `Tab` key toggles focus between them while the modal is open; opening the modal from the Agents tab or the Logs tab SHALL default focus to the Logs pane, unchanged from before the Reminders pane was added, while opening it from the Reminders tab SHALL default focus to the Reminders pane with the reminder that was selected in the Reminders tab highlighted. While the Reminders pane holds focus, pressing `Enter` or `s` on its highlighted reminder starts it (if not running) or stops it (if running); pressing `e` opens the reminder creation/editing form pre-populated with its name and duration; pressing `Delete` opens a bold, red confirmation dialog that deletes the reminder on `Enter` or dismisses it on `Escape` without deleting - since the key labeled "delete" on most Mac keyboards sends a Backspace control code rather than a true forward-delete (reaching `Delete` itself needs `Fn`+`Delete`, or a dedicated key on an external/PC keyboard), and `Backspace` has no other meaning in this pane, the TUI SHALL treat `Backspace` the same as `Delete` here. When the Reminders pane does not hold focus, these keys SHALL NOT act on it. Pressing `R` opens the same form empty, to create a new reminder for this project - unlike `Enter`/`s`/`e`/`Delete`, this SHALL work regardless of which pane currently holds focus, since creating a reminder does not depend on a highlighted selection the way those do, and its `New [R]` hint is shown in the Reminders pane's title at all times.

The Last Prompt pane SHALL show the last-prompt text of whichever agent registered for that project most recently had a status update - the tracked agent with the greatest last-updated timestamp among that project's agents - so it tracks "the agent that just finished or needs input" as the user's attention moves between agents in a multi-agent project. The pane's text SHALL wrap to the pane's width; if the full prompt does not fit within the pane's fixed height, it SHALL be truncated with a trailing `...` rather than overflowing or being made scrollable, consistent with the pane being non-interactive. A project with no tracked agents, or whose most-recently-updated agent has no last-prompt text yet (e.g. first tracked via `PreToolUse` or `PostToolUse` before any `UserPromptSubmit` event was seen for it), SHALL show a placeholder indicating no prompt has been captured, rather than an empty pane.

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
- **THEN** the Agents, Tests, and Reminders panes are rendered side by side, each occupying one third of the modal's top third; the Last Prompt pane occupies a thin, fixed-height strip spanning the full width immediately below them; and the Logs pane occupies the remaining space beneath that

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
- **THEN** its "Logs" pane title shows a hint for the pagination keys

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
- **THEN** the title shows the `New [R]` hint and never a pagination keyboard-shortcut hint

#### Scenario: Tab toggles focus between the Logs and Reminders panes
- **WHEN** the user presses `Tab` while the details modal is open
- **THEN** keyboard focus switches from the Logs pane to the Reminders pane, or from the Reminders pane to the Logs pane, whichever currently holds it

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
- **WHEN** the user presses `Tab` while the details modal is open
- **THEN** keyboard focus toggles only between the Logs pane and the Reminders pane, never landing on the Last Prompt pane

## ADDED Requirements

### Requirement: Prompt entry full-text modal
When the details modal's Logs pane holds focus and the user presses `Enter` on its selected log entry, and that entry is a prompt-category entry, the TUI SHALL open a read-only modal overlay showing that entry's full prompt text - consistently, regardless of whether its displayed snippet happened to be truncated, so the user never has to judge a row's length to know what `Enter` will do. This modal SHALL wrap its text to its own width and, if the full text exceeds the modal's height, SHALL make it scrollable with `j`/`k` (and `g`/`G` to jump to its start/end) rather than truncating it - unlike the non-interactive Last Prompt pane, the entire purpose of this modal is to show text the Logs pane's row might not have fit. The user SHALL be able to close it with `Esc`, returning to the details modal with its Logs pane still focused and its selection unchanged, without altering any tracked agent, test-run, or reminder state. `Enter` on any other selected entry in the Logs pane - any entry that is not prompt-category - SHALL have no effect, unchanged from the Logs pane's behavior before this modal existed. The top-level Logs tab's own `Enter` is unaffected by this requirement: per the "Project details modal" requirement, it always opens the project details modal, regardless of the selected entry's category or truncation.

#### Scenario: Enter on any prompt entry in the details modal's Logs pane opens its full text
- **WHEN** the details modal's Logs pane holds focus and the user presses `Enter` on a selected prompt-category entry, whether or not its shown snippet is truncated
- **THEN** the TUI opens a modal, layered above the details modal, showing that entry's full prompt text

#### Scenario: Enter on a non-prompt entry in the Logs pane has no effect
- **WHEN** the details modal's Logs pane holds focus and the user presses `Enter` on a selected entry that is not prompt-category
- **THEN** nothing happens - the pane's entries and selection are unaffected

#### Scenario: Closing the full-prompt modal returns to the details modal
- **WHEN** the user presses `Esc` while the full-prompt modal is open
- **THEN** the TUI closes it and returns to the details modal with its Logs pane still focused and selection unchanged

#### Scenario: A full prompt that overflows the modal is scrollable
- **WHEN** the selected prompt entry's full text, wrapped to the modal's width, exceeds the modal's height
- **THEN** the user can scroll through it with `j`/`k` (and jump to its start/end with `g`/`G`) rather than it being truncated or left unreachable

#### Scenario: Enter on a prompt entry in the Logs tab is unaffected by this requirement
- **WHEN** the user presses `Enter` on a selected prompt-category entry in the top-level Logs tab, truncated or not
- **THEN** the TUI opens the project details modal, per the "Project details modal" requirement, not this full-prompt modal
