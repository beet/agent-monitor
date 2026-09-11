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
The TUI SHALL display tracked agents and test runs grouped by working directory as one row per project: each distinct working directory forms exactly one row showing that project's name (derived from the working directory), a combined status for its tracked agent(s) and any test run in that directory, and the most recent last-updated time among those members (in the system's local timezone, formatted `%Y-%m-%d %H:%M:%S`), updating as the daemon reports changes. A project row SHALL NOT display per-agent host context or process id as separate columns; the daemon continues tracking that data, it is simply not rendered in the collapsed row. Project rows SHALL be ordered by the most recent last-updated time of any member (agent or test run) within them, most recent first. An agent whose status is "running" SHALL contribute a duration to the row, computed from the current time minus that agent's status-since timestamp, formatted as a compact counter (e.g. `2m14s`) and kept current by the TUI's own periodic redraw rather than only refreshing when the daemon pushes an update. An agent not in "running" status SHALL NOT contribute a duration. A test run whose status is "started" SHALL likewise contribute a live, counting-up duration computed from its run-start timestamp. Once a test run reaches "passed" or "failed", its row SHALL continue to show a duration - the total elapsed time from its run-start timestamp to its last-updated timestamp - rather than showing no duration.

#### Scenario: A new agent starts
- **WHEN** the daemon reports a newly tracked agent
- **THEN** the TUI adds or updates that agent's project row - creating the row if this is the first member tracked for that working directory - without requiring the user to restart the TUI

#### Scenario: An agent's status changes
- **WHEN** the daemon pushes a status update for an agent already shown
- **THEN** the TUI updates that agent's contribution to its project row's combined status in place

#### Scenario: An agent goes stale
- **WHEN** the daemon marks a tracked agent as stale/disconnected
- **THEN** the project row reflects that the agent is no longer active rather than showing its last active status unchanged

#### Scenario: Agents are sorted by recency
- **WHEN** the TUI displays two or more project rows with different most-recent last-updated times among their members
- **THEN** the project with the most recent last-updated time is shown first, and the rest follow in descending order

#### Scenario: An update reorders the list
- **WHEN** a member of a project that is not currently shown first receives a status update
- **THEN** that project's row moves to the top, ahead of projects that have not updated as recently

#### Scenario: Last-updated time is shown in local time
- **WHEN** the TUI renders a project row
- **THEN** the row includes the most recent last-updated time among its members, converted to the system's local timezone and formatted as `%Y-%m-%d %H:%M:%S` (e.g. `2026-09-01 16:32:07`), without a UTC offset or timezone code

#### Scenario: A running agent shows its elapsed duration
- **WHEN** the TUI renders a project row whose agent's status is "running"
- **THEN** the row includes a duration computed from the current time minus that agent's status-since timestamp, formatted compactly (e.g. `9s`, `2m14s`, `1h03m`)

#### Scenario: A non-running agent shows no duration
- **WHEN** the TUI renders a project row for an agent whose status is not "running" (idle, needs input, done, or stale)
- **THEN** the row does not include a running duration for that agent

#### Scenario: The running duration counts up without a new daemon event
- **WHEN** an agent remains "running" and no new event arrives from the daemon for several seconds
- **THEN** the TUI's displayed duration for that agent still increases over that time, driven by the TUI's own periodic redraw

#### Scenario: A status transition resets the displayed duration
- **WHEN** an agent transitions out of and back into "running" status (for example, "running" to "needs input" and back to "running")
- **THEN** the TUI displays a duration counted from the new status-since timestamp, not accumulated from the earlier running period

#### Scenario: A directory with no tracked agent still shows its test run
- **WHEN** a test run is tracked in a working directory with no tracked agent
- **THEN** the TUI displays a row for that project containing only the test run's status and duration, with no agent status contributed

#### Scenario: A test run appears in an agent's group
- **WHEN** a test-run event is reported for a working directory with tracked agent(s)
- **THEN** the TUI updates that project's single row to include the test run's status and duration alongside its agent(s)' statuses, without creating a duplicate row

#### Scenario: A project's row omits host and process id
- **WHEN** the TUI renders a project row
- **THEN** the row does not include separate host-context or process-id columns

#### Scenario: A started test run shows a live elapsed duration
- **WHEN** the TUI renders a project row whose test run status is "started"
- **THEN** the row includes a duration computed from the current time minus that test run's run-start timestamp, formatted compactly like an agent's running duration and kept current by the TUI's own periodic redraw

#### Scenario: A finished test run keeps showing its total duration
- **WHEN** the TUI renders a project row whose test run status is "passed" or "failed"
- **THEN** the row includes a fixed duration computed from that test run's run-start timestamp to its last-updated timestamp, rather than showing no duration

#### Scenario: A new test run resets the displayed duration
- **WHEN** a project's previously finished test run is replaced by a newly reported test run (a different process)
- **THEN** the TUI displays a duration counted from the new run's run-start timestamp, not accumulated from the previous run

### Requirement: Project name column scales with terminal width
The TUI's PROJECT column SHALL NOT be capped at a fixed 20-character width. Its width SHALL scale with the terminal's available width, growing on wider terminals instead of always truncating project names at the same fixed length. The STATUS column SHALL continue to receive the majority of any additional space beyond what PROJECT and UPDATED need, since status segments are typically the widest content in a row. The UPDATED column's width is unaffected by this requirement.

#### Scenario: A wide terminal shows more of a long project name
- **WHEN** the TUI renders in a terminal wide enough to fit a project name longer than 20 characters alongside the STATUS and UPDATED columns
- **THEN** the PROJECT column renders more than 20 characters of that name, rather than clipping it at 20

#### Scenario: A narrow terminal still clips names that don't fit
- **WHEN** the TUI renders in a terminal too narrow to fit a project name in full alongside the STATUS and UPDATED columns
- **THEN** the PROJECT column clips the name to the space available, consistent with existing table-rendering behavior

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) and test-run statuses (started, passed, failed) from one another so the user can scan the list and immediately identify agents or test runs needing attention. Each agent status SHALL be prefixed with a distinct emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🔔, done with ✅, stale with 🕸️, and declined with 🚫. Each test-run status SHALL be prefixed with a distinct emoji marker: started with ⏳, passed with ✅, and failed with ❌. When a project's row combines more than one status - two or more tracked agents in different statuses, and/or an agent alongside a test run - the row SHALL show each distinct status present rather than collapsing them into a single "winning" status, so no status needing attention is hidden behind another.

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
