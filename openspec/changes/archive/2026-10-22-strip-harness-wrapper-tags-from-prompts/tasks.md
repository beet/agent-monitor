## 1. Wrapper-tag stripping in agentmon-report

- [x] 1.1 Add a function (e.g. `strip_harness_wrapper_tags`) in `crates/agentmon-report/src/hook_payload.rs` that scans a prompt string for the recognized tags (`command-message`, `system-reminder`, `task-notification`, `local-command-stdout` removed with contents; `command-name`, `command-args` unwrapped, contents kept), handling same-tag nesting and leaving unmatched/unterminated opening tags untouched, and verify with unit tests covering: a pure `<task-notification>` payload, a `<system-reminder>` followed by real text, a `<command-message>`/`<command-name>`/`<command-args>` slash-command payload, nested tags inside `<task-notification>`, a payload with no recognized tags, and an unterminated opening tag
- [x] 1.2 Trim the result and treat an empty (or whitespace-only) result as "no prompt," and verify with a unit test that a fully-wrapper payload collapses to `None`
- [x] 1.3 Wire this function into the `UserPromptSubmit` branch of `build_event` in `crates/agentmon-report/src/report.rs` (where `payload.prompt.clone()` currently becomes the event's prompt), applying it only to that branch, and verify the existing `build_event_carries_the_prompt_for_user_prompt_submit` test still passes unchanged (plain prompt, no tags) alongside a new test asserting a wrapped payload comes out stripped

## 2. Spec alignment

- [x] 2.1 Run `openspec validate --change strip-harness-wrapper-tags-from-prompts --strict` and resolve any reported issues

## 3. Verification

- [x] 3.1 Run the full workspace test suite (`cargo test --workspace`) and confirm it passes
- [x] 3.2 Manually exercise the fix per `[[agentmon-manual-testing]]`: with an isolated `HOME`, ran `agentd --foreground` and `agentmon-report` directly, sent a `UserPromptSubmit` payload whose `prompt` was wrapped entirely in `<task-notification>` tags, and confirmed (via a `Subscribe`d client reading the daemon's snapshot - the same data the TUI renders) that the agent's `last_prompt` was `None` rather than raw tag text; then sent one wrapped in `<command-message>`/`<command-name>`/`<command-args>` and confirmed `last_prompt` was `Some("/opsx:propose\nAgent prompts appearing as raw XML")` - the command name and arguments, unwrapped
