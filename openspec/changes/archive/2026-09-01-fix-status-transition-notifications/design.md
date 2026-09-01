## Context

See proposal.md - Why for the two bugs this fixes. Relevant current implementation:

- `crates/agentd/src/ingest.rs` `should_notify(previous, current)` fires only when `current` is `Done`/`NeedsInput` **and** `previous != Some(current)` - a same-status repeat never notifies.
- `crates/agentd/src/registry.rs` `Registry::upsert` is last-write-wins: whatever event arrives last simply overwrites the stored status, with no notion of valid/invalid transitions.
- Each hook fire (`UserPromptSubmit`, `Stop`, `Notification`) is delivered over its own one-shot socket connection, handled on its own server thread (`crates/agentd/src/server.rs`). `AgentEvent` carries no timestamp or sequence number, so nothing orders two events for the same session relative to each other - the daemon only sees the order its threads happen to acquire the registry's mutex in.
- Claude Code does not report when a permission prompt is resolved (no hook fires on "user clicked Allow"), so a session can go `NeedsInput` → (silently resolved) → `NeedsInput` again without ever passing back through `Running`.

## Goals / Non-Goals

**Goals:**
- Every distinct "needs input" prompt reported by a hook produces a notification, regardless of what the agent's previous status was.
- A `NeedsInput` event that arrives after a session is already `Done` cannot change its status or produce a notification.

**Non-Goals:**
- Introducing event ordering/sequencing (timestamps, sequence numbers) across hook connections. The `Done`-is-sticky-until-`Running` guard sidesteps the race without needing to know which event actually fired first.
- Deduplicating genuinely repeated `NeedsInput` hook deliveries for the *same* unresolved prompt (e.g. a hook retry). Claude Code has no per-prompt id in its hook payload to distinguish that from a new prompt, and silently dropping a real prompt (today's bug) is worse than an occasional extra notification for the rare duplicate delivery.

## Decisions

**Decision: Stop suppressing repeat `NeedsInput` notifications; keep suppressing repeat `Done` notifications.**
`should_notify` changes from "notify on Done/NeedsInput when status changed" to "notify on NeedsInput unconditionally, and on Done only when status changed". Rationale: `Running` is the only event that resets status between prompts, but permission grants don't emit `Running`, so two prompts in a row look identical to the daemon today and the second is dropped. `Done` doesn't have this problem - a real second `Stop` for the same turn without an intervening `Running` is a duplicate hook delivery, not a new event, so its existing dedupe is left alone.
Alternative considered: plumb a per-prompt identifier through `HookPayload` → `AgentEvent` and dedupe on that instead of on status. Rejected - Claude Code's `Notification` payload doesn't include a stable per-prompt id across all five `NEEDS_INPUT_NOTIFICATION_TYPES`, so this would require guessing at a synthetic key, adding complexity for no real gain over "just always notify".

**Decision: Guard the registry so `NeedsInput` cannot overwrite `Done`; `Running` still can.**
`Registry::upsert` (or a check just before it, in `Ingestor::ingest_event`) drops a `NeedsInput` event when the currently stored status for that session is `Done`. `Running` and other statuses continue to overwrite `Done` normally, since a fresh `UserPromptSubmit` legitimately starts a new turn. This is a semantic transition rule, not a timing fix: it doesn't matter whether the late `idle_prompt` notification actually arrives before or after `Stop` on the wire - once the daemon has recorded `Done`, only `Running` is treated as a legitimate way out of it.
Alternative considered: attach a timestamp or sequence number to each `AgentEvent` and have the registry ignore events older than the current record. Rejected as more invasive (touches the wire protocol between `agentmon-report` and `agentd`, and every existing test that constructs `AgentEvent`) for no behavioral difference from the simpler transition guard in the one case that actually occurs in practice.

## Risks / Trade-offs

- [Risk] A genuine duplicate `NeedsInput` hook delivery for the same unresolved prompt now notifies twice instead of once. → Accepted: this is strictly better than the current failure mode of dropping a real second prompt, and duplicate hook deliveries are not the reported problem.
- [Risk] If a future legitimate flow needs `NeedsInput` to arrive for a session already marked `Done` (e.g. a retroactive correction), the guard silently drops it. → Mitigation: the guard only applies to the `Done` → `NeedsInput` edge; every other transition, including `Done` → `Running`, is unaffected, so a real new turn is never blocked.
