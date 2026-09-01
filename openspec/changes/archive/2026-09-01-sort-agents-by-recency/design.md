## Context

See proposal.md - Why. Relevant current state:

- `AgentInfo.last_updated_ms` (`crates/agentmon-proto/src/lib.rs`) is already a `u64` unix-epoch-milliseconds field, populated by `agentd`'s `now_ms()` (`crates/agentd/src/registry.rs`). No new wire data is needed.
- `App.agents` (`crates/agentmon/src/app.rs`) is a plain `Vec<AgentInfo>` populated in whatever order `apply_snapshot`/`apply_update` receive agents - insertion order, not recency.
- `render_agent_table` (`crates/agentmon/src/ui.rs`) iterates `app.agents` directly to build table rows.
- Neither `agentmon` nor any other crate in the workspace currently depends on a date/time-formatting crate (`chrono`, `time`, etc.) - only `std::time::SystemTime`/`UNIX_EPOCH` are used, for computing `last_updated_ms` itself, not for formatting it.
- `crates/agentmon/src/main.rs`'s `run()` spawns two background threads (a client-forwarding thread and a key-polling thread) before entering the render loop, so any approach to local-time conversion that requires a single-threaded startup window (e.g. capturing a UTC offset once before other threads exist) has to happen at the very top of `main()`, ahead of those spawns.

## Goals / Non-Goals

**Goals:**
- Agents render sorted by `last_updated_ms` descending on every snapshot and incremental update.
- Each row shows its `last_updated_ms` converted to the system's local timezone and formatted `%Y-%m-%d %H:%M:%S` (e.g. `2026-09-01 16:32:07`).

**Non-Goals:**
- Changing the wire protocol or `AgentInfo` - `last_updated_ms` already carries what's needed.
- User-configurable sort order (by name, status, etc.) - recency-descending is the only order this change adds.
- Displaying the UTC offset or a timezone abbreviation (e.g. `AEST`) alongside the timestamp - the row shows bare local time only, matching what's on the user's own clock without extra chrome, and without needing a timezone-abbreviation/tzdata lookup.
- Reintroducing row selection in a sort-aware form - out of scope here; selection is removed rather than fixed, since nothing currently acts on a selected row (see Decisions).

## Decisions

**Decision: Sort in `ui.rs` at render time, not by maintaining `App.agents` in sorted order.**
`render_agent_table` builds a `Vec<&AgentInfo>` (or indices) sorted by `last_updated_ms` descending before building rows, leaving `App.agents` untouched. Rationale: `App` is the daemon's raw, insertion-ordered view of tracked agents; several of its existing tests (e.g. `apply_update_updates_an_existing_agent_in_place`) assert against `app.agents[0]` for reasons unrelated to display order (checking in-place update vs. duplication), and would need unrelated rework if `App.agents` itself were kept sorted. Keeping sort a render-layer, display-only concern avoids that churn and keeps `App` a straightforward record of daemon state.
Alternative considered: sort `App.agents` in place inside `apply_snapshot`/`apply_update`. Rejected - `app.rs`'s existing tests assert on raw `agents` ordering/indices for reasons orthogonal to display order, and all would need rework for no behavioral benefit over sorting in `ui.rs`.

**Decision: Remove row selection/highlighting instead of making it sort-aware.**
Implementing this change surfaced a gap: `render_agent_table` was passing `app.selected` (a plain index into `App.agents`) into `TableState` to highlight a row, but once rendering iterates a *sorted* copy instead of `app.agents` directly, that index no longer points at the same agent - the highlight would silently drift to whatever agent lands at that position after sorting, and drift further on every update that reorders the list. A grep across `input.rs`/`app.rs`/`ui.rs` confirmed nothing else reads `app.selected` - it exists only to drive the highlight and the up/down/`j`/`k` keys that move it, with no actionable behavior (e.g. no "press enter to act on the selected agent") built on top of it yet. Given that, removing `App.selected`, `select_next`/`select_previous`/`clamp_selection`, the up/down/`j`/`k` key handling in `input.rs`, and the `TableState` highlight in `ui.rs` eliminates the drift problem entirely rather than papering over it, and sheds dead code in the process.
Alternative considered: keep selection, and at render time resolve `app.agents[app.selected]`'s `session_id` to its position in the sorted display list for highlighting. Rejected - this would keep alive a feature (row selection) that currently does nothing actionable, purely to preserve highlighting correctness; simpler to remove it now and reintroduce selection later, sort-aware from the start, once something in the TUI actually acts on a selected row.

**Decision: Add `chrono` (not `time`), scoped to the `agentmon` crate, and convert via `chrono::Local`.**
`chrono = { version = "0.4", default-features = false, features = ["clock"] }` converts `last_updated_ms` to a `DateTime<Local>` and formats it with `.format("%Y-%m-%d %H:%M:%S")` - the same strftime-style syntax already chosen for this column, so no translation to a different format-description language is needed. Rationale: showing local time requires detecting the system's UTC offset, and `time`'s equivalent (`UtcOffset::current_local_offset()`) is gated behind its `local-offset` feature *and* requires building with `RUSTFLAGS="--cfg unsound_local_offset"`, because the underlying libc call isn't safe to invoke while another thread concurrently mutates the environment - a real constraint here, since `agentmon`'s `main()` spawns two background threads before its render loop starts (see Context). `chrono::Local` performs the equivalent OS-level lookup without that feature gate or build-flag requirement, and - unlike a startup-only offset capture - re-resolves the local offset on every conversion, so it stays correct across a DST transition that happens while the TUI is running for hours or days.
Alternative considered: keep `time`, capture `UtcOffset::current_local_offset()` once at the very top of `main()` before any thread spawns, and add the `unsound_local_offset` cfg via `.cargo/config.toml`. Rejected - the build-flag change is an unusual addition to the project's build for a single display column, and a startup-only offset would miss any DST transition during a long-running session, which `chrono::Local`'s per-conversion lookup avoids entirely.
Alternative considered: hand-rolled epoch-ms → calendar conversion using only `std`, still leaving local-timezone/DST rules unhandled. Rejected - correctness risk (leap years, month/day boundaries, DST rules) for code this project doesn't otherwise need to own; DST rules in particular are not something to hand-roll.

## Risks / Trade-offs

- [Risk] Sorting on every render (an O(n log n) sort of the full agent list each frame) adds CPU work versus the current direct iteration. → Accepted: agent counts are small (a handful of concurrently tracked Claude Code sessions per user), and `ratatui` already redraws the full table on every frame.
- [Risk] A new dependency (`chrono`) increases build time and supply-chain surface for `agentmon`, and `chrono`'s default feature set is heavier than `time`'s. → Mitigation: scoped to the `agentmon` crate only (not the workspace-shared `agentmon-proto`), with `default-features = false, features = ["clock"]` rather than the full default feature set.
- [Risk] A unit test that asserts an exact, hardcoded rendered local-time string is only deterministic on the machine/timezone it was written for, and would be flaky in CI or for a contributor in a different timezone. → Mitigation attempted: pin the `TZ` environment variable for the test's duration. Verified during implementation that this does not work on macOS - `chrono::Local` reads the system's configured timezone (e.g. via `/etc/localtime`) rather than honoring a runtime `TZ` change, so two conversions under different `TZ` values in the same process produced identical output. Actual mitigation: the test computes its expected string by converting the same known `last_updated_ms` value through `chrono::Local`/`%Y-%m-%d %H:%M:%S` independently, at test run time, rather than asserting a value hardcoded for one timezone - this still exercises the real conversion and format string end to end (catching e.g. a seconds/milliseconds mixup or a wrong field), while remaining correct on whatever machine or CI runner executes it.
