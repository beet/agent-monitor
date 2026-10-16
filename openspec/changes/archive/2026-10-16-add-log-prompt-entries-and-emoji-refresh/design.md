## Context

See `proposal.md` - Why/What Changes for motivation and scope. This note covers the two implementation decisions that aren't fully pinned down by the specs' behavioral language: how a "prompt" log entry is detected and ordered relative to a "started" entry on the daemon side, and the handful of places the shared emoji/notification helpers live in the TUI and daemon code today.

Key existing facts that shape the approach (see the activity-log and agent-monitor-tui specs for current behavior):
- `AgentEvent.prompt` (in `agentmon-proto`) is already `Some(..)` if and only if the hook payload that produced the event was `UserPromptSubmit` (`agentmon-report/src/report.rs::build_event`) - there is no separate "this was a UserPromptSubmit" flag to add.
- `Ingestor::ingest_event` (`agentd/src/ingest.rs`) already computes `is_run_start(previous_status, current_status)` to decide whether to log a "started" entry, and separately `should_notify` to decide whether to log a "done"/"needs input" entry. Both read from one `UpsertOutcome`.
- The TUI's recency sort (`app.rs`, `entries.sort_by_key(|e| std::cmp::Reverse(e.occurred_at_ms))`) is a stable sort, so entries sharing the same `occurred_at_ms` keep their relative order from the input slice - which itself reflects the order the daemon's `ActivityLog` recorded them in (`record()` appends; snapshots/pushes preserve that order).
- The emoji/color tables for agent "done"/"needs input" (`status_label_and_style`) are shared between the Agents tab's live status column and the Logs tab/Logs pane; the log-only "started" labels (`agent_started_text_and_style`, `test_run_started_text_and_style`) are not used anywhere outside the Logs tab/pane. The reminder emoji (⏳/✅) in `reminder_status_cell_text_and_style`/`reminder_agents_column_segment` are separate hardcoded literals, not calls into the agent/test-run style helpers - so they are unaffected by this change's emoji edits unless explicitly touched.

## Goals / Non-Goals

**Goals:**
- Detect a `UserPromptSubmit`-derived event in `Ingestor::ingest_event` without widening `AgentEvent` or `status_for_payload`'s status mapping - reuse the existing `event.prompt.is_some()` signal.
- Guarantee a prompt entry sorts immediately ahead of the "started" entry it precipitates, using the existing stable-sort behavior rather than inventing a secondary sort key.
- Keep the emoji/color change confined to the functions the proposal's scope decision (shared, not Logs-tab-only) calls for, without touching the independently-hardcoded reminder markers.

**Non-Goals:**
- No change to `status_for_payload`'s hook-to-`AgentStatus` mapping; `UserPromptSubmit` continues to map to `AgentStatus::Running`.
- No change to how/when `last_prompt` is set on `AgentInfo`, or to the existing agent-entry prompt-snippet behavior the proposal says to keep.
- No new wire message or `ClientMessage`/`ServerMessage` variant - a prompt entry is just a new `LogCategory` value flowing through the existing log-entry snapshot/push path.

## Decisions

**Detecting a prompt event in `ingest_event`:** branch on `event.prompt.is_some()` (cloned before `self.registry.upsert(event)` consumes it) rather than adding a hook-name field to `AgentEvent`. This mirrors the existing doc-comment invariant ("Only UserPromptSubmit-derived events carry this") and avoids widening the wire type for a signal that's already unambiguous. Alternative considered: have `status_for_payload` return a distinct `AgentStatus` variant for `UserPromptSubmit` (e.g. reusing the data differently) - rejected because it would change the agent's live status transition semantics (still needs to end up `Running`) for no benefit over the existing `prompt` field.

**Ordering the "prompt" entry before "started":** record the "prompt" entry (when present) before the "started"/notify logic in `ingest_event`, and give it the same `occurred_at_ms` the "started" entry would use (`outcome.agent.run_started_ms`) when `is_run_start` is true; otherwise (a follow-up prompt mid-session) timestamp it with the same `now_ms()` every other non-run-start entry uses. Insertion order plus the stable sort already used by both `ActivityLog`'s append-only storage and the TUI's recency sort is what makes "prompt before started" hold whenever the two share a timestamp - no new ordering field or secondary sort key is needed. Alternative considered: give the prompt entry a timestamp one millisecond earlier than the started entry - rejected as a hack that would misrepresent the recorded event time and still not guarantee ordering against a sort that doesn't tie-break on category.

**Prompt entry fields:** carries `working_dir`, `pid` (from the event, like an agent entry), and `prompt` (always `Some`, unlike an agent entry's optional snapshot); `reminder_name`/`reminder_due_at_ms` stay `None`, matching how non-reminder categories already leave them unset.

**Prompt entry rendering text:** follows the existing per-category convention of `{emoji} {category word} {status word}` used by agent/tests/reminder entries, giving `"🗣️ prompt submitted"` with no color (`Style::new()`, the default/unstyled foreground) - the one category where the marker carries no status-specific color, since there is exactly one status.

**Emoji scope (shared vs. Logs-tab-only):** change `status_label_and_style`'s `Done`/`NeedsInput` arms directly (🤖, keeping Green/Yellow-bold), which cascades to both the Agents tab's live status column and the Logs tab per the user's explicit choice. `agent_started_text_and_style` and `test_run_started_text_and_style` are edited to 🤖/🧪 respectively; since neither is called from anywhere but the Logs tab/pane, this is a no-op scope decision for them - they're already Logs-only.

**Reminder's "done" marker:** leave `reminder_status_cell_text_and_style`/`reminder_agents_column_segment`'s hardcoded `"✅"` untouched. Before this change, that reuse was presented in the spec as "the same marker as a done agent" (both ✅); after this change, "done agent" becomes 🤖, so continuing to reuse 🤖 for reminder-done would read as generic "agent-ish" rather than "success," which is a worse fit than keeping ✅ (now justified as reusing the still-unchanged "passed test run" marker instead). No code change, just a spec wording correction - see the `agent-monitor-tui` delta.

**Notification body prefixes:** prepend the emoji and a single space directly in the body-building `format!` calls in `notify.rs` (`notification_script`, `test_run_notification_script`, `reminder_notification_script`) - a plain string prefix, no change to the sound-selection logic or the `Notifier` trait.

## Risks / Trade-offs

- [Shared emoji helper changes the Agents tab, not just Logs] -> Deliberately in scope per the proposal's recorded decision; verified no other call sites exist beyond the Agents tab's status column, the details modal's Agents pane, and the Logs tab/pane (grep-checked against `ui.rs`).
- [A `UserPromptSubmit` event that *doesn't* change status, e.g. a prompt resubmitted while already "running" outside the normal flow] -> Still produces a "prompt" entry on its own (per the activity-log delta's "follow-up prompt" scenario); no "started" entry accompanies it, so there's nothing to misorder.
- [Stable-sort ordering guarantee is implicit, not an explicit sort key] -> Acceptable because both the daemon's append-only `ActivityLog` and the TUI's `sort_by_key(Reverse(...))` are documented as preserving insertion order on ties; a future change to either (e.g. switching to an unstable sort) would need to revisit this. Flagged in the activity-log and agent-monitor-tui deltas so it's test-covered rather than only load-bearing on sort stability.
