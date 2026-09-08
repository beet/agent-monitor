## MODIFIED Requirements

### Requirement: Live agent list
The TUI SHALL display each tracked agent's working directory/project, host context (nvim, standalone terminal, or desktop app), process id, current status, and last-updated time in the system's local timezone formatted `%Y-%m-%d %H:%M:%S`, updating the display as the daemon reports changes. Agents SHALL be displayed sorted by last-updated time, most recent first. An agent whose status is "running" SHALL additionally display the elapsed duration since it entered "running" (per the daemon's status-since timestamp), formatted as a compact counter (e.g. `2m14s`) and kept current by the TUI's own periodic redraw rather than only refreshing when the daemon pushes an update. An agent not in "running" status SHALL NOT display a duration.

#### Scenario: A new agent starts
- **WHEN** the daemon reports a newly tracked agent
- **THEN** the TUI adds a row for it without requiring the user to restart the TUI

#### Scenario: An agent's status changes
- **WHEN** the daemon pushes a status update for an agent already shown
- **THEN** the TUI updates that row's status in place

#### Scenario: An agent goes stale
- **WHEN** the daemon marks a tracked agent as stale/disconnected
- **THEN** the TUI reflects that the agent is no longer active rather than showing its last active status unchanged

#### Scenario: Agents are sorted by recency
- **WHEN** the TUI displays two or more tracked agents with different last-updated times
- **THEN** the agent with the most recent last-updated time is shown as the top row, and the rest follow in descending order of last-updated time

#### Scenario: An update reorders the list
- **WHEN** an agent that is not currently the top row receives a status update
- **THEN** that agent's row moves to the top of the table, ahead of agents that have not updated as recently

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
