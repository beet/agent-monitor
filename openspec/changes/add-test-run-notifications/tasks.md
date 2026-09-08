## 1. Protocol (`agentmon-proto`)

- [x] 1.1 Add `TestRunStatus` enum (`Started`, `Passed`, `Failed`) and `TestRunInfo { cwd, pid, status, last_updated_ms }` to `crates/agentmon-proto/src/lib.rs`, and verify the crate builds with `cargo build -p agentmon-proto`
- [x] 1.2 Add `ClientMessage::ReportTestRun { cwd: PathBuf, pid: u32, status: TestRunStatus }` variant, and verify existing `ClientMessage` (de)serialization tests still pass
- [x] 1.3 Extend `ServerMessage::Snapshot`/`AgentUpdate` (or add parallel variants) to carry tracked test runs alongside agents, and add a round-trip serialization test for the new/extended message shape

## 2. Daemon registry and ingestion (`agentd`)

- [x] 2.1 Add a test-run store to `Registry` (`crates/agentd/src/registry.rs`) keyed by `(cwd, pid)`, with an `upsert_test_run` method implementing last-write-wins per key, and unit tests covering: first "started" event creates an entry, a later "passed"/"failed" updates it in place, two different pids in the same `cwd` don't collide
- [x] 2.2 Add a grouping function that, given the current agents and test runs, produces directory groups (exact `cwd` equality) each containing its member agents and test runs, with unit tests covering: a directory with only agents, only test runs, both, and neither (no group)
- [x] 2.3 Wire `ClientMessage::ReportTestRun` into `crates/agentd/src/ingest.rs` so it updates the registry via 2.1, and add a test that sending this message through the socket updates the registry (mirroring the existing `ReportEvent` ingestion test)
- [x] 2.4 Update the daemon's snapshot/update broadcast path to send grouped agent+test-run state to connected clients per the extended `ServerMessage`, and verify with an integration-style test that a connected client receives a test-run update

## 3. Notification fallback (`agentd`)

- [x] 3.1 In `crates/agentd/src/notify.rs`, add a path that sends a macOS notification identifying the working directory, played with the `Basso` system sound, when a "failed" test-run event's directory group has no tracked agents, and verify with a unit test (using the same mocking approach as the existing completion-notification tests) that: a failing run with no agents notifies with `Basso`, a failing run with a tracked agent does not, and "started"/"passed" never notify

## 4. TUI grouped rendering (`agentmon`)

- [x] 4.1 Update `crates/agentmon/src/client.rs` to receive and store the extended snapshot/update shape (agents + test runs) from the daemon
- [x] 4.2 Update `crates/agentmon/src/app.rs`'s state model to hold directory groups (each with its agents and test runs) instead of a flat agent list, preserving existing per-agent fields (status-since, duration), and add unit tests for: grouping two agents sharing a `cwd`, a test-run-only group, group-level recency sorting, and within-group ordering (agents before test runs, each by their own last-updated time)
- [x] 4.3 Update `crates/agentmon/src/ui.rs` to render directory group sections with their member rows, add the ⏳/✅/❌ test-run status markers alongside the existing agent status markers, and verify by running the TUI against a daemon fed synthetic agent and test-run events (manual/integration check, since this is terminal rendering) - verified via `TestBackend`-rendered assertions, this project's established pattern for every other TUI rendering requirement in `ui.rs`, rather than a literal real-terminal run
- [x] 4.4 Verify existing single-agent-per-directory behavior (no test runs involved) renders identically to before this change, via the existing TUI tests in `crates/agentmon/tests/`

## 5. `agentmon` CLI subcommand for `.rspec-local`

- [x] 5.1 Add argument dispatch to `crates/agentmon/src/main.rs` (mirroring `agentmon-report`'s `match args.next()` pattern) so `agentmon` without arguments still launches the TUI, and a new subcommand (e.g. `init-rspec`) is handled separately
- [x] 5.2 Implement formatter-path resolution using a stable, non-version-pinned install path (see design.md's "path resolution" decision), and a function that writes or idempotently updates `.rspec-local` in the current directory with the `--require`/`--format` options, with unit tests covering: no existing `.rspec-local` (created), an `.rspec-local` already containing the options (no duplication), and an `.rspec-local` with unrelated content (options appended, existing content preserved)
- [x] 5.3 Verify manually in a scratch Rails/RSpec project that running the subcommand produces a working `.rspec-local` and does not modify `.rspec` - confirmed `.rspec` stayed untouched and a real `rspec` run picked up the generated formatter via `.rspec-local` alongside `progress`

## 6. RSpec formatter

- [x] 6.1 Write the Ruby formatter (plain `RSpec::Core::Formatters.register`, no gem dependencies) implementing `start`/`dump_summary`-driven "started"/"passed"/"failed" reporting per `specs/rspec-test-reporting/spec.md`, sending `ReportTestRun` over the daemon's Unix socket with a bounded timeout and all errors swallowed (mirroring `agentmon-report`'s `report.rs` non-blocking guarantee)
- [x] 6.2 Add formatter tests (RSpec's own self-testing story, e.g. running it against a small fixture suite with a mock/stub socket listener) covering: started event on run start, passed event on an all-green fixture suite, failed event on a fixture suite with a failing example, and that a missing daemon socket does not raise or change the run's exit status
- [x] 6.3 Stage the formatter file at a path within this repo suitable for the eventual Homebrew formula to install (see design.md's "Formatter packaging" decision) - do not touch the actual tap repo - staged under `rspec-formatter/` at the repo root

## 7. Documentation

- [x] 7.1 Update `README.md` with setup instructions for the new subcommand and what test-run tracking adds to the daemon/TUI behavior described in "How it works"
