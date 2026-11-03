## Why

An agent's "started" and "done" log entries currently display a snapshot of the agent's last *submitted user prompt* - never anything the agent itself said. The user has to fish the transcript file out of `~/.claude/projects/` by hand to see what an agent actually did or reported before finishing. Claude Code's own `Stop` hook payload already carries the final assistant message's text directly (`last_assistant_message`, documented by Claude Code itself as existing "to avoid the need to read and parse the transcript file"), so the reporting client can surface it as-is and let a "done" entry show what happened, not just what was asked.

## What Changes

- `agentmon-report` reads the `last_assistant_message` field Claude Code's `Stop` hook payload already carries - no transcript file access needed; Claude Code has already resolved which message this is (including excluding sub-agent/Task-tool sidechain turns) on our behalf.
- `AgentEvent` gains a `response: Option<String>` field, populated only for `Stop`-derived events (mirroring how `prompt` is populated only for `UserPromptSubmit`-derived events); any other hook event, or a `Stop` event whose payload carries no `last_assistant_message`, reports `None` rather than failing the hook.
- `LogEntry` gains a `response: Option<String>` field, recorded on a "done" entry at the moment it's logged (not carried forward across later events the way last-prompt snapshots are - a response is only ever stamped once, when its "done" entry is created).
- The TUI's existing full-prompt modal (today reachable only from a prompt-category entry in the details modal's Logs pane) also opens for an agent-category "done" entry that carries a response, showing that response's full text. The top-level Logs tab's `Enter` behavior is unchanged - it continues to always open the project details modal, regardless of entry category, per the existing "Project details modal" requirement.
- Scope for this change: only the final assistant message's text is captured, not every message in the turn, and there is no persistent "last response" tracked on the agent the way `last_prompt` is - it exists only as a snapshot on the "done" log entry itself.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-daemon`: the reporting client's event-building behavior gains response-text capture for `Stop` events, sourced from the hook payload's own `last_assistant_message` field, and the wire event gains a `response` field.
- `activity-log`: a "done" entry additionally records a snapshot of the agent's final response text, when the reporting event carried one.
- `agent-monitor-tui`: the full-text modal requirement (currently scoped to prompt-category entries) also covers an agent-category "done" entry that carries a response.

## Impact

- `crates/agentmon-report/src/hook_payload.rs`: `HookPayload` gains `last_assistant_message: Option<String>`.
- `crates/agentmon-report/src/report.rs`: `build_event` sets `response` from `payload.last_assistant_message` only when `hook_event_name == "Stop"`, truncating to 4,000 characters with a trailing marker if needed.
- `crates/agentmon-proto/src/lib.rs`: `AgentEvent.response` and `LogEntry.response` fields, plus round-trip tests.
- `crates/agentd/src/ingest.rs`: `ingest_event` stamps a "done" `LogEntry`'s `response` from the triggering event.
- `crates/agentmon/src/app.rs` / `input.rs`: widen the existing full-prompt-modal `Enter` handling in the details modal's Logs pane to also match an agent-category "done" entry with a captured response.
- `crates/agentmon/src/ui.rs`: no new rendering code needed for the modal itself (reuses `PromptDetail`); the Logs tab/pane's inline snippet for a "done" entry is unaffected by this change.
