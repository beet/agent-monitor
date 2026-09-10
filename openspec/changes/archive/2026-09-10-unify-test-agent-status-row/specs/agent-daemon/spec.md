## MODIFIED Requirements

### Requirement: Test-run event ingestion
The daemon SHALL accept test-run events (distinct from Claude Code hook events) over the same local socket, each carrying a working directory and a status of "started", "passed", or "failed". The daemon SHALL track a run-start timestamp for each directory's tracked test run: a test-run event whose process id matches the directory's currently tracked test run SHALL leave that run-start timestamp unchanged, since it is the same test process continuing through its lifecycle (started, then passed or failed); a test-run event whose process id does not match (a new process, or no test run currently tracked for that directory) SHALL set the run-start timestamp to the time of that event, since it represents the start of a new run.

#### Scenario: Formatter reports a test-run event
- **WHEN** an RSpec formatter sends a test-run event to the daemon's socket
- **THEN** the daemon accepts the event and updates its tracked test runs accordingly

#### Scenario: A started event begins a new run's timer
- **WHEN** the daemon receives a test-run event for a directory with no currently tracked test run, or whose currently tracked test run has a different process id
- **THEN** the daemon sets that test run's run-start timestamp to the time of this event

#### Scenario: A same-process completion event preserves the run-start timestamp
- **WHEN** the daemon receives a "passed" or "failed" test-run event whose process id matches the directory's currently tracked test run
- **THEN** the daemon keeps that test run's existing run-start timestamp unchanged, updating only its status and last-updated timestamp

### Requirement: Agent list query and live updates
The daemon SHALL let connected clients retrieve the current list of tracked agents and test runs, grouped by working directory, and receive updates as agent status or test-run status changes, without polling being the only option. Every agent sent to a client, in the initial snapshot or an incremental update, SHALL include its status-since timestamp alongside its last-updated timestamp. Every test run sent to a client SHALL include its working directory, status, last-updated timestamp, and run-start timestamp.

#### Scenario: Client requests current agents on connect
- **WHEN** a client (e.g. the TUI) connects to the daemon
- **THEN** the daemon sends the full current list of tracked agents and test runs, grouped by working directory, each agent including its status and status-since timestamp, and each test run including its run-start timestamp

#### Scenario: Client receives incremental updates
- **WHEN** an agent's status changes while a client is connected
- **THEN** the daemon pushes an update for that agent, including its refreshed status-since timestamp, to the connected client without requiring the client to reconnect

#### Scenario: Client receives a test-run update
- **WHEN** a test-run event is reported while a client is connected
- **THEN** the daemon pushes an update for that test run, including its working directory, status, and run-start timestamp, to the connected client without requiring the client to reconnect
