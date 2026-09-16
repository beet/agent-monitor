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
The TUI SHALL display tracked agents and test runs grouped by working directory as one row per project: each distinct working directory forms exactly one row showing that project's name (derived from the working directory), its tracked agent(s)' status(es) in an Agents cell, that project's test run status (if any) in a Tests cell, and the most recent last-updated time among those members (in the system's local timezone, formatted `%Y-%m-%d %H:%M:%S`), updating as the daemon reports changes. A project row SHALL NOT display per-agent host context or process id as separate columns; the daemon continues tracking that data, it is simply not rendered in the collapsed row. Project rows SHALL be ordered by the most recent last-updated time of any member (agent or test run) within them, most recent first. An agent whose status is "running" SHALL contribute a live, counting-up duration to the row, computed from the current time minus that agent's status-since timestamp, formatted as a compact counter (e.g. `2m14s`) and kept current by the TUI's own periodic redraw rather than only refreshing when the daemon pushes an update. An agent whose status is "done" SHALL contribute a fixed duration to the row, computed from that agent's run-started timestamp (the beginning of the run that just completed) to its status-since timestamp (the time it completed) - unlike the "running" duration, this fixed duration does not change on further redraws. An agent in any other status (idle, needs input, stale, or declined) SHALL NOT contribute a duration. A test run whose status is "running" SHALL likewise contribute a live, counting-up duration computed from its run-start timestamp. Once a test run reaches "passed" or "failed", its row SHALL continue to show a duration - the total elapsed time from its run-start timestamp to its last-updated timestamp - rather than showing no duration. When the daemon pushes a removal for a session id, the TUI SHALL drop that session id from its tracked agents, so a pid whose session id has been superseded contributes at most one entry - the current, live session's - to its project row's Agents cell and to that project's Agents pane in the details modal, never a second, frozen entry left over from the superseded session.

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

### Requirement: Project name column scales with terminal width
The TUI's PROJECT column SHALL NOT be capped at a fixed 20-character width. Its width SHALL scale with the terminal's available width, growing on wider terminals instead of always truncating project names at the same fixed length. The Agents and Tests columns SHALL continue to receive the majority of any additional space beyond what PROJECT and UPDATED need, since status segments are typically the widest content in a row. The UPDATED column's width is unaffected by this requirement.

#### Scenario: A wide terminal shows more of a long project name
- **WHEN** the TUI renders in a terminal wide enough to fit a project name longer than 20 characters alongside the Agents, Tests, and UPDATED columns
- **THEN** the PROJECT column renders more than 20 characters of that name, rather than clipping it at 20

#### Scenario: A narrow terminal still clips names that don't fit
- **WHEN** the TUI renders in a terminal too narrow to fit a project name in full alongside the other columns
- **THEN** the PROJECT column clips the name to the space available, consistent with existing table-rendering behavior

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) and test-run statuses (running, passed, failed) from one another so the user can scan the list and immediately identify agents or test runs needing attention. Each agent status SHALL be prefixed with a distinct emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🔔, done with ✅, stale with 👻, and declined with 🚫. Each test-run status SHALL be prefixed with a distinct emoji marker: running with ⏳, passed with ✅, and failed with ❌. When a project's Agents cell reflects more than one tracked agent in different statuses, the cell SHALL show each distinct status present rather than collapsing them into a single "winning" status, so no status needing attention is hidden behind another. On the currently-selected row in the Agents tab or the Logs tab, the TUI SHALL override every status's own foreground color with a single fixed foreground color chosen to stay legible against the row-highlight background, rather than requiring the row-highlight background to avoid every status's own color.

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
- **WHEN** any row in the Agents tab or the Logs tab is the currently-selected (highlighted) row, regardless of which status or statuses it shows
- **THEN** all of that row's status text renders in the same fixed selected-row foreground color, rather than in each status's own color

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
The TUI SHALL organize its display into two tabs: **Agents** (the existing project table) and **Logs** (an aggregated activity log view). The user SHALL be able to cycle between tabs with the `Tab` key, and jump directly to a tab with `A` (Agents) or `L` (Logs).

#### Scenario: Cycling tabs
- **WHEN** the user presses `Tab`
- **THEN** the TUI switches to the other tab

#### Scenario: Jumping directly to a tab
- **WHEN** the user presses `A` or `L`
- **THEN** the TUI switches to the Agents or Logs tab respectively, even if that tab is already active

### Requirement: Agents tab supports row selection
The Agents tab SHALL support moving a selection cursor over its project rows using the `j`/`down` (next row) and `k`/`up` (previous row) keys, so a specific project can be chosen for its details view. The selection SHALL be visually distinguished from unselected rows.

#### Scenario: Moving the selection down
- **WHEN** the user presses `j` or the down arrow while the Agents tab is active
- **THEN** the selection cursor moves to the next project row, if one exists

#### Scenario: Moving the selection up
- **WHEN** the user presses `k` or the up arrow while the Agents tab is active
- **THEN** the selection cursor moves to the previous project row, if one exists

### Requirement: Project details modal
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in either the Agents tab (a project row) or the Logs tab (an activity log entry, using that entry's project) - the same modal, reached from either tab. The modal SHALL show three panes: an Agents pane listing every agent registered for that project with its status and process id, a Tests pane showing that project's last test run if any, and a Logs pane showing that project's activity log entries, most recent first, with each agent-category entry additionally showing the reporting agent's process id. The Agents and Tests panes SHALL be arranged side by side occupying the top third of the modal, and the Logs pane SHALL occupy the remaining bottom two-thirds. Agent and test-run statuses in the Agents and Tests panes, and log entry statuses in the Logs pane, SHALL use the same emoji markers, color styling, and duration as their counterparts in the Agents tab and Logs tab respectively, so status is visually consistent wherever it appears; the Agents and Tests panes SHALL omit the category-word prefix used in the Agents and Logs tabs, while the Logs pane SHALL keep it, consistent with the top-level Logs tab it mirrors. The user SHALL be able to close the modal with `Esc`, returning to whichever tab was active without altering the underlying agent or test-run state.

#### Scenario: Opening a project's details from the Agents tab
- **WHEN** the user presses `Enter` on a selected project row in the Agents tab
- **THEN** the TUI displays a modal overlay with that project's Agents, Tests, and Logs panes

#### Scenario: Opening a project's details from the Logs tab
- **WHEN** the user presses `Enter` on a selected activity log entry in the Logs tab
- **THEN** the TUI displays the same modal overlay, scoped to that entry's project

#### Scenario: Agents and Tests panes share the top third
- **WHEN** the details modal is open
- **THEN** the Agents pane and Tests pane are rendered side by side in the top third of the modal, and the Logs pane occupies the bottom two-thirds beneath them

#### Scenario: Statuses are visually consistent with their tab
- **WHEN** the details modal renders an agent's status, a test run's status, or a log entry's status
- **THEN** that status uses the same emoji marker, color, and duration as it would in the Agents tab or Logs tab

#### Scenario: The Agents pane shows each agent's process id
- **WHEN** the details modal renders its Agents pane
- **THEN** each listed agent's row includes that agent's process id alongside its status

#### Scenario: The Logs pane shows the process id for agent activities
- **WHEN** the details modal's Logs pane renders a log entry whose category is agent
- **THEN** that entry's row includes the reporting agent's process id

#### Scenario: A project with no test run
- **WHEN** the details modal opens for a project with no tracked test run
- **THEN** the Tests pane indicates there is no test run rather than showing an error

#### Scenario: A project with no logged activity
- **WHEN** the details modal opens for a project with no activity log entries
- **THEN** the Logs pane indicates there is no activity rather than showing an error

#### Scenario: Closing the modal
- **WHEN** the user presses `Esc` while the details modal is open
- **THEN** the TUI closes the modal and returns to whichever tab was active when it opened, and the daemon's tracked agents and test runs are unaffected

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects, as a paginated list ordered by recency (most recent first) by default. Each entry SHALL show at least its event time, project, event category, and status. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. A test-run entry whose status is "passed" or "failed", or an agent entry whose status is "done", SHALL additionally show the elapsed duration of that run or task, computed from the most recent preceding "started" entry of the same category logged for the same project.

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

### Requirement: Paginated lists support keyboard navigation
Any paginated list in the TUI (the Logs tab's activity list) SHALL support `j`/`down` and `k`/`up` to move the selection one line at a time, and `d`/page-down and `u`/page-up to move by a full page.

#### Scenario: Moving one line at a time
- **WHEN** the user presses `j`, `down`, `k`, or `up` on a paginated list
- **THEN** the selection moves by exactly one line in the corresponding direction, if a line is available in that direction

#### Scenario: Paging
- **WHEN** the user presses `d`, page-down, `u`, or page-up on a paginated list
- **THEN** the list scrolls by a full page in the corresponding direction, if a page is available in that direction

### Requirement: Keyboard shortcuts help modal
The TUI SHALL open a help modal listing all available keyboard shortcuts when the user presses `?`. The user SHALL be able to close it with `Esc`.

#### Scenario: Opening help
- **WHEN** the user presses `?`
- **THEN** the TUI displays a modal overlay listing the available keyboard shortcuts

#### Scenario: Closing help
- **WHEN** the user presses `Esc` while the help modal is open
- **THEN** the TUI closes the help modal and returns to the previously active tab

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

### Requirement: Status labels are prefixed by category outside dedicated panes
In the Agents tab and the Logs tab, each status label SHALL be prefixed with a category word - "agent" for an agent status or "tests" for a test-run status - immediately after its emoji marker and before the status word (e.g. "✅ agent done", "⏳ tests started"), so a label read in a cross-category view is unambiguous about which kind of status it names. In the details modal, the Agents pane and Tests pane SHALL NOT include this category-word prefix, since the pane itself already establishes the category (e.g. "✅ done", "❌ failed"); the modal's Logs pane SHALL include the category-word prefix, consistent with the top-level Logs tab it mirrors.

#### Scenario: The Agents tab prefixes an agent status with its category word
- **WHEN** the Agents tab renders an agent's status in its Agents column
- **THEN** the label reads the emoji, then "agent", then the status word (e.g. "✅ agent done"), rather than omitting the category word

#### Scenario: The Agents tab prefixes a test-run status with its category word
- **WHEN** the Agents tab renders a project's test-run status in its Tests column
- **THEN** the label reads the emoji, then "tests", then the status word (e.g. "⏳ tests running"), rather than omitting the category word

#### Scenario: The Logs tab prefixes entries with their category word
- **WHEN** the Logs tab renders an entry
- **THEN** the label includes the category word ("agent" or "tests") alongside its emoji and status, consistent with the Agents tab

#### Scenario: The details modal's Agents and Tests panes omit the category-word prefix
- **WHEN** the details modal renders a status in its Agents pane or Tests pane
- **THEN** the label omits the category word (e.g. "✅ done", not "✅ agent done")

#### Scenario: The details modal's Logs pane keeps the category-word prefix
- **WHEN** the details modal renders an entry in its Logs pane
- **THEN** the label includes the category word, consistent with the top-level Logs tab
