## Context

`agentd` currently has no persisted-history concept: `Ingestor::ingest_event` and `Ingestor::ingest_test_run` (`crates/agentd/src/ingest.rs`) call straight into a `Notifier` (`crates/agentd/src/notify.rs`) whenever a transition is notification-worthy, and the `Registry`/test-run store only ever reflect current state. The socket protocol (`agentmon-proto/src/lib.rs`) has `ServerMessage::Snapshot { agents, test_runs }` sent once per `Subscribe`, plus `AgentUpdate`/`TestRunUpdate` pushed live (`crates/agentd/src/server.rs`, `Broadcaster`/`Update` enum). The TUI (`agentmon` crate) is a single always-visible table today: `App` (`crates/agentmon/src/app.rs`) holds `agents`/`test_runs`/`connection` with no selection or tab state, `render()` (`crates/agentmon/src/ui.rs`) draws one `Table`, and `handle_key()` (`crates/agentmon/src/input.rs`) only maps `q`/`Esc` to quit. There is no existing config mechanism in `agentd` (no `Config` struct, no env vars, no CLI flags beyond a manual `--foreground` check) - see proposal.md for why this change exists.

## Goals / Non-Goals

**Goals:**
- Give the daemon a bounded, in-memory record of notification-worthy events, pushed to clients the same way agent/test-run updates already are.
- Give the TUI a way to browse that history (Logs tab) and drill into one project's Agents/Tests/Logs (details modal), without disturbing the existing Agents-tab table behavior.

**Non-Goals:**
- Persistence across daemon restarts (explicitly deferred to a future SQLite-backed change per the proposal).
- A general-purpose config system for `agentd`. The retention cap is a constant for this change, matching the existing pattern of `LIVENESS_SWEEP_INTERVAL` as a `const` in `main.rs`.
- Changing existing Agents-tab row content, grouping, or status rendering - this change only adds a selection cursor and an `Enter` action on top of it.

## Decisions

**Log storage is a small `ActivityLog` type (`crates/agentd/src/activity_log.rs`) owned by `Ingestor`, keyed by insertion order.** `Ingestor` already owns the `Registry` and `Notifier` and is the single call site for both notification triggers (`ingest_event`, `ingest_test_run`). `ActivityLog` wraps a `VecDeque<LogEntry>` in `Arc<Mutex<_>>` (cheap to clone, like `Registry`), capped at `MAX_LOG_ENTRIES = 500` with FIFO eviction, and is appended to right alongside the existing `notifier.notify(...)` / `notifier.notify_test_run(...)` calls - keeping "what notifies" and "what gets logged" defined in exactly one place. Broadcasting a newly-recorded entry to socket subscribers (`server::serve`) is wired via `Ingestor::set_log_listener`, a callback registered once at `serve()` startup - the same pattern `spawn_liveness_sweep` already uses for its own updates - rather than changing `ingest_event`/`ingest_test_run`'s return types, which would have forced every existing call site and test to be touched for a concern (broadcasting) orthogonal to what they return (the updated `AgentInfo`/`TestRunInfo`).

**Cap is a `const MAX_LOG_ENTRIES: usize = 500` in `agentd`, not configurable.** No config mechanism exists today (confirmed: no `Config` struct, no env vars, no CLI flags framework), and the proposal doesn't ask for one. Introducing config plumbing for a single constant is out of scope; if a different cap is needed later it can become a constructor parameter the way `LIVENESS_SWEEP_INTERVAL` already is threaded into `serve()`.

**Protocol: extend `Snapshot` and add one push variant, no new request type.** `ClientMessage::Subscribe` already triggers a snapshot plus a live stream, so the log rides the same lifecycle as agents/test-runs:
- `ServerMessage::Snapshot` gains a `logs: Vec<LogEntry>` field (chronological order).
- A new `ServerMessage::LogAppended { entry: LogEntry }` variant, broadcast the same way `AgentUpdate`/`TestRunUpdate` are today (`Update` enum in `server.rs` gains a third `Log(LogEntry)` arm).
- `LogEntry` (new type in `agentmon-proto`): `{ working_dir: PathBuf, category: LogCategory, status: String, occurred_at_ms: u64 }` where `LogCategory` is `Agent | TestRun`. Reuses the existing status strings already sent for agents/test-runs rather than inventing a parallel enum, and `occurred_at_ms` matches the existing `_ms: u64` (unix epoch milliseconds) convention used by `AgentInfo`/`TestRunInfo` rather than introducing a `chrono` dependency into `agentmon-proto`, which has none today.

Alternative considered: a dedicated `FetchLog`/`LogSnapshot` request-response pair the TUI sends after connecting - rejected because it adds a round trip and a second code path for something `Subscribe` already delivers atomically with the rest of the initial state.

**TUI: new `Tab` enum and `selected` index on `App`; details/help are overlay flags, not separate `App` states.** `App` gains `active_tab: Tab { Agents, Logs }`, `agents_selected: usize`, a `logs` buffer (`Vec<LogEntry>` capped client-side to what the daemon ever sends, i.e. also ≤500) with its own `logs_selected`/`logs_scroll`, and `modal: Option<Modal>` where `Modal` is `Details(working_dir) | Help`. `render()` branches on `active_tab` for the base view and draws the modal on top when `modal.is_some()`, mirroring how it already branches on `ConnectionStatus`. `handle_key()` checks `modal` first (routing `Esc`/navigation to the modal) before falling through to tab-level keys - keeps modal-vs-background key routing in one place instead of duplicated per tab.

**Details modal's Agents/Tests/Logs panes are computed from data `App` already holds.** `directory_groups()` (`app.rs`) already aggregates agents+test-run per working directory for the Agents-tab table; the modal's Agents and Tests panes reuse that same grouping for the selected directory. The Logs pane filters `App.logs` by `working_dir` client-side - no new daemon query needed, consistent with the activity-log spec sending every client the full log and letting the client filter/sort/paginate.

**Sorting/filtering and pagination are pure `App`-state transforms over `logs`, computed on each render.** No incremental-index structures (e.g. a project→entries map) - 500 entries is small enough that a linear filter+sort per redraw is not a performance concern, and keeping it stateless avoids a second source of truth to keep in sync with `App.logs`. Sort cycles Recency → Project → Status → Recency (key `o`); Project and Status filters each cycle through every distinct value present in the full log, alphabetically, then back to "no filter" (keys `p` and `s`); `c` clears both. These specific keys aren't dictated by the proposal - a reasonable assumption, documented in the in-app help modal (`?`).

**Logs-tab paging moves a fixed `LOGS_PAGE_SIZE` (10) lines rather than a terminal-height-derived amount.** Deriving the page size from the rendered viewport would require `render()` to feed the actual visible row count back into `App` (changing it from a read-only view over `App` into something that mutates it), coupling input handling to terminal geometry for a UX detail the spec doesn't require to be exact - it only requires that `d`/`u` move "by a full page, if a page is available". Ratatui's `TableState` auto-scrolls to keep the selected row in view regardless of the exact page size chosen, so a fixed constant satisfies the requirement with far less coupling.

## Risks / Trade-offs

- **Global (not per-project) cap means a noisy project can push a quiet project's history out entirely.** Accepted per your explicit choice in scoping this change; a per-project cap can be revisited later if it becomes a problem in practice.
- **In-memory only: a daemon restart loses all activity history**, same as it already loses nothing today only because it tracks no history. Matches the proposal's explicit deferral of persistence.
- **`LogEntry.status` as a bare `String` duplicates the existing loosely-typed status strings already used for agents/test-runs** rather than a shared enum - consistent with the existing protocol's style, but means a typo in a status string wouldn't be caught at compile time. Mitigated by unit tests asserting the exact strings `Ingestor` emits.

## Migration Plan

Additive protocol change (new `Snapshot` field, new `ServerMessage` variant) - existing clients on the old proto version would fail to deserialize the new field only if `agentmon-proto` isn't updated in lockstep, but this is a single-workspace, matched-version protocol (daemon and TUI ship together via the same Homebrew formula), so no backward-compatibility shimming is needed. No data migration since there is no persisted state yet.
