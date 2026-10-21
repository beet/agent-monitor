use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use agentmon_proto::default_socket_path;
use agentmon_report::debug_log::log;
use agentmon_report::host_context::{detect_host_context, parent_pid};
use agentmon_report::install_hooks::{default_settings_path, install_hooks};
use agentmon_report::report::{connect, read_and_build_event, send_on};

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("install-hooks") => run_install_hooks(),
        // Any other invocation is a hook reporting an event on stdin. This
        // must never fail the calling hook, so every error path below logs
        // and exits 0 rather than propagating a failure - see
        // report::SEND_TIMEOUT's doc comment for why.
        _ => run_report(),
    }
}

fn run_install_hooks() {
    let settings_path = default_settings_path();
    let command = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("agentmon-report: could not determine this binary's path: {err}");
            std::process::exit(1);
        }
    };

    match install_hooks(&settings_path, &command.to_string_lossy()) {
        Ok(()) => println!(
            "agentmon-report: installed hooks in {}",
            settings_path.display()
        ),
        Err(err) => {
            eprintln!(
                "agentmon-report: failed to install hooks in {}: {err}",
                settings_path.display()
            );
            std::process::exit(1);
        }
    }
}

fn run_report() {
    // Captured before anything else in this process - including the debug
    // log write below - so it reflects true hook-fire time rather than
    // being skewed by this process's own startup/detection work. See the
    // agent-daemon spec's "Local socket ingestion API" requirement.
    let fired_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    log("invoked");

    let host_context = detect_host_context();
    let Some(pid) = parent_pid() else {
        log("could not determine parent pid; skipping");
        eprintln!("agentmon-report: could not determine the Claude Code process id; skipping");
        return;
    };
    log(&format!("host_context={host_context:?} pid={pid}"));

    // Connects before reading stdin, not after: the daemon applies events in
    // the order their connections are accepted, so this process must claim
    // its place in that order before doing any payload-size-dependent work -
    // see report::connect's doc comment.
    let mut stream = match connect(&default_socket_path()) {
        Ok(stream) => stream,
        Err(err) => {
            log(&format!("could not reach agentd: {err}"));
            eprintln!("agentmon-report: could not reach agentd, skipping: {err}");
            return;
        }
    };

    let event = match read_and_build_event(io::stdin(), host_context, pid, fired_at_ms) {
        Ok(Some(event)) => event,
        Ok(None) => {
            log("payload parsed but produced no status-bearing event");
            return;
        }
        Err(err) => {
            log(&format!("failed to parse hook payload: {err}"));
            eprintln!("agentmon-report: could not parse hook payload: {err}");
            return;
        }
    };
    log(&format!(
        "built event: session={:?} status={:?} cwd={}",
        event.session_id,
        event.status,
        event.cwd.display()
    ));

    match send_on(&mut stream, &event) {
        Ok(()) => log("sent event to agentd"),
        Err(err) => {
            log(&format!("could not reach agentd: {err}"));
            eprintln!("agentmon-report: could not reach agentd, skipping: {err}");
        }
    }
}
