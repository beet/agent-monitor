## MODIFIED Requirements

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
