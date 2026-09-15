## Why

Real-world use of the Agents/Logs tabs and the project details modal surfaced several rough edges: the Agents tab's single STATUS column conflates two independent things (agent health and test-run health), the details modal has no way to tell which of several agents in a project a status belongs to (no pid shown), the stale 🕸️ emoji renders half-width in many terminals and corrupts the row-highlight background on the row it appears in, a completed agent's total task duration is invisible (only "running" counts up live, "done" shows nothing), and a long session produces an unbroken string of "done" log entries with no paired "started" entry to show when each one's work began.

## What Changes

- Split the Agents tab's combined STATUS column into two columns, **Agents** and **Tests**, so agent health and test-run health are scanned independently instead of sharing one cell.
- Show each agent's process id in the details modal's Agents pane (next to its status), and show the pid alongside each agent-category entry in the details modal's Logs pane.
- Replace the stale status's emoji marker (🕸️) with 👻 everywhere it appears (Agents tab, Logs tab, details modal), fixing the half-width rendering glitch that corrupted the selected-row highlight.
- Track a run-started timestamp per tracked agent that resets each time the agent's status transitions into "running" (unlike a test run's run-start timestamp, which is scoped to a single process and only resets on a new pid, an agent's pid persists across many started → running → done turns within one long-lived session, so the reset trigger here is the transition itself, not a new pid) and use it to show a fixed total duration on a "done" agent, the same way a finished test run already shows one. No other agent status (idle, needs input, stale, declined) gains a duration.
- Log a new "agent started" activity-log event using that same run-started timestamp, emitted whenever an agent's status transitions into "running" from a different status (or is registered for the first time already running) - mirroring how a test run's "started" event is already logged independent of any notification, but recurring every time the shared pid starts a new turn rather than once per pid. Since "started" marks a one-time event rather than an ongoing status, it appears only in the Logs tab and the details modal's Logs pane, not in the Agents tab's new Agents column or the details modal's Agents pane.
- In top-level, cross-category views (the Agents tab and the Logs tab), prefix every status label with its category word - "agent" or "tests" - alongside its emoji and duration (e.g. "✅ agent done 3m12s", "⏳ tests started 12s"), so a label is unambiguous when scanned outside the context of a dedicated pane.
- In the details modal, the Agents pane and Tests pane drop that category-word prefix (e.g. "✅ done 3m12s", "❌ failed 1m02s") since the pane itself already establishes the category; the modal's Logs pane keeps the category-word prefix, consistent with the top-level Logs tab it mirrors.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: the agent registry gains a run-started timestamp per tracked agent, reset each time that agent's status transitions into "running" (including the first time it is registered), and includes it when sending agents to clients.
- `activity-log`: a new capture point logs an "agent started" entry on transition into "running", independent of the existing notification-tied capture rule; each entry gains an optional process id, populated for agent-category entries.
- `agent-monitor-tui`: the Agents tab's STATUS column splits into Agents and Tests columns; the stale emoji marker changes from 🕸️ to 👻; a "done" agent now shows a fixed total duration computed from the new run-started timestamp to the time it completed; the details modal's Agents pane shows each agent's pid and its Logs pane shows pid for agent-category entries; status-label rendering gains a category-word prefix in the Agents/Logs tabs that is dropped in the details modal's Agents/Tests panes (but kept in its Logs pane); the Logs tab and details-modal Logs pane render the new "agent started" event.

## Impact

- `crates/agentmon-proto/src/lib.rs`: `AgentInfo` gains a `run_started_ms` field; `LogEntry` gains an optional `pid` field.
- `crates/agentd/src/registry.rs`: track `run_started_ms` per registry entry, resetting it on every transition into "running" (not just at entry creation), alongside the existing `status_since_ms` handling.
- `crates/agentd/src/ingest.rs`: emit an "agent started" activity-log entry on transition into "running", using `run_started_ms`; populate `LogEntry.pid` from the source event.
- `crates/agentmon/src/ui.rs`: split `render_agent_table`'s STATUS column into Agents/Tests columns; change the stale emoji constant; compute and render a "done" duration from `run_started_ms`; render pid in the details modal's Agents pane and Logs pane; add category-word prefixing for top-level views and omit it in modal panes; render the new "started" agent log status.
- Existing tests across `agentmon-proto`, `agentd`, and `agentmon` covering `AgentInfo`/`LogEntry` JSON round-trips, registry timestamp behavior, activity-log capture, and TUI rendering need updating/extending to match.
