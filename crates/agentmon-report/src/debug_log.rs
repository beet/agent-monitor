use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Appends a line to a small diagnostic log, for figuring out whether (and
/// how) a hook actually invoked this binary - e.g. when a host like the
/// desktop app silently never fires it. Best-effort: a logging failure must
/// never affect the calling hook, so all errors here are swallowed.
pub fn log(message: &str) {
    let Some(path) = log_path() else { return };
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let _ = writeln!(file, "[{millis}] {message}");
}

fn log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("agentmon")
            .join("agentmon-report.log"),
    )
}
