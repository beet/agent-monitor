## ADDED Requirements

### Requirement: Declined permission status
The daemon SHALL treat a `PermissionDenied` hook event as a distinct "declined" status update, so a session whose permission prompt was declined does not remain stuck on "needs input" indefinitely when Claude's next action is a plain-text reply with no further tool call to otherwise clear it.

#### Scenario: A permission prompt is declined
- **WHEN** the daemon receives a `PermissionDenied` event for a tracked session
- **THEN** the daemon updates that session's status to "declined"

#### Scenario: A declined session still resumes normally afterward
- **WHEN** a tracked session whose current status is "declined" reports a `PreToolUse`, `PostToolUse`, or `Stop` event
- **THEN** the daemon updates that session's status to "running" or "done" as normal, the same as it would from any other status

## MODIFIED Requirements

### Requirement: Completion notifications
The daemon SHALL send a macOS user notification when a tracked agent's status transitions to "done" or "needs input", using a status-specific system sound so the two cases are distinguishable by ear. Every hook-reported "needs input" event SHALL produce a notification, even if the agent's status was already "needs input", because each such event represents a distinct blocking prompt; a "done" event SHALL NOT produce an additional notification when the agent's status is already "done". A transition to "declined" SHALL NOT produce a notification, since the user just took that action themselves.

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

#### Scenario: No notification for a declined permission
- **WHEN** a tracked agent's status transitions to "declined"
- **THEN** the daemon does not send a macOS notification for that transition
