## Context

See `proposal.md` - Why. This change is cross-cutting: it adds a new daemon-side subsystem (`agentd`), new wire types (`agentmon-proto`), and a new tab/pane/keybinding surface (`agentmon`). It also had to resolve conflicts against existing conventions, which the user settled via three clarifying questions before this change was written:

1. Reminder create/edit uses a **modal form** (matching the existing help/confirmation modal pattern), not inline row editing.
2. Reminders are **in-memory for the daemon's process lifetime only** - consistent with agents, test runs, and the activity log, none of which survive a daemon restart today. No new persistence layer.
3. `s` stays bound to start/stop on the Reminders tab and pane; the status filter moves to `f` there instead of colliding with it.

Relevant existing architecture this design builds on (see `crates/agentd/src/{server,registry,liveness,notify,activity_log}.rs`):
- The daemon's socket handles each connection with a single `read_message` call: a report (`ReportEvent`/`ReportTestRun`) is applied and the connection closes; a `Subscribe` sends a snapshot and then loops writing broadcast updates forever. No connection is both read from and written to after its first message - subscribers are pure sinks.
- A `Broadcaster` fans out `Update` variants (currently `Agent`, `TestRun`, `Log`, `AgentRemoved`) to every subscriber's channel.
- `liveness.rs` runs a background sweep on a fixed interval, checking all tracked agents against a liveness condition and reporting the ones that changed.
- The activity log is its own module/capability (`activity_log.rs`, capability `activity-log`) even though it's driven by daemon-side agent/test-run logic in `agent-daemon` - the two are split by concern, not folded together.

## Goals / Non-Goals

**Goals:**
- Let a reminder be created, started, stopped, edited, and deleted per-project, with the daemon (not the TUI) owning its lifecycle and firing its completion notification - so a reminder still fires even if the TUI isn't the one watching.
- Reuse existing patterns wherever they fit (broadcast model, sweep-thread model, emoji/duration conventions, paginated-list conventions) rather than inventing new plumbing.

**Non-Goals:**
- Persisting reminders across a daemon restart (explicitly deferred - see Context).
- Recurring/snoozing reminders, or reminders not scoped to a project.
- A generic request/response or error-reporting channel on the daemon socket (see Decisions - "Mutation commands are fire-and-forget").

## Decisions

### New `reminders` capability, not folded into `agent-daemon`
Reminders get their own capability spec (like `activity-log`) rather than becoming more requirements inside `agent-daemon`. Rationale: the feature is a full parallel subsystem (its own registry, lifecycle, cap, sweep, notification) comparable in shape to "agents" or "test runs" themselves, not an incremental extension of existing agent-daemon behavior. Alternative considered: extend `agent-daemon` the way test-run ingestion/grouping/notifications already live there - rejected because test-run behavior was small enough to fold in at the time, while reminders introduce a materially larger, self-contained surface.

### New `ReminderRegistry` module, separate from `registry.rs`
Implementation SHOULD add `crates/agentd/src/reminders.rs` holding a `ReminderRegistry` (same `Clone`-able, `Arc<Mutex<..>>`-backed shape as `Registry`), rather than growing `registry.rs` further. This mirrors how `activity_log.rs` is already its own module. `Ingestor` gains reminder-mutation methods the same way it already wraps `Registry`.

### Wire protocol shape
- `ReminderStatus`: `NotYetStarted | Running | Done` (serde snake_case, matching `AgentStatus`/`TestRunStatus` style).
- `ReminderInfo`: `id`, `cwd`, `name`, `duration_minutes`, `status`, `created_at_ms`, `run_started_ms: Option<u64>` (set on first start, refreshed on every re-start), `last_updated_ms` (bumped on start/stop/finish; falls back to `created_at_ms` when never touched). This mirrors `TestRunInfo`'s `run_started_ms`/`last_updated_ms` pair so elapsed/duration math is identical: running elapsed = now - `run_started_ms`; done elapsed = `last_updated_ms` - `run_started_ms`.
- `ReminderId(String)`: daemon-generated (e.g. a UUID or counter), not client-supplied - unlike `SessionId`, which a hook already knows, nothing gives a client a natural identity for a reminder before the daemon creates it.
- New `ClientMessage` variants: `CreateReminder { cwd, name, duration_minutes }`, `UpdateReminder { id, name, duration_minutes }`, `DeleteReminder { id }`, `StartReminder { id }`, `StopReminder { id }`.
- New `ServerMessage` variants: `Snapshot` gains a `reminders: Vec<ReminderInfo>` field; `ReminderUpdate { reminder: ReminderInfo }`; `ReminderRemoved { id: ReminderId }`.
- `LogCategory` gains `Reminder`; `LogEntry` gains `reminder_name: Option<String>`, populated only for reminder-category entries (parallel to how `pid` is populated only for agent-category entries).

### Mutation commands are fire-and-forget, like existing reports
`CreateReminder`/`UpdateReminder`/`DeleteReminder`/`StartReminder`/`StopReminder` are sent over their own short-lived connection and handled by the server's existing single-message-then-close path - exactly how `ReportEvent`/`ReportTestRun` work today. The daemon applies the change to the `ReminderRegistry` and broadcasts the result (`ReminderUpdate`/`ReminderRemoved`) to every subscriber, including the sender's own long-lived `Subscribe` connection; the TUI learns the outcome (including the daemon-assigned `id` for a new reminder) through that broadcast, not a synchronous reply. Alternative considered: make the daemon read continuously from a subscriber's connection so the TUI could reuse one socket for both subscribing and issuing commands, with a request/response envelope - rejected as a much larger protocol change (the server's per-connection model has never needed to read-after-write) for a feature that doesn't require synchronous confirmation to feel responsive, given the broadcast round-trip is local-socket-fast.

**Trade-off**: the 100-reminder-per-project cap (see specs/reminders - "Per-project reminder cap") has no client-facing error path under this model - a rejected `CreateReminder` is dropped server-side (logged via `eprintln!`, matching how a malformed payload is already handled) and the TUI's form simply closes with no new row appearing. This matches the codebase's existing precedent of not surfacing daemon-side rejections to the client. Revisiting this (e.g. an explicit `ReminderRejected` message) is a safe follow-up that wouldn't change the spec's behavior for the success path.

### Completion sweep
A new `spawn_reminder_sweep(registry: ReminderRegistry, interval: Duration, on_change: impl Fn(ReminderInfo))` in `reminders.rs`, structurally identical to `liveness::spawn_reminder_sweep`: on each tick, find every `Running` reminder whose `now - run_started_ms >= duration_minutes`, transition it to `Done`, append its "finished" log entry, fire its notification, and report it for broadcast. `serve()` spawns this alongside the existing liveness sweep, reusing the same `liveness_interval` passed into `serve` rather than adding a second interval knob - a reminder's minute-granularity duration doesn't need finer polling than agent liveness already uses.

### Notification sound
Reminder completion notifications use the built-in `Blow` system sound - distinct from `Glass`/`Ping` (agent) and `Pop`/`Tink`/`Basso` (test-run), so it's never confused with an existing notification by ear. (Originally `Hero`; changed to `Blow` per user preference after comparing the unused built-in sounds.)

### Recency sort fallback for a reminder with no completed run
The Reminders tab's default "recency" sort is specified as "the last completed run" (stop or finish), which a not-yet-started reminder doesn't have. Implementation SHOULD fall back to the reminder's creation time for that comparison, consistent with the Updated column's own fallback - so new, never-run reminders sort by when they were added rather than being pinned to one end of the list arbitrarily. This is an implementation-level tie-break, not a behavior the spec scenarios needed to pin down.

## Risks / Trade-offs

- **[Risk]** A daemon restart silently drops all reminders, including ones mid-run, with no warning to the user → **Mitigation**: this is consistent with existing agent/test-run/log behavior (none of it survives a restart today), and daemon restarts only happen on an explicit `brew services restart` (e.g. after an upgrade), not unexpectedly.
- **[Risk]** Fire-and-forget mutations mean a rejected `CreateReminder` (cap reached) is invisible to the user beyond "the reminder I just tried to add isn't there" → **Mitigation**: 100 reminders per project is a high bar unlikely to be hit in normal use; documented as a known follow-up rather than blocking this change.
- **[Risk]** Two independently-focusable panes in the details modal (Logs, Reminders) is a new interaction the TUI hasn't had before, raising the chance of a focus-tracking bug (keys applying to the wrong pane) → **Mitigation**: the spec pins a single explicit focus flag and a `Tab`-toggle scenario; task breakdown should include a test asserting the unfocused pane never reacts to its action keys.

## Migration Plan

No data migration - reminders are new, in-memory state. Ship as a normal `agentd`/`agentmon` release per the project's existing OpenSpec apply/archive cadence; no feature flag needed since the new tab/column/messages are purely additive to the wire protocol (existing clients ignore unknown fields via serde's default struct handling only if we're careful - **note for tasks.md**: adding `reminders: Vec<ReminderInfo>` to `Snapshot` is a breaking wire change for any client still running the old `agentmon-proto` types, exactly as every prior field addition to these structs has been; `agentd` and `agentmon` are versioned and upgraded together via Homebrew, consistent with how past additive protocol changes shipped.

## Open Questions

- Exact wording/threshold for a rejected-creation indicator (if this project later decides fire-and-forget isn't acceptable for this case) - doesn't change this change's specs or tasks.
