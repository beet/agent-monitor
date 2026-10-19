## MODIFIED Requirements

### Requirement: Local socket ingestion API
The daemon SHALL expose a local Unix domain socket that accepts status update events from Claude Code hooks. Each accepted connection SHALL be handled independently (a malformed or slow connection SHALL NOT block any other), but the daemon SHALL apply the events carried by those connections to its agent registry and activity log in the order their connections were accepted, rather than in whatever order each connection's own handling happens to finish - so two events reported for the same agent in quick succession (e.g. a `UserPromptSubmit` event immediately followed by a `PreToolUse` event for the newly-started run it precipitated) are always applied in the order Claude Code's hooks actually fired, never reordered by how the daemon happens to schedule the work of reading and parsing each connection's payload.

#### Scenario: Hook reports a status event
- **WHEN** a Claude Code hook (e.g. `Notification`, `Stop`, `SubagentStop`) fires for a tracked session and sends an event to the daemon's socket
- **THEN** the daemon accepts the event and updates its agent registry accordingly

#### Scenario: Malformed event is rejected without crashing
- **WHEN** a client sends a malformed or unrecognized payload to the socket
- **THEN** the daemon rejects the event, logs the issue, and continues serving other connections

#### Scenario: Two events arriving in quick succession are applied in arrival order
- **WHEN** the daemon accepts a connection reporting one event, then accepts a second connection reporting another event for the same agent before the first connection has finished being handled
- **THEN** the daemon applies the first connection's event to the registry and activity log before the second connection's event, regardless of which connection's own read/parse work happens to complete first
