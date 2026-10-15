## 1. Update display formatting

- [x] 1.1 Change `format_last_updated` in `crates/agentmon/src/ui.rs` from `"%Y-%m-%d %H:%M:%S"` to `"%Y-%m-%d %-I:%M:%S %p"` and verify it renders e.g. `2026-10-16 2:05:30 PM` for an afternoon timestamp
- [x] 1.2 Change `format_eta_ms` in `crates/agentmon/src/ui.rs` from `"%H:%M"` to `"%-I:%M %p"` and verify `format_reminder_eta` (which delegates to it) renders e.g. `2:05 PM`

## 2. Update tests

- [x] 2.1 Update the hardcoded `"%Y-%m-%d %H:%M:%S"` expected-string patterns in `crates/agentmon/src/ui.rs`'s test module (the `format_last_updated` test and the two call sites that independently rebuild an expected string) to the new 12-hour pattern, and verify `cargo test -p agentmon` passes
- [x] 2.2 Update the `format_eta_ms` test assertion (`ui.rs:4410`) to expect the new 12-hour ETA format and verify it passes

## 3. Verify end-to-end

- [x] 3.1 Run `cargo build` and `cargo test` across the workspace and confirm no other call site still assumes 24-hour output (found and fixed: the UPDATED column's fixed width, hardcoded to 19 chars for the old `%H:%M:%S` output, needed widening to 22 chars for the longer 12-hour-with-AM/PM output, in 4 places in `ui.rs` - the Agents tab, Logs tab, Reminders tab, and the details modal's Logs pane)
- [x] 3.2 Verified via a throwaway integration test driving the real agentd+agentmon pipeline (real daemon on an isolated socket, real wire protocol, real `App`, real `ui::render` to a `TestBackend`) that the Agents tab's UPDATED column and a running reminder's ETA both render in 12-hour format with AM/PM (e.g. `2026-10-16 8:32:33 AM`, `ETA 8:42 AM`), with no truncation; deleted the scratch test file afterward per the project's manual-testing convention
