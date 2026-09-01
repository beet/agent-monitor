## MODIFIED Requirements

### Requirement: Live agent list
The TUI SHALL display each tracked agent's working directory/project, host context (nvim, standalone terminal, or desktop app), process id, current status, and last-updated time in the system's local timezone formatted `%Y-%m-%d %H:%M:%S`, updating the display as the daemon reports changes. Agents SHALL be displayed sorted by last-updated time, most recent first.

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

### Requirement: Navigation and quit do not affect tracked agents
The TUI SHALL support quitting the application via a keybinding, and quitting the TUI SHALL NOT stop the daemon or any tracked Claude Code agent.

#### Scenario: User quits the TUI
- **WHEN** the user presses the quit key
- **THEN** the TUI process exits while the daemon keeps running and continues tracking agents
