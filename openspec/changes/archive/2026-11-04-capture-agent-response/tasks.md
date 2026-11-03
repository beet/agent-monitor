## 1. Response capture (agentmon-report)

- [x] 1.1 Confirmed empirically (via `strings` on the installed Claude Code CLI binary) that the `Stop` hook payload carries `last_assistant_message: Option<String>` directly - documented in Claude Code's own schema as existing "to avoid the need to read and parse the transcript file." No transcript file access is needed; superseded the original transcript-parsing plan (see design.md's Decisions and Risks)
- [x] 1.2 Add `last_assistant_message: Option<String>` to `HookPayload` in `hook_payload.rs`, with a test that a `Stop` payload parses it and a payload without it parses with `None`
- [x] 1.3 Add a `sanitized_response`-style helper (or extend existing logic) that trims `last_assistant_message`, treats empty/whitespace-only as `None` (mirroring `sanitized_prompt`'s empty-after-trim handling), and truncates to 4,000 characters with a trailing truncation marker - verify with tests covering a normal response, a blank/whitespace-only field, and an overlong response
- [x] 1.4 Wire extraction into `build_event` (`report.rs`): only read `payload.last_assistant_message` when `payload.hook_event_name == "Stop"`; every other hook event sets `response: None` unconditionally even if `last_assistant_message` happens to be present - verify with tests mirroring the existing prompt-only-on-UserPromptSubmit tests

## 2. Wire protocol (agentmon-proto)

- [x] 2.1 Add `response: Option<String>` to `AgentEvent`, with a round-trip JSON test
- [x] 2.2 Add `response: Option<String>` to `LogEntry`, with a round-trip JSON test covering both `Some` and `None`

## 3. Daemon ingestion (agentd)

- [x] 3.1 In `ingest_event` (`ingest.rs`), set the "done" `LogEntry`'s `response` from the triggering `AgentEvent`'s `response` field (not from any carried-forward registry state) - verify with a test that a done event carrying a response produces a done log entry with that response, and a test that a done event carrying none produces a done log entry with `response: None`
- [x] 3.2 Verify a later event's response never alters an already-recorded entry's snapshot (mirrors the existing last-prompt non-mutation test) - add the analogous test for response

## 4. TUI (agentmon)

- [x] 4.1 In `handle_modal_key`'s `ModalFocus::Logs` `Enter` branch (`input.rs`), widen the existing prompt-category-only check so an agent-category entry with status `"done"` and a populated `response` also calls `app.open_prompt_detail(text)` with that response text - verify with a test pressing `Enter` on a done entry with a response and asserting `app.prompt_detail` opens with that text
- [x] 4.2 Verify `Enter` on a done entry with no response still does nothing, and `Enter` on a non-done, non-prompt entry still does nothing - extend the existing "non-prompt entry has no effect" test rather than replacing it
- [x] 4.3 Verify the top-level Logs tab's `Enter` is unaffected (still always opens the details modal) for a done entry carrying a response - extend the existing "regardless of category" test with a done+response case

## 5. Documentation

- [x] 5.1 Update README.md's `classDiagram` (Data model section) to add `AgentEvent.response` and `LogEntry.response`, per CLAUDE.md's diagram-sync instructions
- [x] 5.2 Confirm the README's `stateDiagram-v2` (How it works section) needs no change - this feature adds no new `AgentStatus` value or transition, only a text field riding along with the existing "done" transition

## 6. Full verification

- [x] 6.1 Run the full workspace test suite (`cargo test --workspace`) and confirm it passes
- [x] 6.2 Manually verify end-to-end with isolated `HOME` for every one of `agentd`, `agentmon-report`, and `agentmon` (not just the daemon): run a real Claude Code turn, confirm the resulting "done" log entry's response is readable via `Enter` in both the details modal's Logs pane and (after navigating there) with the top-level Logs tab's `Enter` still just opening the details modal as before. Verified the `agentd` + `agentmon-report` half live: started an isolated `agentd` under a throwaway `HOME`, piped a real `UserPromptSubmit` then `Stop` (carrying `last_assistant_message`) through the real `agentmon-report` binary, and read the daemon's socket `Snapshot` directly - the "done" entry's `response` came back exactly as sent, while "started"/"prompt" entries correctly carried `None`. The `agentmon` TUI half (pressing `Enter` in a live terminal) isn't something this environment can drive interactively; that behavior is covered by `input.rs`/`ui.rs`'s automated keypress-and-assert tests (tasks 4.1-4.3), which already pass - a human should still eyeball it once in a real terminal before relying on this being done.
