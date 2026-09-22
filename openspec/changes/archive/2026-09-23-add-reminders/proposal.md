## Why

Waiting on a remote, async process (a CI build going green, a long-running job) that isn't tied to any tracked Claude Code agent or test run currently has no place in agent-monitor. The user has to hold that "check back in N minutes" reminder in their own head, which adds cognitive load when switching between projects. A project-scoped, timed reminder - independent of agent/test-run activity - lets the user offload that and get pulled back automatically.

## What Changes

- Add a daemon-owned **reminders** capability: named, duration-based (minutes) reminders scoped to a project, with a start/stop/completion lifecycle independent of agent or test-run activity. Reminders are held in the daemon's in-memory state for its process lifetime (consistent with how agents, test runs, and the activity log already work - none of those persist across a daemon restart either), and are capped at 100 per project.
- A running reminder counts up like a running agent/test run and computes a wall-clock ETA; on natural completion it fires a macOS notification (reusing the daemon's existing notification path) and logs a "finished" event. Manually stopping a running reminder does not notify.
- Extend the activity log with a new `reminder` category so started/stopped/finished reminder events appear in the same aggregated log as agent/test-run events, subject to the log's existing global 500-entry cap. A deleted reminder's past log entries keep showing its name.
- Add a **Reminders** tab to the TUI (shortcut `r`), listing every project's reminders across the whole app, paginated and sortable/filterable like the Logs tab, with the same status emoji conventions as running/passed tests and done agents.
- Add a **Reminders** pane to the project details modal, turning the current Agents/Tests 50/50 split into an Agents/Tests/Reminders three-way split. Supports creating (`R`), editing (`e`), deleting (`d`/Del, with a confirmation dialog), and starting/stopping (`s`/`Enter`) a reminder via a modal form.
- Add a **Reminders** column to the Agents tab showing each project's running/most-recently-finished reminder(s), truncated to fit like the existing Agents column.
- Resolve two conflicts with existing TUI conventions: the Reminders tab/pane's status filter cycles on `f` instead of `s` (since `s` is reserved for start/stop there), and reminder creation/editing uses a modal form (matching the existing help/confirmation modal pattern) rather than inline row editing.

## Capabilities

### New Capabilities
- `reminders`: Daemon-side reminder registry and lifecycle (create/edit/delete/start/stop/complete), per-project 100-reminder storage cap, ETA/duration computation, completion notification, and the wire protocol messages a client uses to manage reminders and receive their updates.

### Modified Capabilities
- `agent-monitor-tui`: New Reminders tab; Agents tab gains a Reminders column; details modal gains a Reminders pane (Agents/Tests/Reminders thirds split) with create/edit/delete/start/stop interactions and their keyboard shortcuts; new `r`/`R`/`f`/`s`/`e`/`d` shortcuts and their scoping.
- `activity-log`: `LogCategory` gains a `reminder` variant; new requirement for reminder-specific log entries (started/stopped/finished), including retaining a deleted reminder's name on its past entries; these entries share the log's existing global 500-entry cap.

## Impact

- `crates/agentmon-proto/src/lib.rs`: new `ReminderInfo`/`ReminderStatus` types, `LogCategory::Reminder`, new `ClientMessage` variants (create/edit/delete/start/stop a reminder) and `ServerMessage` variants (reminder snapshot/update/removed).
- `crates/agentd/src/registry.rs` (or a new `reminders.rs`): per-project reminder storage, 100-reminder cap enforcement, lifecycle transitions.
- `crates/agentd/src/liveness.rs` (or a new sweep): periodic check that transitions a running reminder to done and fires a notification once its duration elapses.
- `crates/agentd/src/notify.rs`, `crates/agentd/src/activity_log.rs`: reminder completion notification; reminder log entries.
- `crates/agentd/src/server.rs`, `socket.rs`, `protocol.rs`: handling for the new `ClientMessage`/`ServerMessage` variants.
- `crates/agentmon/src/app.rs`, `ui.rs`, `input.rs`, `client.rs`: Reminders tab, Agents tab column, details modal pane, create/edit form, delete confirmation, new keybindings.
