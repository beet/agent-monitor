## Why

The TUI's "UPDATED" column resets on every hook event (including repeated `PreToolUse`/`PostToolUse` events during a single running turn), so it can't tell the user how long an agent has actually been running. A live duration lets the user spot agents that are stuck or taking unusually long at a glance.

## What Changes

- Daemon registry tracks a separate "status since" timestamp per agent that only updates when the agent's status actually transitions, not on every same-status event (e.g. each tool call during a running turn).
- `AgentInfo` gains a `status_since_ms` field carrying this timestamp to clients.
- TUI displays a live-updating duration (e.g. `2m14s`) for agents whose status is "running", computed from `status_since_ms`.
- TUI redraws on a periodic tick (independent of daemon events) so the running duration counts up in place instead of only refreshing when a new event arrives.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-daemon`: agent registry gains a status-since timestamp that only updates on status transitions, and reports it to clients.
- `agent-monitor-tui`: live agent list displays a running-duration for agents in "running" status, and the TUI refreshes on a timer so that duration counts up live.

## Impact

- `crates/agentmon-proto`: `AgentInfo` schema gains `status_since_ms`.
- `crates/agentd/src/registry.rs`: track and only update `status_since_ms` on real transitions.
- `crates/agentmon/src/ui.rs`: render duration for running agents.
- `crates/agentmon/src/main.rs`: add a periodic tick source feeding the main event loop so the display refreshes without a new daemon event.
