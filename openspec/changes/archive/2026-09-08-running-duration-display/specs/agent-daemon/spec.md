## MODIFIED Requirements

### Requirement: Agent registry
The daemon SHALL maintain a registry of known agents keyed by session identifier, recording working directory, host context (nvim, standalone terminal, or desktop app), process id, current status, last-updated timestamp, and a status-since timestamp marking when the agent most recently entered its current status. The registry SHALL reject a "needs input" event for a session whose current status is already "done", since a finished session cannot legitimately need input again until a new "running" event is reported for it. The registry SHALL hold at most one entry per pid: when an event's session id is new but its pid matches an existing entry under a different session id, the registry SHALL remove that existing entry before registering the new one, since the same pid starting a new session id (for example, via `/clear`) means the old session is gone, not merely quiet. The status-since timestamp SHALL only be set to the current time when an update actually changes the agent's status; an event or internal transition (e.g. marking an agent stale) that leaves the status unchanged SHALL leave the status-since timestamp untouched.

#### Scenario: First event for a session registers a new agent
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid does not match any existing entry
- **THEN** it creates a new registry entry with the reported working directory, host context, pid, and status, and sets both the last-updated and status-since timestamps to the current time

#### Scenario: Subsequent event updates the existing agent
- **WHEN** the daemon receives an event for a session id already in the registry
- **THEN** it updates that entry's status and last-updated timestamp instead of creating a duplicate

#### Scenario: A same-status event does not reset the status-since timestamp
- **WHEN** the daemon receives an event for a session already in the registry and the event's status matches the agent's current status
- **THEN** the daemon updates the last-updated timestamp but leaves the status-since timestamp unchanged

#### Scenario: A status transition resets the status-since timestamp
- **WHEN** the daemon receives an event for a session already in the registry and the event's status differs from the agent's current status
- **THEN** the daemon sets the status-since timestamp to the current time in addition to updating the status and last-updated timestamp

#### Scenario: A late needs-input event cannot override a completed session
- **WHEN** the daemon receives a "needs input" event for a session whose current status is already "done"
- **THEN** the daemon leaves that agent's status as "done" and does not apply the "needs input" event, and does not change its status-since timestamp

#### Scenario: A new running event still clears a completed session's status
- **WHEN** the daemon receives a "running" event for a session whose current status is "done"
- **THEN** it updates that entry's status to "running" as normal and resets the status-since timestamp to the current time

#### Scenario: A new session id for an already-tracked pid replaces the old entry
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid matches an existing entry registered under a different session id
- **THEN** the daemon removes the existing entry for that pid and registers the new session id as the sole entry for that pid, instead of the two entries coexisting

### Requirement: Agent list query and live updates
The daemon SHALL let connected clients retrieve the current list of tracked agents and receive updates as agent status changes, without polling being the only option. Every agent sent to a client, in the initial snapshot or an incremental update, SHALL include its status-since timestamp alongside its last-updated timestamp.

#### Scenario: Client requests current agents on connect
- **WHEN** a client (e.g. the TUI) connects to the daemon
- **THEN** the daemon sends the full current list of tracked agents, each including its status and status-since timestamp

#### Scenario: Client receives incremental updates
- **WHEN** an agent's status changes while a client is connected
- **THEN** the daemon pushes an update for that agent, including its refreshed status-since timestamp, to the connected client without requiring the client to reconnect

### Requirement: Stale agent detection
The daemon SHALL detect when a tracked agent's process is no longer running and mark it as stale rather than continuing to report its last known status indefinitely. Marking an agent stale is a status transition, so it SHALL reset that agent's status-since timestamp to the current time.

#### Scenario: Tracked process has exited
- **WHEN** the daemon checks a tracked agent's pid and finds the process no longer exists
- **THEN** the daemon marks that agent's status as stale/disconnected, resets its status-since timestamp to the current time, and reflects this to connected clients
