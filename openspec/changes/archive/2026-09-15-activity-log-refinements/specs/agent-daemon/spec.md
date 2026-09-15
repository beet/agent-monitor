## MODIFIED Requirements

### Requirement: Agent registry
The daemon SHALL maintain a registry of known agents keyed by session identifier, recording working directory, host context (nvim, standalone terminal, or desktop app), process id, current status, last-updated timestamp, a status-since timestamp marking when the agent most recently entered its current status, and a run-started timestamp marking when the agent most recently transitioned into "running" (or was registered for the first time already "running"). The registry SHALL reject a "needs input" event for a session whose current status is already "done", since a finished session cannot legitimately need input again until a new "running" event is reported for it. The registry SHALL hold at most one entry per pid: when an event's session id is new but its pid matches an existing entry under a different session id, the registry SHALL remove that existing entry before registering the new one, since the same pid starting a new session id (for example, via `/clear`) means the old session is gone, not merely quiet. The status-since timestamp SHALL only be set to the current time when an update actually changes the agent's status; an event or internal transition (e.g. marking an agent stale) that leaves the status unchanged SHALL leave the status-since timestamp untouched. The run-started timestamp SHALL be set to the current time when the entry is created, and reset to the current time again every time the entry's status subsequently transitions to "running" from a different status - including a transition from "done", since one tracked agent's pid persists across many started-running-done turns over the life of a session, unlike a test run's pid-scoped run-start timestamp. The run-started timestamp SHALL remain unchanged by any transition that leaves "running" or that moves between two non-running statuses (into idle, needs input, done, declined, or stale), so it continues to mark the beginning of the most recently started run of work until superseded by the next transition into "running". Replacing a pid's entry this way SHALL permanently supersede the session id it replaced: a later event that names that superseded session id SHALL be treated as unknown rather than as a session the registry has not seen before, so it cannot be re-registered as a new entry and, by the same-pid rule above, cannot evict the live session that replaced it.

#### Scenario: First event for a session registers a new agent
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid does not match any existing entry
- **THEN** it creates a new registry entry with the reported working directory, host context, pid, and status, and sets the last-updated, status-since, and run-started timestamps to the current time

#### Scenario: Subsequent event updates the existing agent
- **WHEN** the daemon receives an event for a session id already in the registry
- **THEN** it updates that entry's status and last-updated timestamp instead of creating a duplicate

#### Scenario: A same-status event does not reset the status-since timestamp
- **WHEN** the daemon receives an event for a session already in the registry and the event's status matches the agent's current status
- **THEN** the daemon updates the last-updated timestamp but leaves the status-since timestamp unchanged

#### Scenario: A status transition resets the status-since timestamp
- **WHEN** the daemon receives an event for a session already in the registry and the event's status differs from the agent's current status
- **THEN** the daemon sets the status-since timestamp to the current time in addition to updating the status and last-updated timestamp

#### Scenario: A transition into running resets the run-started timestamp
- **WHEN** the daemon receives a "running" event for a session whose current status differs from "running" (idle, needs input, done, declined, or stale)
- **THEN** the daemon sets that entry's run-started timestamp to the current time, marking the beginning of this new run, in addition to updating status and status-since timestamp

#### Scenario: A transition away from running leaves the run-started timestamp unchanged
- **WHEN** the daemon receives an event that transitions a "running" session to idle, needs input, done, declined, or stale
- **THEN** the daemon leaves that entry's run-started timestamp unchanged, so it still marks when the run that just ended began

#### Scenario: A late needs-input event cannot override a completed session
- **WHEN** the daemon receives a "needs input" event for a session whose current status is already "done"
- **THEN** the daemon leaves that agent's status as "done" and does not apply the "needs input" event, and does not change its status-since or run-started timestamp

#### Scenario: A new running event still clears a completed session's status
- **WHEN** the daemon receives a "running" event for a session whose current status is "done"
- **THEN** it updates that entry's status to "running" as normal, and resets both the status-since timestamp and the run-started timestamp to the current time, since this begins a new run

#### Scenario: A new session id for an already-tracked pid replaces the old entry
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid matches an existing entry registered under a different session id
- **THEN** the daemon removes the existing entry for that pid and registers the new session id as the sole entry for that pid, with last-updated, status-since, and run-started timestamps all set to the current time rather than inherited from the entry it replaced

#### Scenario: A late event for an already-superseded session id is ignored
- **WHEN** the daemon receives an event naming a session id that was previously removed because another session id took over its pid, and that replacement session id is still the pid's tracked entry
- **THEN** the daemon does not register the superseded session id as a new entry and does not remove or otherwise alter the pid's current entry

#### Scenario: A late event for a superseded session id whose pid has since moved on again
- **WHEN** the daemon receives an event naming a session id that was superseded, and the pid it belonged to is now tracked under yet another, different session id
- **THEN** the daemon still does not register the superseded session id as a new entry, and the pid's currently tracked entry is unaffected

### Requirement: Agent list query and live updates
The daemon SHALL let connected clients retrieve the current list of tracked agents and test runs, grouped by working directory, and receive updates as agent status or test-run status changes, without polling being the only option. Every agent sent to a client, in the initial snapshot or an incremental update, SHALL include its status-since timestamp and its run-started timestamp alongside its last-updated timestamp. Every test run sent to a client SHALL include its working directory, status, last-updated timestamp, and run-start timestamp.

#### Scenario: Client requests current agents on connect
- **WHEN** a client (e.g. the TUI) connects to the daemon
- **THEN** the daemon sends the full current list of tracked agents and test runs, grouped by working directory, each agent including its status, status-since timestamp, and run-started timestamp, and each test run including its run-start timestamp

#### Scenario: Client receives incremental updates
- **WHEN** an agent's status changes while a client is connected
- **THEN** the daemon pushes an update for that agent, including its refreshed status-since timestamp and its current run-started timestamp (refreshed if this update is a transition into running, otherwise unchanged from the value already known to the client), to the connected client without requiring the client to reconnect

#### Scenario: Client receives a test-run update
- **WHEN** a test-run event is reported while a client is connected
- **THEN** the daemon pushes an update for that test run, including its working directory, status, and run-start timestamp, to the connected client without requiring the client to reconnect
