## ADDED Requirements

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
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in either the Agents tab (a project row) or the Logs tab (an activity log entry, using that entry's project) - the same modal, reached from either tab. The modal SHALL show three panes: an Agents pane listing every agent registered for that project with its status, a Tests pane showing that project's last test run if any, and a Logs pane showing that project's activity log entries, most recent first. The Agents and Tests panes SHALL be arranged side by side occupying the top third of the modal, and the Logs pane SHALL occupy the remaining bottom two-thirds. Agent and test-run statuses in the Agents and Tests panes, and log entry statuses in the Logs pane, SHALL use the same emoji markers and color styling as their counterparts in the Agents tab and Logs tab respectively, so status is visually consistent wherever it appears. The user SHALL be able to close the modal with `Esc`, returning to whichever tab was active without altering the underlying agent or test-run state.

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
- **THEN** that status uses the same emoji marker and color as it would in the Agents tab or Logs tab

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
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects, as a paginated list ordered by recency (most recent first) by default. Each entry SHALL show at least its event time, project, event category, and status. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. A test-run entry whose status is "passed" or "failed" SHALL additionally show the elapsed duration of that run, computed from the most recent preceding "started" entry logged for the same project.

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
- **WHEN** the Logs tab renders a "passed" or "failed" test-run entry that has a preceding "started" entry logged for the same project
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
