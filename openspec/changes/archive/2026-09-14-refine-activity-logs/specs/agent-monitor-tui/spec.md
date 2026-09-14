## MODIFIED Requirements

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) and test-run statuses (started, passed, failed) from one another so the user can scan the list and immediately identify agents or test runs needing attention. Each agent status SHALL be prefixed with a distinct emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🔔, done with ✅, stale with 🕸️, and declined with 🚫. Each test-run status SHALL be prefixed with a distinct emoji marker: started with ⏳, passed with ✅, and failed with ❌. When a project's row combines more than one status - two or more tracked agents in different statuses, and/or an agent alongside a test run - the row SHALL show each distinct status present rather than collapsing them into a single "winning" status, so no status needing attention is hidden behind another. No status's text color SHALL match the Agents tab's row-highlight background color, since a status rendered in that color would become invisible - emoji included - on whichever row is currently selected (including the initially-selected row shown as soon as any project is tracked, before the user has moved the selection).

#### Scenario: An agent needs input
- **WHEN** an agent's status is "needs input"
- **THEN** that status's contribution to the row displays the 🔔 marker and is visually distinguished (e.g. color) from other statuses shown, using bold colored text rather than a solid background fill

#### Scenario: Each status has a distinct emoji marker
- **WHEN** the TUI renders an agent's status within a project row
- **THEN** that status is prefixed with the emoji for that status (🔧 running, 💤 idle, 🔔 needs input, ✅ done, 🕸️ stale, 🚫 declined)

#### Scenario: A declined permission is visually distinguished
- **WHEN** an agent's status is "declined"
- **THEN** that status's contribution to the row displays the 🚫 marker and is visually distinguished (e.g. color) from other statuses shown

#### Scenario: A failing test run is visually distinguished
- **WHEN** a test run's status is "failed"
- **THEN** that status's contribution to the row displays the ❌ marker and is visually distinguished (e.g. color) from other statuses shown

#### Scenario: Each test-run status has a distinct emoji marker
- **WHEN** the TUI renders a test run's status within a project row
- **THEN** that status is prefixed with the emoji for that status (⏳ started, ✅ passed, ❌ failed)

#### Scenario: A project with multiple agent statuses shows each one
- **WHEN** a project has two or more tracked agents whose statuses differ from one another
- **THEN** the row's status cell lists each distinct status present among those agents (for example, both running and needs input), rather than showing only one

#### Scenario: An agent status and a test-run status are shown together
- **WHEN** a project has both a tracked agent and a test run whose statuses differ
- **THEN** the row's status cell shows both the agent's status and the test run's status together, rather than picking one over the other

#### Scenario: A running status stays legible on the selected row
- **WHEN** a project row showing the "running" status is the currently-selected (highlighted) row in the Agents tab
- **THEN** the status's label and emoji remain visible, styled in a color distinct from the row-highlight background

## ADDED Requirements

### Requirement: Empty-state placeholders appear in the table body, not the pane title
When the Agents tab has no tracked agents or test runs, and when the Logs tab's full (unfiltered) activity log is empty, the TUI SHALL show that empty state as gray placeholder text in the table body beneath the header, matching the existing convention used when a Logs tab filter matches nothing. The pane title SHALL NOT carry this placeholder text; the Logs tab's title SHALL continue to show its sort/filter control hints regardless of whether the log is empty.

#### Scenario: No agents tracked yet
- **WHEN** the daemon reports no tracked agents and no test runs
- **THEN** the Agents tab's title reads "Agents" and its table body shows gray placeholder text indicating no agents are tracked yet

#### Scenario: No activity logged yet
- **WHEN** the daemon's activity log is empty
- **THEN** the Logs tab's title shows only its sort/filter control hints (no empty-state text) and its table body shows gray placeholder text indicating no activity has been logged yet
