## MODIFIED Requirements

### Requirement: Runs as a macOS background service
The daemon SHALL be installable and runnable as a per-user macOS `launchd` service, in addition to running in the foreground for development. The service SHALL be installed, started, stopped, and restarted via Homebrew's `brew services` interface rather than daemon-specific subcommands, so the plist Homebrew generates always references the current Homebrew-managed binary path and is updated in place on upgrade, not a version-pinned path.

#### Scenario: Install as a login service
- **WHEN** the user runs `brew services start agent-monitor`
- **THEN** a `launchd` user agent plist is written and loaded so the daemon starts automatically and keeps running in the background

#### Scenario: Uninstall the service
- **WHEN** the user runs `brew services stop agent-monitor`
- **THEN** the `launchd` user agent is unloaded and the daemon stops running until started again

#### Scenario: Service survives a Homebrew upgrade with just a restart
- **WHEN** the user runs `brew upgrade` for the package while the service is installed, then runs `brew services restart agent-monitor` (without reinstalling the service)
- **THEN** the daemon that starts is the newly installed version, not the one that was running before the upgrade
