## MODIFIED Requirements

### Requirement: Local socket ingestion API
The daemon SHALL expose a local Unix domain socket that accepts status update events from Claude Code hooks. Each accepted connection SHALL be handled independently (a malformed or slow connection SHALL NOT block any other), but the daemon SHALL apply the events carried by those connections to its agent registry and activity log in the order their connections were accepted, rather than in whatever order each connection's own handling happens to finish - so two events reported for the same agent in quick succession (e.g. a `UserPromptSubmit` event immediately followed by a `PreToolUse` event for the newly-started run it precipitated) are always applied in the order Claude Code's hooks actually fired, never reordered by how the daemon happens to schedule the work of reading and parsing each connection's payload. This guarantee depends on accept order itself tracking true hook-invocation order: a reporting client SHALL open its connection to the socket before performing any payload-size-dependent work (reading its hook payload from stdin and parsing it), so that a larger, slower-to-read payload (e.g. a `UserPromptSubmit` event's full prompt text) cannot cause that event's connection to reach the daemon's accept loop after a smaller, later-fired event's connection (e.g. a `PreToolUse` event for the run that prompt precipitates). A connection that closes without ever sending a message (e.g. because reading or parsing its payload failed after it had already connected) SHALL be treated the same as today's malformed-payload case: rejected without applying anything, and without blocking any other connection's turn.

#### Scenario: Hook reports a status event
- **WHEN** a Claude Code hook (e.g. `Notification`, `Stop`, `SubagentStop`) fires for a tracked session and sends an event to the daemon's socket
- **THEN** the daemon accepts the event and updates its agent registry accordingly

#### Scenario: Malformed event is rejected without crashing
- **WHEN** a client sends a malformed or unrecognized payload to the socket
- **THEN** the daemon rejects the event, logs the issue, and continues serving other connections

#### Scenario: Two events arriving in quick succession are applied in arrival order
- **WHEN** the daemon accepts a connection reporting one event, then accepts a second connection reporting another event for the same agent before the first connection has finished being handled
- **THEN** the daemon applies the first connection's event to the registry and activity log before the second connection's event, regardless of which connection's own read/parse work happens to complete first

#### Scenario: A larger payload's event still applies in true hook-fire order
- **WHEN** a `UserPromptSubmit` hook fires, immediately followed by a `PreToolUse` hook for the run it starts, and the `UserPromptSubmit` event's payload (carrying the full prompt text) takes measurably longer to read and parse than the `PreToolUse` event's payload
- **THEN** the daemon still applies the `UserPromptSubmit` event before the `PreToolUse` event, because the reporting client for each event opened its connection before reading or parsing its own payload

#### Scenario: A connection that never sends a message is dropped without blocking others
- **WHEN** a connection is accepted but closes before any message is read from it (e.g. the reporting client failed to read or parse its payload after connecting)
- **THEN** the daemon rejects it without applying anything to the registry or activity log, and does not block any later-accepted connection's turn
