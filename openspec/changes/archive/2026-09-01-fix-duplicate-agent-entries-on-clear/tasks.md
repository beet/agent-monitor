## 1. Registry dedup by pid

- [x] 1.1 In `crates/agentd/src/registry.rs`, change `Registry::upsert` so that when the incoming event's session id is not already in the registry but its pid matches an existing entry under a different session id, that existing entry is removed before the new one is inserted.
- [x] 1.2 Add a registry unit test: upsert a "session-1"/pid 123 event, then upsert a "session-2"/pid 123 event, and verify `snapshot()` contains exactly one entry, for "session-2".
- [x] 1.3 Add a registry unit test confirming unrelated pids are unaffected: upsert "session-1"/pid 123, then "session-2"/pid 456, and verify `snapshot()` contains both entries.
- [x] 1.4 Run `cargo test -p agentd` and verify all registry and liveness tests pass, including the existing tests that reuse the same session id across events.

## 2. Verify against the spec

- [x] 2.1 Confirm the new scenario "A new session id for an already-tracked pid replaces the old entry" in `openspec/changes/fix-duplicate-agent-entries-on-clear/specs/agent-daemon/spec.md` is covered by the tests added in 1.2, by re-reading the test names against the scenario text.
