## Why

`agentd` currently only tracks Claude Code agent sessions via hook events; it has no visibility into test runs (e.g. RSpec) happening in the same project directories. A developer working alongside a Claude Code agent, or the agent itself, has no way to be notified when a test suite it triggered — or any test suite running in a tracked directory — passes or fails, short of watching the terminal.

## What Changes

- New RSpec formatter (Ruby) that reports test-run start/pass/fail events over `agentd`'s existing Unix socket, identifying itself by working directory rather than by Claude Code session id or process ancestry (RSpec can be launched from an agent's Bash tool, from nvim, or from an unrelated terminal — the formatter has no reliable way to learn a Claude Code `session_id`, and process-ancestry correlation can't see runs launched outside the agent's own subprocess tree).
- `agentd`'s registry is restructured to group entries by working directory: each directory groups zero or more tracked Claude Code agents alongside zero or more in-flight/recent test runs, rather than each agent being a fully independent entry with no relation to others sharing its directory. `session_id` remains each agent's identity; test runs get their own identity. This is additive — it does not change how individual agents are tracked or how existing hook events are processed.
- A test-run event whose directory group contains no tracked agents still surfaces via a plain OS notification on failure, so results are never silently dropped just because no agent happens to be tracked there.
- `agentmon`'s TUI changes from a flat one-row-per-session list to a directory-grouped view, showing each directory's tracked agent(s) together with any test run(s) in progress or recently finished there, including pass/fail status.
- New `agentmon` CLI subcommand that writes a project's `.rspec-local` file (RSpec's supported mechanism for personal, untracked local CLI-option additions) pointing `--require`/`--format` at the formatter's installed path, so no tracked file (`.rspec`, `spec_helper.rb`) needs editing to opt a project in.
- The formatter is packaged as an asset in the `beet/homebrew-agent-monitor` tap so `brew install`/`upgrade` places it at a stable, known path the generated `.rspec-local` can point to. This change prepares the formatter file and its packaging path in this repo; publishing it to the tap itself is deferred to the next explicit release, per this project's existing Homebrew-tap workflow.

## Capabilities

### New Capabilities
- `rspec-test-reporting`: the RSpec formatter's event-reporting behavior, the `agentmon` subcommand that generates a project's `.rspec-local`, and the formatter's packaging as a tap asset.

### Modified Capabilities
- `agent-daemon`: adds a test-run event message distinct from the existing hook-driven `ReportEvent`; restructures the registry to group agents and test runs by working directory; adds the no-tracked-agent OS-notification fallback.
- `agent-monitor-tui`: changes the agent list from a flat per-session view to a directory-grouped view that also displays test-run status.

## Impact

- `crates/agentmon-proto/src/lib.rs`: new message/status types for test-run events.
- `crates/agentd/src/{registry.rs,ingest.rs,notify.rs}`: directory-grouped registry structure, test-run entity handling, OS-notification fallback path.
- `crates/agentmon/src/{app.rs,ui.rs,main.rs}`: grouped-by-directory rendering, test-run status display, and the new `.rspec-local`-generating subcommand (today `agentmon`'s `main.rs` has no argument dispatch at all — it always launches the TUI).
- New Ruby formatter source file, plus wherever this repo stages Homebrew-tap assets for release.
- `beet/homebrew-agent-monitor` tap: formula changes to package the formatter asset (deferred to next explicit release, not part of this change).
- README: setup instructions for the new subcommand and `.rspec-local` generation.
