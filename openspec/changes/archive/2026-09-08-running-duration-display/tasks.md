## 1. Protocol

- [x] 1.1 Add `status_since_ms: u64` to `AgentInfo` in `crates/agentmon-proto/src/lib.rs` and update its sample/fixture construction in tests, verifying the crate builds and existing serialization tests pass
- [x] 1.2 Update the snake_case serialization test (or add one) to cover `status_since_ms`, verifying `cargo test -p agentmon-proto` passes

## 2. Daemon registry

- [x] 2.1 In `Registry::upsert` (`crates/agentd/src/registry.rs`), set `status_since_ms` to now on a new agent, carry it forward unchanged when `event.status == previous_status`, and reset it to now when the status differs, verifying with a unit test asserting a same-status upsert leaves `status_since_ms` unchanged while a transition updates it
- [x] 2.2 In `Registry::mark_stale`, reset `status_since_ms` to now, verifying with a unit test that marking an agent stale updates its `status_since_ms`
- [x] 2.3 Verify the "done" + late "needs input" drop path (already rejects the event) also leaves `status_since_ms` untouched, via a unit test alongside the existing dropped-event test in `registry.rs`

## 3. TUI display

- [x] 3.1 Add a duration-formatting function in `crates/agentmon/src/ui.rs` (e.g. `format_running_duration`) producing `9s` / `2m14s` / `1h03m` from `status_since_ms` and the current time, verifying with unit tests for each of the three ranges
- [x] 3.2 Render the duration next to/within the STATUS cell (or a new column) only when `agent.status == AgentStatus::Running`, verifying with a UI test asserting a running agent's row contains a duration string and a non-running agent's row does not
- [x] 3.3 Add an `AppEvent::Tick` variant in `crates/agentmon/src/main.rs`, spawn a thread sending it roughly once per second onto the shared channel, and redraw on receipt without mutating `app` state, verifying by manual run (`cargo run -p agentmon` against a running daemon) that a running agent's duration visibly counts up without new hook events arriving

## 4. Verification

- [x] 4.1 Run the full workspace test suite (`cargo test --workspace`) and confirm it passes
- [x] 4.2 Manually exercise the golden path end-to-end (start `agentd`, drive a Claude Code session through running → needs input → running again, watch `agentmon`) and confirm the duration resets on each transition back into "running" rather than accumulating across them
