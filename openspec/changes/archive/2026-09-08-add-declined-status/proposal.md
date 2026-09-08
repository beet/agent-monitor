## Why

Claude Code fires a dedicated `PermissionDenied` hook when a user declines a permission prompt, but the daemon doesn't listen for it. Today that leaves a session stuck showing "needs input" indefinitely whenever the next thing Claude does is a plain-text reply with no further tool call (nothing else ever fires to move it off that status).

## What Changes

- Daemon maps the `PermissionDenied` hook event to a new, distinct `AgentStatus::Declined`, instead of leaving the status on "needs input".
- TUI gains a visual treatment (emoji + style) for the "declined" status, consistent with the existing status-distinguishing scheme.
- No macOS notification fires for a transition to "declined" - the user just took that action themselves, so there's nothing to alert them about.
- No running-duration is shown for "declined" (duration display is running-only, already spec'd that way - no change needed there).

## Non-goals

- Fixing the related but separate case where a user cancels/interrupts Claude mid-turn (e.g. Escape key). Claude Code emits no hook event at all for that action - `Stop` explicitly does not fire on user interrupts - so there is no signal the daemon could listen for. This is a known, accepted limitation and out of scope for this change; no heuristic/timeout-based workaround will be added.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-daemon`: adds a new requirement mapping the `PermissionDenied` hook event to the "declined" status, and extends the completion-notifications requirement to confirm no notification fires for it.
- `agent-monitor-tui`: extends the status-visually-distinguishable requirement with an emoji/style for "declined".

## Impact

- `crates/agentmon-proto`: `AgentStatus` gains a `Declined` variant.
- `crates/agentmon-report/src/hook_payload.rs`: map hook_event_name `"PermissionDenied"` to `AgentStatus::Declined`.
- `crates/agentd/src/notify.rs`: no notification for "declined" (falls out naturally from the existing done/needs-input-only check; add a test confirming it).
- `crates/agentmon/src/ui.rs`: add emoji/style for "declined" in `status_label_and_style`.
