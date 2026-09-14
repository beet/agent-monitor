## Why

Real-world use of the activity-logs feature (Logs tab + unified project rows) surfaced three problems: a newly-started agent's "running" status is invisible on the Agents tab because its text color collides with the row-highlight background; long sessions with several status transitions can produce a flickering duplicate project row (with its own shadow activity history) that only resolves itself after further transitions, because a late-arriving hook event for a superseded session id can resurrect it and evict the live one; and the empty-state copy for both tables lives in the pane title instead of the gray placeholder text already used elsewhere (e.g. "no activity matches filter"), which is inconsistent and was called out as worth fixing while investigating the other two.

## What Changes

- Fix the Agents tab's default row highlight (`Color::Blue` background) colliding with the "running" status's text color (also `Color::Blue`), which renders the status label and emoji invisible on the initially-selected row - the reason a fresh agent looks like it has a blank status until the details modal (which doesn't highlight) is opened.
- Fix the daemon registry's same-pid dedup so a late/out-of-order hook event for a session id that has already been superseded (e.g. by a `/clear` under the same pid) cannot resurrect that dead session as a "new" entry and evict the live one - the cause of the transient duplicate project row (with its own activity log slice) that a long session eventually self-heals out of.
- Move the Agents tab's "no agents tracked yet" and the Logs tab's "no activity logged yet" empty-state messages out of the pane title and into gray placeholder text in the table body, matching the existing "No activity matches the current filter" convention.
- Add a concise section to the README documenting the Logs tab / activity-log feature, which shipped without any README coverage.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the "Status is visually distinguishable" requirement gains a constraint that a status's rendered color must not collide with the row-highlight background; the Agents and Logs tabs' empty-state requirements move their placeholder text from the pane title into gray table-body text instead.
- `agent-daemon`: the agent registry's same-pid dedup requirement gains a rule that a superseded session id, once replaced, cannot be resurrected by a late-arriving event for it - such an event must not evict the session that replaced it.

## Impact

- `crates/agentmon/src/ui.rs`: row-highlight color (or the "running" status color) changes to remove the collision; `render_agent_table` and `render_logs_tab` empty-state handling moves from the block title into body text, mirroring the existing filter-mismatch placeholder.
- `crates/agentd/src/registry.rs`: `Registry::upsert`'s same-pid dedup logic gains a way to recognize and drop a late event for an already-superseded session id, rather than treating it as brand new.
- `README.md`: new concise section covering the Logs tab and activity log.
- Existing tests in `ui.rs` (empty-state title assertions) and `registry.rs` (dedup tests) need updating/extending to match.
