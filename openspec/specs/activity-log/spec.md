# activity-log Specification

## Purpose

Gives the daemon a short-lived, bounded history of notification-worthy agent and test-run events, so a client that reconnects or opens after missing a live notification can still see what recently happened.

## Requirements

### Requirement: Activity log captures notification-worthy events
The daemon SHALL append an entry to its activity log at the same point it sends a macOS notification for an agent status transition to "done" or "needs input", and for a test-run event of "started", "passed", or "failed". Each entry SHALL record the working directory, an event category (agent or test-run), the resulting status, and the time the event occurred. Transitions that do not produce a notification today - "declined" and "stale" - SHALL NOT produce a log entry. A repeated "needs input" event for an agent already in that status SHALL produce a separate log entry, consistent with it producing a separate notification.

#### Scenario: Agent completes its task
- **WHEN** a tracked agent's status transitions to "done" and the daemon sends a completion notification
- **THEN** the daemon appends a log entry recording the agent's working directory, category "agent", status "done", and the event time

#### Scenario: Agent needs input
- **WHEN** a tracked agent's status transitions to "needs input" and the daemon sends a notification
- **THEN** the daemon appends a log entry with category "agent" and status "needs input"

#### Scenario: A second needs-input prompt logs again
- **WHEN** the daemon receives a "needs input" event for an agent already in "needs input" status and sends another notification for it
- **THEN** the daemon appends another log entry rather than skipping it as a duplicate

#### Scenario: Test run starts, passes, or fails
- **WHEN** the daemon sends a test-run lifecycle notification for "started", "passed", or "failed"
- **THEN** the daemon appends a log entry recording the test run's working directory, category "test-run", and that status

#### Scenario: A declined permission is not logged
- **WHEN** a tracked agent's status transitions to "declined"
- **THEN** the daemon does not append a log entry for that transition

#### Scenario: A stale agent is not logged
- **WHEN** the daemon marks a tracked agent as stale
- **THEN** the daemon does not append a log entry for that transition

### Requirement: Bounded global retention
The activity log SHALL retain at most 500 entries in total across all projects. When appending a new entry would exceed that cap, the daemon SHALL evict the single oldest entry in the log by event time, regardless of which project or event category it belongs to, before appending the new one.

#### Scenario: Cap reached with entries from one project
- **WHEN** the log already holds 500 entries and a new notification-worthy event occurs
- **THEN** the daemon evicts the oldest entry and appends the new one, keeping the log at 500 entries

#### Scenario: Eviction is global, not per-project
- **WHEN** the log is at capacity and the oldest entry belongs to a different project than the one generating the new event
- **THEN** the daemon evicts that oldest entry across all projects, not an entry scoped to the new event's own project

### Requirement: Clients receive the current log and live updates
The daemon SHALL include the current activity log, in the order the events occurred, in the snapshot it sends a client on connect. As further notification-worthy events occur, the daemon SHALL push each new log entry to connected clients without requiring them to reconnect or re-request the log.

#### Scenario: Client connects and receives log history
- **WHEN** a client (e.g. the TUI) connects to the daemon
- **THEN** the daemon's snapshot to that client includes the current activity log alongside the existing agent and test-run snapshot

#### Scenario: Client receives a new entry as it happens
- **WHEN** a notification-worthy event occurs while a client is connected
- **THEN** the daemon pushes the new log entry to that client in addition to any agent/test-run update the event also produces
