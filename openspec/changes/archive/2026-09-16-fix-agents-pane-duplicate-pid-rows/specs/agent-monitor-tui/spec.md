## MODIFIED Requirements

### Requirement: Live agent list
The TUI SHALL display tracked agents and test runs grouped by working directory as one row per project: each distinct working directory forms exactly one row showing that project's name (derived from the working directory), its tracked agent(s)' status(es) in an Agents cell, that project's test run status (if any) in a Tests cell, and the most recent last-updated time among those members (in the system's local timezone, formatted `%Y-%m-%d %H:%M:%S`), updating as the daemon reports changes. A project row SHALL NOT display per-agent host context or process id as separate columns; the daemon continues tracking that data, it is simply not rendered in the collapsed row. Project rows SHALL be ordered by the most recent last-updated time of any member (agent or test run) within them, most recent first. An agent whose status is "running" SHALL contribute a live, counting-up duration to the row, computed from the current time minus that agent's status-since timestamp, formatted as a compact counter (e.g. `2m14s`) and kept current by the TUI's own periodic redraw rather than only refreshing when the daemon pushes an update. An agent whose status is "done" SHALL contribute a fixed duration to the row, computed from that agent's run-started timestamp (the beginning of the run that just completed) to its status-since timestamp (the time it completed) - unlike the "running" duration, this fixed duration does not change on further redraws. An agent in any other status (idle, needs input, stale, or declined) SHALL NOT contribute a duration. A test run whose status is "started" SHALL likewise contribute a live, counting-up duration computed from its run-start timestamp. Once a test run reaches "passed" or "failed", its row SHALL continue to show a duration - the total elapsed time from its run-start timestamp to its last-updated timestamp - rather than showing no duration. When the daemon pushes a removal for a session id, the TUI SHALL drop that session id from its tracked agents, so a pid whose session id has been superseded contributes at most one entry - the current, live session's - to its project row's Agents cell and to that project's Agents pane in the details modal, never a second, frozen entry left over from the superseded session.

#### Scenario: A new agent starts
- **WHEN** the daemon reports a newly tracked agent
- **THEN** the TUI adds or updates that agent's project row - creating the row if this is the first member tracked for that working directory - without requiring the user to restart the TUI

#### Scenario: An agent's status changes
- **WHEN** the daemon pushes a status update for an agent already shown
- **THEN** the TUI updates that agent's contribution to its project row's Agents cell in place

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

#### Scenario: A done agent shows its total duration
- **WHEN** the TUI renders a project row for an agent whose status is "done"
- **THEN** the row includes a fixed duration computed from that agent's run-started timestamp to its status-since timestamp, rather than showing no duration

#### Scenario: A non-running agent shows no duration
- **WHEN** the TUI renders a project row for an agent whose status is not "running" and not "done" (idle, needs input, stale, or declined)
- **THEN** the row does not include a duration for that agent

#### Scenario: The running duration counts up without a new daemon event
- **WHEN** an agent remains "running" and no new event arrives from the daemon for several seconds
- **THEN** the TUI's displayed duration for that agent still increases over that time, driven by the TUI's own periodic redraw

#### Scenario: A status transition resets the displayed duration
- **WHEN** an agent transitions out of and back into "running" status (for example, "running" to "needs input" and back to "running")
- **THEN** the TUI displays a duration counted from the new status-since timestamp, not accumulated from the earlier running period

#### Scenario: A directory with no tracked agent still shows its test run
- **WHEN** a test run is tracked in a working directory with no tracked agent
- **THEN** the TUI displays a row for that project containing only the test run's status and duration in its Tests cell, with no agent status contributed

#### Scenario: A test run appears in an agent's group
- **WHEN** a test-run event is reported for a working directory with tracked agent(s)
- **THEN** the TUI updates that project's single row to include the test run's status and duration in its Tests cell alongside its agent(s)' statuses in its Agents cell, without creating a duplicate row

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

#### Scenario: A removed session id no longer renders as a duplicate row
- **WHEN** the daemon pushes a removal for a session id whose pid is already tracked under a different, newer session id
- **THEN** the TUI drops the removed session id from its tracked agents, so that pid's project row and the details modal's Agents pane each show only the live session's entry, not a second one with a different duration
