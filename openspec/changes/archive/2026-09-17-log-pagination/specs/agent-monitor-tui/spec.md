## MODIFIED Requirements

### Requirement: Logs tab shows an aggregated, paginated activity list
The Logs tab SHALL display the activity log entries received from the daemon, aggregated across all projects, as a paginated list ordered by recency (most recent first) by default. Each entry SHALL show at least its project, event category, status, and event time, with event time as the last (rightmost) column, consistent with the Agents tab's UPDATED column. Each entry's status SHALL use the same emoji marker and color styling as that status uses in the Agents tab, so status is visually distinguishable the same way it already is there. A test-run entry whose status is "passed" or "failed", or an agent entry whose status is "done", SHALL additionally show the elapsed duration of that run or task, computed from the most recent preceding "started" entry of the same category logged for the same project. Whenever the list holds more entries than fit on a single page, the Logs tab SHALL display a Ratatui vertical `Scrollbar` widget along its right edge, and its heading SHALL append a pagination keyboard-shortcut hint alongside its existing sort and filter hints; neither the scrollbar nor the pagination hint SHALL be shown when every (filtered) entry already fits on a single page.

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

### Requirement: Project details modal
The TUI SHALL open a details modal overlay for a project when the user presses `Enter` on a selected row in either the Agents tab (a project row) or the Logs tab (an activity log entry, using that entry's project) - the same modal, reached from either tab. The modal SHALL show three panes: an Agents pane listing every agent registered for that project with its status and process id, a Tests pane showing that project's last test run if any, and a Logs pane showing that project's activity log entries, most recent first, with each entry's event time rendered in a fixed-width column at the far right of the pane - aligned at the same horizontal position on every row regardless of that row's category/status text length, consistent with how the Agents tab's UPDATED column stays fixed regardless of its other cells' content - and each agent-category entry additionally showing the reporting agent's process id. In the Logs pane, each test-run-category entry SHALL be prefixed with a tree branch marker (`├─ `) immediately before its status emoji, so it reads as nested beneath the agent activity it ran under, while agent-category entries SHALL render with no such prefix, forming the unindented trunk of the list; this prefix is purely visual and SHALL NOT change entry ordering, the existing category-word prefix, or the fixed-width right-aligned timestamp column. The Logs pane SHALL NOT display column headers. The Agents and Tests panes SHALL be arranged side by side occupying the top third of the modal, and the Logs pane SHALL occupy the remaining bottom two-thirds. Agent and test-run statuses in the Agents and Tests panes, and log entry statuses in the Logs pane, SHALL use the same emoji markers, color styling, and duration as their counterparts in the Agents tab and Logs tab respectively, so status is visually consistent wherever it appears; the Agents and Tests panes SHALL omit the category-word prefix used in the Agents and Logs tabs, while the Logs pane SHALL keep it, consistent with the top-level Logs tab it mirrors. The user SHALL be able to close the modal with `Esc`, returning to whichever tab was active without altering the underlying agent or test-run state. When the Logs pane holds more entries than fit on a single page, it SHALL behave as a paginated list per the "Paginated lists support keyboard navigation" requirement: it SHALL display a highlighted selected row using the same row-highlight style as the Logs tab, a Ratatui vertical `Scrollbar` widget along its right edge, and a pagination keyboard-shortcut hint appended to its "Logs" pane title - the only hint shown there, since the modal's Logs pane has no sort or filter controls of its own. Neither the selection highlight, the scrollbar, nor the pagination hint SHALL be shown when the Logs pane's entries all fit on a single page.

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
- **WHEN** the details modal's Logs pane renders two or more entries whose category/status text differs in length
- **THEN** every entry's event time starts at the same fixed horizontal column position, at the far right of the pane, rather than immediately trailing each entry's own text at a position that varies with that text's length, and no column headers are shown above the entries

#### Scenario: Test-run entries render nested under the agent trunk
- **WHEN** the details modal's Logs pane renders a log entry whose category is test-run
- **THEN** that entry's row is prefixed with `├─ ` before its status emoji, and its event time still starts at the pane's fixed right-hand column

#### Scenario: Agent entries render unindented
- **WHEN** the details modal's Logs pane renders a log entry whose category is agent
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

### Requirement: Paginated lists support keyboard navigation
Any paginated list in the TUI (the Logs tab's activity list and the details modal's Logs pane) SHALL support `j`/`down` and `k`/`up` to move the selection one line at a time, and `d`/page-down and `u`/page-up to move by a full page. A page is however many entries currently fit in the list's rendered area at the current terminal size. Paging SHALL overlap the previous page by exactly 1 row: paging down SHALL advance the page's top row by (page size - 1) rows, and paging up SHALL move the page's top row back by (page size - 1) rows, in both cases selecting the new page's top row, clamped so the page never scrolls past the list's first or last entry. While the details modal is open, its Logs pane is itself a paginated list whose `j`/`k`/`d`/`u`/page-down/page-up keys apply only to that pane; the Logs tab underneath SHALL NOT receive or react to those keys while the modal is open, consistent with the details modal taking precedence for keyboard shortcuts over whatever page is open beneath it.

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
- **WHEN** the details modal is open and the user presses `j`, `k`, `d`, `u`, page-down, or page-up
- **THEN** the details modal's Logs pane selection and page update accordingly, and the Logs tab's own selection and page underneath remain unchanged
