## Why

The project details modal is currently a fixed-size overlay centered on screen. For a project with many agents, reminders, or log entries, its panes paginate quickly even though the terminal has unused space around the modal's border. Letting the user maximise the modal to fill the content area gives more room to read without changing any of the modal's existing pane layout or interaction rules.

## What Changes

- Add a `m` keyboard shortcut that, while the details modal is open, toggles it between its existing windowed size and a maximised size that takes over the entire screen, including the rows the tab bar normally occupies - the modal's own keyboard shortcuts take priority over everything else while it's open, so the tab bar can't actually be interacted with regardless, and reserving space for it while maximised would only waste room.
- The maximised/windowed choice is a single sticky preference, not per-project: closing the modal and reopening it later (from the same tab or a different one) reopens it in whichever state - maximised or windowed - it was last left in.
- `Esc` behavior changes contextually: while maximised, `Esc` reverts the modal to windowed mode (it does not close the modal); while windowed, `Esc` continues to close the modal exactly as before.
- The modal frame shows a `Maximise [m]` keyboard shortcut hint in its top-right corner. The hint is rendered in bold while the modal is maximised, and in normal weight while windowed, so the modal's current size state is visible without it being a separate line of text.
- No change to the modal's five-pane layout, focus cycling, pagination, or any pane's content rules - maximising only changes the amount of vertical space the existing layout has to work with.

## Capabilities

### Modified Capabilities
- `agent-monitor-tui`: the "Project details modal" requirement gains a maximise/restore toggle (`m`), a sticky maximised/windowed state that survives closing and reopening the modal, a contextual `Esc` behavior (restore-then-close), and a `Maximise [m]` hint in the modal's top-right corner that renders bold while maximised.

## Impact

- `crates/agentmon/src/app.rs`: modal state needs a persisted maximised/windowed flag (outlives a single modal open/close), and `Esc` handling needs to branch on it.
- `crates/agentmon/src/input.rs`: new `m` key handling while the details modal is open/focused.
- `crates/agentmon/src/ui.rs`: modal sizing/layout needs a maximised render path that fills the entire terminal (covering the tab bar rather than leaving space for it), and the modal frame needs the new top-right `Maximise [m]` hint (bold when maximised).
- `openspec/specs/agent-monitor-tui/spec.md`: "Project details modal" requirement and scenarios updated per the delta spec.
