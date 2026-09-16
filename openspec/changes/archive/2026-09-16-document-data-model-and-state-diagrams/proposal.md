## Why

The README documents `agentmon-proto`'s wire protocol only in prose, with no diagram of the data model (`AgentInfo`, `TestRunInfo`, `LogEntry`, and the enums/messages that connect them), making it harder to see the shape of the system at a glance. The README's existing agent-status `stateDiagram-v2` has also drifted from the daemon's actual behavior: the `agent-daemon` spec defines a `declined` status (entered via a `PermissionDenied` hook event, exiting back to `running`/`done` like any other status) that the diagram never mentions. Nothing in the repo currently obliges either diagram to be kept current as the code changes.

## What Changes

- Add a Mermaid ER diagram to the README documenting the daemon's data model: `AgentInfo`, `TestRunInfo`, `LogEntry`, and the `ClientMessage`/`ServerMessage` protocol envelopes from `agentmon-proto`, including the enums that constrain their fields (`AgentStatus`, `TestRunStatus`, `HostContext`, `LogCategory`).
- Update the README's existing agent-status `stateDiagram-v2` to add the missing `Declined` status and its transitions (any status → `Declined` on `PermissionDenied`, unconditionally; `Declined` → `Running` on `UserPromptSubmit`/`PreToolUse`/`PostToolUse`, → `Done` on `Stop`, → `Stale` via the liveness sweep), matching the `agent-daemon` spec and the daemon's implementation.
- Create a root `CLAUDE.md` with standing instructions to keep both diagrams in sync: update the schema diagram whenever `agentmon-proto`'s structs/enums change, and update the state diagram whenever agent-status hook handling or the status enum changes.
- No behavior, code, or spec changes - documentation only.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
(none - this change touches only README.md and CLAUDE.md, not spec-level behavior)

## Impact

- `README.md`: new ER diagram section; corrected/updated state diagram.
- `CLAUDE.md` (new file at repo root): maintenance instructions for both diagrams.
- No code, protocol, or spec changes.
