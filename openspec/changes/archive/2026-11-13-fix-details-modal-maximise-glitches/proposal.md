## Why

Toggling the details modal's maximised state has two rough edges. First, restoring it from maximised to windowed can leave stale characters on screen: ratatui's incremental diffing assumes consecutive buffer writes advance the real cursor exactly as far as its own width model predicts, and the Logs pane's `🗣️` marker (`U+1F5E3` + the `U+FE0F` variation selector, needed to force wide/emoji rendering) can disagree with how the terminal actually advances the cursor for that sequence. When that happens, one or more real screen columns are skipped and keep showing a fragment of whatever was drawn there in the previous frame - most visibly right after maximise/un-maximise forces a full reflow of every row, but reproducible on any redraw where a `🗣️`-led row replaces a `🤖`-led row (or vice versa) at the same screen position. Second, the modal's `Maximise [m]` hint never changes to reflect the modal's current state, even though it already goes bold specifically to signal "maximised" - the hint's own label stays wrong at the moment it matters most.

## What Changes

- Force a full terminal redraw whenever the details modal's maximised state is toggled, so every cell is freshly written via an explicit cursor move instead of relying on ratatui's incremental diff - this sidesteps the consecutive-write misalignment rather than trying to fix emoji width accounting.
- Change the modal's top-right hint from always reading `Maximise [m]` to reading `Un-maximise [m]` while the modal is maximised, and `Maximise [m]` while windowed - the bold/non-bold styling is unchanged, this only changes the label text itself.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-monitor-tui`: the "Project details modal" requirement's maximise/restore behavior - toggling maximised state now forces a full redraw (no stale characters left on screen), and the top-right hint's text now reflects the modal's current size instead of always reading `Maximise [m]`.

## Impact

- `crates/agentmon/src/main.rs`: the main draw loop needs a way to force the next `terminal.draw` to treat the whole screen as dirty when the maximised toggle fires.
- `crates/agentmon/src/input.rs`: the `m`/`Esc` handling that toggles `modal_maximised` needs to signal that a full redraw is required.
- `crates/agentmon/src/ui.rs`: the hint's rendered text in `render_details_modal` changes from a fixed string to one that depends on `app.modal_maximised`.
- Existing tests asserting the literal string `"Maximise [m]"` while maximised (`ui.rs` around the maximise-hint-style tests) need their expectations updated to `"Un-maximise [m]"`.
- `openspec/specs/agent-monitor-tui/spec.md`'s "Project details modal" requirement and its scenarios.
