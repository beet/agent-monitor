## Context

`run_install` in `crates/agentd/src/main.rs` currently computes the binary path with `std::env::current_exe()`, then passes it to `launchd::install` (`crates/agentd/src/launchd.rs`), which bakes that path into the plist's `ProgramArguments`. On macOS, `current_exe()` canonicalizes symlinks (via `realpath`), so invoking the Homebrew-linked `agentd` (`/usr/local/bin/agentd`, a symlink Homebrew repoints on every `brew link`/`upgrade`) resolves down to the version-pinned Cellar path (`Cellar/agent-monitor/0.1.0/bin/agentd`) before it's written to the plist. See proposal.md - Why.

## Goals / Non-Goals

**Goals:**
- When `agentd install` is invoked via the normal Homebrew-managed entrypoint (`agentd` found on `PATH`), the plist ends up referencing that stable, PATH-visible location rather than its resolved Cellar target.
- Keep working for non-Homebrew invocations (dev builds, `cargo install`, `--foreground`) exactly as today.

**Non-Goals:**
- Auto-detecting or shelling out to `brew --prefix` - adds a runtime dependency on Homebrew being installed and doesn't help non-brew installs.
- Automatically migrating an already-installed (pre-fix) launchd service - that requires a one-time manual reinstall (see Migration Plan).

## Decisions

**Resolve the binary path from `argv[0]` made absolute, without canonicalizing.**

When a shell finds `agentd` via `PATH` and executes it, `argv[0]` is set to the resolved-by-PATH-search location (e.g. `/usr/local/bin/agentd`) but the OS does not further follow that path's own symlink chain to fill in `argv[0]` - that only happens if the program itself asks for the canonical path, which is what `current_exe()` does. So reading `argv[0]` directly and just making it absolute (joining with the current directory if it's relative, e.g. `./target/debug/agentd`) preserves whatever symlink Homebrew manages, letting the OS follow it fresh each time `launchd` execs the plist's `ProgramArguments`.

This is implemented as a small pure function, `resolve_binary_path(arg0: &Path, cwd: &Path) -> PathBuf`, called from `run_install` with `env::args_os().next()` and `env::current_dir()`. Keeping it pure (no direct env access inside the function) makes it unit-testable without needing to control the test binary's own argv/cwd.

Alternatives considered:
- **Keep `current_exe()`, strip the version segment with a Homebrew-path heuristic**: fragile (Cellar layout isn't guaranteed), and does nothing for non-Homebrew installs.
- **Shell out to `brew --prefix agent-monitor`**: adds a hard runtime dependency on `brew` being on `PATH` at install time, and doesn't apply to `cargo install` users at all.

## Risks / Trade-offs

- [If a user invokes `agentd install` via a version-pinned path directly (e.g. runs the Cellar binary rather than the `PATH`-linked one) the plist still pins that version] → Documented as the supported invocation being through the normal `agentd` on `PATH`; this matches how the README already tells users to run it after `brew install`.
- [Users who installed the service before this fix still have an old plist pinned to a version-pinned path] → One-time `agentd uninstall && agentd install` after upgrading to a version with this fix; noted in Migration Plan below. After that, later upgrades only need a restart.

## Migration Plan

- No data migration. Existing installs continue running on their old pinned path until the user re-runs `agentd install` (or `agentd uninstall && agentd install`) once, after upgrading to a version containing this fix. Documented in README.md's new Upgrade section (task 3.1).
