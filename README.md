# agent-monitor

A TUI + background daemon for monitoring Claude Code agent sessions on your machine — see what's running, idle, waiting on input, or done, with a macOS notification when a session finishes.

Three binaries:

- **`agentd`** — background daemon, tracks agent status via Claude Code hooks, sends notifications
- **`agentmon`** — the TUI that shows the live list
- **`agentmon-report`** — the hook command that reports status to the daemon

## Install

```
brew tap beet/agent-monitor
brew install agent-monitor
```

If Homebrew refuses the tap as untrusted, run `brew trust beet/agent-monitor` first.

## Setup

```
agentmon-report install-hooks   # registers hooks in ~/.claude/settings.json
agentd install                  # installs and starts the daemon as a launchd service
```

Then run `agentmon` in a terminal to see tracked sessions.

## Upgrade

```
brew upgrade agent-monitor
pkill agentd   # the launchd service restarts it automatically
```

If you installed the service before `agentd install` started pinning a stable path (anything installed via `brew install`/`brew upgrade` before this README section existed), run this once to pick it up; after that, upgrades only need the two commands above:

```
agentd uninstall
agentd install
```

## Supported hosts

Works for Claude Code sessions in a terminal or nvim's embedded terminal. The desktop app isn't supported — it runs sessions in a sandboxed environment that can't execute local hooks (it has its own built-in notifications instead).

## Uninstall

```
agentd uninstall
brew uninstall agent-monitor
```

Then remove the `agentmon-report` entries from `~/.claude/settings.json`'s `hooks` section.
