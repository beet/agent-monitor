## 1. Add timestamp formatting dependency

- [x] 1.1 Add `chrono = { version = "0.4", default-features = false, features = ["clock"] }` to `crates/agentmon/Cargo.toml` and verify `cargo build -p agentmon` succeeds

## 2. Sort agents by recency at render time

- [x] 2.1 In `crates/agentmon/src/ui.rs`, add a helper that returns `app.agents` sorted by `last_updated_ms` descending (e.g. a `Vec<&AgentInfo>`), and use it in `render_agent_table` instead of iterating `app.agents` directly
- [x] 2.2 Add a unit test in `ui.rs` asserting that when agents are applied out of recency order, the rendered table shows the most recently updated agent's identifying text (e.g. project name or pid) before an older agent's
- [x] 2.3 Add a unit test in `ui.rs` asserting that an incremental update via `App::apply_update` which changes a non-top agent's `last_updated_ms` to the newest value moves that agent's row to the top on the next render

## 3. Add local-time last-updated column

- [x] 3.1 In `crates/agentmon/src/ui.rs`, add a function that converts an `AgentInfo`'s `last_updated_ms` (unix epoch milliseconds) to the system's local timezone and formats it as `%Y-%m-%d %H:%M:%S` using `chrono::Local`
- [x] 3.2 Add an "UPDATED" column to the header and each row in `render_agent_table`, adjusting column `Constraint`s so the table still fits typical terminal widths
- [x] 3.3 Add a unit test in `ui.rs` that, for a known `last_updated_ms` value, independently computes the expected `%Y-%m-%d %H:%M:%S` string via `chrono::Local` at test run time (not a hardcoded string - confirmed during implementation that `chrono::Local` does not honor a runtime `TZ` override on macOS) and asserts the rendered table contains it

## 4. Remove row selection/highlighting

- [x] 4.1 In `crates/agentmon/src/app.rs`, remove the `selected` field, `select_next`, `select_previous`, and `clamp_selection`, and remove their calls from `apply_snapshot`/`apply_update`
- [x] 4.2 In `crates/agentmon/src/input.rs`, remove the `Down`/`Char('j')` and `Up`/`Char('k')` match arms from `handle_key`, leaving only the quit keys (also dropped `handle_key`'s now-unused `&mut App` parameter and updated the `main.rs` call site)
- [x] 4.3 In `crates/agentmon/src/ui.rs`, remove the `TableState` selection/highlight (`with_selected`, `row_highlight_style`) from `render_agent_table`
- [x] 4.4 Remove or rewrite the now-obsolete tests: `app.rs`'s `select_next_and_previous_move_within_bounds`, `selection_is_a_no_op_with_no_agents`, `selection_clamps_when_the_list_shrinks`; `input.rs`'s `down_and_j_select_the_next_row`, `up_and_k_select_the_previous_row` (kept `unrecognized_keys_are_ignored`, dropping its `app.selected` assertion)

## 5. Update existing coverage and verify

- [x] 5.1 Review remaining tests in `crates/agentmon/src/app.rs` and `crates/agentmon/src/ui.rs` that assert on row order or column layout (e.g. `seeded_agents_are_rendered_in_the_table`, `needs_input_is_visually_distinguished_from_other_statuses`) and update them for the new column and sort behavior
- [x] 5.2 Run `cargo test -p agentmon` and confirm the full suite passes, including the new sort-order and timestamp tests from sections 2 and 3, and that no test references removed selection behavior
