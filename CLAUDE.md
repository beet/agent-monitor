# agent-monitor

## Keeping the README's diagrams in sync

The README (`README.md`) has two Mermaid diagrams that describe this system's shape. Neither is generated - both must be hand-updated whenever the thing they describe changes, or they will drift the way the state diagram already had before it was corrected.

### Data model diagram ("Data model" section)

A `classDiagram` covering `agentmon-proto`'s wire types: `AgentEvent`, `AgentInfo`, `TestRunInfo`, `LogEntry`, `ClientMessage`, `ServerMessage`, and the enums that constrain their fields (`AgentStatus`, `TestRunStatus`, `HostContext`, `LogCategory`).

Update it whenever `crates/agentmon-proto/src/lib.rs` changes in a way that affects this diagram: a struct or enum gains, loses, or renames a field/variant; a new message type is added to `ClientMessage`/`ServerMessage`; or a type moves to a different crate. Keep the field types and enum variants in the diagram byte-for-byte matched to the current source.

### Agent-status state diagram ("How it works" section)

A `stateDiagram-v2` covering every `AgentStatus` transition the daemon actually implements, keyed to the `agent-daemon` spec (`openspec/specs/agent-daemon/spec.md`).

Update it whenever any of these change: the `AgentStatus` enum (`crates/agentmon-proto/src/lib.rs`); the hook-event-to-status mapping (`crates/agentmon-report/src/hook_payload.rs`); the transition-applying logic in the registry (`crates/agentd/src/registry.rs`); the liveness sweep (`crates/agentd/src/liveness.rs`); or the `agent-daemon` spec's status-transition requirements. Re-derive the transition list from the spec and implementation rather than editing the diagram by feel - a status that exists in the enum but has no arrows (like `Idle`, which no hook currently produces) should stay out of the diagram, with a note explaining why, rather than being silently added or silently left stale.
