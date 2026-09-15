## MODIFIED Requirements

### Requirement: Activity log captures notification-worthy events
The daemon SHALL append an entry to its activity log at the same point it sends a macOS notification for an agent status transition to "done" or "needs input", and for a test-run event of "started", "passed", or "failed". The daemon SHALL also append an entry, independent of any notification, whenever an agent's status transitions to "running" from a different status, or is registered for the first time already "running" - recorded with status "started" - so each agent's later "done"/"needs input" entries have a corresponding start point in the log. Each entry SHALL record the working directory, an event category (agent or test-run), the resulting status, and the time the event occurred; an agent-category entry SHALL additionally record that agent's process id. Transitions that do not produce a notification and are not a transition into "running" - "declined" and "stale" - SHALL NOT produce a log entry. A repeated "needs input" event for an agent already in that status SHALL produce a separate log entry, consistent with it producing a separate notification.

#### Scenario: Agent completes its task
- **WHEN** a tracked agent's status transitions to "done" and the daemon sends a completion notification
- **THEN** the daemon appends a log entry recording the agent's working directory, category "agent", status "done", process id, and the event time

#### Scenario: Agent needs input
- **WHEN** a tracked agent's status transitions to "needs input" and the daemon sends a notification
- **THEN** the daemon appends a log entry with category "agent", status "needs input", and that agent's process id

#### Scenario: A second needs-input prompt logs again
- **WHEN** the daemon receives a "needs input" event for an agent already in "needs input" status and sends another notification for it
- **THEN** the daemon appends another log entry rather than skipping it as a duplicate

#### Scenario: A new agent's first running event is logged as started
- **WHEN** the daemon registers a session id it has not seen before with an initial status of "running"
- **THEN** the daemon appends a log entry with category "agent", status "started", and that agent's process id, even though no notification is sent for this transition

#### Scenario: An agent resuming into running logs a new started entry
- **WHEN** an already-tracked agent's status transitions to "running" from idle, needs input, done, declined, or stale
- **THEN** the daemon appends an "agent"/"started" log entry for that transition, independent of whether a notification is also sent for it

#### Scenario: Continuing to run produces no additional started entries
- **WHEN** the daemon receives an event for an agent whose status is already "running" and remains "running"
- **THEN** the daemon does not append another "started" log entry, since no status transition occurred

#### Scenario: Test run starts, passes, or fails
- **WHEN** the daemon sends a test-run lifecycle notification for "started", "passed", or "failed"
- **THEN** the daemon appends a log entry recording the test run's working directory, category "test-run", and that status

#### Scenario: A declined permission is not logged
- **WHEN** a tracked agent's status transitions to "declined"
- **THEN** the daemon does not append a log entry for that transition

#### Scenario: A stale agent is not logged
- **WHEN** the daemon marks a tracked agent as stale
- **THEN** the daemon does not append a log entry for that transition
