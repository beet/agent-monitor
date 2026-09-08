## Why

The TUI currently distinguishes agent statuses with text labels and color/style alone (`crates/agentmon/src/ui.rs:103-122`). Adding a per-status emoji makes the status scannable at a glance without reading text, and the current "needs input" treatment (black text on a solid yellow background) is hard to read — plain colored text is more legible.

## What Changes

- Prefix each status label with a distinct emoji: running → 🔧, idle → 💤, needs input → 🔔, done → ✅, stale → 🕸️. (🔧 and 🔔 were chosen over ⚙️/⚠️ because those two render as thin monochrome outlines rather than filled color glyphs in the user's environment.)
- Change the "needs input" style from black-on-yellow solid background to bold yellow foreground text with no background fill.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-monitor-tui`: the "Status is visually distinguishable" requirement now specifies an emoji marker per status in addition to color, and the needs-input scenario reflects colored text instead of a solid background.

## Impact

- `crates/agentmon/src/ui.rs`: `status_label_and_style` labels and the `NeedsInput` `Style`.
- Existing tests asserting plain-text status labels (e.g. `contains("running")`) and the needs-input visual-distinction test in `crates/agentmon/src/ui.rs` will need their expected strings/styles updated to match.
