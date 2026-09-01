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
The daemon SHALL maintain a registry of known agents keyed by session identifier, recording working directory, host context (nvim, standalone terminal, or desktop app), process id, current status, and last-updated timestamp. The registry SHALL reject a "needs input" event for a session whose current status is already "done", since a finished session cannot legitimately need input again until a new "running" event is reported for it. The registry SHALL hold at most one entry per pid: when an event's session id is new but its pid matches an existing entry under a different session id, the registry SHALL remove that existing entry before registering the new one, since the same pid starting a new session id (for example, via `/clear`) means the old session is gone, not merely quiet.

#### Scenario: First event for a session registers a new agent
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid does not match any existing entry
- **THEN** it creates a new registry entry with the reported working directory, host context, pid, and status

#### Scenario: Subsequent event updates the existing agent
- **WHEN** the daemon receives an event for a session id already in the registry
- **THEN** it updates that entry's status and last-updated timestamp instead of creating a duplicate

#### Scenario: A late needs-input event cannot override a completed session
- **WHEN** the daemon receives a "needs input" event for a session whose current status is already "done"
- **THEN** the daemon leaves that agent's status as "done" and does not apply the "needs input" event

#### Scenario: A new running event still clears a completed session's status
- **WHEN** the daemon receives a "running" event for a session whose current status is "done"
- **THEN** it updates that entry's status to "running" as normal

#### Scenario: A new session id for an already-tracked pid replaces the old entry
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid matches an existing entry registered under a different session id
- **THEN** the daemon removes the existing entry for that pid and registers the new session id as the sole entry for that pid, instead of the two entries coexisting

### Requirement: Resuming from needs input
The daemon SHALL treat a `PreToolUse` hook event as a "running" status update, so a session currently in "needs input" status (for example, because of an unresolved permission or elicitation prompt) returns to "running" once Claude resumes work - not only when the user submits a new prompt via `UserPromptSubmit`.

#### Scenario: Tool use resumes after a needs-input prompt is resolved
- **WHEN** a tracked session whose current status is "needs input" reports a `PreToolUse` event
- **THEN** the daemon updates that session's status to "running"

#### Scenario: A fresh session's first tool invocation is tracked as running
- **WHEN** the daemon receives a `PreToolUse` event for a session id it has not seen before
- **THEN** it registers a new agent entry with status "running"

#### Scenario: Tool use during an already-running session is a no-op status change
- **WHEN** the daemon receives a `PreToolUse` event for a session whose current status is already "running"
- **THEN** the daemon updates the entry's last-updated timestamp and its status remains "running"

### Requirement: Agent list query and live updates
The daemon SHALL let connected clients retrieve the current list of tracked agents and receive updates as agent status changes, without polling being the only option.

#### Scenario: Client requests current agents on connect
- **WHEN** a client (e.g. the TUI) connects to the daemon
- **THEN** the daemon sends the full current list of tracked agents and their statuses

#### Scenario: Client receives incremental updates
- **WHEN** an agent's status changes while a client is connected
- **THEN** the daemon pushes an update for that agent to the connected client without requiring the client to reconnect

### Requirement: Completion notifications
The daemon SHALL send a macOS user notification when a tracked agent's status transitions to "done" or "needs input", using a status-specific system sound so the two cases are distinguishable by ear. Every hook-reported "needs input" event SHALL produce a notification, even if the agent's status was already "needs input", because each such event represents a distinct blocking prompt; a "done" event SHALL NOT produce an additional notification when the agent's status is already "done".

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

### Requirement: Stale agent detection
The daemon SHALL detect when a tracked agent's process is no longer running and mark it as stale rather than continuing to report its last known status indefinitely.

#### Scenario: Tracked process has exited
- **WHEN** the daemon checks a tracked agent's pid and finds the process no longer exists
- **THEN** the daemon marks that agent's status as stale/disconnected and reflects this to connected clients

### Requirement: Runs as a macOS background service
The daemon SHALL be installable and runnable as a per-user macOS `launchd` service, in addition to running in the foreground for development. The installed service SHALL reference the daemon binary by a Homebrew-managed path that Homebrew updates in place on upgrade, not a version-pinned path, so the service picks up a new version after a restart without being reinstalled.

#### Scenario: Install as a login service
- **WHEN** the user runs the daemon's install command
- **THEN** a `launchd` user agent plist is written and loaded so the daemon starts automatically and keeps running in the background

#### Scenario: Uninstall the service
- **WHEN** the user runs the daemon's uninstall command
- **THEN** the `launchd` user agent is unloaded and its plist is removed

#### Scenario: Service survives a Homebrew upgrade with just a restart
- **WHEN** the user runs `brew upgrade` for the package while the service is installed, then restarts the daemon (without re-running the install command)
- **THEN** the daemon that starts is the newly installed version, not the one that was running before the upgrade

### Requirement: Socket lifecycle and local-only access
The daemon SHALL create its socket at a well-known per-user path, recover cleanly from a stale socket file left by a previous crash, and restrict access to the local user.

#### Scenario: Starting with a leftover stale socket file
- **WHEN** the daemon starts and finds a socket file at its path with no daemon listening on it
- **THEN** the daemon removes the stale file and binds a fresh socket instead of failing to start

#### Scenario: Socket is not reachable over the network
- **WHEN** the daemon creates its socket
- **THEN** the socket is a filesystem-local Unix domain socket with permissions restricting access to the owning user, not a network-exposed endpoint
