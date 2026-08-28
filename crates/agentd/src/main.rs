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
    let binary_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("agentd: could not determine this binary's path: {err}");
            std::process::exit(1);
        }
    };

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
