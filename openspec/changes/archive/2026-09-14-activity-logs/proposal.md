## Why

When several projects are active at once, a notification arrives, the user can't context-switch immediately, and by the time they return the notification is gone and the current agent list only reflects present state — not what happened while they were away. There is no record of recent completions, needs-input prompts, or test-run results to catch up on.

## What Changes

- Daemon keeps a bounded, in-memory activity log of notification-worthy events (agent done, agent needs-input, test-run started/passed/failed), capped by a global count so memory stays bounded without a time-based sweep. Oldest entries are evicted once the cap is reached. Declined and stale transitions are not logged, matching what already does/doesn't notify today.
- Daemon exposes the current log to connected clients (initial snapshot) and pushes new entries as they occur, alongside the existing agent/test-run snapshot and update messages.
- TUI gains two tabs: **Agents** (today's project table, now explicitly named as a tab) and **Logs** (a paginated, most-recent-first list of activity entries aggregated across all projects), switchable via `Tab`, or directly via `A`/`L`.
- TUI's Logs tab supports sorting (default: recency; also Project, Status) and filtering (by Project, by Status), with keyboard-driven pagination (`j`/`down`, `k`/`up` to move one line; `d`/page-down, `u`/page-up to page).
- Selecting a project row in the Agents tab opens a details modal overlay for that project, with panes for its registered agents (with status), its last test run (if any), and its recent activity log entries for that project.
- A `?` keybinding opens a help modal listing all keyboard shortcuts.
- In-memory only for this change; moving the log to SQLite-backed persistence is explicitly deferred to a future change.

## Capabilities

### New Capabilities
- `activity-log`: Daemon-side capture, bounded retention, and query/push API for a log of notification-worthy agent and test-run events.

### Modified Capabilities
- `agent-monitor-tui`: Adds Agents/Logs tabs, a per-project details modal (agents/tests/logs panes), a keyboard shortcuts help modal, and the keybindings for tab switching, row selection, and paginated list navigation.

## Impact

- `agentd`: new in-memory log store keyed by insertion order with a global cap; hooks into the existing notification-send paths (done, needs-input, test started/passed/failed) to append an entry at the same point a notification fires; new protocol messages for log snapshot/push.
- `agentmon-proto`: new message/event variants for activity-log entries (snapshot + incremental push).
- `agentmon`: TUI gains tab state, a details modal, a help modal, and paginated/sortable/filterable list rendering for the Logs tab; existing single-view table becomes the Agents tab's content.
- No changes to `agentmon-report` or the RSpec test-run ingestion format.
