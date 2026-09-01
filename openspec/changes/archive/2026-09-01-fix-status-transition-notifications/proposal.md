## Why

`agentd` currently gets two related status-transition cases wrong. First, when a session needs input more than once before finishing (e.g. two separate permission prompts in the same turn), only the first notification fires — nothing resets the status to `Running` in between, so the second "needs input" event is silently swallowed as a same-status repeat. Second, Claude Code's own idle-timeout `Notification` (`idle_prompt`) can arrive at the daemon after that session's `Stop` event, because each hook fires over its own one-shot socket connection with no ordering guarantee; when that happens, a session that already finished gets flipped back to "needs input" and an incorrect notification fires for a session the user has already seen complete.

## What Changes

- `agentd`'s notification dedupe no longer suppresses a `NeedsInput` event just because the agent was already `NeedsInput` — every hook-reported need for input notifies, since each one represents a distinct blocking prompt. `Done` keeps its existing same-status suppression.
- The registry rejects a `NeedsInput` event for a session whose current status is already `Done`, since a finished session cannot legitimately need input again until a new `UserPromptSubmit` (`Running`) is reported. This closes the race where a delayed `idle_prompt` notification lands after `Stop`.

## Capabilities

### Modified Capabilities
- `agent-daemon`: the "Completion notifications" requirement changes so repeated `NeedsInput` transitions each notify; the "Agent registry" requirement gains a rule that a late `NeedsInput` event cannot override an agent already marked `Done`.

## Impact

- `crates/agentd/src/ingest.rs`: `should_notify` no longer suppresses repeat `NeedsInput` transitions.
- `crates/agentd/src/registry.rs`: `Registry::upsert` gains a guard so `NeedsInput` events are ignored while the stored status is `Done`.
- Existing tests in `ingest.rs` and `registry.rs` covering same-status suppression and upsert behavior need corresponding updates; new tests cover the "second prompt still notifies" and "late needs-input after done is ignored" cases.
