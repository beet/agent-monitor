## Context

See proposal.md - Why. Relevant existing structure:

- `agentmon-proto` defines the wire types (`AgentEvent`, `ClientMessage`, `ServerMessage`, `AgentInfo`, `AgentStatus`) shared by `agentd`, `agentmon`, and `agentmon-report`.
- `agentd`'s `Registry` (`crates/agentd/src/registry.rs`) is a `HashMap<SessionId, AgentInfo>` behind an `Arc<Mutex<_>>`, with pid-based dedup on new sessions. It has no notion of working directory as an index - `cwd` is just a field on `AgentInfo`.
- `agentmon-report` (the hook binary) parses Claude Code's hook JSON, maps it to an `AgentStatus`, and sends `ClientMessage::ReportEvent` over the daemon's Unix socket, bounded by a 500ms timeout with all errors swallowed (a hook must never fail or block the user's turn).
- `agentmon` (the TUI binary) currently has no argument dispatch at all - `main()` always launches the TUI.
- Established correlation constraint (settled in prior discussion): an RSpec run has no reliable way to learn its enclosing Claude Code `session_id`, and pid-ancestry only covers runs an agent's own Bash tool spawned directly - not runs from nvim or another terminal. Exact working-directory match is the only correlation basis that covers all launch paths, given rspec is always run from the project root Claude Code itself runs in.

## Goals / Non-Goals

**Goals:**
- Add a test-run entity to `agentd` that is tracked independently of agent sessions, grouped with them by exact working-directory match.
- Keep the existing session-keyed agent registry behavior completely unchanged - this is additive.
- Give `agentmon`'s TUI a directory-grouped view without losing any of its existing per-agent behavior (duration counters, status markers, sorting, reconnect handling).
- Ship an RSpec formatter and a paired `agentmon` subcommand that together let a project opt in without editing any tracked file.

**Non-Goals:**
- Prefix/subdirectory `cwd` matching - out of scope per the settled correlation basis (rspec always runs from the agent's own project root).
- Any framework other than RSpec.
- Publishing to the `beet/homebrew-agent-monitor` tap as part of this change - the formula update happens at the next explicit release, per this project's established tap workflow.
- Per-example (as opposed to per-suite) test event reporting - the daemon and TUI only need suite-level started/passed/failed.

## Decisions

### Registry: a parallel directory index, not a rekey
Keep `Registry`'s existing `HashMap<SessionId, AgentInfo>` untouched. Add:
- A new `HashMap<PathBuf, TestRunInfo>` (or `Vec<TestRunInfo>` per directory, if a directory can have more than one concurrent run - see below) for tracked test runs, keyed by `(cwd, pid)` so multiple runs in the same directory don't collide.
- A read-side grouping function that, given a snapshot of agents and test runs, groups both by exact `cwd` equality for `Snapshot`/`AgentUpdate`-equivalent messages to clients.

Rejected alternative: rekeying the whole registry by `cwd` (raised and discussed with the user). Rejected because it would break the existing one-entry-per-session model that already correctly supports multiple concurrent same-directory agents, for no behavioral gain - grouping only needs to happen at the query/display boundary, not in the storage model.

### Protocol: new message variants, not an overloaded `AgentEvent`
Add to `agentmon-proto`:
- `ClientMessage::ReportTestRun { cwd: PathBuf, pid: u32, status: TestRunStatus }` - distinct from `ReportEvent`, since a test run is not a Claude Code hook event and carries no `session_id`.
- `TestRunStatus { Started, Passed, Failed }` - separate enum from `AgentStatus`, since the two lifecycles don't share transition rules (no "declined", no "stale" sweep for a test run - a run's own process exiting without a final event is simply the last state it reported).
- `AgentInfo`/`ServerMessage` gain a parallel `TestRunInfo { cwd, pid, status, last_updated_ms }` and the corresponding snapshot/update variants (or `ServerMessage::Snapshot`/`AgentUpdate` are extended to carry both - implementation detail for tasks.md to pin down).

`pid` is included on `ReportTestRun` for uniqueness within a directory (two concurrent `rspec` invocations in the same repo), not for correlation - correlation to agents is by `cwd` alone, per the settled design.

### Test-run lifecycle in the registry: last-write-wins per (cwd, pid)
A "started" event creates or replaces the entry for that `(cwd, pid)`; "passed"/"failed" update it in place. No expiry rule is introduced - a finished run's row persists until superseded by the next run at the same `(cwd, pid)`, mirroring how an agent's row persists until its next transition. This avoids inventing a TTL/expiry policy the user hasn't asked for.

### Fallback notification: daemon-side, not formatter-side
The "no tracked agent" fallback notification (per spec) is sent by `agentd`, using its existing macOS-notification path (`crates/agentd/src/notify.rs`), not by the Ruby formatter shelling out to `osascript`/`terminal-notifier` itself. This keeps notification logic in one place, reuses the existing sound/notification plumbing, and keeps the formatter a thin, dependency-free reporter consistent with its "must never block or fail the run" requirement.

### `agentmon` subcommand and formatter path resolution
`agentmon` gains argument dispatch (mirroring `agentmon-report`'s `match args.next()` pattern) for a new subcommand, e.g. `agentmon init-rspec`, that:
1. Resolves the installed formatter's path. Per the spec requirement that this path survive Homebrew upgrades, this SHALL be a stable, non-version-pinned path (Homebrew's `opt_prefix`-style symlink, not a versioned `Cellar` path) - the same reasoning already applied to the daemon's `launchd` plist referencing the current Homebrew-managed binary path.
2. Writes `--require '<path>' --format AgentMonitorRspecFormatter --format progress` into `.rspec-local` in the current directory, appending only if not already present (idempotent).

### Formatter packaging
The formatter ships as a single Ruby file with no gem dependencies (plain `RSpec::Core::Formatters.register`), installed by the Homebrew formula to a fixed `share/`-style path. Packaging the actual formula change is out of scope for this change (see Non-Goals) - this change only needs to land the formatter file at a path the `init-rspec` subcommand can locate today (e.g. relative to the `agentmon` binary's own install location) so both pieces are ready when the tap is next updated.

## Risks / Trade-offs

- **[Risk]** Two directories that are actually the same project (e.g. one path taken via a symlink, another via its real path) won't match under exact `cwd` equality → **Mitigation**: none needed now; this is the same limitation the existing agent registry already has (agents are compared by their raw reported `cwd`), so it's not a regression, and canonicalizing paths can be added later without a spec change if it turns out to matter in practice.
- **[Risk]** A long-lived directory group accumulates finished test-run rows with no expiry, if a project runs many one-off `rspec` invocations with different pids over time → **Mitigation**: last-write-wins per `(cwd, pid)` bounds this to "however many distinct pids have run there since the daemon started," which in practice is small; revisit only if this proves to be a real problem.
- **[Trade-off]** Grouping is computed at query time rather than maintained incrementally → simpler to implement and reason about; acceptable given the registry's scale (a handful to a few dozen entries on one developer's machine), not a service handling many clients' worth of state.
