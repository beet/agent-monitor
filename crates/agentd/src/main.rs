use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agentd::ingest::Ingestor;
use agentd::launchd;
use agentd::notify::OsaScriptNotifier;
use agentd::registry::Registry;
use agentd::server::serve;
use agentd::socket::{bind_socket, default_socket_path, BindError};

const LIVENESS_SWEEP_INTERVAL: Duration = Duration::from_secs(5);

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("install") => run_install(),
        Some("uninstall") => run_uninstall(),
        Some("--foreground") | None => run_server(),
        Some(other) => {
            eprintln!("agentd: unknown argument '{other}'");
            eprintln!("usage: agentd [--foreground] | agentd install | agentd uninstall");
            std::process::exit(1);
        }
    }
}

/// Runs the daemon in the foreground: used both for local development
/// (`agentd --foreground`) and by launchd itself, which expects a
/// supervised, non-daemonizing child process rather than one that forks.
fn run_server() {
    let path = default_socket_path();
    match bind_socket(&path) {
        Ok(listener) => {
            println!("agentd listening on {}", path.display());
            let ingestor = Ingestor::new(Registry::new(), Arc::new(OsaScriptNotifier));
            serve(listener, ingestor, LIVENESS_SWEEP_INTERVAL);
        }
        Err(BindError::AlreadyRunning) => {
            eprintln!(
                "agentd is already running (socket at {} is active)",
                path.display()
            );
            std::process::exit(1);
        }
        Err(BindError::Io(err)) => {
            eprintln!("failed to bind agentd socket at {}: {err}", path.display());
            std::process::exit(1);
        }
    }
}

fn run_install() {
    let Some(arg0) = std::env::args_os().next() else {
        eprintln!("agentd: could not determine this binary's invoked path (no argv[0])");
        std::process::exit(1);
    };
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("agentd: could not determine the current directory: {err}");
            std::process::exit(1);
        }
    };
    let binary_path = resolve_binary_path(Path::new(&arg0), &cwd);

    match launchd::install(launchd::LABEL, &binary_path) {
        Ok(plist_path) => println!(
            "agentd: installed and started the launchd service ({})",
            plist_path.display()
        ),
        Err(err) => {
            eprintln!("agentd: failed to install the launchd service: {err}");
            std::process::exit(1);
        }
    }
}

fn run_uninstall() {
    match launchd::uninstall(launchd::LABEL) {
        Ok(()) => println!("agentd: uninstalled the launchd service"),
        Err(err) => {
            eprintln!("agentd: failed to uninstall the launchd service: {err}");
            std::process::exit(1);
        }
    }
}

/// Resolves the binary path to install into the launchd plist, without
/// following symlinks. `current_exe()` canonicalizes symlinks, which would
/// bake a Homebrew-managed `agentd` invocation down to its version-pinned
/// Cellar target; using `arg0` as-is instead preserves the stable,
/// Homebrew-repointed path so an upgrade only needs a restart, not a
/// reinstall. See design.md - Decisions.
fn resolve_binary_path(arg0: &Path, cwd: &Path) -> PathBuf {
    if arg0.is_absolute() {
        arg0.to_path_buf()
    } else {
        cwd.join(arg0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_binary_path_keeps_an_absolute_arg0_unchanged() {
        let arg0 = Path::new("/usr/local/bin/agentd");
        let cwd = Path::new("/Users/beet/some/project");

        assert_eq!(resolve_binary_path(arg0, cwd), PathBuf::from(arg0));
    }

    #[test]
    fn resolve_binary_path_joins_a_relative_arg0_onto_cwd() {
        let arg0 = Path::new("target/debug/agentd");
        let cwd = Path::new("/Users/beet/Documents/Projects/enclaudinate");

        assert_eq!(
            resolve_binary_path(arg0, cwd),
            PathBuf::from("/Users/beet/Documents/Projects/enclaudinate/target/debug/agentd")
        );
    }
}
