## Why

`agentd install` writes its launchd plist with the binary path from `std::env::current_exe()`, which macOS resolves through symlinks down to the version-pinned Homebrew Cellar path (e.g. `Cellar/agent-monitor/0.1.0/bin/agentd`). After `brew upgrade agent-monitor`, that exact path may no longer exist, and even when it does, launchd keeps running the old binary until the service is reinstalled. Users have to run `agentd uninstall && agentd install` around every upgrade instead of just restarting the daemon.

## What Changes

- `agentd install` resolves its own binary path without following symlinks (using the invoked path made absolute, not canonicalized), so the plist references a stable, Homebrew-managed path whose target is updated in place by `brew link`/`brew upgrade` rather than a version-pinned one.
- A plain `brew upgrade agent-monitor` followed by a daemon restart (e.g. `launchctl kickstart -k`) now runs the new version, without requiring `agentd uninstall && agentd install`.

## Capabilities

### Modified Capabilities
- `agent-daemon`: the "Runs as a macOS background service" requirement now specifies that the installed service path survives a Homebrew upgrade without reinstalling.

## Impact

- `crates/agentd/src/main.rs`: `run_install` computes the binary path.
- `crates/agentd/src/launchd.rs`: `install` writes that path into the plist; may need a helper for the non-canonicalizing path resolution and its tests.
- `README.md`: new Upgrade section documenting the restart-only upgrade path and the one-time migration step for pre-fix installs.
