## MODIFIED Requirements

### Requirement: Runs as a macOS background service
The daemon SHALL be installable and runnable as a per-user macOS `launchd` service, in addition to running in the foreground for development. The installed service SHALL reference the daemon binary by a Homebrew-managed path that Homebrew updates in place on upgrade, not a version-pinned path, so the service picks up a new version after a restart without being reinstalled.

#### Scenario: Install as a login service
- **WHEN** the user runs the daemon's install command
- **THEN** a `launchd` user agent plist is written and loaded so the daemon starts automatically and keeps running in the background

#### Scenario: Uninstall the service
- **WHEN** the user runs the daemon's uninstall command
- **THEN** the `launchd` user agent is unloaded and its plist is removed

#### Scenario: Service survives a Homebrew upgrade with just a restart
- **WHEN** the user runs `brew upgrade` for the package while the service is installed, then restarts the daemon (without re-running the install command)
- **THEN** the daemon that starts is the newly installed version, not the one that was running before the upgrade
