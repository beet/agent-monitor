## Why

The just-shipped test-run tracking (v0.5.0) identifies a tracked test run by `(cwd, pid)`, so every sequential `rspec` invocation in a directory - a normal edit/test loop - creates a new permanent row that never goes away. Confirmed live: a user saw four separate `test run` rows for the same directory (different pids, different timestamps) stacked in the TUI. This was a named, deliberately deferred risk in the original design ("revisit only if this proves to be a real problem") and it now has.

## What Changes

- A tracked test run's identity changes from `(cwd, pid)` to `cwd` alone. Reporting a new test-run event for a directory replaces any previously tracked test run there, regardless of process id - a directory's test-run row always reflects only the most recently reported run, the same way an agent's row already works (one row per identity, latest status wins).
- `pid` remains on the wire protocol and in `TestRunInfo` as a display field; it just drops out of the identity/dedup key.
- No protocol shape change (`ClientMessage::ReportTestRun` and `TestRunInfo` are unchanged), no TUI change - this is entirely a daemon-side registry fix.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-daemon`: the "Agents and test runs are grouped by working directory" requirement's test-run identity changes from `(cwd, pid)` to `cwd` alone.

## Impact

- `crates/agentd/src/registry.rs`: test-run store rekeyed from `HashMap<(PathBuf, u32), TestRunInfo>` to `HashMap<PathBuf, TestRunInfo>`; `upsert_test_run` now always overwrites whatever is tracked at that `cwd` instead of matching on `(cwd, pid)`.
- `crates/agentd/src/registry.rs` tests: `different_pids_in_the_same_directory_do_not_collide` inverts - two different pids reporting to the same directory now collapse to one entry (the latest), not two.
- No changes needed in `agentmon` (client/app/ui already just render whatever `TestRunInfo` list they're given) or the RSpec formatter (already reports `(cwd, pid, status)` unaware of daemon-side storage).
