## Why

Claude Code sessions run scattered across an nvim terminal and standalone terminal windows, with no single place to see which agents are active, idle, waiting on input, or finished. Long-running agent tasks currently require manually checking back on each terminal/window, and there's no notification when a task completes in the background. A local daemon + TUI gives a unified view and timely completion alerts across these hosts.

(The desktop app was originally in scope too, but was descoped during end-to-end verification: it executes sessions in a sandboxed environment that can't run the local shell-command hooks this change relies on - see design.md's "Desktop app sessions are unreachable" risk. It also already has its own built-in notifications.)

## What Changes

- New background daemon (`agentd`) that:
  - Listens on a local Unix domain socket for status events.
  - Receives status updates from Claude Code hooks (`Notification`, `Stop`, `SubagentStop`, etc.) configured to call into the daemon on each tracked session.
  - Maintains an in-memory registry of known agents (session id, working directory, host context, PID, current status, last-updated timestamp).
  - Fires a macOS user notification (via `osascript`/`UserNotifications`) when an agent's status transitions to "done" (or "needs input").
  - Can be installed and run as a macOS `launchd` user service (`launchctl load`/`unload`), and also runnable in the foreground for development.
- New Rust TUI (`agentmon`) that:
  - Connects to the daemon's Unix socket and renders a live-updating list of agents: working directory / project, host context (nvim / terminal / desktop), PID, and status.
  - Runs as a normal foreground terminal application; does not itself track agents (the daemon is the source of truth).
- Installation/config tooling: a way to install the Claude Code hook configuration (`settings.json` hooks) that point at the daemon socket, and a launchd `.plist` template for the daemon.

## Capabilities

### New Capabilities

- `agent-daemon`: Background service that ingests agent status events (via Claude Code hooks) over a local socket, tracks agent state, exposes it to clients, sends macOS notifications on completion, and can run as a macOS launchd service.
- `agent-monitor-tui`: Rust terminal UI that connects to the daemon and displays the live list of tracked agents and their statuses.

### Modified Capabilities

(none - greenfield project, no existing specs)

## Impact

- New Rust workspace with at least two binaries: `agentd` (daemon) and `agentmon` (TUI), likely sharing a common crate for the IPC protocol and agent/status data model.
- New local Unix domain socket (e.g. under `~/Library/Application Support/agentmon/agentd.sock`).
- New launchd `.plist` for running the daemon as a user service (`~/Library/LaunchAgents`).
- Requires configuring Claude Code hooks (in user or project `settings.json`) to call the daemon on relevant lifecycle events.
- New dependency on macOS notification APIs (user-facing permission prompt likely required on first notification).
