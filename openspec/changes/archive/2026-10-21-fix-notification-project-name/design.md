## Context

`crates/agentd/src/notify.rs`'s three script-building functions (`notification_script`, `test_run_notification_script`, `reminder_notification_script`) each independently compute a `project` string via `cwd.file_name().map(...).unwrap_or_else(|| cwd.display().to_string())`. None of them have access to `Registry`, and `Notifier`'s trait methods only ever receive a raw `cwd: &Path` (or an `AgentInfo` whose `cwd` field is the raw path) - never a resolved display name. `Registry::project_name(&self, cwd: &Path) -> String` (`crates/agentd/src/registry.rs`) already implements exactly the right resolution (custom name if set, else the directory-derived default) and is already called from every `Ingestor` method that needs an effective name (e.g. `ingest_rename_project`'s collision check). All three notifier call sites live in `crates/agentd/src/ingest.rs`'s `Ingestor` methods, which already hold `self.registry`.

See proposal.md for why this matters; see `specs/project-renaming/spec.md`'s new "Desktop notifications use the effective display name" scenario for the behavior contract.

## Goals / Non-Goals

**Goals:**
- Notification text for agent, test-run, and reminder notifications reflects the project's effective display name (custom name if set, else directory-derived default).
- No change to `agentmon-proto` wire types - this is purely a daemon-internal fix.

**Non-Goals:**
- No change to how `Registry::project_name` itself resolves names, or to the rename/validation flow.
- No change to notification sound selection, emoji, or timing - only the project-name substring.

## Decisions

**Resolve the name at the `ingest.rs` call site, not inside `notify.rs`.**

`Ingestor` already has `self.registry` in scope at all three call sites (`ingest_event`, `ingest_test_run`, `ingest_reminder_completed`). Resolving there and passing a plain `&str` into `Notifier` keeps `notify.rs` free of any dependency on `Registry` - it stays a pure "render notification text" module, matching how `crates/agentmon/src/ui.rs` already resolves `app.project_name(cwd)` before handing a display string to rendering code, rather than threading a registry into the renderer.

Alternative considered: thread a `Registry` (or `Arc<Registry>`) reference into `OsaScriptNotifier` and have `notify.rs` call `project_name` itself. Rejected - it would make every `Notifier` implementation (including test doubles) depend on `Registry`, for no benefit over resolving once at the call site.

**Change the `Notifier` trait to take a resolved project name instead of a raw `cwd`.**

```rust
pub trait Notifier: Send + Sync {
    fn notify(&self, agent: &AgentInfo, project_name: &str);
    fn notify_test_run(&self, _project_name: &str, _status: TestRunStatus) {}
    fn notify_reminder(&self, _project_name: &str, _name: &str) {}
}
```

`agent: &AgentInfo` is kept on `notify` (it's still needed for `status`, `host_context`), but the `project` string inside `notification_script` is now a parameter instead of being recomputed from `agent.cwd`. `notify_test_run` and `notify_reminder` drop their `cwd: &Path` parameter entirely in favor of `project_name: &str`, since the project name was the only thing either function used `cwd` for.

Alternative considered: keep `cwd: &Path` on the trait and add a second `project_name: &str` parameter alongside it. Rejected - `cwd` would become dead weight on every implementor once `notify.rs` no longer needs it for display, and no other use of `cwd` exists in that module.

Call sites become:
```rust
// ingest_event
self.notifier.notify(&outcome.agent, &self.registry.project_name(&outcome.agent.cwd));

// ingest_test_run
let name = self.registry.project_name(&test_run.cwd);
self.notifier.notify_test_run(&name, status);

// ingest_reminder_completed
self.notifier.notify_reminder(&self.registry.project_name(&reminder.cwd), &reminder.name);
```

## Risks / Trade-offs

- [Existing `notify.rs` unit tests construct `AgentInfo`/raw `cwd`/`&Path` arguments directly and assert on the directory-derived text] → Tests update to pass an explicit `project_name: &str` (e.g. `"enclaudinate"`, `"project"`) instead of relying on path-derived defaults; this is strictly more direct and doesn't reduce coverage.
- [A future notifier call site might be added that forgets to resolve through `Registry::project_name` and passes a raw path again] → The trait signature change itself is the guard: `notify_test_run`/`notify_reminder` no longer accept a `Path` at all, so passing a raw `cwd` would not type-check: a caller is forced to produce a `&str`, which the existing `self.registry.project_name(...)` call is the natural way to get.
