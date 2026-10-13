## Why

When several agents are in flight across different projects, agent-monitor already tells the user which one just finished or needs input - but not what it was doing. The user has to context-switch back to each terminal/editor to remember what they asked. The `UserPromptSubmit` hook already carries the prompt text to `agentmon-report`, which currently parses and discards it (`crates/agentmon-report/src/hook_payload.rs`). Capturing and surfacing it closes that gap with no new hook registration needed.

## What Changes

- The `UserPromptSubmit` hook payload's `prompt` field is captured and forwarded to the daemon as part of its existing `AgentEvent` report (still sent only for `UserPromptSubmit`; other hook events continue to carry no prompt text).
- The daemon's agent registry tracks each agent's most recent prompt text, carrying it forward across subsequent events (`PreToolUse`, `PostToolUse`, `Stop`, etc.) that don't themselves carry a new prompt, and sends it to clients as part of `AgentInfo`.
- A new `started`/`done` agent-category activity log entry additionally records the agent's last prompt as of that event (frozen at record time, like `LogEntry`'s existing `reminder_name`), and the Logs tab / details modal's Logs pane render a truncated one-line snippet of it alongside those entries.
- The details modal gains a new "Last Prompt" pane, a thin fixed-height strip above the Logs pane (space taken from the Logs pane's existing two-thirds share), showing the full last-prompt text of whichever tracked agent in that project most recently had a status update. A project with no agent that has a captured prompt yet shows a placeholder instead of the pane being empty.
- Captured prompt text is capped at a fixed length (truncated with a trailing marker) everywhere it's stored, to bound daemon memory and activity-log entry size against arbitrarily large pasted prompts.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: the agent registry and `AgentEvent`/`AgentInfo` wire shape gain last-prompt tracking and transmission.
- `activity-log`: agent-category log entries gain a recorded prompt snapshot.
- `agent-monitor-tui`: the Logs tab/pane render a prompt snippet on agent entries, and the details modal gains a "Last Prompt" pane with adjusted pane proportions.

## Impact

- `crates/agentmon-proto/src/lib.rs`: `AgentEvent` gains `prompt: Option<String>`; `AgentInfo` gains `last_prompt: Option<String>`; `LogEntry` gains `prompt: Option<String>`. Per `CLAUDE.md`, the README's data-model Mermaid diagram must be updated to match.
- `crates/agentmon-report/src/hook_payload.rs`: `HookPayload` gains an optional `prompt` field, populated only for `UserPromptSubmit`.
- `crates/agentd/src/registry.rs`: `Registry::upsert` carries forward `last_prompt` when an event doesn't supply a new one; activity-log recording (wherever agent `started`/`done` entries are built) captures the agent's current `last_prompt`.
- `crates/agentmon` (TUI): Logs tab/pane rendering, and the details modal's layout and new Last Prompt pane.
- No wire/schema version negotiation exists in this project, so this is a coordinated client+daemon upgrade like any other proto change here.
