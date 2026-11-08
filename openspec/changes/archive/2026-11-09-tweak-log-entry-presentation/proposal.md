## Why

An "agent started" log entry's inline prompt snippet clutters the Logs tab/pane with text the user usually doesn't need at a glance, while the one piece of context that actually distinguishes a "done" entry - its response - is currently rendered in the same color as the status label, making it harder to visually pick the agent's own words out of the row.

## What Changes

- An agent-category "started" log entry no longer shows its last-prompt snippet inline in the Logs tab or the details modal's Logs pane; the row shows only its status (and process id, in the modal pane).
- Pressing `Enter` on a selected "started" log entry in the details modal's Logs pane - when it carries a last-prompt snapshot - opens the existing full-text modal to show that prompt in full, the same mechanism already used for a "done" entry's response and for prompt-category entries. A "started" entry with no last-prompt snapshot has no effect on `Enter`, consistent with a "done" entry that carries no response.
- A "done" agent-category entry's quoted response snippet (in both the Logs tab and the details modal's Logs pane) renders in the same unstyled/default color already used for a prompt-category entry's own snippet and the Last Prompt pane, instead of inheriting the row's status color (green). The "🤖 done" label and duration keep the status color; only the quoted snippet's styling changes.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: changes the "Logs pane shows the process id for agent activities" scenario to exclude "started" from the inline-snippet entries (leaving "needs input" as the only inline-snippet status alongside process id); extends the "Prompt entry full-text modal" requirement so an agent-category "started" entry with a last-prompt snapshot also opens the full-text modal on `Enter`; and changes the color styling of a "done" entry's quoted response snippet in both the Logs tab and the details modal's Logs pane.

## Impact

- Affected code: `crates/agentmon/src/ui.rs` (row text/style construction for the Logs tab and the details modal's Logs pane), `crates/agentmon/src/input.rs` (the details modal's Logs pane `Enter` handling).
- No changes to `agentmon-proto`, `agentd`, or the `activity-log` capability - the daemon already records a last-prompt snapshot on every "started" entry; this change only alters how the TUI presents it.
