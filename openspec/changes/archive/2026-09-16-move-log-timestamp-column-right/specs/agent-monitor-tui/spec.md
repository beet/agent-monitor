## MODIFIED Requirements

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects, as a paginated list ordered by recency (most recent first) by default. Each entry SHALL show at least its project, event category, status, and event time, with event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. A test-run entry whose status is "passed" or "failed", or an agent entry whose status is "done", SHALL additionally show the elapsed duration of that run or task, computed from the most recent preceding "started" entry of the same category logged for the same project.

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

### Requirement: Project details modal
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in either the Agents tab (a project row) or the Logs tab (an activity log entry, using that entry's project) - the same modal, reached from either tab. The modal SHALL show three panes: an Agents pane listing every agent registered for that project with its status and process id, a Tests pane showing that project's last test run if any, and a Logs pane showing that project's activity log entries, most recent first, with each entry's event time trailing its category/status text rather than leading it, and each agent-category entry additionally showing the reporting agent's process id. The Logs pane SHALL NOT display column headers. The Agents and Tests panes SHALL be arranged side by side occupying the top third of the modal, and the Logs pane SHALL occupy the remaining bottom two-thirds. Agent and test-run statuses in the Agents and Tests panes, and log entry statuses in the Logs pane, SHALL use the same emoji markers, color styling, and duration as their counterparts in the Agents tab and Logs tab respectively, so status is visually consistent wherever it appears; the Agents and Tests panes SHALL omit the category-word prefix used in the Agents and Logs tabs, while the Logs pane SHALL keep it, consistent with the top-level Logs tab it mirrors. The user SHALL be able to close the modal with `Esc`, returning to whichever tab was active without altering the underlying agent or test-run state.

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

#### Scenario: The Logs pane's timestamp trails each entry's text
- **WHEN** the details modal's Logs pane renders a log entry
- **THEN** that entry's event time appears after its category/status text rather than before it, and no column headers are shown above the entries
