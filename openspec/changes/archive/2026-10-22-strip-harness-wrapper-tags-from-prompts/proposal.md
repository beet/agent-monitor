## Why

Claude Code's `UserPromptSubmit` hook payload's `prompt` field is not always literally what a human typed: a slash command's expansion, an autonomous-loop or scheduled wake-up re-entering the model, and a backgrounded task's completion notification all resubmit a turn wrapped in the harness's own structural tags (e.g. `<command-message>`/`<command-name>`/`<command-args>`, `<system-reminder>`, `<task-notification><task-id>...</task-id>...</task-notification>`). `agentmon-report` forwards that payload's `prompt` text to the daemon verbatim, and the TUI renders it unmodified wherever a last-prompt snapshot or prompt-category log entry is shown, so the user occasionally sees raw tag soup instead of anything resembling a human-authored prompt - sporadically, because it only happens on these harness-injected turns rather than on a plainly typed message, which is why it's been hard to pin down.

## What Changes

- `agentmon-report` recognizes Claude Code's known harness-injected wrapper tags in a `UserPromptSubmit` payload's `prompt` text and strips them before constructing the reported event, keeping whatever human-authored text survives outside those tags.
- When stripping leaves no human-authored text at all (e.g. a prompt consisting entirely of a `<task-notification>` wrapper with no user content), the resulting event SHALL carry no prompt text, the same as an event that never had one - not an empty or whitespace-only string - so it does not overwrite an agent's last prompt or produce a prompt-category log entry.
- No change to how the daemon's registry, activity log, or the TUI store, log, or render prompt text once an event carries it - they continue to treat whatever text (if any) an event carries as the authoritative prompt, verbatim.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-daemon`: the reporting client's handling of a `UserPromptSubmit` payload's `prompt` field now strips known harness-injected wrapper tags before it becomes an event's prompt text, and reports no prompt text at all when nothing human-authored survives that stripping.

## Impact

- `crates/agentmon-report/src/hook_payload.rs` and/or `crates/agentmon-report/src/report.rs`: new stripping step between parsing the raw hook payload and building the `ReportEvent`'s prompt field.
- `crates/agentmon-proto`: no wire-format change expected - the prompt field remains an optional string, just filtered earlier.
- `crates/agentd` (registry, activity log) and `crates/agentmon` (TUI rendering): no behavior change - both already handle "no prompt text" for an event, and both already render whatever text they're given verbatim.
- `openspec/specs/agent-daemon/spec.md`: delta adding this stripping/suppression behavior to the existing "Agent registry" requirement's prompt-text handling.
