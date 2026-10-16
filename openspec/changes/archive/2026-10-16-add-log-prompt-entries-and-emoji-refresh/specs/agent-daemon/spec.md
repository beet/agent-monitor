## MODIFIED Requirements

### Requirement: Test-run lifecycle notifications
The daemon SHALL send a macOS user notification identifying the working directory whenever a tracked test run's status is reported as "started", "passed", or "failed", independent of whether that directory has a tracked agent - so a test run can be started, stopped, and re-run multiple times over the course of one long agent session, and each event is heard on its own rather than only when nothing else would surface it. A "started" test run SHALL play the built-in `Pop` system sound; a "passed" test run SHALL play the built-in `Tink` system sound; a "failed" test run SHALL play the built-in `Basso` system sound. `Pop`, `Tink`, and `Basso` SHALL each be distinct from one another and from the `Glass`/`Ping` sounds used for agent completion notifications, so a test-run event is never confused with a different event or with an agent notification by ear. The notification body SHALL be prefixed with an emoji matching the event, so the notification is identifiable at a glance before reading its text: 🧪 for "started", ✅ for "passed", and ❌ for "failed".

#### Scenario: A test run starts
- **WHEN** the daemon receives a "started" test-run event
- **THEN** the daemon sends a macOS notification identifying the working directory, with its body prefixed by 🧪, played with the built-in `Pop` system sound, regardless of whether that directory has a tracked agent

#### Scenario: A test run fails with no tracked agent
- **WHEN** the daemon receives a "failed" test-run event whose directory group contains no tracked agent
- **THEN** the daemon sends a macOS notification identifying the working directory, with its body prefixed by ❌, played with the built-in `Basso` system sound

#### Scenario: A test run fails with a tracked agent present
- **WHEN** the daemon receives a "failed" test-run event whose directory group contains a tracked agent
- **THEN** the daemon sends a macOS notification identifying the working directory, with its body prefixed by ❌, played with `Basso`, the same as it would if no agent were tracked

#### Scenario: A test run passes
- **WHEN** the daemon receives a "passed" test-run event
- **THEN** the daemon sends a macOS notification identifying the working directory, with its body prefixed by ✅, played with the built-in `Tink` system sound, regardless of whether that directory has a tracked agent

#### Scenario: Repeated lifecycle events each notify independently
- **WHEN** a tracked test run in the same directory starts, fails, and later starts and fails again
- **THEN** the daemon sends a separate notification for each event, since each represents a distinct occurrence rather than a repeat of an earlier one

### Requirement: Completion notifications
The daemon SHALL send a macOS user notification when a tracked agent's status transitions to "done" or "needs input", using a status-specific system sound so the two cases are distinguishable by ear. Every hook-reported "needs input" event SHALL produce a notification, even if the agent's status was already "needs input", because each such event represents a distinct blocking prompt; a "done" event SHALL NOT produce an additional notification when the agent's status is already "done". A transition to "declined" SHALL NOT produce a notification, since the user just took that action themselves. The notification body SHALL be prefixed with 🤖 for both "done" and "needs input", so the notification is identifiable as agent activity at a glance before reading its text.

#### Scenario: Agent completes its task
- **WHEN** a tracked agent's status transitions from "running" to "done"
- **THEN** the daemon sends a macOS notification identifying the agent (working directory / project and host context), with its body prefixed by 🤖, played with the built-in `Glass` system sound

#### Scenario: Agent needs input
- **WHEN** a tracked agent's status transitions to "needs input"
- **THEN** the daemon sends a macOS notification identifying the agent, with its body prefixed by 🤖, played with the built-in `Ping` system sound

#### Scenario: A second needs-input prompt in the same turn still notifies
- **WHEN** the daemon receives a "needs input" event for an agent that is already in the "needs input" status
- **THEN** the daemon sends another macOS notification, since it represents a new blocking prompt rather than a repeat of the same one

#### Scenario: No duplicate notification for an unchanged status
- **WHEN** the daemon receives another event that reports "done" for an agent already in the "done" status
- **THEN** the daemon does not send an additional notification for that transition

#### Scenario: No notification for a declined permission
- **WHEN** a tracked agent's status transitions to "declined"
- **THEN** the daemon does not send a macOS notification for that transition
