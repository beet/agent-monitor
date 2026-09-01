## Context

See proposal.md - Why. The hook-to-status mapping lives in `crates/agentmon-report/src/hook_payload.rs::status_for_payload`, and the installed hook set lives in `crates/agentmon-report/src/install_hooks.rs::HOOK_EVENTS`. Both were last touched when `UserPromptSubmit` was added (see `openspec/changes/archive/2026-08-28-agent-monitor-tui/design.md`) specifically so status could return to "running" - but only via a freshly typed prompt. `PreToolUse` is a existing Claude Code hook (fires immediately before any tool invocation) that was never wired up.

## Goals / Non-Goals

**Goals:**
- Give the daemon a "running" signal that fires on resumption regardless of what resolved the "needs input" state (new prompt or an approved permission/elicitation prompt).
- Keep the fix minimal: no changes to notification behavior, the done/needs-input ordering guard, or the registry's transition logic - `PreToolUse` reuses the existing `Running` status path exactly like `UserPromptSubmit` does today.

**Non-Goals:**
- Don't attempt to catch resumption when Claude continues without invoking any tool (e.g., pure text generation after an elicitation dialog). `PreToolUse` is the best available signal in Claude Code's current hook set; a turn that resolves without any further tool call still won't show "running" until `Stop`, but this is a strict improvement over never recovering until `Stop` at all, and no worse than today's behavior in that case.
- Don't change what counts as "needs input" - the `Notification` matcher and its notification-type list are untouched.

## Decisions

- **Use `PreToolUse` rather than `PostToolUse`.** `PreToolUse` fires the moment Claude commits to running a tool, which is the earliest and clearest "work has resumed" signal. `PostToolUse` would report resumption only after the tool already finished, understating how long the session was actually active.
- **No matcher restriction on the `PreToolUse` hook.** Unlike `Notification` (which is matcher-restricted to only the needs-input notification types to avoid firing the reporter on irrelevant notifications), every `PreToolUse` invocation is relevant here - each one means the agent is actively working, which is exactly the "running" signal wanted.
- **Map unconditionally to `Running`, with no new registry guard.** `PreToolUse` events pass through `Registry::upsert` exactly like existing `UserPromptSubmit`-derived `Running` events: they overwrite any prior status unconditionally, including "needs input" and "done". This matches how `Running` already behaves today (see the existing "A new running event still clears a completed session's status" scenario) and needs no new special-casing.

## Risks / Trade-offs

- [`PreToolUse` fires on every single tool call, not just the first one after resumption] → Acceptable: repeated `Running` events for an already-`Running` session are inert no-ops (no notification, status unchanged besides the timestamp), matching the existing behavior of repeated same-status events.
- [Existing installs won't get the new hook until `install-hooks` is re-run] → Documented in proposal.md's Impact section; `install_hooks::upsert_hook` is already idempotent per-event, so re-running is safe and only adds the missing `PreToolUse` entry without touching the other three.
