## 1. Pin the Agents tab selection to the open modal's project

- [x] 1.1 In `crates/agentmon/src/app.rs`, change `clamp_agents_selected` so that when `self.modal` is `Some(Modal::Details(cwd))`, it re-resolves `agents_selected` to the index of the row in `visible_agent_groups()` whose `cwd` matches the modal's `cwd` (falling back to length-based clamping, as today, if that project is no longer present in the visible rows). Verify with a unit test that asserts the index tracks the project across a re-sort.
- [x] 1.2 Verify `apply_snapshot`, `apply_update`, `remove_agent`, and `apply_test_run_update` all route through the updated `clamp_agents_selected` with no other changes needed, since they already call it as their last step.

## 2. Tests

- [x] 2.1 Add a test that opens the details modal for project A, then applies an update to project B that moves B ahead of A in recency order, and asserts `agents_selected` still indexes A's row (not B's).
- [x] 2.2 Add a test that closes the modal after such a reorder and asserts the Agents tab's selected row is still A's, matching the "Closing the modal after a reorder leaves the right row highlighted" scenario.
- [x] 2.3 Add a test that applies a search filter (or an agent removal) that drops the modal's project out of `visible_agent_groups()` while the modal is open, and asserts `agents_selected` falls back to length-based clamping without panicking, matching the "modal's project drops out of the visible list" scenario.
- [x] 2.4 Run `cargo test -p agentmon` and confirm all tests pass.

## 3. Manual verification

- [x] 3.1 Per the project's manual-testing approach (isolated `HOME`, running `agentd` + `agentmon` + `agentmon-report`), reproduce the original bug with two or more tracked projects: open the details modal on one project, trigger an update on another project that reorders the list, close the modal, and confirm the highlight remains on the originally-viewed project.
