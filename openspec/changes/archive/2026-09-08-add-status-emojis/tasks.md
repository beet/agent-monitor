## 1. Status labels and styles

- [x] 1.1 Update `status_label_and_style` in `crates/agentmon/src/ui.rs` to prefix each label with its emoji (🔧 running, 💤 idle, 🔔 NEEDS INPUT, ✅ done, 🕸️ stale) and verify by running the crate and visually inspecting the table
- [x] 1.2 Change the `NeedsInput` `Style` from black-on-yellow background to bold yellow foreground text with no background, and verify the row no longer sets `Color::Black`/`.bg(...)`

## 2. Update existing tests

- [x] 2.1 Update assertions in `crates/agentmon/src/ui.rs` tests that check for the plain-text status labels (e.g. `contains("running")`) to account for the emoji prefix, and verify `cargo test -p agentmon` passes
- [x] 2.2 Update `needs_input_is_visually_distinguished_from_other_statuses` to assert the new bold-foreground-only style, and verify the test passes
