## 1. Data model diagram

- [x] 1.1 Add a Mermaid `erDiagram` (or `classDiagram`, whichever renders the relationships more clearly) to README.md covering `AgentInfo`, `TestRunInfo`, and `LogEntry` from `crates/agentmon-proto/src/lib.rs`, including their key fields and the enums that constrain them (`AgentStatus`, `TestRunStatus`, `HostContext`, `LogCategory`); verify every field name and type in the diagram matches the current struct/enum definitions in `agentmon-proto/src/lib.rs`
- [x] 1.2 Include the `ClientMessage`/`ServerMessage` envelope variants in the diagram (or an adjoining note) so the diagram also conveys how these types travel over the socket, not just their shape at rest; verify each variant listed matches `agentmon-proto/src/lib.rs`
- [x] 1.3 Render the diagram (e.g. via a local Mermaid preview or GitHub's README preview) and verify it displays without syntax errors

## 2. State diagram correction

- [x] 2.1 Re-derive the full set of `AgentStatus` transitions from the `agent-daemon` spec (`openspec/specs/agent-daemon/spec.md`) and the daemon's implementation (`crates/agentd/src/registry.rs`'s `upsert`, `crates/agentmon-report/src/hook_payload.rs`'s `status_for_payload`, and the liveness sweep in `crates/agentd/src/liveness.rs`), including that `PermissionDenied` moves *any* prior status to `Declined` unconditionally (no guard), and `Declined` resumes to `Running` on `UserPromptSubmit`/`PreToolUse`/`PostToolUse`, to `Done` on `Stop`, and to `Stale` via the liveness sweep like any other non-stale status; verify the new transition list has no gaps against the spec's "Declined permission status" and "Resuming from needs input" requirements
- [x] 2.2 Update the existing `stateDiagram-v2` in README.md to add `Declined` and its transitions, keeping the existing note about `Idle` being defined but unproduced; verify the diagram still renders and every transition shown corresponds to a requirement/scenario in the `agent-daemon` spec
- [x] 2.3 Re-check the rest of the existing diagram's transitions (`Running`, `NeedsInput`, `Done`, `Stale`) line-by-line against the current spec and implementation, and correct any other drift found; verify by cross-referencing each arrow to a spec scenario

## 3. Maintenance instructions

- [x] 3.1 Create `CLAUDE.md` at the repo root with a section instructing future changes to update the README's data-model diagram whenever `agentmon-proto`'s structs or enums change; verify the file exists and reads clearly on its own
- [x] 3.2 Add a section to the same `CLAUDE.md` instructing future changes to update the README's state diagram whenever agent-status hook handling, the `AgentStatus` enum, or the `agent-daemon` spec's status-transition requirements change; verify the instruction names the README section and the `agent-daemon` spec as the two things to keep in sync
- [x] 3.3 Verify `CLAUDE.md` sits at the repo root (alongside `Cargo.toml`) so Claude Code loads it automatically for future sessions in this repo
