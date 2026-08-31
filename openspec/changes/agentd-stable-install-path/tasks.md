## 1. Stable path resolution

- [x] 1.1 Add `resolve_binary_path(arg0: &Path, cwd: &Path) -> PathBuf` to `crates/agentd/src/main.rs` (or `launchd.rs`): returns `arg0` unchanged if absolute, otherwise joins it onto `cwd`, without calling `canonicalize`. Add unit tests for both the absolute and relative-`arg0` cases.
- [x] 1.2 Update `run_install` to compute the binary path via `resolve_binary_path(env::args_os().next()..., env::current_dir()?)` instead of `env::current_exe()`, and verify `cargo build -p agentd` succeeds.

## 2. Verification

- [x] 2.1 Run `cargo test -p agentd` and confirm all tests pass, including the new `resolve_binary_path` cases.
- [x] 2.2 Manually verify on this machine: symlink a fake `agentd` path (e.g. `ln -s <built-binary> /tmp/agentd-link`), run `/tmp/agentd-link install`, and confirm the written plist (`~/Library/LaunchAgents/<label>.plist`) references `/tmp/agentd-link`, not the canonicalized target. Then `agentd uninstall` to clean up. (Adapted to use a throwaway test label via a new ignored test, `install_writes_the_symlink_path_not_its_canonicalized_target`, since the real `agentd install` CLI is hardcoded to the production label and this machine has a live daemon running under it.)

## 3. Documentation

- [x] 3.1 Add an "Upgrade" section to README.md: `brew upgrade agent-monitor` then restart the daemon (`pkill agentd`, relying on the plist's `KeepAlive` to relaunch it); note that installs from before this fix need one `agentd uninstall && agentd install` first to pick up the new stable-path plist, after which future upgrades only need the restart.
