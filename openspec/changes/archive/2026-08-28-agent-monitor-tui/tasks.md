## 1. Project setup

- [x] 1.1 Initialize a Rust workspace (`Cargo.toml` with members) and verify `cargo build` succeeds on an empty workspace
- [x] 1.2 Create `agentmon-proto` crate for shared types (agent id, status enum, host context enum, event/message structs) and verify it compiles with `cargo check -p agentmon-proto`
- [x] 1.3 Add serialization (serde + JSON) to the shared types and verify round-trip (de)serialization via a unit test

## 2. Daemon core: socket and registry

- [x] 2.1 Create `agentd` binary crate that binds a Unix domain socket at a per-user path, handling the stale-socket-file case, and verify with a test that starts the daemon twice in sequence (second run cleans up first's leftover socket) per the agent-daemon spec's socket lifecycle requirement
- [x] 2.2 Set socket file permissions to owner-only and verify with a test asserting mode bits after bind
- [x] 2.3 Implement the in-memory agent registry (keyed by session id, storing cwd, host context, pid, status, last-updated) and verify with unit tests for insert-new and update-existing behavior
- [x] 2.4 Implement the framed JSON protocol (newline-delimited messages) for both short-lived event-reporting connections and long-lived client (snapshot + streaming update) connections, and verify with an integration test that sends events and reads back a snapshot

## 3. Daemon: status ingestion and notifications

- [x] 3.1 Implement the event-ingestion handler that updates the registry from incoming hook events, rejecting malformed payloads without crashing, and verify with tests covering valid and malformed payloads
- [x] 3.2 Implement macOS notification dispatch (via `osascript` or `UserNotifications`) fired on transition to "done" or "needs input", suppressing duplicate notifications for repeated same-status events, and verify with a test that asserts exactly one notification call across repeated identical-status events
- [x] 3.3 Implement the periodic liveness sweep that checks tracked pids and marks agents stale when their process no longer exists, and verify with a test using a spawned-then-killed subprocess
- [x] 3.4 Wire ingestion, registry updates, and notification dispatch into the socket server and verify end-to-end with an integration test that posts an event and asserts a registry change plus notification trigger

## 4. Hook reporter CLI

- [x] 4.1 Create `agentmon-report` binary that reads a Claude Code hook JSON payload from stdin, determines host context (nvim / standalone terminal / desktop) from the process tree, and writes a status event to the daemon's socket; verify with a test using a sample hook payload and a mock socket listener
- [x] 4.2 Handle the daemon-not-running case in `agentmon-report` by failing fast without blocking the calling hook, and verify with a test that runs the reporter with no daemon listening
- [x] 4.3 Write a setup command (e.g. `agentmon-report install-hooks`) that adds/updates the relevant hook entries in the user's Claude Code `settings.json`, and verify with a test asserting the resulting JSON contains the expected hook commands without clobbering existing unrelated settings

## 5. Daemon as a macOS service

- [x] 5.1 Add `agentd --foreground` mode for development and verify it runs and logs to stdout
- [x] 5.2 Create the launchd `.plist` template (with `KeepAlive` and `ThrottleInterval`) and an `agentd install`/`agentd uninstall` command that writes/loads or unloads/removes it from `~/Library/LaunchAgents`, and verify by installing, checking `launchctl list` shows it running, then uninstalling and checking it's gone

## 6. TUI: connection and live list

- [x] 6.1 Create `agentmon` binary crate using a Rust TUI framework, connect to the daemon socket on startup, and verify it renders a clear "daemon not running" message when the socket is unreachable
- [x] 6.2 Render the initial agent snapshot as a list (working directory/project, host context, pid, status) and verify visually against a daemon seeded with sample agents
- [x] 6.3 Apply incremental updates pushed by the daemon (new agent added, status changed, agent marked stale) to the live list and verify with an integration test driving a mock daemon connection through an add/update/stale sequence
- [x] 6.4 Apply distinct visual styling per status (running, idle, needs input, done, stale) and verify visually that "needs input" is clearly distinguished from other states

## 7. TUI: interaction and resilience

- [x] 7.1 Implement row selection and a quit keybinding, and verify quitting the TUI leaves the daemon process and its tracked agents unaffected
- [x] 7.2 Implement reconnect-with-backoff when the daemon connection drops, repopulating the list once reconnected, and verify with a test that stops and restarts a mock daemon while the TUI is running

## 8. End-to-end verification

- [x] 8.1 Run `agentmon-report install-hooks` to register the `UserPromptSubmit`/`Stop`/`Notification` hooks in the real `~/.claude/settings.json`, and verify the file contains the three new hook entries
- [x] 8.2 Install and start the real daemon with `agentd install` (or run `agentd --foreground` in a spare terminal instead, if not ready to install the launchd service permanently), and verify `launchctl list | grep agentmon` shows it running (skip if using `--foreground`)
- [x] 8.3 Open `agentmon` in a terminal and verify it connects cleanly (no "daemon not running" message, shows an empty agent list)
- [x] 8.4 Start a real Claude Code session and give it a task; verify the session appears in the TUI list, its status updates live from running to done as it works, and a macOS notification fires on completion
- [x] 8.5 Repeat 8.4 with a session in nvim's embedded terminal, and verify the TUI's host column reads "nvim" for it
- [x] 8.6 Repeat 8.4 with a session in a standalone terminal, and verify the TUI's host column reads "terminal" for it
- [ ] ~~8.7 Repeat 8.4 with a session in the Claude Code desktop app, and verify the TUI's host column reads "desktop" for it~~ - **Descoped**: confirmed during testing that the desktop app runs sessions in a sandboxed environment that can't execute local shell-command hooks, so this host is unreachable with the mechanism this change implements. See design.md's "Desktop app sessions are unreachable" risk and updated Non-Goals. Not pursuing further: the desktop app already has its own built-in notifications.

## 9. Homebrew distribution

- [x] 9.1 Push the repo to a GitHub remote and create an initial tagged release (e.g. `v0.1.0`), and verify the release's source tarball URL is reachable (`curl -sI` returns 200)
- [x] 9.2 Create a Homebrew tap repo (e.g. `beet/homebrew-agentmon`) or a `Formula/` directory in an existing tap, and verify `brew tap` can add it locally
- [x] 9.3 Write `Formula/agentmon.rb` that builds all three binaries (`agentd`, `agentmon`, `agentmon-report`) from the tagged source tarball via `cargo install` per crate, with `depends_on "rust" => :build`, and verify `brew install --build-from-source` succeeds locally from the tap
- [x] 9.4 Add a `caveats` block to the formula documenting the required post-install steps (`agentmon-report install-hooks`, `agentd install`), and verify the message is shown after `brew install`
- [x] 9.5 Verify end-to-end on a clean machine (or a clean `brew` state): `brew tap beet/agent-monitor && brew install agent-monitor` installs all three binaries onto `PATH`, and each runs (`agentd --foreground`, `agentmon`, `agentmon-report install-hooks`). Confirmed on a second, genuinely separate machine (required `brew trust beet/agent-monitor` first - now documented in the tap README).
- [x] 9.6 Document the release process for future version bumps (tag a new version, update the formula's `url`/`sha256`/`version`, push to the tap) so updates don't require re-deriving these steps
