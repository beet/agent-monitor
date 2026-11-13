## 1. `App` tracks a one-shot full-redraw signal

- [x] 1.1 Add a private `needs_full_redraw: bool` field to `App` (defaulting to `false` in `App::new()`), set it to `true` inside `toggle_modal_maximised()` (`crates/agentmon/src/app.rs:732`), and add `pub fn take_needs_full_redraw(&mut self) -> bool` that returns the current value and resets it to `false`. Verify with a unit test that calling `toggle_modal_maximised()` makes the next `take_needs_full_redraw()` return `true`, and that a second call right after returns `false` (one-shot).

## 2. Route the `Esc`-while-maximised path through the same toggle

- [x] 2.1 In `crates/agentmon/src/input.rs`'s `Esc` handling (around line 282), replace the direct `app.modal_maximised = false;` with a call to `app.toggle_modal_maximised();`. Verify the existing `esc_restores_a_maximised_details_modal_instead_of_closing_it` and related `Esc`/maximise tests in `input.rs` still pass unchanged, and add a test asserting `take_needs_full_redraw()` is `true` after `Esc` restores a maximised modal, not just after `m`.

## 3. Force a full terminal redraw after the toggle

- [x] 3.1 In `crates/agentmon/src/main.rs`'s main loop, immediately before `terminal.draw(...)`, call `app.take_needs_full_redraw()` and, if it returns `true`, force a full redraw first. Verify by code review (this module has no existing test harness around the real `Terminal`) that the call sits before `terminal.draw` on every loop iteration, not just the first. Implemented as `terminal.resize(terminal.size()?.into())?`, not `terminal.clear()?` as originally planned - see design.md's corrected decision and task 5's note on why `clear()` is unsafe here.

## 4. Hint text reflects the modal's current size

- [x] 4.1 In `crates/agentmon/src/ui.rs::render_details_modal`, change the top-right hint's text from the fixed string `"Maximise [m]"` to `if app.modal_maximised { "Un-maximise [m]" } else { "Maximise [m]" }`, keeping the existing bold/non-bold `maximise_hint_style` logic unchanged. Update `details_modal_maximise_hint_is_bold_while_maximised` (and any other test asserting the literal `"Maximise [m]"` string while `modal_maximised` is `true`) to expect `"Un-maximise [m]"` instead, and verify `cargo test -p agentmon` passes.

## 5. Manual verification of the original bug

- [x] 5.1 Reproduce the original repro (open the details modal on a project with enough log entries to paginate so both `🤖`-led and `🗣️`-led rows are visible, press `m` to maximise, then `m` again to restore) and confirm no stale characters remain on screen after restoring, and that the top-right hint reads `Maximise [m]` while windowed and `Un-maximise [m]` while maximised. Verified by driving the real `agentmon`/`agentd` binaries under a pty with a standards-faithful headless terminal emulator (`pyte`) and diffing the rendered screen grid before vs. after the maximise/restore round trip, byte for byte - this is how the bug was originally confirmed during exploration, and rerunning it against the fix shows `NO DIFF between before and after` (previously showed a `🗣arted  pid ...` corruption at the row that changed leading emoji). The first attempt at this task also caught a real bug in the task-3 mechanism: `Terminal::clear()` hung/raced against the key-reading thread, as described in task 3 and design.md's corrected decision - this was fixed before this verification passed.

## 6. Final checks

- [x] 6.1 Run `cargo clippy -p agentmon -p agentmon-proto` and `cargo test -p agentmon` and confirm all pass. Skipped `cargo fmt`: the installed rustfmt (1.9.0, no `rustfmt.toml` pinning an older edition of its defaults) reformats the *entire* workspace far beyond this change's touched lines, which would bury this diff in unrelated noise - confirmed, then reverted, during this session. The added code was hand-matched to the surrounding style instead.
