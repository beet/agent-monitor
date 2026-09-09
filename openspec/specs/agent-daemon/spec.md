# agent-daemon Specification

## Purpose

Tracks the status of Claude Code agent sessions running anywhere on the machine (nvim terminal, standalone terminal, desktop app) via hook-reported events, and notifies the user when an agent finishes or needs input.

## Requirements

### Requirement: Local socket ingestion API
The daemon SHALL expose a local Unix domain socket that accepts status update events from Claude Code hooks.

#### Scenario: Hook reports a status event
- **WHEN** a Claude Code hook (e.g. `Notification`, `Stop`, `SubagentStop`) fires for a tracked session and sends an event to the daemon's socket
- **THEN** the daemon accepts the event and updates its agent registry accordingly

#### Scenario: Malformed event is rejected without crashing
- **WHEN** a client sends a malformed or unrecognized payload to the socket
- **THEN** the daemon rejects the event, logs the issue, and continues serving other connections

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

### Requirement: Resuming from needs input
The daemon SHALL treat a `PreToolUse` or `PostToolUse` hook event as a "running" status update, so a session currently in "needs input" status (for example, because of an unresolved permission or elicitation prompt) returns to "running" once Claude resumes work - not only when the user submits a new prompt via `UserPromptSubmit`, and not only on the next distinct tool invocation's `PreToolUse`. A `PostToolUse` event covers the case a `PreToolUse` event cannot: a tool call that itself raised the "needs input" prompt (or several, in sequence) has no further `PreToolUse` event until it completes, so `PostToolUse` is the signal that resumption happened as soon as that call finishes.

#### Scenario: Tool use resumes after a needs-input prompt is resolved
- **WHEN** a tracked session whose current status is "needs input" reports a `PreToolUse` event
- **THEN** the daemon updates that session's status to "running"

#### Scenario: A completed tool call resumes a session stuck on its own prompt
- **WHEN** a tracked session whose current status is "needs input" reports a `PostToolUse` event
- **THEN** the daemon updates that session's status to "running"

#### Scenario: A fresh session's first tool invocation is tracked as running
- **WHEN** the daemon receives a `PreToolUse` event for a session id it has not seen before
- **THEN** it registers a new agent entry with status "running"

#### Scenario: Tool use during an already-running session is a no-op status change
- **WHEN** the daemon receives a `PreToolUse` or `PostToolUse` event for a session whose current status is already "running"
- **THEN** the daemon updates the entry's last-updated timestamp and its status remains "running"

### Requirement: Declined permission status
The daemon SHALL treat a `PermissionDenied` hook event as a distinct "declined" status update, so a session whose permission prompt was declined does not remain stuck on "needs input" indefinitely when Claude's next action is a plain-text reply with no further tool call to otherwise clear it.

#### Scenario: A permission prompt is declined
- **WHEN** the daemon receives a `PermissionDenied` event for a tracked session
- **THEN** the daemon updates that session's status to "declined"

#### Scenario: A declined session still resumes normally afterward
- **WHEN** a tracked session whose current status is "declined" reports a `PreToolUse`, `PostToolUse`, or `Stop` event
- **THEN** the daemon updates that session's status to "running" or "done" as normal, the same as it would from any other status

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

### Requirement: Test-run event ingestion
The daemon SHALL accept test-run events (distinct from Claude Code hook events) over the same local socket, each carrying a working directory and a status of "started", "passed", or "failed".

#### Scenario: Formatter reports a test-run event
- **WHEN** an RSpec formatter sends a test-run event to the daemon's socket
- **THEN** the daemon accepts the event and updates its tracked test runs accordingly

### Requirement: Agents and test runs are grouped by working directory
The daemon SHALL group tracked agents and test runs by exact working directory match: each distinct working directory forms a group containing zero or more tracked agents (keyed by session id, as today) and at most one tracked test run. This grouping is additive to agent tracking - it SHALL NOT change how individual agent events are processed, keyed, or deduplicated. A test run SHALL be identified by its working directory alone, independent of any agent: reporting a new test-run event for a directory SHALL replace any previously tracked test run for that directory, regardless of process id, so a directory's test-run row always reflects only the most recently reported run rather than accumulating one entry per invocation.

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

#### Scenario: A later test run replaces the previous one in the same directory
- **WHEN** the daemon receives a test-run event for a working directory that already has a tracked test run, reported by a different process id than the one currently tracked
- **THEN** the daemon replaces the tracked test run with the new one instead of tracking both, so repeated invocations in the same directory (for example, an edit/test loop) never accumulate more than one test-run row per directory

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

### Requirement: Completion notifications
The daemon SHALL send a macOS user notification when a tracked agent's status transitions to "done" or "needs input", using a status-specific system sound so the two cases are distinguishable by ear. Every hook-reported "needs input" event SHALL produce a notification, even if the agent's status was already "needs input", because each such event represents a distinct blocking prompt; a "done" event SHALL NOT produce an additional notification when the agent's status is already "done". A transition to "declined" SHALL NOT produce a notification, since the user just took that action themselves.

#### Scenario: Agent completes its task
- **WHEN** a tracked agent's status transitions from "running" to "done"
- **THEN** the daemon sends a macOS notification identifying the agent (working directory / project and host context), played with the built-in `Glass` system sound

#### Scenario: Agent needs input
- **WHEN** a tracked agent's status transitions to "needs input"
- **THEN** the daemon sends a macOS notification identifying the agent, played with the built-in `Ping` system sound

#### Scenario: A second needs-input prompt in the same turn still notifies
- **WHEN** the daemon receives a "needs input" event for an agent that is already in the "needs input" status
- **THEN** the daemon sends another macOS notification, since it represents a new blocking prompt rather than a repeat of the same one

#### Scenario: No duplicate notification for an unchanged status
- **WHEN** the daemon receives another event that reports "done" for an agent already in the "done" status
- **THEN** the daemon does not send an additional notification for that transition

#### Scenario: No notification for a declined permission
- **WHEN** a tracked agent's status transitions to "declined"
- **THEN** the daemon does not send a macOS notification for that transition

### Requirement: Stale agent detection
The daemon SHALL detect when a tracked agent's process is no longer running and mark it as stale rather than continuing to report its last known status indefinitely. Marking an agent stale is a status transition, so it SHALL reset that agent's status-since timestamp to the current time.

#### Scenario: Tracked process has exited
- **WHEN** the daemon checks a tracked agent's pid and finds the process no longer exists
- **THEN** the daemon marks that agent's status as stale/disconnected, resets its status-since timestamp to the current time, and reflects this to connected clients

### Requirement: Runs as a macOS background service
The daemon SHALL be installable and runnable as a per-user macOS `launchd` service, in addition to running in the foreground for development. The service SHALL be installed, started, stopped, and restarted via Homebrew's `brew services` interface rather than daemon-specific subcommands, so the plist Homebrew generates always references the current Homebrew-managed binary path and is updated in place on upgrade, not a version-pinned path.

#### Scenario: Install as a login service
- **WHEN** the user runs `brew services start agent-monitor`
- **THEN** a `launchd` user agent plist is written and loaded so the daemon starts automatically and keeps running in the background

#### Scenario: Uninstall the service
- **WHEN** the user runs `brew services stop agent-monitor`
- **THEN** the `launchd` user agent is unloaded and the daemon stops running until started again

#### Scenario: Service survives a Homebrew upgrade with just a restart
- **WHEN** the user runs `brew upgrade` for the package while the service is installed, then runs `brew services restart agent-monitor` (without reinstalling the service)
- **THEN** the daemon that starts is the newly installed version, not the one that was running before the upgrade

### Requirement: Socket lifecycle and local-only access
The daemon SHALL create its socket at a well-known per-user path, recover cleanly from a stale socket file left by a previous crash, and restrict access to the local user.

#### Scenario: Starting with a leftover stale socket file
- **WHEN** the daemon starts and finds a socket file at its path with no daemon listening on it
- **THEN** the daemon removes the stale file and binds a fresh socket instead of failing to start

#### Scenario: Socket is not reachable over the network
- **WHEN** the daemon creates its socket
- **THEN** the socket is a filesystem-local Unix domain socket with permissions restricting access to the owning user, not a network-exposed endpoint
