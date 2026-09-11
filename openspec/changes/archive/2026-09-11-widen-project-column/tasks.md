## 1. Layout change

- [x] 1.1 In `crates/agentmon/src/ui.rs`, change the `widths` array in `render_agent_table` from `[Constraint::Length(20), Constraint::Fill(1), Constraint::Length(19)]` to `[Constraint::Fill(2), Constraint::Fill(3), Constraint::Length(19)]` and verify `cargo build -p agentmon` succeeds
- [x] 1.2 Run the TUI (or a manual/snapshot check) against project names longer than 20 characters at a few terminal widths and verify PROJECT renders more than 20 characters on wide terminals while STATUS remains readable

## 2. Verification

- [x] 2.1 Run `cargo test -p agentmon` and confirm the existing test suite still passes
