## MODIFIED Requirements

### Requirement: Activity log captures notification-worthy events
The daemon SHALL append an entry to its activity log at the same point it sends a macOS notification for an agent status transition to "done" or "needs input", for a test-run event of "started", "passed", or "failed", and for a reminder completing on its own ("finished"). The daemon SHALL also append an entry, independent of any notification, whenever an agent's status transitions to "running" from a different status, or is registered for the first time already "running" - recorded with status "started" - so each agent's later "done"/"needs input" entries have a corresponding start point in the log; and whenever a reminder is started ("started", including re-running a previously completed reminder) or a running reminder is stopped ("stopped"), even though starting and stopping a reminder do not themselves send a notification. The daemon SHALL also append an entry, independent of any notification and independent of any other entry the same event produces, whenever a `UserPromptSubmit` hook event is reported for a tracked agent - recorded with category "prompt" and status "submitted". Each entry SHALL record the working directory, an event category (agent, test-run, reminder, or prompt), the resulting status, and the time the event occurred; an agent-category entry SHALL additionally record that agent's process id and a snapshot of that agent's current last-prompt text, if any, as of that event (frozen at record time, the same way a reminder-category entry freezes the reminder's name, so a later prompt submitted for that agent does not alter an entry already recorded); a prompt-category entry SHALL additionally record that agent's process id and the submitted prompt text verbatim - unlike an agent-category entry's snapshot, a prompt-category entry's prompt text is never absent, since the event that produces it always carries one; and a reminder-category entry SHALL additionally record the reminder's name as of that event, so a later-deleted reminder's past entries keep showing it. A reminder-category "started" entry SHALL additionally record the reminder's due time (its ETA) as of that event, computed from its start time and the reminder's duration at the moment it starts; this recorded ETA SHALL NOT change afterward even if the reminder's duration is later edited while it runs. When a `UserPromptSubmit` event also satisfies the "started" condition above (the agent's status transitions into "running" as a result), the daemon SHALL record that event's "prompt" entry before its "agent"/"started" entry, with both entries sharing the same event-occurred time, so the prompt entry consistently takes precedence in recency order over the started entry it precipitated, rather than the two landing in an arbitrary order relative to each other. Transitions that do not produce a notification and are not a transition into "running" - "declined" and "stale" - SHALL NOT produce a log entry. A repeated "needs input" event for an agent already in that status SHALL produce a separate log entry, consistent with it producing a separate notification. Editing a reminder's name or duration SHALL NOT produce a log entry.

#### Scenario: Agent completes its task
- **WHEN** a tracked agent's status transitions to "done" and the daemon sends a completion notification
- **THEN** the daemon appends a log entry recording the agent's working directory, category "agent", status "done", process id, its current last-prompt snapshot (if any), and the event time

#### Scenario: Agent needs input
- **WHEN** a tracked agent's status transitions to "needs input" and the daemon sends a notification
- **THEN** the daemon appends a log entry with category "agent", status "needs input", that agent's process id, and its current last-prompt snapshot (if any)

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

#### Scenario: A reminder is started
- **WHEN** a reminder is started, whether for the first time or as a re-run after completing
- **THEN** the daemon appends a log entry with category "reminder", status "started", the reminder's current name, and its due time (ETA) as of that event

#### Scenario: A reminder's recorded ETA does not follow a later edit
- **WHEN** a running reminder's duration is edited after it started, changing its current due time
- **THEN** the ETA already recorded on its "started" log entry is unchanged, continuing to reflect the due time as of when it started

#### Scenario: A running reminder is stopped
- **WHEN** a client stops a reminder whose status is running
- **THEN** the daemon appends a log entry with category "reminder", status "stopped", and the reminder's current name, even though stopping sends no notification

#### Scenario: A reminder finishes on its own
- **WHEN** a running reminder's elapsed time reaches its duration and the daemon sends its completion notification
- **THEN** the daemon appends a log entry with category "reminder", status "finished", and the reminder's current name

#### Scenario: A deleted reminder's entries keep its name
- **WHEN** a reminder that has previously logged activity is deleted
- **THEN** its existing log entries continue to display the name they were recorded with

#### Scenario: Editing a reminder does not log an entry
- **WHEN** a client edits a reminder's name or duration
- **THEN** the daemon does not append a log entry for that edit

#### Scenario: An agent entry with no captured prompt records none
- **WHEN** a "started", "done", or "needs input" entry is recorded for an agent that has no last-prompt text tracked yet (e.g. the daemon began tracking it mid-session, via `PreToolUse`, before any `UserPromptSubmit` event was seen)
- **THEN** the entry records no prompt snapshot rather than an error or placeholder text

#### Scenario: A later prompt does not alter an already-recorded entry's snapshot
- **WHEN** an agent submits a new prompt after one of its "started" or "done" entries has already been recorded
- **THEN** that earlier entry's recorded prompt snapshot is unchanged, even though the agent's current last prompt (as shown elsewhere) now reflects the new one

#### Scenario: A prompt submission is logged on its own
- **WHEN** the daemon receives a `UserPromptSubmit`-derived event for a tracked agent
- **THEN** the daemon appends a log entry with category "prompt", status "submitted", that agent's process id, the submitted prompt text, and the event time

#### Scenario: A prompt that also starts a run logs before the started entry
- **WHEN** a `UserPromptSubmit` event's resulting status transition also satisfies the "started" condition (entering "running" from any other status, or a brand-new registration already "running")
- **THEN** the daemon appends the "prompt" entry before the "agent"/"started" entry it precipitated, and both entries share the same event-occurred time

#### Scenario: A follow-up prompt mid-session logs with no accompanying started entry
- **WHEN** the daemon receives a `UserPromptSubmit`-derived event for an agent whose status is already "running" (so no "started" entry is produced for it)
- **THEN** the daemon still appends a "prompt" entry on its own, recorded at the time of that event
