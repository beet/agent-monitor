use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_home_dir() -> PathBuf {
    // Kept short: default_socket_path() nests this under
    // "Library/Application Support/agentmon/agentd.sock", and macOS caps
    // sockaddr_un's sun_path at 104 bytes.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        % 1_000_000;
    let dir = PathBuf::from(format!("/tmp/ad-fg-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn foreground_mode_runs_and_logs_to_stdout() {
    let home = unique_home_dir();

    let mut child = Command::new(env!("CARGO_BIN_EXE_agentd"))
        .arg("--foreground")
        .env("HOME", &home)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn agentd --foreground");

    let stdout = child.stdout.take().expect("captured stdout");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) > 0 {
            let _ = tx.send(line);
        }
    });

    let line = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("agentd --foreground should print a startup line to stdout");
    assert!(
        line.contains("agentd listening on"),
        "unexpected stdout line: {line}"
    );

    let _ = child.kill();
    let _ = child.wait();
}
