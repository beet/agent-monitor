## Context

Today `agentmon`'s TUI already computes a `DirectoryGroup` per working directory (`crates/agentmon/src/app.rs`) and the daemon's `Registry` computes the equivalent server-side (`crates/agentd/src/registry.rs`), but `ui.rs` still flattens each group into one row per agent plus one row per test run, repeating the PROJECT cell. See proposal.md - Why for the workflow motivating a single row per project. `TestRunInfo` (`crates/agentmon-proto/src/lib.rs`) currently carries only `last_updated_ms`; there is no timestamp marking when the current run started, which `format_running_duration` (already used for agents in `ui.rs`) needs a `status_since_ms`-equivalent to compute against.

## Goals / Non-Goals

**Goals:**
- One rendered row per project (working directory), combining every tracked agent's status and the directory's test run (if any) into a single STATUS cell.
- Test runs get a duration using the same live-counter mechanism agents already use, extended to also show a final elapsed time once the run finishes.
- Keep the daemon's per-agent data (host, pid, session id) fully intact server-side; only the TUI's rendering collapses rows.

**Non-Goals:**
- No new "details" drill-down view for host/pid - the proposal explicitly defers that.
- No change to how agents or test runs are keyed, deduplicated, or grouped server-side (`Registry::upsert`, `Registry::upsert_test_run`, and the exact-working-directory grouping rule are unchanged).
- No change to notification behavior (`agentd/src/notify.rs`) - it already operates on individual agent/test-run transitions, independent of how the TUI renders rows.

## Decisions

**`TestRunInfo` gains `run_started_ms: u64`, computed in `Registry::upsert_test_run`.**
Mirrors `AgentInfo::status_since_ms`, but keyed on process id rather than status: when the incoming event's pid matches the directory's currently tracked test run, `run_started_ms` carries over unchanged (same process, later lifecycle stage); otherwise it's set to `now`. This piggybacks on the existing replace-on-cwd-key upsert in `registry.rs` - no new storage, just one more field to preserve or reset alongside the swap. Alternative considered: a separate `HashMap<PathBuf, u64>` tracking run starts independently of the current `TestRunInfo` - rejected as redundant state that could drift from the `TestRunInfo` it's describing.

**`app.rs`'s `DirectoryGroup` becomes the sole grouping the TUI renders from, unchanged in shape.**
`directory_groups()` already returns one `DirectoryGroup` per cwd with `agents: Vec<AgentInfo>` and `test_runs: Vec<TestRunInfo>` (currently always 0 or 1 test run per the daemon's own invariant). The row-collapsing work happens entirely in `ui.rs`'s rendering: instead of `flat_map`-ing agent rows then test-run rows, build one `Row` per group. `DirectoryGroup` itself does not need a rename or restructure for this - "project" is a rendering-level concept card the row layout expresses, not a new server-side identity.

**Combined STATUS cell: list distinct statuses in a fixed order, each with its own duration when applicable.**
Build the cell as segments joined by ` · `: iterate the group's agents in `AgentStatus` declaration order (Running, Idle, NeedsInput, Done, Stale, Declined), emit one segment per *distinct* status present (not one per agent - two "running" agents contribute a single "🔧 running" segment, per the user's "list each distinct status" answer), then append the test run's segment last if present. A running/started segment includes a duration only when exactly one contributing member holds that status; if two or more agents share "running", the segment omits a duration rather than arbitrarily picking one agent's elapsed time (a minor detail not covered by the clarifying questions - flagged here rather than in Open Questions since it doesn't change the spec or task breakdown, just this rendering rule).

**HOST and PID columns are deleted, not hidden.**
The `Table`'s `widths` and header shrink to `[PROJECT, STATUS, UPDATED]`. `AgentInfo.host_context` and `.pid` remain on the wire and in the registry (per the agent-daemon spec, unchanged) - simply unused by `ui.rs`'s row-building for now.

**Test-run duration reuses `format_running_duration`.**
For `TestRunStatus::Started`, call it as agents do: `format_running_duration(run_started_ms, now_ms)`. For `Passed`/`Failed`, call it with `(run_started_ms, last_updated_ms)` instead of `now_ms` - the same function produces a fixed duration once both endpoints are fixed, so no new formatting function is needed.

## Risks / Trade-offs

- [Losing at-a-glance host/pid] → Accepted per the user's explicit choice; the daemon keeps the data so a future details view isn't blocked by this change.
- [Combined STATUS cell could grow long with many distinct statuses] → Unlikely in practice (a project realistically has 1-2 agents plus a test run); no truncation added now, revisit if it becomes a problem.
- [`run_started_ms` reset-on-new-pid could misfire if a test runner is re-exec'd under the same pid] → Same class of edge case the existing pid-based dedup already accepts elsewhere in this codebase (e.g. agent session/pid replacement); not a new risk introduced by this change.

## Migration Plan

This is a local, single-binary TUI/daemon pair with no persisted state across restarts (`Registry` is in-memory) and no external API consumers besides the bundled TUI and RSpec formatter. Ship daemon and TUI together; an older TUI connecting to a newer daemon would simply ignore the added `run_started_ms` field (serde default-tolerant since it's always present, not optional), and there's no need to support a newer TUI against an older daemon. No rollback beyond reverting the commit.
