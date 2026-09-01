## MODIFIED Requirements

### Requirement: Agent registry
The daemon SHALL maintain a registry of known agents keyed by session identifier, recording working directory, host context (nvim, standalone terminal, or desktop app), process id, current status, and last-updated timestamp. The registry SHALL reject a "needs input" event for a session whose current status is already "done", since a finished session cannot legitimately need input again until a new "running" event is reported for it.

#### Scenario: First event for a session registers a new agent
- **WHEN** the daemon receives an event for a session id it has not seen before
- **THEN** it creates a new registry entry with the reported working directory, host context, pid, and status

#### Scenario: Subsequent event updates the existing agent
- **WHEN** the daemon receives an event for a session id already in the registry
- **THEN** it updates that entry's status and last-updated timestamp instead of creating a duplicate

#### Scenario: A late needs-input event cannot override a completed session
- **WHEN** the daemon receives a "needs input" event for a session whose current status is already "done"
- **THEN** the daemon leaves that agent's status as "done" and does not apply the "needs input" event

#### Scenario: A new running event still clears a completed session's status
- **WHEN** the daemon receives a "running" event for a session whose current status is "done"
- **THEN** it updates that entry's status to "running" as normal

### Requirement: Completion notifications
The daemon SHALL send a macOS user notification when a tracked agent's status transitions to "done" or "needs input", using a status-specific system sound so the two cases are distinguishable by ear. Every hook-reported "needs input" event SHALL produce a notification, even if the agent's status was already "needs input", because each such event represents a distinct blocking prompt; a "done" event SHALL NOT produce an additional notification when the agent's status is already "done".

#### Scenario: Agent completes its task
- **WHEN** a tracked agent's status transitions from "running" to "done"
- **THEN** the daemon sends a macOS notification identifying the agent (working directory / project and host context), played with the built-in `Glass` system sound

#### Scenario: Agent needs input
- **WHEN** a tracked agent's status transitions to "needs input"
- **THEN** the daemon sends a macOS notification identifying the agent, played with the built-in `Ping` system sound

#### Scenario: A second needs-input prompt in the same turn still notifies
- **WHEN** the daemon receives a "needs input" event for an agent that is already in the "needs input" status
- **THEN** the daemon sends another macOS notification, since it represents a new blocking prompt rather than a repeat of the same one

#### Scenario: No duplicate notification for an unchanged status
- **WHEN** the daemon receives another event that reports "done" for an agent already in the "done" status
- **THEN** the daemon does not send an additional notification for that transition
