## Purpose

Gives the user a single, live-updating terminal view of every Claude Code agent session tracked by the daemon, across nvim, standalone terminals, and the desktop app.

## ADDED Requirements

### Requirement: Connect to the daemon
The TUI SHALL connect to the daemon's local socket on startup and clearly inform the user when the daemon is unreachable.

#### Scenario: Daemon is running
- **WHEN** the TUI starts and the daemon's socket is reachable
- **THEN** the TUI connects and begins displaying tracked agents

#### Scenario: Daemon is not running
- **WHEN** the TUI starts and cannot reach the daemon's socket
- **THEN** the TUI displays a clear message that the daemon is not running instead of showing a blank or misleading agent list

### Requirement: Live agent list
The TUI SHALL display each tracked agent's working directory/project, host context (nvim, standalone terminal, or desktop app), process id, and current status, updating the display as the daemon reports changes.

#### Scenario: A new agent starts
- **WHEN** the daemon reports a newly tracked agent
- **THEN** the TUI adds a row for it without requiring the user to restart the TUI

#### Scenario: An agent's status changes
- **WHEN** the daemon pushes a status update for an agent already shown
- **THEN** the TUI updates that row's status in place

#### Scenario: An agent goes stale
- **WHEN** the daemon marks a tracked agent as stale/disconnected
- **THEN** the TUI reflects that the agent is no longer active rather than showing its last active status unchanged

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale) from one another so the user can scan the list and immediately identify agents needing attention.

#### Scenario: An agent needs input
- **WHEN** an agent's status is "needs input"
- **THEN** that row is visually distinguished (e.g. color or marker) from rows in other states

### Requirement: Navigation and quit do not affect tracked agents
The TUI SHALL support selecting a row and quitting the application via a keybinding, and quitting the TUI SHALL NOT stop the daemon or any tracked Claude Code agent.

#### Scenario: User quits the TUI
- **WHEN** the user presses the quit key
- **THEN** the TUI process exits while the daemon keeps running and continues tracking agents

### Requirement: Reconnect after daemon restart
The TUI SHALL detect when its connection to the daemon drops and attempt to reconnect, resuming display of current agent state once reconnected.

#### Scenario: Daemon restarts while the TUI is open
- **WHEN** the daemon process restarts (e.g. after an update) while the TUI is running
- **THEN** the TUI detects the dropped connection, retries connecting, and repopulates the agent list once the daemon is back
