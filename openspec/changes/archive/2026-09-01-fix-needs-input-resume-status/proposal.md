## Why

Once a tracked session transitions to "needs input" because of a mid-turn `Notification` (a permission prompt, an elicitation dialog, etc.), it never transitions back to "running" when the user resolves that prompt and Claude resumes work - it stays "needs input" until the turn's `Stop` event finally reports "done". This is because the only hook currently wired to produce a "running" status, `UserPromptSubmit`, fires when the user types a new chat message, not when they resolve a permission/elicitation prompt through the CLI's own UI. The daemon has no signal for "processing resumed" in that case, so agentmon keeps showing a stale "NEEDS INPUT" for the rest of the turn even while the agent is actively working.

## What Changes

- Wire the `PreToolUse` hook (fires whenever Claude Code is about to invoke a tool) into `agentmon-report`'s installed hook set, mapped to the "running" status.
- This gives the daemon a resumption signal that doesn't depend on how the "needs input" state was resolved: a fresh chat message (`UserPromptSubmit`, already handled) or an approved permission/elicitation prompt (newly handled via the next tool invocation).
- No change to the "needs input" or "done" mappings, and no change to the existing done/needs-input ordering guard.
- Add a Mermaid state diagram to `README.md` documenting the hook-driven status state machine (`Running` / `NeedsInput` / `Done` / `Stale`), including the new `PreToolUse` resumption edge, so the hook-to-status mapping is discoverable without reading the source.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: the registry's hook-to-status mapping now includes `PreToolUse` as an additional source of "running" transitions, so a session can leave "needs input" when work resumes, not only when a new prompt is submitted.

## Impact

- `crates/agentmon-report/src/hook_payload.rs`: `status_for_payload` gains a `PreToolUse` -> `Running` mapping.
- `crates/agentmon-report/src/install_hooks.rs`: `HOOK_EVENTS` gains a `PreToolUse` entry so it's installed into `settings.json` (existing installs are updated the next time `install-hooks` runs).
- `crates/agentd`: no code changes expected - `PreToolUse` events flow through the existing `AgentEvent` / registry / notify path unchanged, since "running" already has no special notification or ordering behavior.
- Existing installations need to re-run the hook installer to pick up the new `PreToolUse` hook entry.
- `README.md`: gains a "How it works" section with a Mermaid `stateDiagram-v2` of the status state machine.
