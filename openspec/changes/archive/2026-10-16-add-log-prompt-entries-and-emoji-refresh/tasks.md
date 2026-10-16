## 1. Proto: add the prompt log category

- [x] 1.1 Add `LogCategory::Prompt` to `crates/agentmon-proto/src/lib.rs` and verify `cargo build -p agentmon-proto` succeeds and existing (de)serialization tests still pass

## 2. Daemon: record prompt log entries

- [x] 2.1 In `Ingestor::ingest_event` (`crates/agentd/src/ingest.rs`), record a `LogCategory::Prompt`/`"submitted"` entry whenever `event.prompt.is_some()`, carrying the event's `pid` and prompt text, before the existing "started" and notify/log blocks - verify with a new `ingest.rs` unit test asserting a prompt-bearing event produces a "prompt"/"submitted" log entry
- [x] 2.2 Set that entry's `occurred_at_ms` to `outcome.agent.run_started_ms` when `is_run_start(...)` is true for this event, and to `now_ms()` otherwise, so it matches the "started" entry's timestamp exactly when one is also produced - verify with a unit test asserting a run-starting prompt event's "prompt" entry and "started" entry share the same `occurred_at_ms`
- [x] 2.3 Verify a unit test confirms the "prompt" entry is appended (recorded) before the "started" entry within the same `ingest_event` call, e.g. by asserting their relative order in `activity_log().snapshot()`
- [x] 2.4 Verify a unit test confirms a `UserPromptSubmit` event for an already-"running" agent still records a standalone "prompt" entry with no accompanying "started" entry

## 3. TUI: render the prompt category

- [x] 3.1 Add a `LogCategory::Prompt => "prompt"` arm to `log_category_label` (`crates/agentmon/src/ui.rs`)
- [x] 3.2 Add a `LogCategory::Prompt => ""` arm to `log_entry_tree_prefix` (no tree-branch indent, consistent with agent/reminder entries)
- [x] 3.3 Add a `LogCategory::Prompt` arm to `log_status_cell_text_and_style` rendering `"🗣️ prompt submitted"` with `Style::new()` (no color)
- [x] 3.4 Extend `log_entry_status_line`'s prompt-snippet appending (currently gated on `entry.category == LogCategory::Agent`) to also cover `LogCategory::Prompt`, so a prompt entry always shows its submitted text
- [x] 3.5 Verify with `ui.rs` unit tests: a prompt entry renders `"🗣️ prompt submitted"` in the default style, includes its prompt snippet, and sorts immediately ahead of a same-timestamp "started" entry after the Logs tab's recency sort

## 4. TUI: emoji changes for agent/test-run log and status markers

- [x] 4.1 Change `status_label_and_style`'s `AgentStatus::Done` and `AgentStatus::NeedsInput` arms to use the 🤖 marker, keeping their existing Green and Yellow/Bold styles
- [x] 4.2 Change `agent_started_text_and_style` to return `"🤖 agent started"` (keeping Blue)
- [x] 4.3 Change `test_run_started_text_and_style` to return `"🧪 tests started"` (keeping Blue)
- [x] 4.4 Update existing `ui.rs` unit tests/assertions that hardcode the old ✅/🔔/⏳ text for agent done/needs-input/started and test-run started (e.g. around lines ~2370-2420, ~2710-2720, ~3040-3045, ~3395-3405 per current line numbers) to expect 🤖/🧪
- [x] 4.5 Verify `cargo test -p agentmon` passes with the updated expectations, confirming the Agents tab's live status column and the Logs tab both reflect the new markers

## 5. Daemon: notification body emoji prefixes

- [x] 5.1 In `crates/agentd/src/notify.rs`, prefix `status_notification`'s body in `notification_script` with 🤖 for both `Done`/`NeedsInput` outcomes
- [x] 5.2 Prefix `test_run_notification_script`'s body with 🧪/✅/❌ for started/passed/failed respectively
- [x] 5.3 Prefix `reminder_notification_script`'s body with ⏰
- [x] 5.4 Update/add `notify.rs` unit tests asserting each notification's body string starts with the expected emoji, and verify `cargo test -p agentd` passes

## 6. Full verification

- [x] 6.1 Run `cargo test --workspace` and confirm it passes
- [x] 6.2 Run `cargo clippy --workspace` and confirm no new warnings
- [x] 6.3 Manually smoke-test with the TUI against a live daemon (per the project's manual-testing setup): submit a prompt that starts a new session and confirm a 🗣️ entry appears directly above the 🤖 "agent started" entry in the Logs tab, and that a `done`/`needs input` transition shows 🤖 in both the Agents tab and the Logs tab
