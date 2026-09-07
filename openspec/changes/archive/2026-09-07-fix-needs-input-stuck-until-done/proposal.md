## Why

A prior fix ([`fix-needs-input-resume-status`](../archive/2026-09-01-fix-needs-input-resume-status/proposal.md)) wired `PreToolUse` to the "running" status so a session could leave "needs input" once Claude resumed work, not only on a fresh `UserPromptSubmit`. In practice this only helps for the *first* prompt in a turn: `PreToolUse` fires once, before the tool call that itself raises the permission/elicitation prompt, and it does not fire again while that same tool call is paused on the prompt or after the prompt is resolved. A tool call can also raise several prompts in sequence (e.g. multiple elicitation dialogs within one call) with no intervening tool invocation at all. In both cases nothing reports "running" again until the whole turn's `Stop` event, so a session that has entered "needs input" effectively stays there - repeatedly re-notifying on each subsequent prompt - until it flips straight to "done". This is the behavior the design doc's own non-goal anticipated ("a turn that resolves without any further tool call still won't show running until Stop") but it turns out to be the common case, not an edge case, defeating the intent of the original fix.

## What Changes

- Wire the `PostToolUse` hook (fires immediately after any tool invocation completes, regardless of what happened during it - permission prompts, elicitation dialogs, or none) into `agentmon-report`'s installed hook set and its hook-to-status mapping, alongside the existing `PreToolUse` -> "running" mapping.
- `PostToolUse` closes the gap `PreToolUse` cannot: it fires once the tool call that raised a "needs input" prompt has actually finished, so the session reports "running" again as soon as that pause ends, rather than waiting for a later distinct tool call's `PreToolUse` or the turn's `Stop`.
- No change to the "needs input" or "done" mappings, the done/needs-input ordering guard, or the registry's transition logic - `PostToolUse` reuses the existing unconditional "running" transition exactly like `PreToolUse` and `UserPromptSubmit` do today.
- Update the Mermaid state diagram in `README.md` to include the new `PostToolUse` resumption edge.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: the registry's hook-to-status mapping now also treats `PostToolUse` as a "running" transition, so a session leaves "needs input" as soon as the tool call that raised the prompt finishes, not only on the next distinct tool invocation or a fresh prompt.

## Impact

- `crates/agentmon-report/src/hook_payload.rs`: `status_for_payload` gains a `PostToolUse` -> `Running` mapping.
- `crates/agentmon-report/src/install_hooks.rs`: `HOOK_EVENTS` gains a `PostToolUse` entry so it's installed into `settings.json` (existing installs are updated the next time `install-hooks` runs).
- `crates/agentd`: no code changes expected - `PostToolUse` events flow through the existing `AgentEvent` / registry / notify path unchanged, since "running" already has no special notification or ordering behavior.
- Existing installations need to re-run the hook installer to pick up the new `PostToolUse` hook entry.
- `README.md`: the state-diagram section gains the `NeedsInput -> Running` / `Done -> Running` edges via `PostToolUse`.
