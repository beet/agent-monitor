## Why

The details modal's Agents pane shows two rows for the same pid, each with a different duration. `Registry::upsert` already retires an old session's entry when a new session id takes over its pid (e.g. `/clear`), but that retirement is never broadcast to already-connected clients - only the new agent is published (`crates/agentd/src/server.rs:84`, `Update::Agent(agent)`). A long-running `agentmon` client therefore keeps the old, now-frozen `AgentInfo` in `App.agents` forever alongside the live one, since its own dedup is keyed by `session_id` (`crates/agentmon/src/app.rs:120-127`), not pid. Both the top-level Agents tab's grouped cell and the details modal's per-agent Agents pane render from that same list, so the stale entry surfaces as a second line with its own independently-computed duration.

## What Changes

- The daemon's server layer broadcasts a removal for every session id that `Registry::upsert` retires when a pid changes session id, not just the replacement agent's update.
- `Ingestor::ingest_event` exposes the retired session ids alongside the resulting agent so the server can broadcast them.
- The wire protocol gains a message for removing a specific tracked agent by session id, sent to already-connected clients.
- `agentmon`'s client applies that removal by dropping the named session id from its local agent list, so a superseded session's stale row disappears without requiring a TUI restart or reconnect.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: the "Agent list query and live updates" requirement gains a scenario where a same-pid session replacement also pushes a removal for the superseded session id to connected clients.
- `agent-monitor-tui`: the "Live agent list" requirement gains a scenario where the TUI drops a removed session id from its tracked agents, so a pid never renders more than one row (in the Agents tab's grouped cell or the details modal's Agents pane) once its old session has been superseded.

## Impact

- `crates/agentmon-proto/src/lib.rs`: `ServerMessage` gains a new variant for removing a tracked agent by session id.
- `crates/agentd/src/registry.rs`: `UpsertOutcome` exposes the session ids retired during this upsert (the `retiring` list already computed in `upsert`, currently discarded).
- `crates/agentd/src/ingest.rs`: `Ingestor::ingest_event` returns the retired session ids alongside the agent.
- `crates/agentd/src/server.rs`: broadcasts the new removal message for each retired session id, in addition to the existing `Update::Agent` publish.
- `crates/agentmon/src/client.rs`: dispatches the new server message to the app.
- `crates/agentmon/src/app.rs`: `App` gains a method to remove an agent by session id, called on receipt of the new message.
