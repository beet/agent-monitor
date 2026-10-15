## Why

A project's displayed name is always derived from its working directory's final path segment. On codebases where the worktree/branch naming convention bakes in a ticket number, that's enough to tell projects apart at a glance. On codebases that don't, two or more rows can look identical or meaninglessly similar, and there's no way to attach something the user can actually correlate back to (e.g. a GitHub PR number). The user wants to set their own display name per project to close that gap.

## What Changes

- Adds a `Rename [n]` keyboard shortcut, shown in the Agents tab's title next to the existing `Filter [/]` hint, and in the details modal's title next to the project name.
- Pressing the shortcut opens a rename form, pre-populated with the project's current displayed name. `Esc` cancels without changing anything; `Enter` validates and applies the new name, refreshing every view that displays that project (Agents tab, details modal, Logs tab, Reminders tab) immediately.
- Validates that a submitted name is non-blank and unique across every project the daemon currently knows about - including projects still showing their default (directory-derived) name. An invalid submission is rejected inline; the form stays open so the user can correct it.
- A renamed project's historic activity-log entries display the project's current name, not whatever name was in effect when each entry was recorded - the daemon has no per-entry name to begin with, it always derives the displayed name from the working directory at render/query time.
- Custom names live only in the daemon's in-memory state, same lifetime as every other piece of daemon state (tracked agents, test runs, reminders, the activity log): a daemon restart loses them, consistent with existing behavior rather than introducing a new persistence mechanism.

## Capabilities

### New Capabilities
- `project-renaming`: The daemon-side rules for giving a project (working directory) a custom display name - validation (non-blank, unique among all known projects whether renamed or not), how a rename is requested and broadcast to connected clients, and that custom names share the daemon's in-memory, restart-clears-everything lifetime.

### Modified Capabilities
- `agent-monitor-tui`: Adds the `Rename [n]` shortcut hint to the Agents tab title and the details modal title; adds the rename form (pre-populated, `Esc`/`Enter` behavior, inline validation error); and clarifies that every view showing a project name (Agents tab, details modal, Logs tab, Reminders tab) always renders its current name, including for historic activity-log entries recorded before the rename.

## Impact

- `crates/agentmon-proto`: a new `ClientMessage` variant to request a rename, and a `ServerMessage` variant (or an addition to `Snapshot`) to carry the current project-name mapping to clients.
- `crates/agentd`: the registry gains a `cwd -> custom name` map, validation logic for the rename request, and broadcasts the result to connected clients.
- `crates/agentmon`: a new rename form modal (mirroring the existing reminder form's shape), the two new keyboard-shortcut hints, and every place that currently calls `project_name(cwd)` to resolve names through the custom-name map first.
- `README.md`'s data-model diagram, if the wire-protocol shape changes in a way that affects it (per this repo's `CLAUDE.md`).
