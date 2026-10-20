## 1. Notifier trait and implementation

- [x] 1.1 Change the `Notifier` trait in `crates/agentd/src/notify.rs` so `notify` takes an additional `project_name: &str` parameter, and `notify_test_run`/`notify_reminder` replace their `cwd: &Path` parameter with `project_name: &str`; verify the crate still compiles once call sites (task group 2) are updated.
- [x] 1.2 Update `notification_script`, `test_run_notification_script`, and `reminder_notification_script` to take `project_name: &str` and use it directly in place of the `cwd.file_name()...` computation; verify `cwd`/`Path` is no longer imported or referenced anywhere in `notify.rs` outside of `#[cfg(test)]` sample data that still needs a `PathBuf` for `AgentInfo.cwd`.
- [x] 1.3 Update the `OsaScriptNotifier` impl methods to thread `project_name` through to the script-building functions; verify with `cargo build -p agentd`.

## 2. Resolve the project name at call sites

- [x] 2.1 In `crates/agentd/src/ingest.rs`'s `ingest_event`, pass `&self.registry.project_name(&outcome.agent.cwd)` as the new `notify` argument; verify `cargo build -p agentd` succeeds.
- [x] 2.2 In `ingest_test_run`, resolve `self.registry.project_name(&test_run.cwd)` into a local and pass it to `notifier.notify_test_run`; verify `cargo build -p agentd` succeeds.
- [x] 2.3 In `ingest_reminder_completed`, pass `&self.registry.project_name(&reminder.cwd)` as the new `notify_reminder` argument; verify `cargo build -p agentd` succeeds.

## 3. Tests

- [x] 3.1 Update `notify.rs`'s existing unit tests to call the script-building functions with an explicit `project_name: &str` (e.g. `"enclaudinate"`, `"project"`) instead of a `cwd`/`PathBuf`, keeping their existing assertions on sound names and emoji prefixes; verify `cargo test -p agentd notify` passes.
- [x] 3.2 Add a unit test asserting that when a custom name differs from the directory-derived default, the notification body/text contains the custom name and not the raw directory name, for at least one of `notification_script`, `test_run_notification_script`, or `reminder_notification_script`; verify the new test passes and fails if `project_name` is replaced with the raw `cwd` basename.
- [x] 3.3 Run the full workspace test suite (`cargo test --workspace`) and verify it passes.

## 4. Spec sync

- [x] 4.1 Run `openspec validate --change fix-notification-project-name --strict` and verify it reports no errors.
