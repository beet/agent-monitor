## MODIFIED Requirements

### Requirement: Completion notifications
The daemon SHALL send a macOS user notification when a tracked agent's status transitions to "done" or "needs input", using a status-specific system sound so the two cases are distinguishable by ear.

#### Scenario: Agent completes its task
- **WHEN** a tracked agent's status transitions from "running" to "done"
- **THEN** the daemon sends a macOS notification identifying the agent (working directory / project and host context), played with the built-in `Glass` system sound

#### Scenario: Agent needs input
- **WHEN** a tracked agent's status transitions to "needs input"
- **THEN** the daemon sends a macOS notification identifying the agent, played with the built-in `Ping` system sound

#### Scenario: No duplicate notification for an unchanged status
- **WHEN** the daemon receives another event that reports the same status the agent is already in
- **THEN** the daemon does not send an additional notification for that transition
