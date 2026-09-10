## REMOVED Requirements

### Requirement: Notification fallback for a test run with no tracked agent
**Reason**: Superseded by "Test-run lifecycle notifications", which notifies for every "started", "passed", or "failed" test-run event regardless of whether its directory has a tracked agent. A long-running agent session can start, stop, and re-run tests several times before the agent itself finishes, and the user wants to hear each test-run event as it happens rather than only when no agent happens to be tracked in that directory.
**Migration**: No user action needed. The new requirement still plays `Basso` for every failure (previously only when no agent was tracked, now unconditionally) and adds `Tink` for passes and `Pop` for starts.

## ADDED Requirements

### Requirement: Test-run lifecycle notifications
The daemon SHALL send a macOS user notification identifying the working directory whenever a tracked test run's status is reported as "started", "passed", or "failed", independent of whether that directory has a tracked agent - so a test run can be started, stopped, and re-run multiple times over the course of one long agent session, and each event is heard on its own rather than only when nothing else would surface it. A "started" test run SHALL play the built-in `Pop` system sound; a "passed" test run SHALL play the built-in `Tink` system sound; a "failed" test run SHALL play the built-in `Basso` system sound. `Pop`, `Tink`, and `Basso` SHALL each be distinct from one another and from the `Glass`/`Ping` sounds used for agent completion notifications, so a test-run event is never confused with a different event or with an agent notification by ear.

#### Scenario: A test run starts
- **WHEN** the daemon receives a "started" test-run event
- **THEN** the daemon sends a macOS notification identifying the working directory, played with the built-in `Pop` system sound, regardless of whether that directory has a tracked agent

#### Scenario: A test run fails with no tracked agent
- **WHEN** the daemon receives a "failed" test-run event whose directory group contains no tracked agent
- **THEN** the daemon sends a macOS notification identifying the working directory, played with the built-in `Basso` system sound

#### Scenario: A test run fails with a tracked agent present
- **WHEN** the daemon receives a "failed" test-run event whose directory group contains a tracked agent
- **THEN** the daemon sends a macOS notification identifying the working directory, played with `Basso`, the same as it would if no agent were tracked

#### Scenario: A test run passes
- **WHEN** the daemon receives a "passed" test-run event
- **THEN** the daemon sends a macOS notification identifying the working directory, played with the built-in `Tink` system sound, regardless of whether that directory has a tracked agent

#### Scenario: Repeated lifecycle events each notify independently
- **WHEN** a tracked test run in the same directory starts, fails, and later starts and fails again
- **THEN** the daemon sends a separate notification for each event, since each represents a distinct occurrence rather than a repeat of an earlier one
