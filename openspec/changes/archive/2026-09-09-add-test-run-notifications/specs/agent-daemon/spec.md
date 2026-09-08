## MODIFIED Requirements

### Requirement: Agent list query and live updates
The daemon SHALL let connected clients retrieve the current list of tracked agents and test runs, grouped by working directory, and receive updates as agent status or test-run status changes, without polling being the only option. Every agent sent to a client, in the initial snapshot or an incremental update, SHALL include its status-since timestamp alongside its last-updated timestamp. Every test run sent to a client SHALL include its working directory, status, and last-updated timestamp.

#### Scenario: Client requests current agents on connect
- **WHEN** a client (e.g. the TUI) connects to the daemon
- **THEN** the daemon sends the full current list of tracked agents and test runs, grouped by working directory, each agent including its status and status-since timestamp

#### Scenario: Client receives incremental updates
- **WHEN** an agent's status changes while a client is connected
- **THEN** the daemon pushes an update for that agent, including its refreshed status-since timestamp, to the connected client without requiring the client to reconnect

#### Scenario: Client receives a test-run update
- **WHEN** a test-run event is reported while a client is connected
- **THEN** the daemon pushes an update for that test run, including its working directory and status, to the connected client without requiring the client to reconnect

## ADDED Requirements

### Requirement: Test-run event ingestion
The daemon SHALL accept test-run events (distinct from Claude Code hook events) over the same local socket, each carrying a working directory and a status of "started", "passed", or "failed".

#### Scenario: Formatter reports a test-run event
- **WHEN** an RSpec formatter sends a test-run event to the daemon's socket
- **THEN** the daemon accepts the event and updates its tracked test runs accordingly

### Requirement: Agents and test runs are grouped by working directory
The daemon SHALL group tracked agents and test runs by exact working directory match: each distinct working directory forms a group containing zero or more tracked agents (keyed by session id, as today) and zero or more tracked test runs. This grouping is additive to agent tracking - it SHALL NOT change how individual agent events are processed, keyed, or deduplicated. A test run SHALL be identified independently of any agent (for example, by working directory and process id together), so that multiple test runs and multiple agents can coexist in the same directory's group without colliding.

#### Scenario: A test run in a directory with a tracked agent
- **WHEN** a test-run event's working directory exactly matches a directory with at least one tracked agent
- **THEN** the daemon records the test run in that directory's group alongside its tracked agent(s)

#### Scenario: A test run in a directory with no tracked agent
- **WHEN** a test-run event's working directory does not match any tracked agent's working directory
- **THEN** the daemon still records the test run, in a group with no agent members

#### Scenario: A directory with multiple tracked agents
- **WHEN** two or more agents share the same working directory (for example, two terminal tabs each running their own Claude Code session there)
- **THEN** a test-run event for that directory is recorded in the single group containing all of them, without the daemon selecting or favoring one agent over another

#### Scenario: Grouping does not affect agent identity
- **WHEN** the daemon processes an agent hook event
- **THEN** it applies the existing session-id-keyed registry behavior unchanged, independent of how many test runs share that agent's directory group

### Requirement: Notification fallback for a test run with no tracked agent
When a test-run event reports "failed" status and its directory group contains no tracked agent, the daemon SHALL send a plain macOS user notification identifying the working directory, played with the built-in `Basso` system sound, since there is no agent row that would otherwise surface the failure to the user. `Basso` SHALL be distinct from the `Glass` and `Ping` sounds used for agent completion notifications, so a test failure is not confused with either by ear.

#### Scenario: A failing test run with no tracked agent
- **WHEN** the daemon receives a "failed" test-run event whose directory group contains no tracked agent
- **THEN** the daemon sends a macOS notification identifying the working directory, played with the built-in `Basso` system sound

#### Scenario: A failing test run with a tracked agent does not duplicate via this fallback
- **WHEN** the daemon receives a "failed" test-run event whose directory group contains at least one tracked agent
- **THEN** the daemon does not send this fallback notification, since the failure is visible through the tracked agent's group in a connected client

#### Scenario: A passing or started test run never triggers the fallback
- **WHEN** the daemon receives a "started" or "passed" test-run event whose directory group contains no tracked agent
- **THEN** the daemon does not send a macOS notification for it
