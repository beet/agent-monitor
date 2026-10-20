## Why

Desktop notifications sent by `agentd` (agent status, test-run, and reminder notifications) display the raw working-directory basename instead of a project's custom display name. A user who renames a project via `RenameProject` still sees the old directory-derived name in every notification banner, even though the TUI and every other client-facing surface correctly show the custom name. This silently violates the existing `project-renaming` spec's requirement that a custom name be used "everywhere the daemon or a connected client refers to" the project.

## What Changes

- Notification composition in `crates/agentd/src/notify.rs` (`notification_script`, `test_run_notification_script`, `reminder_notification_script`) stops deriving the displayed project text from `cwd.file_name()` and instead resolves it through `Registry::project_name(cwd)`, so a custom name (if one is set) is used, falling back to the existing directory-derived default otherwise.
- The `Notifier` trait methods gain access to the resolved project name (either by taking a pre-resolved `&str` from the call site in `crates/agentd/src/ingest.rs`, or by threading a `Registry` reference into `notify.rs` - left to design.md) so every call site in `ingest.rs` passes the registry-resolved name rather than a raw `cwd`.
- Add an explicit scenario to the `project-renaming` spec calling out notification text as one of the "everywhere" surfaces a custom name must apply to, so this surface can't silently regress again.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `project-renaming`: Add a scenario making explicit that desktop notification text (agent status, test-run, and reminder notifications) must use a project's effective display name (custom name if set, otherwise the directory-derived default), not the raw working-directory path.

## Impact

- `crates/agentd/src/notify.rs`: all three notification-composition functions change their inputs and no longer compute `project` from `cwd.file_name()` directly.
- `crates/agentd/src/ingest.rs`: the three notifier call sites (`notify`, `notify_test_run`, `notify_reminder`) change to supply a registry-resolved project name.
- `openspec/specs/project-renaming/spec.md`: one new scenario under "Custom project display name" (or a new requirement, per design.md) naming notifications explicitly.
- No wire-protocol (`agentmon-proto`) changes - `AgentInfo`/`TestRunInfo`/`ReminderInfo` keep their existing `cwd` fields; only the daemon-local notification path changes.
