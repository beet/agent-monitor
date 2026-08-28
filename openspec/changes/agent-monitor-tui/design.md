## Context

See proposal.md - Why. This is a greenfield Rust project (repo currently empty aside from planning artifacts). Claude Code sessions the user runs are hosted in three different environments (nvim embedded terminal, standalone terminal, desktop app), and for the two locally-executing ones the session is the same Claude Code binary/process, which supports lifecycle hooks configured via `settings.json` regardless of which host UI it's embedded in - making hooks a host-agnostic signal there, unlike trying to fingerprint the surrounding terminal/process tree. The desktop app turned out to be the exception: see the "Desktop app sessions are unreachable" risk below.

## Goals / Non-Goals

**Goals:**

- One daemon process is the single source of truth for agent state; the TUI is a thin, disposable client.
- Works identically for every locally-executing host environment a session runs in (nvim, standalone terminal).
- Low-latency completion notifications (seconds, not polling intervals of a minute+).
- Daemon survives TUI restarts; TUI survives daemon restarts (reconnect).

**Non-Goals:**

- Controlling or interacting with agents (sending input, killing sessions) - this change is observability only.
- Tracking non-Claude-Code processes or remote/SSH sessions.
- Cross-machine or multi-user visibility - single local user, single machine.
- A GUI or menu-bar app - terminal UI only for this change.
- **Desktop app session tracking** (descoped during end-to-end verification, see the "Desktop app sessions are unreachable" risk below) - the desktop app already has its own built-in notifications, reducing the value of chasing this further.

## Decisions

### Hooks as the status source, no file-watching fallback

Claude Code hooks (`Notification`, `Stop`, `SubagentStop`, and similar lifecycle hooks) can run an arbitrary command with structured JSON on stdin, and are invoked by Claude Code itself. Since the same binary handles hooks whether it's embedded in nvim, a standalone terminal, or the desktop app, this is naturally host-agnostic and avoids parsing terminal/session log formats, which was the alternative considered and rejected as more fragile and higher-latency (see the earlier clarification with the user). The hook command will be a small CLI (e.g. `agentmon-report`) that reads the hook's JSON payload, adds identifying context (cwd, a host-context guess, pid), and writes it to the daemon's Unix socket.

Trade-off accepted: agents without the hook configured in their `settings.json` simply won't appear. This change includes a setup helper to install the hook config, rather than a passive fallback, to keep the daemon's ingestion path simple.

**Hook-to-status mapping (decided during implementation of `agentmon-report`):** `UserPromptSubmit` → `running`, `Stop` → `done`, `Notification` → `needs_input` (only when `notification_type` is one of `permission_prompt`, `idle_prompt`, `elicitation_dialog`, `elicitation_url_dialog`, `agent_needs_input` - enforced both by the `settings.json` hook `matcher` and again defensively in code). `UserPromptSubmit` was added beyond the proposal's illustrative hook list specifically so a session's status can return to `running` after each turn - without it, `Stop` firing on every turn completion combined with the notify-on-transition-only rule would mean a long session only ever gets one "done" notification, for its first turn. `SubagentStop` was left unwired: it shares its parent session's `session_id`, and marking the whole session "done" when just one subagent finishes doesn't reflect the session's actual state.

### Unix domain socket for IPC, framed JSON messages

Chosen over a loopback HTTP server: no port management, file-permission-based access control (0600, owning user only) instead of relying on binding to loopback, lower overhead for a long-lived push connection. The TUI keeps one persistent connection open and receives a stream of newline-delimited JSON frames (initial full snapshot, then incremental updates); hook events are short-lived connections that send one event and close.

### Agent identity: session id + working directory + host context

Each agent is keyed by Claude Code's session id. Host context is derived from information available to the hook command at invocation time (e.g. parent process inspection to distinguish nvim's embedded terminal vs. a standalone terminal emulator vs. the desktop app's own process tree). Working directory comes from the hook payload / `cwd` at invocation. This tuple is what the TUI renders per proposal.md's "identity" decision.

### Daemon as a launchd user agent

A per-user `LaunchAgent` plist (not a system-wide `LaunchDaemon`) is used since this tracks a single user's interactive sessions and needs the user's own permission context for notifications. The daemon binary supports both `--foreground` (development) and being launched by `launchd` (which expects a long-running process, restart-on-crash via `KeepAlive`).

### Stale detection via periodic liveness sweep

Rather than only reacting to hook events, the daemon periodically checks whether each tracked pid still exists and marks agents stale if not. This covers the case where a session is killed without a final hook firing (e.g. terminal closed, process killed -9).

## Risks / Trade-offs

- [Hook configuration is a manual/setup step] → Ship an install helper that patches the user's Claude Code `settings.json` hooks section, and have the TUI clearly show "daemon running, 0 agents" vs "daemon unreachable" so a missing-hooks setup doesn't look like a crash.
- [Host-context detection (nvim vs standalone terminal) relies on inspecting the process tree, which is inherently a bit heuristic] → Treat host context as best-effort labeling only; never let it affect status tracking or notification correctness, only the display label.
- [launchd `KeepAlive` restarting a crash-looping daemon] → Use `ThrottleInterval` in the plist and log crash reasons; this is standard launchd practice.
- [Socket file left behind after an unclean shutdown could block restart] → Daemon checks for a stale socket (connect-test, then unlink) before binding, per the agent-daemon spec's socket lifecycle requirement.
- **[Desktop app sessions are unreachable]** → Confirmed during end-to-end verification (task 8.7): the desktop app executes sessions in a sandboxed environment (evidence: `~/Library/Application Support/Claude/claude-code-vm`) that cannot exec local shell commands, so `type: "command"` hooks - the only kind this change implements - never fire for desktop-hosted sessions; `agentmon-report`'s process-tree walk never even runs. Claude Code hooks reportedly also support a `type: "http"` webhook variant that can reach out of that sandbox, which could enable desktop support in a future change (the daemon would need an HTTP listener alongside the Unix socket), but that's out of scope here. `HostContext::Desktop` remains in the data model as a label, but in practice it will rarely if ever be populated with the current hook mechanism. The desktop app has its own built-in notifications, which reduces how much this actually matters.

## Migration Plan

Net-new capability, no existing users or data to migrate. Rollout is: build daemon + TUI + install helper, install the launchd service and hooks locally, dogfood, then treat `tasks.md` completion as "ready to use." Rollback is simply `launchctl unload` the plist and remove the hook entries from `settings.json`; no persisted state outside the daemon's in-memory registry needs cleanup.
