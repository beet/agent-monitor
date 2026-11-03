## Why

A "done" agent-category log entry's inline row (in both the top-level Logs tab and the details modal's Logs pane) still shows a snippet of the *user's last submitted prompt*, even though the entry now carries a snapshot of the *agent's final response* text (added by the "capture-agent-response" change, archived 2026-11-04). That change only wired the response into the full-text modal reached via `Enter`; the row's own always-visible snippet - what you see without pressing anything - was left showing the prompt, which is backwards for an entry whose whole point is reporting what finished. The user should see what the agent said it did, not what they originally asked.

## What Changes

- A "done" agent-category entry's inline snippet (in the Logs tab and the details modal's Logs pane) shows a truncated snippet of its recorded **response** text, when one was captured, instead of its last-prompt snapshot.
- A "done" entry recorded with no captured response (e.g. the triggering turn ended with only tool calls and no trailing text, or an older daemon never captured one) keeps showing its last-prompt snippet, unchanged from today - there is no substitute response text to show in that case, and falling back avoids a done entry regressing to showing nothing.
- "started" and "needs input" agent-category entries are unaffected - they have no response field at all and continue to show their last-prompt snippet exactly as today.
- The full-text modal reached via `Enter` on a "done" entry (added by the prior change) is unaffected - it already shows the response's full text and already has no effect when none was captured.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: the Logs tab's and details modal's Logs pane's inline-snippet rendering for a "done" agent-category entry now prefers its captured response text over its last-prompt snapshot.

## Impact

- `crates/agentmon/src/ui.rs`: `log_entry_status_line` - for an agent-category entry with status "done", show a snippet of `entry.response` when `Some`, falling back to `entry.prompt` when `None`; "started"/"needs input" entries keep using `entry.prompt` as today.
- `openspec/specs/agent-monitor-tui/spec.md`: update the "Logs tab shows an aggregated, paginated activity list" and "Project details modal" requirements (and their scenarios) to describe the done-entry snippet's response-first, prompt-fallback behavior.
- No wire protocol or daemon changes - `LogEntry.response` already exists and is already populated; this is purely a TUI rendering fix.
