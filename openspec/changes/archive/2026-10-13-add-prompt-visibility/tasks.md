## 1. Wire protocol

- [x] 1.1 Add `prompt: Option<String>` to `agentmon_proto::AgentEvent` and `last_prompt: Option<String>` to `agentmon_proto::AgentInfo` in `crates/agentmon-proto/src/lib.rs`, and verify the crate's existing round-trip tests (`agent_event_round_trips_through_json`, `agent_info_round_trips_through_json`) still pass with the new fields `None` by default
- [x] 1.2 Add `prompt: Option<String>` to `agentmon_proto::LogEntry`, following the existing `reminder_name`/`reminder_due_at_ms` pattern, and verify `server_message_snapshot_round_trips_through_json` still passes
- [x] 1.3 Update the README's data-model Mermaid `classDiagram` to add the three new fields to `AgentEvent`, `AgentInfo`, and `LogEntry`, per `CLAUDE.md`'s sync requirement

## 2. Hook capture

- [x] 2.1 Add `prompt: Option<String>` to `HookPayload` in `crates/agentmon-report/src/hook_payload.rs` and a test asserting a `UserPromptSubmit` payload's `prompt` field parses correctly
- [x] 2.2 At the call site that builds `AgentEvent` from a parsed `HookPayload` (wherever `agentmon-report`'s binary/library currently constructs it), set `AgentEvent.prompt` to the payload's `prompt` only when `hook_event_name == "UserPromptSubmit"`, `None` otherwise, and add a test covering both cases

## 3. Registry

- [x] 3.1 Add a `MAX_LAST_PROMPT_CHARS: usize = 4_000` constant and a truncation helper in `crates/agentd/src/registry.rs` that truncates on a `char` boundary and appends a trailing marker, with a unit test for an overlong input
- [x] 3.2 In `Registry::upsert`, set the new entry's `last_prompt` from `event.prompt` (truncated) when `Some`, else carry forward `previous.last_prompt` (or `None` for a newly created or same-pid-replacement entry), with unit tests for: prompt-bearing event sets it, non-prompt-bearing event on an existing agent carries it forward unchanged, and a same-pid replacement starts with `None` even when the replaced entry had a value
- [x] 3.3 Thread `last_prompt` through wherever the registry/ingestor builds a `LogEntry` for an agent "started"/"done"/"needs input" event, snapshotting the agent's *current* `last_prompt` at record time (not the triggering event's own `prompt`), with a test asserting a later prompt doesn't retroactively change an already-recorded entry

## 4. TUI: Logs tab and details modal's Logs pane

- [x] 4.1 Render a truncated, single-line prompt snippet (trailing `...` if it doesn't fit the row) after an agent-category entry's existing status/duration text in the Logs tab, when that entry's `prompt` is `Some`; no snippet when `None` - verify by running the TUI against a daemon reporting a `UserPromptSubmit`-derived event and confirming the Logs tab shows it
- [x] 4.2 Apply the same rendering to the details modal's Logs pane (shared rendering code path if one already exists between the tab and the pane)

## 5. TUI: details modal Last Prompt pane

- [x] 5.1 Add a `LastPrompt` pane to the details modal's layout: a fixed-height strip (e.g. 4 rows) spanning the modal's full width, inserted between the existing top-third row (Agents/Tests/Reminders) and the Logs pane, with the Logs pane's area reduced by that same height; verify the modal still renders without panicking at a small terminal size
- [x] 5.2 Compute the pane's content as the `last_prompt` of the project's tracked agent with the greatest `last_updated_ms`; show a placeholder string (e.g. "No prompt captured yet") when the project has no tracked agents or that agent's `last_prompt` is `None`
- [x] 5.3 Wrap the prompt text to the pane's width and truncate with a trailing `...` if it exceeds the pane's fixed height, with no scrolling/selection behavior for this pane
- [x] 5.4 Confirm the pane is excluded from the `Tab` focus cycle (only Logs/Reminders panes toggle) and from the Agents/Tests-style non-interactive treatment - add/extend a test alongside the modal's existing focus-cycling tests if one exists
- [x] 5.5 Update any existing modal snapshot/layout tests that assert exact pane proportions or the "four panes" count to reflect the new fifth pane

## 6. Verification

- [x] 6.1 Run the full workspace test suite (`cargo test --workspace`) and confirm it's green
- [x] 6.2 Manually verify end-to-end against a live `agentd` + `agentmon` + a real `UserPromptSubmit` hook invocation (isolate `HOME` per the project's manual-testing convention): submit a prompt, confirm it appears in the Logs tab's "started" entry, the details modal's Last Prompt pane, and persists through subsequent tool-use events without being cleared
- [x] 6.3 Manually verify the multi-agent case: two agents in the same project, confirm the Last Prompt pane follows whichever one most recently updated
