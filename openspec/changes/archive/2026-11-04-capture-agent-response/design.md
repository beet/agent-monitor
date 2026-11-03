## Context

See proposal.md - Why. The approach turned out simpler than it first looked:

- **Where the data lives.** The initial plan was to have `agentmon-report` read the session's transcript file (`~/.claude/projects/<cwd-slug>/<session-id>.jsonl`) directly, since that's where a turn's messages are durably recorded. Inspecting the Claude Code binary's own embedded schema definitions (via `strings` on the installed CLI) during implementation showed this is unnecessary: the `Stop` hook's JSON payload already carries a `last_assistant_message: Option<String>` field, documented in Claude Code's own schema as existing specifically "to avoid the need to read and parse the transcript file." Claude Code has already resolved "which message, excluding sidechain/sub-agent turns" on our behalf.
- **Where extraction happens.** Still `agentmon-report` (the hook subprocess) - it already parses the `Stop` payload's other fields (`session_id`, `cwd`, etc.) via `HookPayload`; `last_assistant_message` is just one more field on that same struct. `agentd` stays a dumb event-applier, untouched by this change.
- **The hook-process constraint already on record.** `report.rs` states outright: "Hook subprocesses must never block or fail the calling hook." With no file I/O involved, this constraint is satisfied by construction rather than by a bounded-read algorithm - there's nothing for this feature to block on.

## Goals / Non-Goals

**Goals:**
- Capture the final assistant message's text at `Stop` time and carry it through to a "done" log entry, viewable the same way a prompt entry's full text is viewable today.
- Degrade to no response (never an error, never a blocked hook) when the payload carries none.

**Non-Goals (locked in during exploration):**
- Capturing anything beyond the final assistant message - no aggregation of every assistant aside across a multi-tool-call turn.
- A persistent `AgentInfo.last_response` / "Last Response" pane mirroring `last_prompt`'s carried-forward semantics. The response lives only as a snapshot on its "done" log entry.
- Any of the other transcript fields surfaced during exploration - `ai-title`, `turn_duration`/`messageCount`, `away_summary`, per-turn token/cost usage. Each is independently interesting but out of scope here.

## Decisions

**Read `last_assistant_message` straight off the `Stop` payload - no file access.** Confirmed present (via the installed CLI's own embedded schema strings) as `last_assistant_message: Option<String>` on the `Stop` hook's JSON input, already holding exactly "the final assistant message's text," already excluding sidechain/sub-agent turns. This replaces the originally-planned transcript-file read entirely: `HookPayload` gains one new optional string field, parsed the same way every other field on it already is.

**`response` on `AgentEvent` mirrors `prompt`'s shape, not its persistence.** `prompt: Option<String>` is populated only for `UserPromptSubmit`; `response: Option<String>` is populated only for `Stop`. But `prompt` also feeds `AgentInfo.last_prompt`, a registry field carried forward and re-stamped onto every later entry. `response` has no registry counterpart by design (see Non-Goals) - `ingest_event` reads the triggering event's `response` directly into the "done" `LogEntry` it is already recording, with no new field on `AgentInfo` or `Registry`.

**4,000-character cap, same as last-prompt text.** The existing last-prompt truncation (`agent-daemon` spec, "Agent registry") exists so one entry can't grow unbounded in the daemon's in-memory, 500-entry-capped activity log. A captured response is exactly the same kind of free-text, same storage, same risk - so it gets the same cap, applied in `build_event` right after reading `payload.last_assistant_message`.

**No new sensitivity category.** `add-prompt-visibility`'s design.md already accepted that free-text prompt content sits in the same local, in-memory, no-new-persistence, no-new-network trust boundary as everything else the daemon tracks. A captured response is the same trust boundary, just a second field - not a new exposure to reason about.

## Risks / Trade-offs

- **[Risk]** `last_assistant_message` is confirmed present via the installed CLI's own schema strings, but it is an undocumented implementation detail of Claude Code's hook payload, not a published, versioned contract - a future release could rename or drop it. **Mitigation:** the field is already `Option`-shaped by Claude Code itself and parsed as optional on our side too; its absence degrades to "no response," never a hook failure or crash - the feature silently stops working rather than breaking anything, and is easy to notice and fix.
- **[Risk]** Claude Code's own doc comment doesn't specify exactly how `last_assistant_message` behaves in edge cases (e.g. a turn that ends with only tool calls and no trailing text). **Mitigation:** treated as "no response" whenever the field is absent or empty after trimming, consistent with how an absent `prompt` is already handled - no new failure mode, just an `Option` that's sometimes `None`.

## Migration Plan

Purely additive - a new `Option` field on `AgentEvent` and `LogEntry`, both already `serde`-derived with other optional fields following the same pattern. No persisted state anywhere (the activity log is in-memory and already resets on daemon restart), so there is nothing to backfill or migrate. Older daemon/report binaries and newer ones are not required to interoperate (this is a single-user local tool restarted together), so no wire-compatibility shimming is needed.
