## 1. Hook mapping

- [x] 1.1 Add `"PreToolUse" => Some(AgentStatus::Running)` to `status_for_payload` in `crates/agentmon-report/src/hook_payload.rs`, and verify a `PreToolUse` payload test asserts `Some(AgentStatus::Running)`.
- [x] 1.2 Add a `PreToolUse` entry (no matcher) to `HOOK_EVENTS` in `crates/agentmon-report/src/install_hooks.rs`, and verify `creates_settings_file_with_hooks_when_none_exists` (or an updated version of it) asserts a `PreToolUse` hook group is written.

## 2. Registry behavior

- [x] 2.1 Add a registry test asserting a `PreToolUse`-derived `Running` event clears an existing "needs input" status for a session, mirroring the existing `running_event_still_clears_a_done_session` test in `crates/agentd/src/registry.rs`.
- [x] 2.2 Confirm (via test, no code change expected per design.md) that a `PreToolUse` event for a session already "running" leaves status unchanged and does not trigger a notification, by extending `crates/agentd/src/ingest.rs`'s existing coverage for same-status transitions.

## 3. Documentation

- [x] 3.1 Add a "How it works" section to `README.md` with a Mermaid `stateDiagram-v2` covering all five statuses (`Running`, `NeedsInput`, `Done`, `Idle`, `Stale`) and the hooks that drive each transition (`UserPromptSubmit`, `PreToolUse`, `Stop`, `Notification`, and the liveness sweep's stale detection), including the new `NeedsInput -> Running` edge via `PreToolUse` and the existing `Done -> NeedsInput` guard (drawn as a self-loop/ignored-event note, since that event is dropped rather than applied). Verify by rendering the block and checking it matches the transitions in `crates/agentd/src/registry.rs` and `crates/agentmon-report/src/hook_payload.rs`.

## 4. Verification

- [x] 4.1 Run `cargo test --workspace` and verify all tests pass, including the new `PreToolUse` coverage in `agentmon-report` and `agentd`.
- [x] 4.2 Manually verify end-to-end: install hooks with the updated `agentmon-report install-hooks`, trigger a permission-prompt "needs input" state in a real Claude Code session, approve it, and confirm agentmon shows "running" again before the turn's `Stop`.
