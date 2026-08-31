## Why

Completion notifications currently play no sound (or whatever the system default is) regardless of whether an agent finished successfully or is waiting on input. The user has to glance at the banner to tell which happened. Distinct built-in macOS sounds per status let the user tell them apart by ear alone.

## What Changes

- `agentd`'s notifier plays a specific built-in macOS notification sound based on the agent's status: `Glass` when a tracked agent transitions to "done", `Ping` when it transitions to "needs input".
- The sound is fixed per status (not user-configurable in this change).

## Capabilities

### Modified Capabilities
- `agent-daemon`: the "Completion notifications" requirement now specifies that the notification includes a status-specific system sound (`Glass` for done, `Ping` for needs input) instead of no sound.

## Impact

- `crates/agentd/src/notify.rs`: `OsaScriptNotifier::notify` gains a `sound name` clause in the generated AppleScript, chosen from the agent's status.