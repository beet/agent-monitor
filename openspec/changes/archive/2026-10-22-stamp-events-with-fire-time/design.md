## Context

See proposal.md - Why. Two facts from investigating the existing code matter for the approach:

- `agentmon-report`'s current pre-connect sequence (`crates/agentmon-report/src/main.rs`) already does slow, jittery work *before* it opens its socket connection: `detect_host_context()` and `parent_pid()` each shell out to `ps`, up to 12 times walking process ancestry (`crates/agentmon-report/src/host_context.rs`). The prior fix (`fix-prompt-log-ordering-and-truncation`, commit `bff9d1e`) moved `connect()` ahead of reading/parsing stdin, but that fix sits *after* this `ps`-shelling work, so it only closed the payload-size part of the race, not the larger host-context-detection part.
- The TUI already sorts the Logs tab's visible entries by `occurred_at_ms` at render time (`crates/agentmon/src/app.rs`, `visible_logs_default_sorted_most_recent_first`) rather than trusting insertion/arrival order. So the display-order bug is not a missing sort - it's that `occurred_at_ms` (and the registry's `run_started_ms`/`status_since_ms`/`last_updated_ms` it's derived from) is currently stamped with the daemon's own processing-time `now_ms()`, which only matches true hook-fire order when connections happen to arrive in that order.

## Goals / Non-Goals

**Goals:**
- Make every timestamp the daemon records for a hook-reported event reflect when that hook actually fired, not when the daemon happened to process it.
- Fix the reported symptom (a "started" entry displayed before the "prompt" entry that caused it) without adding latency to the common (non-racing) path.

**Non-Goals:**
- Guaranteeing which event "wins" the `is_run_start` transition (and therefore which event's `fired_at_ms` becomes `run_started_ms`) when two events for the same agent race each other at the daemon. That still depends on `EventSequencer`'s accept-order application, unchanged by this design (see Risks). This change only fixes what time gets *recorded* against each event, not which event the registry treats as the one that started the run.
- Eliminating all residual ordering risk. A one-shot process per hook invocation still has OS-level fork/exec scheduling jitter before it can run any of its own code; this design shrinks that window to "process startup" instead of "process startup + up to 12 `ps` calls + payload parse", but doesn't claim to close it to zero.
- Changing how test-run or reminder events are timestamped - neither is produced by a one-shot process racing a sibling hook invocation, so both keep using daemon-side `now_ms()`.

## Decisions

**Capture `fired_at_ms` as the first instruction in `agentmon-report`'s `main`/`run_report`, before `detect_host_context()` or `parent_pid()`.** This is the earliest point in the client's own code, and specifically precedes the two `ps`-shelling calls identified above as the dominant source of jitter today. Capturing it after `connect()` (as the prior fix did for stdin-reading) was considered and rejected: `connect()` itself is downstream of host-context/parent-pid detection in the current code, so that would still let `ps`-call jitter corrupt the timestamp.

**Add `fired_at_ms: u64` as a required field on `AgentEvent`, not `Option<u64>`.** An optional field with a server-side fallback to `now_ms()` was considered, to tolerate a daemon and reporter from different builds talking to each other during a rolling upgrade. Rejected: this tool has no such window - the daemon, reporter, and TUI are built and restarted together (`brew services restart` on upgrade, per the existing release workflow), and a silent fallback would let the exact bug this change fixes reappear invisibly whenever the field happened to be absent, rather than surfacing as a build/version mismatch.

**Thread `fired_at_ms` through the registry, not just the activity log.** `AgentInfo.run_started_ms`, `status_since_ms`, and `last_updated_ms` all switch from `now_ms()` to the triggering event's `fired_at_ms`, in addition to `LogEntry.occurred_at_ms`. Doing only the log entry was considered and rejected: `ingest_event` already depends on `run_started_ms` and a "started" log entry's `occurred_at_ms` being identical (see `crates/agentd/src/ingest.rs`'s comment on reusing `run_started_ms` rather than a fresh `now_ms()` call) - stamping one from client time and the other from server time would break that invariant and desynchronize the Logs tab's duration math from the agent's own displayed timestamps.

**Leave `EventSequencer` and `ActivityLog`'s insertion-order storage unchanged.** Two alternatives were considered and rejected:
- *A bounded reorder/debounce buffer* that delays applying each `ReportEvent` until a short window has passed, then applies events in `fired_at_ms` order: this would also fix which event gets `is_run_start` credit in the race case, but adds latency (on the order of the buffer window) to every single event, including the overwhelming majority that never race anything - not justified for a bug that is purely about displayed timestamps, not about notification promptness or final status correctness.
- *Re-sorting `ActivityLog`'s storage by `occurred_at_ms` on insert* (instead of always appending): unnecessary once entries carry correct `occurred_at_ms` values, because the TUI already sorts by `occurred_at_ms` for display (see Context). Daemon-side storage order only affects which entry is evicted first at the 500-entry cap, and the margin of error there (a handful of milliseconds, in the rare race case) isn't worth the added complexity.

**Test-run and reminder events are out of scope.** Both are reported through mechanisms (an RSpec formatter; direct TUI-issued commands) that don't involve a one-shot process racing a sibling hook invocation, so there's no analogous ordering defect to fix.

## Risks / Trade-offs

- **[Risk]** `fired_at_ms` can end up attached to the "wrong" transition: in the race case, whichever event the daemon happens to apply first while the agent's previous status is still non-"running" is the one that gets `is_run_start = true` and therefore becomes `run_started_ms` - and that may be the later-fired event, not the earlier-fired `UserPromptSubmit`. → **Mitigation**: this does not reproduce the reported bug (every entry's own *displayed* timestamp is still its own correct `fired_at_ms`, so the prompt entry still sorts before the started entry), and is called out explicitly as a Non-Goal above rather than silently accepted.
- **[Risk]** Required-field change to `AgentEvent` breaks wire compatibility between an old reporter and a new daemon (or vice versa) if they're ever run from mismatched builds. → **Mitigation**: matches this project's existing single-version-at-a-time deployment model; no mitigation beyond "upgrade together," which is already how `brew services restart` works for this tool.
- **[Risk]** Every existing test/fixture that constructs an `AgentEvent` literal needs a `fired_at_ms` value added, across `agentmon-proto`, `agentd`, and `agentmon-report`. → **Mitigation**: mechanical, enumerated in tasks.md; the compiler enforces completeness (a missing field is a build error, not a silent gap).
- **[Risk]** `CLAUDE.md` requires the README's data-model `classDiagram` to stay byte-for-byte matched to `AgentEvent`'s fields. → **Mitigation**: tasks.md includes updating the README diagram alongside the struct change.

## Migration Plan

No phased rollout: this ships as one release, since the daemon, reporter, and TUI are already upgraded and restarted together (`brew services restart agent-monitor` after `brew upgrade`, per the project's release workflow). Rollback, if needed, is the same as for any other release: reinstall the previous version.

## Open Questions

- Whether `run_started_ms` attribution should eventually be made race-proof too (e.g. via a short reorder buffer limited to the `is_run_start` decision only, not general event application) is left for a future change if it turns out to matter in practice - it doesn't affect this change's specs, approach, or tasks.
