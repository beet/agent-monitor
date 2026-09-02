## 1. Remove self-managed launchd code

- [x] 1.1 Delete `crates/agentd/src/launchd.rs` and its `mod launchd;` export in `crates/agentd/src/lib.rs`, and verify `cargo build -p agentd` fails only on the now-dangling references fixed in 1.2
- [x] 1.2 Remove the `install`/`uninstall` match arms, `run_install`, `run_uninstall`, and `resolve_binary_path` from `crates/agentd/src/main.rs`, leaving `agentd` / `agentd --foreground` as the only entry points, and verify `cargo build -p agentd` succeeds
- [x] 1.3 Update the usage string in `main.rs` to drop `install`/`uninstall`, and verify `cargo test -p agentd` passes with no remaining references to the deleted module (also deleted `crates/agentd/tests/launchd_live.rs`, an integration test that only exercised the removed `launchd` module)

## 2. Add the Homebrew service block

- [x] 2.1 Clone `beet/homebrew-agent-monitor` to scratch and add a `service do` block to `Formula/agent-monitor.rb`: `run [opt_bin/"agentd"]`, `keep_alive true`, `run_type :immediate`, `restart_delay 5`, `log_path`/`error_log_path` pointed at `~/Library/Logs/agentmon/agentd.{out,err}.log`, and verify `brew style` passes on the formula (also verified `ruby -c` syntax)
- [x] 2.2 Update the formula's `caveats` (or equivalent install-time message) to reference `brew services start agent-monitor` instead of `agentd install`, and verify by reading the rendered caveats text

## 3. Update documentation

- [x] 3.1 Update `README.md`'s Setup section: replace `agentd install` with `brew services start agent-monitor`
- [x] 3.2 Update `README.md`'s Upgrade section: replace `pkill agentd` with `brew services restart agent-monitor`, and delete the stable-path migration caveat since `brew services` always targets the current `opt/` path

## 4. Roll out on both machines

- [ ] 4.1 On this machine, run `agentd uninstall` using the currently-installed (pre-migration) binary, confirm `~/Library/LaunchAgents/com.agentmon.agentd.plist` is gone via `launchctl list | grep agentmon`
- [ ] 4.2 Upgrade to the release containing this change, run `brew services start agent-monitor`, and verify via `brew services list` that `agent-monitor` shows `started` and via `launchctl list | grep agent-monitor` that exactly one instance is running
- [ ] 4.3 Repeat 4.1-4.2 on the second machine
- [ ] 4.4 Verify end-to-end: trigger a Claude Code session completion and confirm the macOS notification still fires, then verify `brew services restart agent-monitor` picks up a subsequent version bump without a manual reinstall
