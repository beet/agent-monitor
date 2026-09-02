## Context

`agentd install`/`agentd uninstall` (`crates/agentd/src/main.rs`, `crates/agentd/src/launchd.rs`) render a `launchd` plist, write it to `~/Library/LaunchAgents/com.agentmon.agentd.plist`, and load/unload it via `launchctl`. `run_install` deliberately resolves the binary path from `argv[0]` rather than `current_exe()` to avoid baking in a version-pinned Cellar path (see the archived `agentd-stable-install-path` change) — Homebrew's own `brew services` feature solves this same problem by construction, since it always points its generated plist at the formula's `opt/` symlink. See proposal.md - Why.

The Homebrew tap (`beet/homebrew-agent-monitor`, `Formula/agent-monitor.rb`) is a separate repo from this one, cloned to scratch when tap updates are needed ([[agent-monitor-release-workflow]]). This change's formula edit is not a routine version bump — it adds a `service do` block — so it's tracked as a task here even though it executes against the tap's own checkout, not this repo.

## Goals / Non-Goals

**Goals:**
- Replace `agentd install`/`agentd uninstall` with a Homebrew `service do` block that `brew services start/stop/restart agent-monitor` drives.
- Preserve the daemon's current runtime behavior exactly: same entry point (`agentd` / `agentd --foreground`), same `RunAtLoad`/`KeepAlive`/restart-throttle semantics, same log locations.
- Delete the now-redundant plist-rendering and arg0-resolution code.

**Non-Goals:**
- Any compatibility shim or auto-migration for existing installs. This runs on two machines the maintainer controls directly; the pre-migration `agentd uninstall` is run by hand before upgrading, once, on each.
- Changing the daemon's socket protocol, registry, or notification behavior.
- The notification-icon work discussed separately — out of scope for this change.

## Decisions

**Use a `service do` block in the tap formula, keyed to the existing plist semantics.**

Homebrew's `service` DSL supports the same knobs the hand-rolled plist used:
- `run [opt_bin/"agentd"]` — no `--foreground` flag needed; bare `agentd` already runs the server in the foreground (`main.rs`: `None => run_server()`), which is what `launchd` needs (a supervised, non-daemonizing child).
- `keep_alive true` and `run_type :immediate` for the current `RunAtLoad`/`KeepAlive` pairing.
- `restart_delay` set to `5` to match the existing `ThrottleInterval` (crash-loop protection).
- `log_path`/`error_log_path` kept at `~/Library/Logs/agentmon/agentd.{out,err}.log` (a string path, not the Homebrew `var/log` convention) so nothing else that might reference this log location breaks, and so the change is behavior-preserving rather than also relocating logs.

Alternatives considered:
- **Keep the custom plist code, just fix nothing**: leaves ~70 lines of code solving a problem Homebrew already solves, for no benefit.
- **Move logs to Homebrew's `var/log` convention while migrating**: bundles an unrelated behavior change into a change that's supposed to be a pure mechanism swap; deferred as a separate future change if ever wanted.

**Drop `agentd install`/`uninstall` outright rather than deprecating them first.**

Since this runs on exactly two maintainer-controlled machines, there's no installed base to support through a deprecation window. Removing them now avoids maintaining two parallel install mechanisms even briefly.

Alternatives considered:
- **Keep `agentd uninstall` as an undocumented escape hatch**: unnecessary given the manual pre-upgrade step is already the agreed plan, and it would leave dead code whose only caller is a one-time manual step the maintainer already knows how to do differently (uninstall from the *old* binary, not the new one).

## Risks / Trade-offs

- [Running `brew services start agent-monitor` before manually uninstalling the old self-managed LaunchAgent would leave two daemons running, double-binding the socket] → Mitigated procedurally, not in code: the maintainer runs `agentd uninstall` (old binary) before upgrading, on both machines, as agreed in proposal.md. Documented as a task step, not automated.
- [Homebrew's `service` DSL doesn't expose an exact `ThrottleInterval` key — `restart_delay` is the closest equivalent but its semantics may differ slightly] → Low impact: both exist purely to prevent a crash-loop from spinning; an approximate match is acceptable, and behavior can be observed with `brew services info agent-monitor` before switching over.

## Migration Plan

- No code-level or data migration. Sequenced deploy:
  1. Land this change's code removal (`agentd install`/`uninstall` gone) in the same release as the tap's `service do` block — they must ship together, since a released `agentd` without `install` and a tap formula without `service` would leave no way to run it as a background service at all.
  2. On each of the two machines: run `agentd uninstall` using the *currently installed* (pre-migration) `agentd` binary.
  3. Upgrade (`brew upgrade agent-monitor`) to the release containing this change.
  4. Run `brew services start agent-monitor`.
- Rollback: `brew services stop agent-monitor`, downgrade the formula, re-run the old `agentd install`.
