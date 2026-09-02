## Why

`agentd` currently manages its own `launchd` user-agent plist via `agentd install`/`agentd uninstall`, including a workaround (`resolve_binary_path`) to avoid baking a version-pinned Cellar path into the plist. Homebrew's `brew services` mechanism already solves the same problem natively — it always points a generated plist at the current `opt/` symlink — so this custom code is solving a problem Homebrew already solves for formulas that declare a `service` block. Migrating removes ~70 lines of plist-rendering/install/uninstall code and the symlink-resolution workaround, in favor of the standard `brew services start/stop/restart agent-monitor` commands.

## What Changes

- **BREAKING**: Remove `agentd install` and `agentd uninstall` CLI subcommands. `agentd` (or `agentd --foreground`) remains the sole entry point, unchanged, for both direct/dev use and for `launchd` to exec.
- Remove `crates/agentd/src/launchd.rs` (plist rendering, install, uninstall) and the `resolve_binary_path` arg0-vs-symlink workaround in `main.rs`.
- Add a `service do` block to the Homebrew tap's `Formula/agent-monitor.rb` (separate repo, `beet/homebrew-agent-monitor`) so `brew services start/stop/restart agent-monitor` manages the daemon's `launchd` user agent instead.
- Update `README.md`'s Setup and Upgrade sections to use `brew services start agent-monitor` / `brew services restart agent-monitor` in place of `agentd install` / `pkill agentd`.
- No migration tooling or compatibility shim: this is run on exactly two machines, both controlled by the maintainer, who will run `agentd uninstall` (from the currently-installed pre-migration build) by hand on each before upgrading.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-daemon`: The "Runs as a macOS background service" requirement changes from the daemon self-managing its own `launchd` plist (via `install`/`uninstall` subcommands and an arg0-based stable-path workaround) to Homebrew's `service` block managing the plist, with `brew services` as the install/uninstall/upgrade-survival interface instead of daemon subcommands.

## Impact

- Code: `crates/agentd/src/main.rs`, `crates/agentd/src/launchd.rs` (deleted), `crates/agentd/src/lib.rs` (module export).
- Docs: `README.md` (Setup, Upgrade sections).
- External repo: `beet/homebrew-agent-monitor` tap, `Formula/agent-monitor.rb` — adds a `service do` block; this repo's `.openspec.yaml`/edit roots don't cover that repo, so the corresponding task is tracked here but executed against the tap's own checkout.
- No changes to the daemon's socket protocol, registry, or notification behavior.
