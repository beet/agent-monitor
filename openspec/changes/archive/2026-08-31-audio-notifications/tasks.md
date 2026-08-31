## 1. Sound mapping

- [x] 1.1 Add a function mapping `AgentStatus` to its notification sound name (`Done` → `"Glass"`, `NeedsInput` → `"Ping"`) in `crates/agentd/src/notify.rs`, with a unit test asserting both mappings
- [x] 1.2 Update `OsaScriptNotifier::notify` to append a `sound name <name>` clause to the generated `display notification` AppleScript, and update `applescript_literal`/script formatting as needed so the sound name is safely interpolated

## 2. Verification

- [x] 2.1 Update/extend the existing `notify.rs` tests to cover the new sound clause in the generated script for both `Done` and `NeedsInput`, and confirm `cargo test -p agentd` passes
- [x] 2.2 Manually run `agentd` locally, trigger a "done" and a "needs input" transition, and confirm Glass and Ping play respectively
