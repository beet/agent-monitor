## Why

In the details modal's Logs pane, agent and test-run entries are interleaved in one flat, most-recent-first list. A project's test runs are conceptually children of the agent activity that triggered them, but nothing in the rendering conveys that relationship - every entry looks like an independent, equally-weighted row.

## What Changes

- Prefix test-run category entries in the details modal's Logs pane with a tree branch character (`├─ `) so they read as nested underneath the agent activity, while agent category entries remain unprefixed, forming the trunk of the tree.
- The tree prefix is purely visual: it does not change entry ordering (still most recent first), the existing category-word prefix, status emoji, duration, or the fixed-width right-aligned timestamp column.

## Capabilities

### Modified Capabilities
- `agent-monitor-tui`: the details modal's Logs pane requirement gains a rule that test-run entries render with a leading tree branch prefix and agent entries do not.

## Impact

- Affected code: the details modal's Logs pane rendering in `crates/agentmon/src/ui.rs` (and any log-line formatting helpers it shares with the top-level Logs tab, which is unaffected by this change).
- No daemon, protocol, or data model changes - display-only within the details modal.
