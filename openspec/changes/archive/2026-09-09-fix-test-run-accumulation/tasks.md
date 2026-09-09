## 1. Registry fix (`agentd`)

- [x] 1.1 In `crates/agentd/src/registry.rs`, rekey the test-run store from `HashMap<(PathBuf, u32), TestRunInfo>` to `HashMap<PathBuf, TestRunInfo>`, and update `upsert_test_run` to always overwrite whatever entry exists at that `cwd` (dropping `pid` from the identity/dedup key while keeping it as a field on `TestRunInfo`), verified by `cargo build -p agentd`
- [x] 1.2 Update `different_pids_in_the_same_directory_do_not_collide` to assert the new behavior (rename if needed): two different pids reporting to the same directory collapse to exactly one entry - the latest - not two - renamed to `different_pids_in_the_same_directory_collapse_to_one_entry`
- [x] 1.3 Add a test asserting that a later test run for the same directory replaces an earlier one from a *different* pid (the accumulation bug's exact repro: sequential invocations, each a new pid, must never coexist) - `a_later_test_run_from_a_different_pid_replaces_the_previous_one`; also added `different_directories_each_keep_their_own_test_run` to confirm the fix didn't over-collapse across directories
- [x] 1.4 Re-run the full `agentd` test suite (`cargo test -p agentd`) and confirm `directory_groups`/`ingest_test_run`/`notify` tests (which exercise test-run behavior indirectly) still pass unmodified - 56/56 passed

## 2. Verification

- [x] 2.1 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets` to confirm no regressions outside `agentd` - 132/132 passed, no new warnings
- [x] 2.2 Manually verify against an isolated daemon (per the established preview approach: isolated `HOME`, never the default socket) that repeated `rspec` invocations in the same directory show exactly one test-run row that updates in place, not one row per invocation - reproduced the exact bug (4 sequential invocations) against the fixed daemon; confirmed exactly one row survived, reflecting the latest pid/status
