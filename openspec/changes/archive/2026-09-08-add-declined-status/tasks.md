## 1. Protocol

- [x] 1.1 Add a `Declined` variant to `AgentStatus` in `crates/agentmon-proto/src/lib.rs` (snake_case serialization via the existing `#[serde(rename_all = "snake_case")]`), verifying with a test that it serializes as `"declined"`

## 2. Hook mapping

- [x] 2.1 Map `hook_event_name == "PermissionDenied"` to `Some(AgentStatus::Declined)` in `status_for_payload` (`crates/agentmon-report/src/hook_payload.rs`), verifying with a unit test parsing a `PermissionDenied` payload
- [x] 2.2 Register `"PermissionDenied"` in `HOOK_EVENTS` (`crates/agentmon-report/src/install_hooks.rs`) so Claude Code actually invokes the reporter for it - discovered during implementation, not in the original task list: without this the daemon-side mapping never receives an event to map. Verified via the existing generic `install_hooks` tests (which iterate `HOOK_EVENTS`), no test changes needed.
- [x] 2.3 Add a unit test in `crates/agentd/src/registry.rs` confirming a session in "declined" status resumes normally on a subsequent `PreToolUse`/`PostToolUse` (to "running") or `Stop` (to "done") event, using the existing generic transition logic

## 3. Notifications

- [x] 3.1 Add a unit test in `crates/agentd/src/notify.rs` confirming no notification script is produced for a "declined" status (alongside the existing `notification_script_is_none_for_non_attention_states` case)

## 4. TUI display

- [x] 4.1 Add a `Declined` arm to `status_label_and_style` in `crates/agentmon/src/ui.rs` using the 🚫 emoji and a red, non-bold style, verifying with a unit test that a declined agent's row renders the 🚫 marker and is visually distinguished (differing style) from other statuses
- [x] 4.2 Confirm (via existing generic logic, add a test if not already covered) that a "declined" agent's row shows no running-duration text, consistent with other non-running statuses

## 5. Verification

- [x] 5.1 Run the full workspace test suite (`cargo test --workspace`) and confirm it passes
- [x] 5.2 Manually verify end-to-end using an isolated `HOME`-scoped daemon+client pair (per the established safe-testing pattern - never against the real production socket): send a synthetic `PermissionDenied` event and confirm the TUI shows "declined" with the 🚫 marker, then send a `PreToolUse` event and confirm it moves back to "running"
