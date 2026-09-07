## Context

See proposal.md - Why. `status_for_payload` (`crates/agentmon-report/src/hook_payload.rs`) and `HOOK_EVENTS` (`crates/agentmon-report/src/install_hooks.rs`) already map `PreToolUse` to "running" per the prior `fix-needs-input-resume-status` change (see `openspec/changes/archive/2026-09-01-fix-needs-input-resume-status/design.md`). That design explicitly chose `PreToolUse` over `PostToolUse` on the reasoning that `PreToolUse` is "the earliest and clearest 'work has resumed' signal," and accepted as a non-goal that "a turn that resolves without any further tool call still won't show running until Stop."

In practice that non-goal is the dominant path, not an edge case: `PreToolUse` fires *before* the permission/elicitation check that can raise "needs input" for that same tool call, so once the prompt appears there is no later `PreToolUse` for it - only `PostToolUse`, once the call actually finishes, or the turn's `Stop`. A single tool call can also raise multiple prompts in sequence (e.g. several elicitation dialogs within one call) with no `PreToolUse` between them at all. The net effect matches the reported bug: a session that reaches "needs input" stays there - re-notifying on each subsequent prompt - until it jumps straight to "done".

## Goals / Non-Goals

**Goals:**
- Give the daemon a "running" signal that fires as soon as the specific tool call responsible for a "needs input" prompt finishes, regardless of how many prompts it raised or whether Claude happens to invoke another tool afterward.
- Keep the fix additive to the existing mapping: `PostToolUse` reuses the same unconditional "running" transition as `PreToolUse` and `UserPromptSubmit` (see the existing "A new running event still clears a completed session's status" / "clears a needs-input session" behavior in `crates/agentd/src/registry.rs`) - no new registry states or guards.

**Non-Goals:**
- Don't try to report "running" *during* the paused tool call, only once it resolves - Claude Code's hook set has no "prompt answered, still executing" event, and `PostToolUse` is the earliest point after resumption that is actually observable.
- Don't change what counts as "needs input" or the existing done/needs-input ordering guard - the `Notification` matcher and the done-blocks-needs-input rule are untouched.

## Decisions

- **Add `PostToolUse` alongside `PreToolUse`, not instead of it.** `PreToolUse` remains correct and useful for the case it already covers well: a session that finished a turn (or was otherwise idle) and starts a brand new tool call. `PostToolUse` covers the complementary case: the specific call that raised a "needs input" prompt finishing after the prompt resolves. Both map to the same "running" status, so having both fire back-to-back for a normal (non-paused) tool call is an inert no-op, matching how repeated same-status events already behave.
- **No matcher restriction on `PostToolUse`, mirroring `PreToolUse`.** Every `PostToolUse` invocation means the agent is actively past a tool call, which is exactly the "running" signal wanted; unlike `Notification`, there's no need to filter by tool name or outcome.
- **Map unconditionally to `Running`, with no new registry guard.** Same reasoning as the original `PreToolUse` decision: `PostToolUse`-derived `Running` events overwrite any prior status (including "needs input" and "done") exactly like existing `Running` events, needing no special-casing in `Registry::upsert`.

## Risks / Trade-offs

- [`PostToolUse` fires on every tool call, doubling the already-frequent `Running` events from `PreToolUse`] → Acceptable: repeated `Running` events for an already-`Running` session are inert no-ops (no notification, status unchanged besides the timestamp), same as today.
- [A tool call that never completes (denied permission, crashes, hangs) never fires `PostToolUse`, so the session can still stay "needs input" until `Stop`] → Accepted as an inherent limit of the available hook set; this is strictly better than the current behavior, not a regression, and `Stop` remains the eventual backstop.
- [Existing installs won't get the new hook until `install-hooks` is re-run] → Same as the prior fix; `install_hooks::upsert_hook` is already idempotent per-event, so re-running only adds the missing `PostToolUse` entry without touching the other four.
