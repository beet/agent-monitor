## Context

See `proposal.md` for motivation. Relevant current shapes:

- `crates/agentmon-report/src/hook_payload.rs`'s `HookPayload` parses `session_id`, `cwd`, `hook_event_name`, `notification_type` and discards everything else, including `UserPromptSubmit`'s `prompt` field.
- `agentmon_proto::AgentEvent` (hook report -> daemon) and `AgentInfo` (daemon -> clients) both lack any prompt field today.
- `crates/agentd/src/registry.rs`'s `Registry::upsert` builds a fresh `AgentInfo` from each `AgentEvent` plus the previous entry (for carried-over fields like `status_since_ms`/`run_started_ms`); a same-pid replacement builds an entirely fresh entry rather than inheriting from the one it replaces.
- The activity log (`LogEntry`) already freezes point-in-time data onto an entry (`reminder_name`, `reminder_due_at_ms`); the same pattern applies to a prompt snapshot.
- Per `CLAUDE.md`, the README's data-model Mermaid diagram must be updated byte-for-byte for any `agentmon-proto` field change.
- Two decisions were confirmed with the user directly (see conversation): the Last Prompt pane shows the most-recently-updated agent's prompt when a project tracks several, and the pane is a thin fixed strip carved out of the Logs pane's share rather than shrinking the top third.

## Goals / Non-Goals

**Goals:**
- Capture `UserPromptSubmit`'s prompt text end to end: hook -> `agentmon-report` -> daemon registry -> `AgentInfo`/`LogEntry` -> TUI.
- Bound memory/storage growth from arbitrarily large pasted prompts.
- Keep the existing modal layout's other behavior (focus cycling, pagination, Agents/Tests/Reminders panes) unchanged except for the space given up to the new pane.

**Non-Goals:**
- No persistence across daemon restart - last-prompt text is in-memory only, consistent with every other piece of agent/test-run/reminder state.
- No history of multiple past prompts per agent beyond what's already frozen onto individual activity-log entries; the registry tracks only the single current "last prompt" per agent.
- No change to which hook events are registered or to `install_hooks.rs`'s matcher - `UserPromptSubmit` is already installed with no matcher restricting it.

## Decisions

**Truncation cap: 4,000 characters, applied once in the registry.** `Registry::upsert` is the single point that writes `last_prompt`, so capping there (rather than in `agentmon-report` or at display time) means every consumer - `AgentInfo`, and any `LogEntry` snapshot taken from it - already sees a bounded value, and the cap can't be bypassed by a future second writer. 4,000 characters comfortably covers a few paragraphs of typed instruction (the TUI display truncates far more aggressively than this anyway) while keeping one agent's entry, and one log entry, from growing to megabytes if a huge file or diff gets pasted into a prompt. Truncate by byte-safe `char` boundary, not raw bytes, to avoid splitting a multi-byte UTF-8 character.

**`AgentEvent.prompt` is `Option<String>`, populated only for `UserPromptSubmit`.** `agentmon-report`'s existing `status_for_payload` dispatch already distinguishes hook event types; the binary that builds `AgentEvent` from `HookPayload` sets `prompt: payload.prompt.clone()` when `hook_event_name == "UserPromptSubmit"` (where `HookPayload.prompt: Option<String>` is a new field), and `None` otherwise. This keeps the "only `UserPromptSubmit` carries a prompt" rule in one place rather than re-deriving it in the registry.

**Carry-forward, not clear-on-silence, semantics in the registry.** `Registry::upsert` sets `last_prompt` from the incoming event only when the event's `prompt` is `Some`; when `None`, it copies the previous entry's `last_prompt` forward unchanged (mirroring how `status_since_ms`/`run_started_ms` conditionally carry over today). A same-pid replacement entry starts with `last_prompt: None` regardless, matching how every other field on a replacement entry is fresh rather than inherited (see "Agent registry" spec delta) - a new session id is a new agent identity, not a continuation.

**`LogEntry.prompt: Option<String>` is set from the agent's *current* `last_prompt` at the moment a "started"/"done"/"needs input" entry is recorded, not from the triggering event's own `prompt` field.** This matters because a "started" entry is often produced by a resumption (`PreToolUse`/`PostToolUse` after a needs-input prompt clears) that carries no prompt of its own; snapshotting the registry's current value - rather than the event's - is what makes "what was this agent asked to do" show up on entries that aren't themselves `UserPromptSubmit`-triggered. This is the same snapshot-at-record-time shape `reminder_name`/`reminder_due_at_ms` already use.

**"Most recently active agent" for the Last Prompt pane = greatest `last_updated_ms` among the project's tracked agents.** This is the same timestamp already used to order project rows and pick each row's most-recent-last-updated time elsewhere in the TUI spec, so no new concept is introduced - just applied at agent granularity within one project instead of across all of a project's members.

**Pane sizing: fixed height in terminal rows (not a percentage), taken from the Logs pane's area.** A percentage-based strip would shrink or grow with terminal size, which is pointless for a pane whose content is capped to a handful of wrapped lines regardless of window size; a small fixed height (e.g. 4 rows, including the pane's border) is simplest to reason about and matches how little vertical space a "reminder of what this agent is doing" needs. Exact row count is an implementation/tuning detail, not a spec-level fact.

**No new field negotiation/versioning.** This project has no wire-compatibility story beyond "daemon and clients are upgraded together" (see other historical proto changes); `prompt`/`last_prompt` being `Option` means an old client talking to a new daemon (or vice versa) still deserializes successfully via `serde`'s default-on-missing-field behavior for `Option<T>`, it just won't show the new data - acceptable since this project has never supported mixed-version daemon/client operation.

## Risks / Trade-offs

- **Sensitive content in prompts.** Prompts can contain anything the user typed, including secrets accidentally pasted in. This is no new exposure beyond what already exists in the Claude Code transcript and hook payload itself - the daemon already runs locally over a user-only Unix socket - but it does mean prompt text now also sits in the (still local-only, in-memory, 500-entry-capped) activity log, which previously held no free-text content. Mitigation: no new persistence or network exposure is introduced; this is purely in-memory, same trust boundary as everything else the daemon tracks.
- **Modal layout churn.** Reducing the Logs pane's share to make room for the Last Prompt strip changes a carefully-tuned existing layout. Mitigation: fixed small height rather than a percentage keeps the regression to "Logs pane is a few rows shorter," not a proportional squeeze.
- **"Most recently active agent" can flip while the user is reading it.** If a project has two busy agents, the Last Prompt pane's content can change out from under the user mid-read when the other agent produces an event. This is the same trade-off the project-row aggregation already accepts elsewhere (e.g. the Agents cell showing multiple statuses) and matches the explicit design decision made for this pane; no mitigation beyond what the spec already states (it tracks "most recently active," by design).
