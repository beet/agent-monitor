## Context

See proposal.md - Why for the root cause. Concretely: `Registry::upsert` (`crates/agentd/src/registry.rs:119-196`) already computes a `retiring: Vec<SessionId>` list (lines 150-159) when a new session id takes over a pid, and removes those entries from its own map, but `UpsertOutcome` (returned to callers) only carries the *new* agent - the retired ids are discarded on the spot. `Ingestor::ingest_event` (`crates/agentd/src/ingest.rs:61-96`) and `server::handle_connection` (`crates/agentd/src/server.rs:83-85`) only ever publish `Update::Agent(agent)`. Every other subscribed client's `App.agents` (`crates/agentmon/src/app.rs`) therefore keeps the superseded session's entry forever, since `apply_update` dedups by `session_id`, not pid.

This touches the wire protocol (`agentmon-proto`), the daemon's registry/ingest/server layers, and the TUI client - a small cross-cutting change, hence this design doc.

## Goals / Non-Goals

**Goals:**
- Propagate a pid's session retirement from the registry, through the daemon's broadcast layer, to every already-connected client.
- Make the fix symmetric with how `Update::Agent`/`AgentUpdate` already flows, reusing the same broadcaster/subscriber plumbing rather than adding a parallel channel.

**Non-Goals:**
- Deduplicating client-side by pid as a defense in depth. The registry already guarantees at most one live entry per pid; the gap is purely that removals aren't propagated. Adding client-side pid dedup on top would mask future protocol bugs instead of surfacing them, so it's left out.
- Changing how the liveness sweep marks agents stale - that path already publishes `Update::Agent` for the existing session id (it doesn't create a new one), so it needs no removal message.
- Persisting removal history across a client reconnect - a fresh `Subscribe` always gets `Registry::snapshot()`, which by construction never contains a retired session, so there's nothing to reconcile on reconnect.

## Decisions

**Add a new `ServerMessage::AgentRemoved { session_id }` variant rather than overloading `AgentUpdate`.** An update needs a full `AgentInfo`; a removal only needs the session id being dropped. Reusing `AgentUpdate` would force a sentinel status or an `Option<AgentInfo>`, both murkier than a dedicated variant. Mirrors how `LogAppended` and `TestRunUpdate` are already separate, purpose-specific messages.

**Carry retired session ids on `UpsertOutcome` instead of having `Registry` publish directly.** `Registry` has no reference to the `Broadcaster` (server.rs owns it) and shouldn't gain one - it stays a pure state machine, consistent with how it already returns the new agent for the caller to publish rather than publishing itself. `UpsertOutcome` gains `retired_session_ids: Vec<SessionId>` (defaults to empty for every other path).

**`Ingestor::ingest_event` returns `(AgentInfo, Vec<SessionId>)` instead of a new named struct.** The only two callers are `server.rs`'s two call sites (both already destructure the outcome informally); a tuple keeps the diff small. If a third field is ever needed here, revisit.

**Server publishes the removal(s) after the new agent's update, same as it already orders `record_log` before notification inside `ingest_event`.** Order doesn't matter for correctness (the client's `apply_update`/removal handler are independent of each other and idempotent), but publishing the replacement first then the removal reads naturally as "here's what's live now, and here's what's gone."

**Client removal handler matches on `session_id` only, ignoring pid.** The daemon is the sole source of truth for which session ids are retired; the client doesn't need to re-derive or double check pid identity.

## Risks / Trade-offs

- [Adding a `ServerMessage` variant is a wire-format change] → Both `agentd` and `agentmon` ship from this same repo and are always upgraded together (see agent-daemon spec's "Runs as a macOS background service" - installed via the same Homebrew formula), so there's no cross-version compatibility window to design around.
- [A removal could theoretically race a slow client's in-flight `AgentUpdate` for the same session id if ordering were ever handled over multiple connections] → Not applicable here: `Broadcaster::publish` fans out over one `mpsc::Sender` per subscriber, which preserves send order, and both messages go to the same per-client channel.

## Migration Plan

No data migration. This is a protocol/behavior fix within a single deployable pair (`agentd` + `agentmon`); rolling out the new daemon binary and TUI binary together (as today's release process already does) is sufficient. No feature flag needed - the old duplicate-row behavior was never intentional.
