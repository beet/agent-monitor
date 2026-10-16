## Why

The Logs tab and the details modal's Logs pane record every prompt a user sends an agent, but only as a snippet piggybacked onto the "agent started"/"done"/"needs input" entry - there's no entry marking the moment a prompt was actually submitted, and a follow-up prompt mid-session leaves no trace at all. Separately, several of the emoji markers chosen early on (⏳ for "started", the per-status agent markers) no longer read as clearly as a dedicated "robot" marker would for agent activity, or a "test tube" marker would for test runs, and the macOS notification banners carry no at-a-glance visual cue of which kind of event just happened until the user reads the text.

## What Changes

- Add a new `prompt` log-entry category (emoji 🗣️, no color) recorded every time a `UserPromptSubmit` hook event is reported, independent of whether that submission also starts a new run. Its entry carries the submitted prompt's snippet, the same way an agent entry already does.
- When a prompt submission also begins a new run (the common case - a fresh session's first prompt), the new `prompt` entry SHALL sort immediately before the resulting `agent` "started" entry rather than appearing after it or interleaved arbitrarily.
- Change the agent "started", "done", and "needs input" log-entry/status emoji markers from ⏳/✅/🔔 to 🤖 (the existing text colors - Blue/Green/Yellow-bold respectively - are unchanged). This marker is shared code, so it also changes the Agents tab's live per-agent status badges and the details modal's Agents pane for "done" and "needs input" (idle/running/stale/declined markers are unaffected).
- Change the test-run "started" log-entry emoji marker from ⏳ to 🧪 (same Blue color). The live "running" badge shown in the Agents tab's TESTS column and the modal's Tests pane is unaffected (it is a separate, unchanged marker/status). "passed" and "failed" markers/colors are unchanged everywhere.
- Prefix each macOS notification's body text with the emoji matching its event, so the notification banner is identifiable at a glance before reading the text:
  - Agent finished: 🤖
  - Agent needs input: 🤖
  - Test run started: 🧪
  - Test run passed: ✅
  - Test run failed: ❌
  - Reminder completed: ⏰

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `activity-log`: adds the `prompt` log-entry category - what triggers it, what it carries, its ordering relative to a same-event "started" entry, and its inclusion in the existing bounded-retention/snapshot/live-update behavior.
- `agent-monitor-tui`: changes the emoji markers for agent "started"/"done"/"needs input" and test-run "started", and adds the 🗣️ marker (uncolored) for the new `prompt` log-entry category, across the Logs tab, the details modal's Logs/Agents panes, and the Agents tab's live status column.
- `agent-daemon`: prefixes the agent completion/needs-input notification body with 🤖 and the test-run lifecycle notification body with 🧪/✅/❌.
- `reminders`: prefixes the reminder completion notification body with ⏰.

## Impact

- `crates/agentmon-proto/src/lib.rs`: add `LogCategory::Prompt`.
- `crates/agentd/src/ingest.rs`: record a `prompt` log entry whenever `AgentEvent.prompt` is present, ordered before any "started" entry produced by the same event.
- `crates/agentmon/src/ui.rs`: emoji changes in `status_label_and_style`, `agent_started_text_and_style`, `test_run_started_text_and_style`; new rendering case for `LogCategory::Prompt` in `log_status_cell_text_and_style`/`log_category_label`/`log_entry_tree_prefix`/`log_entry_status_line`.
- `crates/agentd/src/notify.rs`: prepend emoji to the agent, test-run, and reminder notification body strings.
- No wire-format compatibility concerns beyond the additive `LogCategory` variant - `agentmon-proto` is shared by daemon and TUI built together.
