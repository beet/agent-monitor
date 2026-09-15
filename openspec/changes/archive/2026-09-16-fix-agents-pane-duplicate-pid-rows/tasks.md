## 1. Protocol

- [x] 1.1 Add `ServerMessage::AgentRemoved { session_id: SessionId }` to `crates/agentmon-proto/src/lib.rs` and verify existing serde round-trip tests for `ServerMessage` still pass alongside a new one covering the added variant

## 2. Daemon registry and ingest

- [x] 2.1 Add `retired_session_ids: Vec<SessionId>` to `UpsertOutcome` in `crates/agentd/src/registry.rs`, populated from the existing `retiring` list computed in `Registry::upsert`, and verify `registry.rs`'s existing "a new session id for a tracked pid replaces the old entry" test still passes plus a new assertion that the outcome reports the replaced session id
- [x] 2.2 Change `Ingestor::ingest_event` in `crates/agentd/src/ingest.rs` to return `(AgentInfo, Vec<SessionId>)`, threading `outcome.retired_session_ids` through the early `stale_event_ignored` return (empty vec) and the normal path, and verify `ingest.rs`'s existing tests compile against the new return shape

## 3. Daemon broadcast

- [x] 3.1 Add an `AgentRemoved(SessionId)` variant to the internal `Update` enum in `crates/agentd/src/server.rs`, map it to `ServerMessage::AgentRemoved` in the subscriber loop's match, and publish one `Update::AgentRemoved` per retired session id returned by `ingest_event` at both call sites in `handle_connection`, verified by a new server test asserting a subscribed client receives an `AgentRemoved` message when a same-pid session replacement occurs

## 4. TUI client

- [x] 4.1 Add a branch for `ServerMessage::AgentRemoved { session_id }` in `crates/agentmon/src/client.rs`'s message loop that calls a new `App` method, verified by a client test asserting the branch is reached
- [x] 4.2 Add `App::remove_agent(&mut self, session_id: &SessionId)` in `crates/agentmon/src/app.rs` that drops the matching entry from `self.agents` and re-clamps selection, verified by a new `app.rs` unit test asserting the agent list no longer contains that session id afterward

## 5. Integration verification

- [x] 5.1 Add an end-to-end test (daemon + client, alongside the existing session-replacement coverage in `registry.rs`/`server.rs`) simulating a `/clear`-style same-pid session change while a client is connected, asserting the client ends up with exactly one agent entry for that pid and that the details modal's Agents pane (`crates/agentmon/src/ui.rs`) renders exactly one line for it - covering the `agent-daemon` and `agent-monitor-tui` delta scenarios added in this change
- [x] 5.2 Run the full workspace test suite (`cargo test --workspace`) and confirm it passes
