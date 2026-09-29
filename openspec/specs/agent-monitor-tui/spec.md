# agent-monitor-tui Specification

## Purpose

Gives the user a single, live-updating terminal view of every Claude Code agent session tracked by the daemon, across nvim, standalone terminals, and the desktop app.

## Requirements

### Requirement: Connect to the daemon
The TUI SHALL connect to the daemon's local socket on startup and clearly inform the user when the daemon is unreachable.

#### Scenario: Daemon is running
- **WHEN** the TUI starts and the daemon's socket is reachable
- **THEN** the TUI connects and begins displaying tracked agents

#### Scenario: Daemon is not running
- **WHEN** the TUI starts and cannot reach the daemon's socket
- **THEN** the TUI displays a clear message that the daemon is not running instead of showing a blank or misleading agent list

### Requirement: Live agent list
The TUI SHALL display tracked agents, test runs, and reminders grouped by working directory as one row per project: each distinct working directory forms exactly one row showing that project's name (derived from the working directory), its tracked agent(s)' status(es) in an Agents cell, that project's test run status (if any) in a Tests cell, and the most recent last-updated time among those members (agent, test run, or reminder) (in the system's local timezone, formatted `%Y-%m-%d %H:%M:%S`), updating as the daemon reports changes. A project row SHALL NOT display per-agent host context or process id as separate columns; the daemon continues tracking that data, it is simply not rendered in the collapsed row. Project rows SHALL be ordered by the most recent last-updated time of any member (agent, test run, or reminder) within them, most recent first. An agent whose status is "running" SHALL contribute a live, counting-up duration to the row, computed from the current time minus that agent's status-since timestamp, formatted as a compact counter (e.g. `2m14s`) and kept current by the TUI's own periodic redraw rather than only refreshing when the daemon pushes an update. An agent whose status is "done" SHALL contribute a fixed duration to the row, computed from that agent's run-started timestamp (the beginning of the run that just completed) to its status-since timestamp (the time it completed) - unlike the "running" duration, this fixed duration does not change on further redraws. An agent in any other status (idle, needs input, stale, or declined) SHALL NOT contribute a duration. A test run whose status is "running" SHALL likewise contribute a live, counting-up duration computed from its run-start timestamp. Once a test run reaches "passed" or "failed", its row SHALL continue to show a duration - the total elapsed time from its run-start timestamp to its last-updated timestamp - rather than showing no duration. When the daemon pushes a removal for a session id, the TUI SHALL drop that session id from its tracked agents, so a pid whose session id has been superseded contributes at most one entry - the current, live session's - to its project row's Agents cell and to that project's Agents pane in the details modal, never a second, frozen entry left over from the superseded session. A reminder's last-updated time (its most recent started/stopped/finished event, or its creation time if it has none) SHALL count toward its project's most-recent last-updated time the same way an agent's or test run's does, even though the reminder's own status is not rendered in this row (see "Agents tab shows a Reminders column").

#### Scenario: A new agent starts
- **WHEN** the daemon reports a newly tracked agent
- **THEN** the TUI adds or updates that agent's project row - creating the row if this is the first member tracked for that working directory - without requiring the user to restart the TUI

#### Scenario: An agent's status changes
- **WHEN** the daemon pushes a status update for an agent already shown
- **THEN** the TUI updates that agent's contribution to its project row's Agents cell in place

#### Scenario: An agent goes stale
- **WHEN** the daemon marks a tracked agent as stale/disconnected
- **THEN** the project row reflects that the agent is no longer active rather than showing its last active status unchanged

#### Scenario: Agents are sorted by recency
- **WHEN** the TUI displays two or more project rows with different most-recent last-updated times among their members
- **THEN** the project with the most recent last-updated time is shown first, and the rest follow in descending order

#### Scenario: An update reorders the list
- **WHEN** a member of a project that is not currently shown first receives a status update
- **THEN** that project's row moves to the top, ahead of projects that have not updated as recently

#### Scenario: Last-updated time is shown in local time
- **WHEN** the TUI renders a project row
- **THEN** the row includes the most recent last-updated time among its members, converted to the system's local timezone and formatted as `%Y-%m-%d %H:%M:%S` (e.g. `2026-09-01 16:32:07`), without a UTC offset or timezone code

#### Scenario: A running agent shows its elapsed duration
- **WHEN** the TUI renders a project row whose agent's status is "running"
- **THEN** the row includes a duration computed from the current time minus that agent's status-since timestamp, formatted compactly (e.g. `9s`, `2m14s`, `1h03m`)

#### Scenario: A done agent shows its total duration
- **WHEN** the TUI renders a project row for an agent whose status is "done"
- **THEN** the row includes a fixed duration computed from that agent's run-started timestamp to its status-since timestamp, rather than showing no duration

#### Scenario: A non-running agent shows no duration
- **WHEN** the TUI renders a project row for an agent whose status is not "running" and not "done" (idle, needs input, stale, or declined)
- **THEN** the row does not include a duration for that agent

#### Scenario: The running duration counts up without a new daemon event
- **WHEN** an agent remains "running" and no new event arrives from the daemon for several seconds
- **THEN** the TUI's displayed duration for that agent still increases over that time, driven by the TUI's own periodic redraw

#### Scenario: A status transition resets the displayed duration
- **WHEN** an agent transitions out of and back into "running" status (for example, "running" to "needs input" and back to "running")
- **THEN** the TUI displays a duration counted from the new status-since timestamp, not accumulated from the earlier running period

#### Scenario: A directory with no tracked agent still shows its test run
- **WHEN** a test run is tracked in a working directory with no tracked agent
- **THEN** the TUI displays a row for that project containing only the test run's status and duration in its Tests cell, with no agent status contributed

#### Scenario: A test run appears in an agent's group
- **WHEN** a test-run event is reported for a working directory with tracked agent(s)
- **THEN** the TUI updates that project's single row to include the test run's status and duration in its Tests cell alongside its agent(s)' statuses in its Agents cell, without creating a duplicate row

#### Scenario: A project's row omits host and process id
- **WHEN** the TUI renders a project row
- **THEN** the row does not include separate host-context or process-id columns

#### Scenario: A started test run shows a live elapsed duration
- **WHEN** the TUI renders a project row whose test run status is "running"
- **THEN** the row includes a duration computed from the current time minus that test run's run-start timestamp, formatted compactly like an agent's running duration and kept current by the TUI's own periodic redraw

#### Scenario: A finished test run keeps showing its total duration
- **WHEN** the TUI renders a project row whose test run status is "passed" or "failed"
- **THEN** the row includes a fixed duration computed from that test run's run-start timestamp to its last-updated timestamp, rather than showing no duration

#### Scenario: A new test run resets the displayed duration
- **WHEN** a project's previously finished test run is replaced by a newly reported test run (a different process)
- **THEN** the TUI displays a duration counted from the new run's run-start timestamp, not accumulated from the previous run

#### Scenario: A removed session id no longer renders as a duplicate row
- **WHEN** the daemon pushes a removal for a session id whose pid is already tracked under a different, newer session id
- **THEN** the TUI drops the removed session id from its tracked agents, so that pid's project row and the details modal's Agents pane each show only the live session's entry, not a second one with a different duration

#### Scenario: A project's reminder activity affects its recency ordering
- **WHEN** a project's most recent reminder event (started, stopped, or finished) is more recent than any of its agents' or test runs' last-updated times
- **THEN** that project's row is ordered using the reminder's last-updated time, and sorts ahead of projects whose most recent activity (of any kind) is older

#### Scenario: The updated column reflects reminder activity
- **WHEN** a project's most recent activity is a reminder event rather than an agent or test-run update
- **THEN** the row's displayed last-updated time is that reminder's last-updated time

### Requirement: Project name column scales with terminal width
The TUI's PROJECT column SHALL NOT be capped at a fixed 20-character width. Its width SHALL scale with the terminal's available width, growing on wider terminals instead of always truncating project names at the same fixed length. The Agents, Tests, and Reminders columns SHALL each be sized to the widest content actually present among the currently displayed rows (never narrower than that column's own header), rather than a fixed proportion of the row's width - so a column whose current content is short or empty does not reserve space it isn't using, and a column that needs more room (e.g. a long reminder name) can draw on that freed-up space instead of being truncated while its neighbors sit mostly blank. Any space left over once PROJECT, Agents, Tests, and Reminders all have what they need SHALL be split among PROJECT, Agents, and Tests in a 2:3:2 ratio; Agents and Tests SHALL continue to receive the majority of it between them, since status segments are typically the widest content in a row. The Reminders column SHALL NOT receive a share of this leftover space beyond what its own content needs. To keep one unusually long row from dominating and squeezing every other column down to nothing, each of PROJECT, Agents, Tests, and Reminders SHALL be capped at half of the total space available to these four columns; content that doesn't fit within a column's resulting width is truncated per the "Overflowing content is truncated with an ellipsis" requirement (PROJECT is the exception - it clips without an ellipsis, consistent with existing table-rendering behavior for very long names). The UPDATED column's width is unaffected by this requirement.

#### Scenario: A wide terminal shows more of a long project name
- **WHEN** the TUI renders in a terminal wide enough to fit a project name longer than 20 characters alongside the Agents, Tests, and UPDATED columns
- **THEN** the PROJECT column renders more than 20 characters of that name, rather than clipping it at 20

#### Scenario: A narrow terminal still clips names that don't fit
- **WHEN** the TUI renders in a terminal too narrow to fit a project name in full alongside the other columns
- **THEN** the PROJECT column clips the name to the space available, consistent with existing table-rendering behavior

#### Scenario: A column with short content frees up space for its neighbors
- **WHEN** the Agents and Tests columns' current content is short (e.g. a single agent idle, no test run) while the Reminders column needs more room than a fixed proportional share would give it
- **THEN** the Agents and Tests columns shrink to what their content needs and the Reminders column uses the space that frees up, rather than staying cramped next to mostly-blank columns

#### Scenario: A column with blank content still shows its header
- **WHEN** a column (e.g. Reminders) has no content to show for any currently displayed row
- **THEN** that column is still at least as wide as its own header text

#### Scenario: One row's unusually long content does not starve the other columns
- **WHEN** a single row's content in one column (e.g. a very long reminder name) would otherwise claim most of the table's width
- **THEN** that column is capped at half of the four dynamic columns' total available space, and its content is truncated with an ellipsis to fit rather than shrinking every other column to fit it

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) and test-run statuses (running, passed, failed) from one another so the user can scan the list and immediately identify agents or test runs needing attention. Each agent status SHALL be prefixed with a distinct emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🔔, done with ✅, stale with 👻, and declined with 🚫. Each test-run status SHALL be prefixed with a distinct emoji marker: running with ⏳, passed with ✅, and failed with ❌. A reminder's status SHALL reuse these existing emoji markers rather than introducing new ones: running reuses ⏳ (the same marker as a running test run) and done reuses ✅ (the same marker as a done agent or a passed test run), so a reminder's status is instantly recognizable by an emoji the user already associates with "in progress" or "finished". In the Agents tab's Reminders column, a reminder's entire status segment - its emoji, its name, and its trailing elapsed/ETA/outcome text - SHALL be colored as one unit: the same Blue used for a running test run while running, and the same Green used for a done agent or a passed test run while done; this matches how the Agents and Tests columns already color their entire status text rather than only a leading emoji, and how the Reminders tab and the details modal's Reminders pane already color a reminder's full status text. In the Logs tab and the details modal's Logs pane, a reminder-category log entry SHALL instead use a single fixed marker, ⏰, regardless of whether its status is started, stopped, or finished - unlike agent and test-run entries, which use a different emoji per status - since a reminder log entry's value is in marking it as a reminder event at a glance, not in re-deriving its status from an already text-labeled entry. Its color SHALL still vary by status, though: "finished" (a natural completion) SHALL use the same Green used for a "done" agent or a "passed" test run; "stopped" (a manual interruption) SHALL use a color distinct from both "started" and "finished", since it is neither the beginning nor the successful end of a run. When a project's Agents cell reflects more than one tracked agent in different statuses, the cell SHALL show each distinct status present rather than collapsing them into a single "winning" status, so no status needing attention is hidden behind another. On the currently-selected row in the Agents tab, the Logs tab, or the Reminders tab, the TUI SHALL override every status's own foreground color with a single fixed foreground color chosen to stay legible against the row-highlight background, rather than requiring the row-highlight background to avoid every status's own color.

#### Scenario: An agent needs input
- **WHEN** an agent's status is "needs input"
- **THEN** that status's contribution to the row displays the 🔔 marker and is visually distinguished (e.g. color) from other statuses shown, using bold colored text rather than a solid background fill

#### Scenario: Each status has a distinct emoji marker
- **WHEN** the TUI renders an agent's status within a project row
- **THEN** that status is prefixed with the emoji for that status (🔧 running, 💤 idle, 🔔 needs input, ✅ done, 👻 stale, 🚫 declined)

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
- **THEN** that status displays the ✅ marker, the same marker a done agent or a passed test run uses

#### Scenario: A running reminder's full Agents-tab segment is colored blue
- **WHEN** the Agents tab's Reminders column renders a reminder whose status is running
- **THEN** the whole segment - the ⏳ marker, the reminder's name, and its elapsed/ETA text - is colored the same Blue used for a running test run, not only the marker

#### Scenario: A done reminder's full Agents-tab segment is colored green
- **WHEN** the Agents tab's Reminders column renders a reminder whose status is done
- **THEN** the whole segment - the ✅ marker, the reminder's name, and its elapsed/outcome text - is colored the same Green used for a done agent or a passed test run, not only the marker

#### Scenario: A reminder log entry always shows the clock marker
- **WHEN** the Logs tab or the details modal's Logs pane renders a log entry whose category is reminder
- **THEN** that entry displays the ⏰ marker regardless of whether its status is started, stopped, or finished

#### Scenario: A finished reminder log entry is green
- **WHEN** the Logs tab or the details modal's Logs pane renders a reminder log entry whose status is finished
- **THEN** that entry's text is colored the same Green used for a "done" agent or a "passed" test run

#### Scenario: A stopped reminder log entry is visually distinct from started and finished
- **WHEN** the Logs tab or the details modal's Logs pane renders a reminder log entry whose status is stopped
- **THEN** that entry's text is colored differently from both a "started" and a "finished" reminder log entry

### Requirement: Navigation and quit do not affect tracked agents
The TUI SHALL support quitting the application via a keybinding, and quitting the TUI SHALL NOT stop the daemon or any tracked Claude Code agent.

#### Scenario: User quits the TUI
- **WHEN** the user presses the quit key
- **THEN** the TUI process exits while the daemon keeps running and continues tracking agents

### Requirement: Reconnect after daemon restart
The TUI SHALL detect when its connection to the daemon drops and attempt to reconnect, resuming display of current agent state once reconnected.

#### Scenario: Daemon restarts while the TUI is open
- **WHEN** the daemon process restarts (e.g. after an update) while the TUI is running
- **THEN** the TUI detects the dropped connection, retries connecting, and repopulates the agent list once the daemon is back

### Requirement: Tab navigation between Agents and Logs
The TUI SHALL organize its display into three tabs: **Agents** (the existing project table), **Logs** (an aggregated activity log view), and **Reminders** (an aggregated, cross-project reminder list). The user SHALL be able to cycle between tabs with the `Tab` key, and jump directly to a tab with `A`/`a` (Agents), `L`/`l` (Logs), or `R`/`r` (Reminders).

#### Scenario: Cycling tabs
- **WHEN** the user presses `Tab`
- **THEN** the TUI switches to the next tab in the Agents -> Logs -> Reminders -> Agents cycle

#### Scenario: Jumping directly to a tab
- **WHEN** the user presses `A`/`a`, `L`/`l`, or `R`/`r`
- **THEN** the TUI switches to the Agents, Logs, or Reminders tab respectively, even if that tab is already active

### Requirement: Agents tab supports row selection
The Agents tab SHALL support moving a selection cursor over its project rows using the `j`/`down` (next row) and `k`/`up` (previous row) keys, so a specific project can be chosen for its details view. The selection SHALL be visually distinguished from unselected rows.

#### Scenario: Moving the selection down
- **WHEN** the user presses `j` or the down arrow while the Agents tab is active
- **THEN** the selection cursor moves to the next project row, if one exists

#### Scenario: Moving the selection up
- **WHEN** the user presses `k` or the up arrow while the Agents tab is active
- **THEN** the selection cursor moves to the previous project row, if one exists

### Requirement: Project details modal
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in the Agents tab (a project row), the Logs tab (an activity log entry, using that entry's project), or the Reminders tab (a reminder row, using that reminder's project) - the same modal, reached from any of the three tabs. The modal SHALL show four panes: an Agents pane listing every agent registered for that project with its status and process id, a Tests pane showing that project's last test run if any, a Reminders pane listing that project's reminders with each row showing that reminder's name, its duration (in minutes) immediately after the name, and its status - truncated with a trailing `...` rather than overflowing the pane's right border if it doesn't fit, the same ellipsis-truncation behavior the Agents tab's AGENTS and REMINDERS columns use - and a Logs pane showing that project's activity log entries, most recent first, with each entry's event time rendered in a fixed-width column at the far right of the pane - aligned at the same horizontal position on every row regardless of that row's category/status text length, consistent with how the Agents tab's UPDATED column stays fixed regardless of its other cells' content - and each agent-category entry additionally showing the reporting agent's process id. In the Logs pane, each test-run-category entry SHALL be prefixed with a tree branch marker (`├─ `) immediately before its status emoji, so it reads as nested beneath the agent activity it ran under, while agent-category and reminder-category entries SHALL render with no such prefix, forming the unindented trunk of the list; this prefix is purely visual and SHALL NOT change entry ordering, the existing category-word prefix, or the fixed-width right-aligned timestamp column. The Logs pane SHALL NOT display column headers. The Agents, Tests, and Reminders panes SHALL be arranged side by side, each occupying one third of the modal's top third (by width), and the Logs pane SHALL occupy the remaining bottom two-thirds, unchanged from before the Reminders pane was added. Agent and test-run statuses in the Agents and Tests panes, reminder statuses in the Reminders pane, and log entry statuses in the Logs pane, SHALL use the same emoji markers, color styling, and duration as their counterparts in the Agents tab, Reminders tab, and Logs tab respectively, so status is visually consistent wherever it appears; the Agents, Tests, and Reminders panes SHALL omit the category-word prefix used in the Agents, Reminders, and Logs tabs, since each pane itself already establishes the category (e.g. "✅ done", "❌ failed", "✅ done, 10m"); the modal's Logs pane SHALL include the category-word prefix, consistent with the top-level Logs tab it mirrors. The user SHALL be able to close the modal with `Esc`, returning to whichever tab was active without altering the underlying agent, test-run, or reminder state. The Reminders pane SHALL be paginated per the "Paginated lists support keyboard navigation" requirement when it holds more reminders than fit on a single page, but unlike the Logs pane it SHALL NOT show a pagination keyboard-shortcut hint in its title; instead its title SHALL always show a hint for creating a new reminder (`New [R]`), regardless of whether its reminders overflow a page. Of the modal's four panes, only the Logs pane and the Reminders pane support a highlighted selection and keyboard interaction; the Agents and Tests panes remain non-interactive information displays. Exactly one of the Logs pane and Reminders pane holds keyboard focus at a time, and the `Tab` key toggles focus between them while the modal is open; opening the modal from the Agents tab or the Logs tab SHALL default focus to the Logs pane, unchanged from before the Reminders pane was added, while opening it from the Reminders tab SHALL default focus to the Reminders pane with the reminder that was selected in the Reminders tab highlighted. While the Reminders pane holds focus, pressing `Enter` or `s` on its highlighted reminder starts it (if not running) or stops it (if running); pressing `e` opens the reminder creation/editing form pre-populated with its name and duration; pressing `Delete` opens a bold, red confirmation dialog that deletes the reminder on `Enter` or dismisses it on `Escape` without deleting - since the key labeled "delete" on most Mac keyboards sends a Backspace control code rather than a true forward-delete (reaching `Delete` itself needs `Fn`+`Delete`, or a dedicated key on an external/PC keyboard), and `Backspace` has no other meaning in this pane, the TUI SHALL treat `Backspace` the same as `Delete` here. When the Reminders pane does not hold focus, these keys SHALL NOT act on it. Pressing `R` opens the same form empty, to create a new reminder for this project - unlike `Enter`/`s`/`e`/`Delete`, this SHALL work regardless of which pane currently holds focus, since creating a reminder does not depend on a highlighted selection the way those do, and its `New [R]` hint is shown in the Reminders pane's title at all times.

#### Scenario: Opening a project's details from the Agents tab
- **WHEN** the user presses `Enter` on a selected project row in the Agents tab
- **THEN** the TUI displays a modal overlay with that project's Agents, Tests, Reminders, and Logs panes, with the Logs pane focused

#### Scenario: Opening a project's details from the Logs tab
- **WHEN** the user presses `Enter` on a selected activity log entry in the Logs tab
- **THEN** the TUI displays the same modal overlay, scoped to that entry's project, with the Logs pane focused

#### Scenario: Opening a project's details from the Reminders tab
- **WHEN** the user presses `Enter` on a selected reminder row in the Reminders tab
- **THEN** the TUI displays the same modal overlay, scoped to that reminder's project, with the Reminders pane focused and that reminder highlighted

#### Scenario: Agents and Tests panes share the top third
- **WHEN** the details modal is open
- **THEN** the Agents, Tests, and Reminders panes are rendered side by side, each occupying one third of the modal's top third, and the Logs pane occupies the bottom two-thirds beneath them

#### Scenario: Statuses are visually consistent with their tab
- **WHEN** the details modal renders an agent's status, a test run's status, a reminder's status, or a log entry's status
- **THEN** that status uses the same emoji marker, color, and duration as it would in the Agents tab, Reminders tab, or Logs tab

#### Scenario: The Agents pane shows each agent's process id
- **WHEN** the details modal renders its Agents pane
- **THEN** each listed agent's row includes that agent's process id alongside its status

#### Scenario: The Logs pane shows the process id for agent activities
- **WHEN** the details modal's Logs pane renders a log entry whose category is agent
- **THEN** that entry's row includes the reporting agent's process id

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
- **THEN** it renders a highlighted selected row and a vertical scrollbar along its right edge

#### Scenario: The Logs pane's title gains a pagination hint once its entries overflow a page
- **WHEN** the details modal's Logs pane holds more entries than fit on a single page
- **THEN** its "Logs" pane title shows a hint for the pagination keys

#### Scenario: No selection, scrollbar, or hint when the Logs pane's entries all fit on one page
- **WHEN** the details modal opens for a project whose activity log entries all fit within the Logs pane's single page
- **THEN** the pane renders with no highlighted row, no scrollbar, and no pagination hint in its title

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

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects, as a paginated list ordered by recency (most recent first) by default. Each entry SHALL show at least its project, event category, status, and event time, with event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. A test-run entry whose status is "passed" or "failed", an agent entry whose status is "done", or a reminder entry whose status is "finished" or "stopped", SHALL additionally show the elapsed duration of that run/task/reminder, computed from the most recent preceding "started" entry of the same category logged for the same project - for a reminder entry, additionally matched to the "started" entry recording the same reminder name, since a project can have more than one reminder interleaved in the log and an agent/test-run has no equivalent identity to disambiguate by. A reminder entry whose status is "started" SHALL additionally show its recorded ETA (the reminder's local-time due time, `HH:MM`, as recorded on that entry - see the activity-log capability), rather than the elapsed duration shown for other completed entries, since a just-started reminder has no elapsed run to show. Whenever the list holds more entries than fit on a single page, the Logs tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) entry already fits on a single page.

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
- **THEN** the row includes that entry's recorded ETA (`HH:MM`, local time) alongside its name

#### Scenario: Interleaved reminders in the same project do not cross-match durations
- **WHEN** the Logs tab renders a "finished" or "stopped" reminder entry, and a different reminder in the same project also has "started"/"finished"/"stopped" entries interleaved with it in the log
- **THEN** the displayed duration is computed only from that reminder's own preceding "started" entry (matched by reminder name), not another reminder's

### Requirement: Logs tab supports sorting and filtering
The Logs tab SHALL support sorting its entries by recency (the default), by Project, or by Status, and filtering the displayed entries by Project or by Status.

#### Scenario: Changing sort order
- **WHEN** the user selects Project or Status as the sort option on the Logs tab
- **THEN** the displayed entries reorder accordingly instead of by recency

#### Scenario: Filtering by project
- **WHEN** the user applies a Project filter on the Logs tab
- **THEN** only entries belonging to that project are displayed

#### Scenario: Filtering by status
- **WHEN** the user applies a Status filter on the Logs tab
- **THEN** only entries matching that status are displayed

### Requirement: Reminders tab shows an aggregated, paginated reminder list
The Reminders tab SHALL display every project's reminders, aggregated across all projects, as a paginated list ordered by recency (the last time each reminder completed a run - stopped or naturally finished - most recent first) by default. Each row SHALL show the reminder's Project, Name, Duration (in minutes), Status, and Updated columns. The Status column SHALL be blank when the reminder has no started/stopped/finished log event yet (it is still not-yet-started). While a reminder is running, the Status column SHALL show its live elapsed duration and its estimated time of completion, formatted like "Running 1m23s, ETA: 12:34", prefixed with the ⏳ marker, and kept current by the TUI's own periodic redraw, consistent with a running agent's or test run's duration. Once a reminder is done (stopped or finished), the Status column SHALL show its elapsed duration formatted like "Done 10m", prefixed with the ✅ marker. The Updated column SHALL show the time of the reminder's most recent started, stopped, or finished log event, or, if it has none, the time the reminder was created. Whenever the list holds more reminders than fit on a single page, the Reminders tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, positioned so the scrollbar's thumb reaches the very bottom of its track when the list's last page is being shown (and the very top when its first page is being shown), and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) reminder already fits on a single page.

#### Scenario: Reminders tab lists reminders across projects
- **WHEN** the user switches to the Reminders tab
- **THEN** the TUI displays reminders from every project, most recent first by their last completed run, split into pages

#### Scenario: A reminder with no runs shows a blank status
- **WHEN** the Reminders tab renders a reminder that is still not-yet-started
- **THEN** its Status column is blank

#### Scenario: A running reminder shows its live duration and ETA
- **WHEN** the Reminders tab renders a reminder whose status is running
- **THEN** its Status column shows the ⏳ marker, the current elapsed duration, and the estimated completion time (e.g. "⏳ Running 1m23s, ETA: 12:34"), updating as the TUI redraws

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

### Requirement: Reminders tab supports sorting and filtering
The Reminders tab SHALL support sorting its reminders by recency (the default, per the last completed run), by Project, or by Status, and filtering the displayed reminders by Project or by Status, mirroring the Logs tab's sort/filter controls. Because `s` is reserved on the Reminders tab for starting/stopping the highlighted reminder, the status filter SHALL cycle on `f` instead of `s`; the sort-cycle key (`o`), the project-filter-cycle key (`p`), and the clear-filters key (`c`) SHALL remain the same as the Logs tab's.

#### Scenario: Changing sort order
- **WHEN** the user selects Project or Status as the sort option on the Reminders tab
- **THEN** the displayed reminders reorder accordingly instead of by recency

#### Scenario: Filtering by project
- **WHEN** the user applies a Project filter on the Reminders tab
- **THEN** only reminders belonging to that project are displayed

#### Scenario: Filtering by status
- **WHEN** the user presses `f` to apply a Status filter on the Reminders tab
- **THEN** only reminders matching that status are displayed

#### Scenario: s is reserved for start/stop, not the status filter
- **WHEN** the user presses `s` on the Reminders tab
- **THEN** the highlighted reminder starts or stops rather than the status filter cycling, unlike on the Logs tab where `s` cycles the status filter

### Requirement: Reminders tab row selection and start/stop
The Reminders tab SHALL support moving a selection cursor over its reminder rows using `j`/`down` and `k`/`up`, consistent with other paginated lists. Pressing `Enter` on the highlighted reminder SHALL open the project details modal scoped to that reminder's project, with the Reminders pane focused and that reminder highlighted. Pressing `s` on the highlighted reminder SHALL start it if it is not running, or stop it if it is running - the same toggle the details modal's Reminders pane supports.

#### Scenario: Opening details from the Reminders tab
- **WHEN** the user presses `Enter` on a selected reminder row in the Reminders tab
- **THEN** the TUI displays the details modal scoped to that reminder's project, with the Reminders pane focused and that reminder highlighted

#### Scenario: Starting a reminder from the Reminders tab
- **WHEN** the user presses `s` on a highlighted reminder that is not running
- **THEN** the TUI sends a start request for that reminder

#### Scenario: Stopping a reminder from the Reminders tab
- **WHEN** the user presses `s` on a highlighted reminder that is running
- **THEN** the TUI sends a stop request for that reminder, and no notification is sent

### Requirement: Paginated lists support keyboard navigation
Any paginated list in the TUI (the Logs tab's activity list, the Reminders tab's reminder list, the details modal's Logs pane, and the details modal's Reminders pane) SHALL support `j`/`down` and `k`/`up` to move the selection one line at a time, `d`/page-down and `u`/page-up to move by a full page, and `g`/`G` to jump directly to the list's first or last entry respectively. A page is however many entries currently fit in the list's rendered area at the current terminal size. Paging SHALL overlap the previous page by exactly 1 row: paging down SHALL advance the page's top row by (page size - 1) rows, and paging up SHALL move the page's top row back by (page size - 1) rows, in both cases selecting the new page's top row, clamped so the page never scrolls past the list's first or last entry. Pressing `g` SHALL select the list's first entry and scroll the page so that entry is the page's top row; pressing `G` SHALL select the list's last entry and scroll the page so the last page is showing, consistent with how `d`/page-down already clamps at the list's last page. While the details modal is open, its Logs pane and Reminders pane are each paginated lists, but only the one currently holding keyboard focus (per the "Project details modal" requirement's `Tab`-toggled focus) reacts to `j`/`k`/`d`/`u`/`g`/`G`/page-down/page-up; the other pane, and the Agents, Logs, and Reminders tabs underneath, SHALL NOT receive or react to those keys while the modal is open, consistent with the details modal taking precedence for keyboard shortcuts over whatever page is open beneath it.

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
- **THEN** only the pane currently holding focus (the Logs pane or the Reminders pane) updates its selection and page; the other pane, and the Agents, Logs, and Reminders tabs underneath, remain unchanged

#### Scenario: Jumping to the first entry
- **WHEN** the user presses `g` on a paginated list that is not already showing its first page
- **THEN** the selection moves to the list's first entry and the page scrolls so that entry is the top row shown

#### Scenario: Jumping to the last entry
- **WHEN** the user presses `G` on a paginated list that is not already showing its last page
- **THEN** the selection moves to the list's last entry and the page scrolls so the list's final page is showing, with the last entry visible

#### Scenario: Jumping to the first or last entry when already there is a no-op
- **WHEN** the user presses `g` while the list's first entry is already selected, or `G` while the list's last entry is already selected
- **THEN** the selection and page remain unchanged

### Requirement: Keyboard shortcuts help modal
The TUI SHALL open a help modal listing all available keyboard shortcuts when the user presses `?`. The user SHALL be able to close it with `Esc`. The listed shortcuts SHALL include `g`/`G` (jump to first/last entry) alongside the other paginated-list navigation keys (`j`/`k`, `d`/`u`).

#### Scenario: Opening help
- **WHEN** the user presses `?`
- **THEN** the TUI displays a modal overlay listing the available keyboard shortcuts

#### Scenario: Closing help
- **WHEN** the user presses `Esc` while the help modal is open
- **THEN** the TUI closes the help modal and returns to the previously active tab

#### Scenario: Help lists the jump-to-start/end shortcuts
- **WHEN** the user opens the help modal
- **THEN** the listed shortcuts include `g` and `G` for jumping to a paginated list's first and last entry

### Requirement: Empty-state placeholders appear in the table body, not the pane title
When the Agents tab has no tracked agents or test runs, and when the Logs tab's full (unfiltered) activity log is empty, the TUI SHALL show that empty state as gray placeholder text in the table body beneath the header, matching the existing convention used when a Logs tab filter matches nothing. The pane title SHALL NOT carry this placeholder text; the Logs tab's title SHALL continue to show its sort/filter control hints regardless of whether the log is empty.

#### Scenario: No agents tracked yet
- **WHEN** the daemon reports no tracked agents and no test runs
- **THEN** the Agents tab's title reads "Agents" and its table body shows gray placeholder text indicating no agents are tracked yet

#### Scenario: No activity logged yet
- **WHEN** the daemon's activity log is empty
- **THEN** the Logs tab's title shows only its sort/filter control hints (no empty-state text) and its table body shows gray placeholder text indicating no activity has been logged yet

### Requirement: Agents tab splits agent and test-run status into separate columns
The Agents tab SHALL display a project's agent status(es) and its test-run status in two separate columns, **Agents** and **Tests**, instead of one combined STATUS column. The Agents column SHALL show that project's tracked agent(s)' status(es); the Tests column SHALL show that project's test run status, if any, or remain empty if the project has no tracked test run.

#### Scenario: The Agents tab header shows separate columns
- **WHEN** the Agents tab renders its header row
- **THEN** it shows "AGENTS" and "TESTS" as separate column headers rather than a single "STATUS" header

#### Scenario: A project with only agents leaves the Tests column empty
- **WHEN** a project has tracked agent(s) but no tracked test run
- **THEN** its row's Agents column shows the agent status(es) and its Tests column is empty

#### Scenario: A project with only a test run leaves the Agents column empty
- **WHEN** a project has a tracked test run but no tracked agent
- **THEN** its row's Tests column shows the test run's status and its Agents column is empty

### Requirement: Agents tab shows a Reminders column
The Agents tab SHALL display a fourth column, **Reminders**, alongside PROJECT, Agents, Tests, and UPDATED. A project with a running reminder SHALL show a status emoji per the "Status is visually distinguishable" requirement, then that reminder's name (bold), then its live elapsed duration and ETA, formatted like "⏳ **Check the thing** 1m23s, ETA 12:34" - without the category-word prefix the Agents and Tests columns carry, per the "Status labels are prefixed by category outside dedicated panes" requirement's exception for this column. A project whose most recent reminder activity is a stop or a natural finish, with no reminder currently running, SHALL show its emoji, name, and outcome and duration, formatted like "✅ **Check the thing** completed/stopped, 10m". A project with two or more reminders with activity to show SHALL present them separated by a bullet, ordered by recency, the same convention the Agents column already uses for multiple agents. A project with no reminders SHALL leave the Reminders column empty. If either the Agents column's or the Reminders column's content exceeds its available column width, the TUI SHALL truncate it with a trailing `...` rather than wrapping or overflowing into adjacent columns.

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

### Requirement: Agents tab supports incremental search by project name
The Agents tab SHALL support filtering its project rows by a search string matched as a case-insensitive substring against each row's project name (the name derived from its working directory, as already shown in the PROJECT column). Pressing `/` while the Agents tab is active SHALL enter search-editing mode, replacing the tab's title with a live text-entry prompt showing the search string typed so far and a cursor at its end. While editing, each character typed SHALL append to the search string and each `Backspace` SHALL remove its last character, and the Agents tab's displayed rows SHALL update immediately after every such change to show only rows whose project name matches the current search string, or every row if the search string is empty. Rows that match the search string SHALL retain the same relative order (most recent update first) they would have with no filter applied. This search filter SHALL apply only to the Agents tab; the Logs and Reminders tabs' own filters are unaffected and unaffected by it.

#### Scenario: Entering search-editing mode
- **WHEN** the user presses `/` while the Agents tab is active
- **THEN** the Agents tab's title becomes a text-entry prompt showing a cursor, and no rows are hidden yet beyond whatever filter was already applied

#### Scenario: Typing narrows the list as you go
- **WHEN** the user is in search-editing mode and types a character
- **THEN** the character is appended to the search string and the Agents tab immediately shows only rows whose project name contains that search string, case-insensitively

#### Scenario: Backspace removes the last character
- **WHEN** the user is in search-editing mode and presses `Backspace`
- **THEN** the last character of the search string is removed and the displayed rows update to match the shorter string

#### Scenario: Matching rows keep their existing relative order
- **WHEN** more than one project row matches the current search string
- **THEN** the matching rows are displayed in the same relative order (most recent update first) they would use with no filter applied

#### Scenario: A newly-tracked agent for a matching project appears while filtered
- **WHEN** a search filter is applied on the Agents tab and an agent is newly tracked for a project whose name matches that filter
- **THEN** that project's row appears in the filtered list like any other row that matches

### Requirement: Committing, cancelling, and clearing the Agents tab search
Pressing `Enter` while in search-editing mode SHALL commit the current search string as the Agents tab's applied filter and exit search-editing mode; if the committed string is empty, this SHALL clear any previously applied filter instead. Pressing `Escape` while in search-editing mode SHALL discard the in-progress edit and exit search-editing mode, restoring whichever filter (if any) was applied before `/` was pressed, leaving it unchanged. Re-entering search-editing mode with `/` while an applied filter is already present SHALL start the edit buffer from that filter's text rather than empty, so refining or clearing it does not require retyping it.

#### Scenario: Committing a non-empty search
- **WHEN** the user presses `Enter` in search-editing mode with a non-empty search string
- **THEN** that string becomes the Agents tab's applied filter, search-editing mode ends, and the tab's title displays the applied filter

#### Scenario: Committing an empty search clears the filter
- **WHEN** the user presses `Enter` in search-editing mode with an empty search string
- **THEN** any previously applied Agents tab filter is cleared, search-editing mode ends, and every row is shown again

#### Scenario: Cancelling an edit restores the prior filter
- **WHEN** the user presses `Escape` in search-editing mode after having already applied a filter, and edits it further before cancelling
- **THEN** search-editing mode ends and the previously applied filter is restored unchanged, discarding the in-progress edit

#### Scenario: Cancelling an edit with no prior filter clears the prompt
- **WHEN** the user presses `Escape` in search-editing mode and no filter was applied beforehand
- **THEN** search-editing mode ends with no filter applied and every row is shown

#### Scenario: Re-editing an applied filter starts from its text
- **WHEN** the user presses `/` while an applied filter is already showing in the Agents tab title
- **THEN** search-editing mode begins with the edit buffer pre-filled with that filter's text and the cursor at its end

### Requirement: Agents tab title reflects search state
The Agents tab's title SHALL show a `Filter [/]` keyboard shortcut hint whenever search-editing mode is not active, matching the hint convention already used by the Logs and Reminders tabs' own sort/filter controls, which stay visible whether or not a filter is currently applied. When a search filter is applied (and search-editing mode is not active), the title SHALL display the applied filter text immediately after that hint, bold and in a color distinct from the title's normal text, so an active filter remains obvious at a glance without hiding how to change it.

#### Scenario: Hint shown with no applied filter
- **WHEN** the Agents tab is active, not in search-editing mode, and no filter is applied
- **THEN** the tab's title includes a `Filter [/]` keyboard shortcut hint

#### Scenario: Hint remains visible alongside an applied filter
- **WHEN** the Agents tab has an applied search filter and is not in search-editing mode
- **THEN** the tab's title still includes the `Filter [/]` keyboard shortcut hint, immediately followed by the applied filter text

#### Scenario: Applied filter is visually distinguished
- **WHEN** the Agents tab has an applied search filter and is not in search-editing mode
- **THEN** the filter text shown in the title is bold and a distinct color from the title's normal text

### Requirement: Status labels are prefixed by category outside dedicated panes
In the Agents tab and the Logs tab, each status label SHALL be prefixed with a category word - "agent" for an agent status, "tests" for a test-run status, or "reminder" for a reminder-category log entry - immediately after its emoji marker and before the status word (e.g. "✅ agent done", "⏳ tests started", "⏰ reminder finished"), so a label read in a cross-category view is unambiguous about which kind of status it names. The Agents tab's Reminders column is the one exception to this cross-category rule: it SHALL NOT include the category-word prefix, since the column's narrow width prioritizes the reminder's own name (the primary content of that cell) and the REMINDERS column header already establishes the category. In the details modal, the Agents pane, Tests pane, and Reminders pane SHALL NOT include this category-word prefix, since the pane itself already establishes the category (e.g. "✅ done", "❌ failed"); the modal's Logs pane SHALL include the category-word prefix, consistent with the top-level Logs tab it mirrors. The top-level Reminders tab's Status column SHALL likewise NOT include this category-word prefix, since the tab already dedicates its entire view to reminders, consistent with how the modal's Agents and Tests panes omit it.

#### Scenario: The Agents tab prefixes an agent status with its category word
- **WHEN** the Agents tab renders an agent's status in its Agents column
- **THEN** the label reads the emoji, then "agent", then the status word (e.g. "✅ agent done"), rather than omitting the category word

#### Scenario: The Agents tab prefixes a test-run status with its category word
- **WHEN** the Agents tab renders a project's test-run status in its Tests column
- **THEN** the label reads the emoji, then "tests", then the status word (e.g. "⏳ tests running"), rather than omitting the category word

#### Scenario: The Agents tab's Reminders column omits the category-word prefix
- **WHEN** the Agents tab renders a project's reminder status in its Reminders column
- **THEN** the label omits the category word (e.g. an emoji followed directly by the reminder's bold name), unlike its Agents and Tests columns

#### Scenario: The Logs tab prefixes entries with their category word
- **WHEN** the Logs tab renders an entry
- **THEN** the label includes the category word ("agent", "tests", or "reminder") alongside its emoji and status, consistent with the Agents tab

#### Scenario: The details modal's Agents and Tests panes omit the category-word prefix
- **WHEN** the details modal renders a status in its Agents pane, Tests pane, or Reminders pane
- **THEN** the label omits the category word (e.g. "✅ done", not "✅ agent done")

#### Scenario: The details modal's Logs pane keeps the category-word prefix
- **WHEN** the details modal renders an entry in its Logs pane
- **THEN** the label includes the category word, consistent with the top-level Logs tab

#### Scenario: The Reminders tab omits the category-word prefix
- **WHEN** the Reminders tab renders a reminder's status in its Status column
- **THEN** the label omits the category word (e.g. "⏳ running"), since the tab already dedicates its view to reminders

### Requirement: Reminder creation and editing form
The TUI SHALL provide a modal form, opened from the details modal's Reminders pane, for creating (`R`) or editing (`e`) a reminder, with a Name field and a Duration-in-minutes field. `Tab` SHALL move focus between the two fields. When opened for editing, both fields SHALL be pre-populated with the highlighted reminder's current name and duration. Pressing `Enter` SHALL save the form - creating a new reminder for the modal's project, or applying the edited name/duration to the reminder being edited - and close the form. Pressing `Escape` SHALL close the form without creating a reminder or applying any edit made since it was opened. While the form is open, the TUI SHALL show the terminal's own text cursor positioned immediately after the active field's current text (before the Duration field's trailing "m"), so the user can see where the next typed character will land; the cursor SHALL move as focus moves between fields and as characters are typed or removed. Once the daemon confirms a newly-created reminder (the daemon assigns its id, so this cannot happen synchronously with `Enter`), the TUI SHALL highlight it in the Reminders pane - focusing that pane if it does not already hold focus - and start it, the same as if the user had pressed `Enter`/`s` on it themselves; this SHALL NOT happen for an edited (as opposed to newly-created) reminder.

#### Scenario: A newly-created reminder is highlighted and started automatically
- **WHEN** the user creates a reminder and the daemon confirms it with the id it assigned
- **THEN** the TUI highlights that reminder in the Reminders pane, focusing the pane if needed, and sends a start request for it - without the user needing to select and start it themselves

#### Scenario: Editing a reminder does not auto-start it
- **WHEN** the user edits an existing reminder's name or duration and the daemon confirms the change
- **THEN** the TUI does not send a start request for it, and does not change which reminder is highlighted

#### Scenario: Creating a reminder
- **WHEN** the user presses `R` in the details modal's Reminders pane, enters a name and a duration, and presses `Enter`
- **THEN** the TUI creates a new reminder for that project with the entered name and duration, and closes the form

#### Scenario: Canceling a new reminder
- **WHEN** the user presses `R`, enters some text, and then presses `Escape`
- **THEN** the TUI closes the form without creating a reminder

#### Scenario: Editing a reminder
- **WHEN** the user presses `e` on a highlighted reminder, the form opens pre-populated with its current name and duration, the user changes a field, and presses `Enter`
- **THEN** the TUI applies the change to that reminder and closes the form

#### Scenario: Discarding an edit
- **WHEN** the user presses `e` on a highlighted reminder, changes a field, and presses `Escape`
- **THEN** the TUI closes the form and the reminder's name and duration remain as they were before the form opened

#### Scenario: Tab moves between the form's fields
- **WHEN** the reminder form is open and the user presses `Tab`
- **THEN** focus moves from the Name field to the Duration field, or back, so the user can edit either without leaving the form

#### Scenario: The cursor tracks the active field as the user types
- **WHEN** the reminder form is open and the user types or deletes characters in the active field
- **THEN** the terminal's cursor is shown immediately after that field's current text, advancing or retreating with each keystroke

#### Scenario: The cursor moves when focus moves between fields
- **WHEN** the reminder form is open and the user presses `Tab`
- **THEN** the cursor moves to the end of the newly-focused field's current text

### Requirement: Reminder deletion confirmation
Pressing `Delete` (or `Backspace` - see the "Project details modal" requirement's note on Mac keyboards) on a highlighted reminder in the details modal's Reminders pane SHALL open a bold, red confirmation dialog naming the reminder. Pressing `Enter` while it is open SHALL delete the reminder and close the dialog. Pressing `Escape` SHALL dismiss the dialog without deleting the reminder. Deleting a reminder SHALL NOT remove or alter its past activity log entries; those entries SHALL continue to show the reminder's name.

#### Scenario: Opening the delete confirmation
- **WHEN** the user presses `Delete` or `Backspace` on a highlighted reminder
- **THEN** the TUI opens a bold, red confirmation dialog naming that reminder

#### Scenario: Confirming deletion
- **WHEN** the confirmation dialog is open and the user presses `Enter`
- **THEN** the TUI deletes the reminder and closes the dialog

#### Scenario: Dismissing the confirmation
- **WHEN** the confirmation dialog is open and the user presses `Escape`
- **THEN** the TUI closes the dialog and the reminder is not deleted

#### Scenario: A deleted reminder's log entries keep its name
- **WHEN** the user views the Logs tab or the details modal's Logs pane after a reminder has been deleted
- **THEN** that reminder's past log entries still display its name
