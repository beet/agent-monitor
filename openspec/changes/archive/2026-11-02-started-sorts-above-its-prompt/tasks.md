## 1. Daemon recording order

- [x] 1.1 In `crates/agentd/src/ingest.rs::Ingestor::ingest_event`, swap the order of the two `record_log` calls so the "agent"/"started" entry is appended before the "prompt"/"submitted" entry when `run_starting` is true (both still using `outcome.agent.run_started_ms` as their shared `occurred_at_ms`). A follow-up prompt that does not start a run (`run_starting` false) is unaffected - it still just appends its own standalone prompt entry. Verify by running `cargo test -p agentd ingest::`.
- [x] 1.2 Update the doc comment above `ingest_event` (and the inline comment just above the prompt-recording block) to describe the new order and its rationale, replacing the references to the prompt entry being recorded first.

## 2. Daemon tests

- [x] 2.1 In `crates/agentd/src/ingest.rs` tests, update `a_prompt_that_starts_a_run_is_recorded_before_the_started_entry` (rename to `a_prompt_that_starts_a_run_is_recorded_after_the_started_entry`) to assert `logs[0].category == Agent` / `status == "started"` and `logs[1].category == Prompt`. Verify it passes.
- [x] 2.2 Confirm `a_prompt_that_starts_a_run_shares_the_started_entrys_timestamp` still passes unchanged (it only compares the two entries' timestamps, not their order).
- [x] 2.3 Confirm `a_started_entry_snapshots_the_agents_current_last_prompt`, `a_followup_prompt_mid_session_logs_on_its_own_with_no_started_entry`, and `a_later_prompt_does_not_alter_an_already_recorded_entrys_snapshot` still pass unchanged (none assert the swapped pair's relative order). Run `cargo test -p agentd` to confirm the full daemon suite is green.

## 3. TUI test

- [x] 3.1 In `crates/agentmon/src/app.rs`, rename `a_same_timestamp_prompt_entry_sorts_ahead_of_the_started_entry_it_precipitated` to `a_same_timestamp_started_entry_sorts_ahead_of_the_prompt_entry_it_precipitated`, build its `apply_log_snapshot` input in the new daemon order (started entry first, prompt entry second - matching what `ingest_event` now actually emits), and assert `visible[0]` is the "started" entry and `visible[1]` is the "prompt" entry. Verify with `cargo test -p agentmon app::tests::a_same_timestamp_started_entry_sorts_ahead_of_the_prompt_entry_it_precipitated`.
- [x] 3.2 Run the full `cargo test -p agentmon` suite to confirm no other test depends on the old ordering (e.g. nothing in `ui.rs`'s rendering tests hardcodes which of the two rows comes first).

## 4. Manual verification

- [x] 4.1 Rebuild and restart the daemon and TUI (per the project's manual-testing convention: isolate `HOME` for the test run), submit a prompt that starts a new run, and confirm the Logs tab and the details modal's Logs pane both show the "agent started — "<prompt>"" row immediately above the standalone "🗣️ <prompt>" row for that turn, under the default recency sort.
