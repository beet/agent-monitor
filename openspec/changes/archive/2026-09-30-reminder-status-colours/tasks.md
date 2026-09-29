## 1. Fix Agents tab reminder segment coloring

- [x] 1.1 In `crates/agentmon/src/ui.rs`, update `reminder_agents_column_segment`'s `ReminderStatus::Running` arm so the name and elapsed/ETA spans are colored `Color::Blue` (matching the emoji), not just the leading `"⏳ "` span
- [x] 1.2 In the same function's `ReminderStatus::Done` arm, color the name and elapsed/outcome spans `Color::Green` (matching the emoji), not just the leading `"✅ "` span
- [x] 1.3 Verify visually: run the TUI with a running and a done reminder present and confirm each Agents-tab REMINDERS segment reads as one uniformly-colored blue/green unit, matching the coloring already seen on a running/passed Tests cell and a done Agents cell in the same row

## 2. Test coverage

- [x] 2.1 Add/extend a unit test in `crates/agentmon/src/ui.rs` asserting that every `Span` in a running reminder's `reminder_agents_column_segment` output (emoji, name, and duration/ETA text) has `fg == Some(Color::Blue)`
- [x] 2.2 Add/extend a unit test asserting that every `Span` in a done reminder's `reminder_agents_column_segment` output (emoji, name, and duration/outcome text) has `fg == Some(Color::Green)`
- [x] 2.3 Run `cargo test -p agentmon` and verify all tests pass, including the existing `agents_tab_reminders_column_*` tests
