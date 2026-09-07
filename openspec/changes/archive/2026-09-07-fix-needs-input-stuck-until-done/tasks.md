## 1. Hook mapping

- [x] 1.1 Add `"PostToolUse" => Some(AgentStatus::Running)` to `status_for_payload` in `crates/agentmon-report/src/hook_payload.rs`, and verify a `PostToolUse` payload test asserts `Some(AgentStatus::Running)`.
- [x] 1.2 Add a `PostToolUse` entry (no matcher) to `HOOK_EVENTS` in `crates/agentmon-report/src/install_hooks.rs`, and verify `creates_settings_file_with_hooks_when_none_exists` (or an updated version of it) asserts a `PostToolUse` hook group is written.

## 2. Registry behavior

- [x] 2.1 Add a registry test asserting a `PostToolUse`-derived `Running` event clears an existing "needs input" status for a session, mirroring the existing `running_event_clears_a_needs_input_session` test in `crates/agentd/src/registry.rs`.
- [x] 2.2 Confirm (via test, no code change expected per design.md) that a `PostToolUse` event for a session already "running" leaves status unchanged and does not trigger a notification, extending `crates/agentd/src/ingest.rs`'s existing coverage for same-status transitions.

## 3. Documentation

- [x] 3.1 Update the `stateDiagram-v2` block in `README.md` so the `NeedsInput -> Running` and `Done -> Running` edges list `PostToolUse` alongside `PreToolUse` / `UserPromptSubmit`. Verify by checking the block matches the transitions in `crates/agentd/src/registry.rs` and `crates/agentmon-report/src/hook_payload.rs`.

## 4. Verification

- [x] 4.1 Run `cargo test --workspace` and verify all tests pass, including the new `PostToolUse` coverage in `agentmon-report` and `agentd`.
- [x] 4.2 Manually verify end-to-end: install hooks with the updated `agentmon-report install-hooks`, trigger a permission-prompt "needs input" state in a real Claude Code session on a tool call that itself raises the prompt, approve it, and confirm agentmon shows "running" again as soon as that tool call finishes - before any further tool call or the turn's `Stop`.
