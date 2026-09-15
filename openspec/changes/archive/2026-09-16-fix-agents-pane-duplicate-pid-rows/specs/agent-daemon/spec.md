## MODIFIED Requirements

### Requirement: Agent list query and live updates
The daemon SHALL let connected clients retrieve the current list of tracked agents and test runs, grouped by working directory, and receive updates as agent status or test-run status changes, without polling being the only option. Every agent sent to a client, in the initial snapshot or an incremental update, SHALL include its status-since timestamp and its run-started timestamp alongside its last-updated timestamp. Every test run sent to a client SHALL include its working directory, status, last-updated timestamp, and run-start timestamp. When a session id is retired because a new session id has taken over its pid (per the agent registry's same-pid dedup rule), the daemon SHALL push a removal for that retired session id to every connected client, so an already-connected client's local copy of the superseded session is dropped rather than lingering as a frozen duplicate alongside the pid's new, live entry.

#### Scenario: Client requests current agents on connect
- **WHEN** a client (e.g. the TUI) connects to the daemon
- **THEN** the daemon sends the full current list of tracked agents and test runs, grouped by working directory, each agent including its status, status-since timestamp, and run-started timestamp, and each test run including its run-start timestamp

#### Scenario: Client receives incremental updates
- **WHEN** an agent's status changes while a client is connected
- **THEN** the daemon pushes an update for that agent, including its refreshed status-since timestamp and its current run-started timestamp (refreshed if this update is a transition into running, otherwise unchanged from the value already known to the client), to the connected client without requiring the client to reconnect

#### Scenario: Client receives a test-run update
- **WHEN** a test-run event is reported while a client is connected
- **THEN** the daemon pushes an update for that test run, including its working directory, status, and run-start timestamp, to the connected client without requiring the client to reconnect

#### Scenario: A same-pid session replacement removes the superseded session from connected clients
- **WHEN** an event's new session id causes the registry to retire an existing entry that shares its pid under a different session id, while a client is connected
- **THEN** the daemon pushes both the new agent's update and a removal naming the retired session id to that client, without requiring the client to reconnect
