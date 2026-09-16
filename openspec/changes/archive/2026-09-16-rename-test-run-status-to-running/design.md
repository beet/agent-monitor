## Context

See proposal.md - Why. `TestRunStatus` (`crates/agentmon-proto/src/lib.rs`) is one enum used for three distinct purposes today: the wire value the RSpec formatter sends in `ClientMessage::ReportTestRun`, the persisted/live `TestRunInfo.status` field the daemon tracks and broadcasts, and (via `ingest.rs`'s `test_run_log_status` and `notify.rs`'s `test_run_notification_script`) the literal word used in activity-log entries and notification text. Agents avoid this conflation entirely: `AgentStatus` has no "started" variant at all, and the activity log's one-time "started" entry is a hardcoded string produced by a dedicated renderer (`agent_started_text_and_style` in `crates/agentmon/src/ui.rs`), never derived from `AgentStatus::Running`.

## Goals / Non-Goals

**Goals:**
- Make the live/ongoing `TestRunStatus` variant read "running" everywhere the TUI displays an in-progress test run, matching `AgentStatus::Running`.
- Keep the RSpec formatter, the wire JSON it sends, the activity log's entries, and macOS notification text unchanged - all still say "started" for the one-time begin event, per the unmodified rspec-test-reporting and activity-log specs.

**Non-Goals:**
- Splitting `TestRunStatus` into two separate types (a "reported event" type and a "live status" type). A single renamed variant with a serde override accomplishes the same outward behavior with far less churn, since the same three call sites (report, persist, display) already share one value end-to-end - only the display layer needs a second, independent rendering path (mirroring agents).
- Touching `crates/agentmon-report` (the RSpec formatter) or any spec other than `agent-monitor-tui` - nothing about the reported event, its wire shape, or its logged/notified text changes.

## Decisions

**Rename the enum variant, and pin its wire representation with `#[serde(rename = "started")]`, rather than introducing a second enum.** `TestRunStatus` flows: RSpec formatter → `ClientMessage::ReportTestRun.status` → `Registry::upsert_test_run` → `TestRunInfo.status` → `ServerMessage::Snapshot`/`TestRunUpdate` → `agentmon`'s `App.test_runs` → `ui.rs`'s renderer. Every one of those hops except the last should be unaffected by this change. Renaming the Rust identifier but pinning its serde tag keeps the JSON wire bytes ("started") byte-for-byte identical before and after this change - `agentmon-report`, `agentd`, and `agentmon` all keep working together whether or not they've been rebuilt yet, since none of them observe the Rust-level identifier, only the wire string.

**Give the Logs tab/pane's "started" log entry its own renderer (`test_run_started_text_and_style`), rather than continuing to route it through `test_run_status_cell_text_and_style(TestRunStatus::Running, ...)`.** After the rename, that function's `Running` arm must say "running" (for the live Tests-column/Tests-pane cell), so it can no longer also serve the log entry's "started" text. This mirrors `agent_started_text_and_style`'s existing independence from `AgentStatus`'s renderer exactly, rather than inventing a new pattern.

**Leave `ingest.rs::test_run_log_status` and `notify.rs::test_run_notification_script`'s string literals unchanged (just re-pointed at the renamed match arm).** Both already return/use the hardcoded literal `"started"` for this variant; renaming `Started` to `Running` only renames the match arm they're keyed off, not the strings they produce.

## Risks / Trade-offs

- [A reader of `TestRunStatus::Running`'s serde attribute could assume it's a mistake or leftover] → The variant carries a doc comment explaining the intentional split between the wire/log word ("started") and the Rust/display name ("running"), pointing at `agent_started_text_and_style` as the precedent.
- [Two near-identical-looking renderers (`test_run_status_cell_text_and_style` and `test_run_started_text_and_style`) for test runs, where before there was one] → Accepted; this is the same shape agents already use (`agent_status_text_and_style` plus `agent_started_text_and_style`), so it's consistent with the rest of the file rather than a new pattern to learn.

## Migration Plan

No data migration and no wire-compatibility window to manage - the JSON wire value is unchanged, so `agentd` and `agentmon` can be rebuilt and deployed independently or together without any ordering constraint. Existing activity-log entries already on disk (if the daemon persisted any - it currently does not persist across restarts) would be unaffected either way, since their stored status string was always `"started"`.
