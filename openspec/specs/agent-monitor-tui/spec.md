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

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) from one another so the user can scan the list and immediately identify agents needing attention. Each status SHALL be prefixed with a distinct emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🔔, done with ✅, stale with 🕸️, and declined with 🚫.

#### Scenario: An agent needs input
- **WHEN** an agent's status is "needs input"
- **THEN** that row displays the 🔔 marker and is visually distinguished (e.g. color) from rows in other states, using bold colored text rather than a solid background fill

#### Scenario: Each status has a distinct emoji marker
- **WHEN** the TUI renders a row for an agent
- **THEN** the status cell is prefixed with the emoji for that status (🔧 running, 💤 idle, 🔔 needs input, ✅ done, 🕸️ stale, 🚫 declined)

#### Scenario: A declined permission is visually distinguished
- **WHEN** an agent's status is "declined"
- **THEN** that row displays the 🚫 marker and is visually distinguished (e.g. color) from rows in other states

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
