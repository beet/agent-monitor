## MODIFIED Requirements

### Requirement: Live agent list
The TUI SHALL display tracked agents and test runs grouped by working directory: each directory forms a section showing that directory's tracked agent(s) (working directory/project, host context, process id, current status, last-updated time in the system's local timezone formatted `%Y-%m-%d %H:%M:%S`) together with any test run(s) in that same directory (status and last-updated time), updating as the daemon reports changes. Directory groups SHALL be ordered by the most recent last-updated time of any member (agent or test run) within them, most recent first. Within a group, agent rows SHALL be shown before test-run rows, and agent rows SHALL be ordered by their own last-updated time, most recent first. An agent whose status is "running" SHALL additionally display the elapsed duration since it entered "running" (per the daemon's status-since timestamp), formatted as a compact counter (e.g. `2m14s`) and kept current by the TUI's own periodic redraw rather than only refreshing when the daemon pushes an update. An agent not in "running" status SHALL NOT display a duration.

#### Scenario: A new agent starts
- **WHEN** the daemon reports a newly tracked agent
- **THEN** the TUI adds a row for it, in its working directory's group, without requiring the user to restart the TUI

#### Scenario: An agent's status changes
- **WHEN** the daemon pushes a status update for an agent already shown
- **THEN** the TUI updates that row's status in place

#### Scenario: An agent goes stale
- **WHEN** the daemon marks a tracked agent as stale/disconnected
- **THEN** the TUI reflects that the agent is no longer active rather than showing its last active status unchanged

#### Scenario: Agents are sorted by recency
- **WHEN** the TUI displays two or more directory groups with different most-recent last-updated times among their members
- **THEN** the group with the most recent last-updated time is shown first, and the rest follow in descending order

#### Scenario: An update reorders the list
- **WHEN** a member of a group that is not currently shown first receives a status update
- **THEN** that member's group moves to the top, ahead of groups that have not updated as recently

#### Scenario: Last-updated time is shown in local time
- **WHEN** the TUI renders an agent's row
- **THEN** the row includes that agent's last-updated time converted to the system's local timezone and formatted as `%Y-%m-%d %H:%M:%S` (e.g. `2026-09-01 16:32:07`), without a UTC offset or timezone code

#### Scenario: A running agent shows its elapsed duration
- **WHEN** the TUI renders a row for an agent whose status is "running"
- **THEN** the row includes a duration computed from the current time minus that agent's status-since timestamp, formatted compactly (e.g. `9s`, `2m14s`, `1h03m`)

#### Scenario: A non-running agent shows no duration
- **WHEN** the TUI renders a row for an agent whose status is not "running" (idle, needs input, done, or stale)
- **THEN** the row does not include a running duration

#### Scenario: The running duration counts up without a new daemon event
- **WHEN** an agent remains "running" and no new event arrives from the daemon for several seconds
- **THEN** the TUI's displayed duration for that agent still increases over that time, driven by the TUI's own periodic redraw

#### Scenario: A status transition resets the displayed duration
- **WHEN** an agent transitions out of and back into "running" status (for example, "running" to "needs input" and back to "running")
- **THEN** the TUI displays a duration counted from the new status-since timestamp, not accumulated from the earlier running period

#### Scenario: A directory with no tracked agent still shows its test run
- **WHEN** a test run is tracked in a working directory with no tracked agent
- **THEN** the TUI displays a group for that directory containing only the test run, with no agent rows

#### Scenario: A test run appears in an agent's group
- **WHEN** a test-run event is reported for a working directory with a tracked agent
- **THEN** the TUI adds or updates that test run's row within the same directory group as the agent, without creating a duplicate group

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) and test-run statuses (started, passed, failed) from one another so the user can scan the list and immediately identify agents or test runs needing attention. Each agent status SHALL be prefixed with a distinct emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🔔, done with ✅, stale with 🕸️, and declined with 🚫. Each test-run status SHALL be prefixed with a distinct emoji marker: started with ⏳, passed with ✅, and failed with ❌.

#### Scenario: An agent needs input
- **WHEN** an agent's status is "needs input"
- **THEN** that row displays the 🔔 marker and is visually distinguished (e.g. color) from rows in other states, using bold colored text rather than a solid background fill

#### Scenario: Each status has a distinct emoji marker
- **WHEN** the TUI renders a row for an agent
- **THEN** the status cell is prefixed with the emoji for that status (🔧 running, 💤 idle, 🔔 needs input, ✅ done, 🕸️ stale, 🚫 declined)

#### Scenario: A declined permission is visually distinguished
- **WHEN** an agent's status is "declined"
- **THEN** that row displays the 🚫 marker and is visually distinguished (e.g. color) from rows in other states

#### Scenario: A failing test run is visually distinguished
- **WHEN** a test run's status is "failed"
- **THEN** that row displays the ❌ marker and is visually distinguished (e.g. color) from rows in other states

#### Scenario: Each test-run status has a distinct emoji marker
- **WHEN** the TUI renders a row for a test run
- **THEN** the status cell is prefixed with the emoji for that status (⏳ started, ✅ passed, ❌ failed)
