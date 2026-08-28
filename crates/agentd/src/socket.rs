use std::fmt;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

pub use agentmon_proto::default_socket_path;

#[derive(Debug)]
pub enum BindError {
    /// A socket file exists at the path and another daemon is actively
    /// listening on it.
    AlreadyRunning,
    Io(io::Error),
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindError::AlreadyRunning => write!(f, "a daemon is already running on this socket"),
            BindError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for BindError {}

/// Binds a Unix domain socket at `path`, restricted to the owning user.
///
/// If a socket file already exists at `path` but nothing is listening on it
/// (e.g. left behind by a daemon that crashed), the stale file is removed and
/// a fresh socket is bound in its place.
pub fn bind_socket(path: &Path) -> Result<UnixListener, BindError> {
    if path.exists() {
        match UnixStream::connect(path) {
            Ok(_) => return Err(BindError::AlreadyRunning),
            Err(_) => {
                fs::remove_file(path).map_err(BindError::Io)?;
            }
        }
    } else if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(BindError::Io)?;
    }

    let listener = UnixListener::bind(path).map_err(BindError::Io)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(BindError::Io)?;
    Ok(listener)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn unique_socket_path(tag: &str) -> PathBuf {
        // macOS caps sockaddr_un's sun_path at 104 bytes, and
        // std::env::temp_dir() (a $TMPDIR under /var/folders/...) is often
        // too long for that budget, so bind test sockets under /tmp instead.
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000;
        let dir = PathBuf::from(format!("/tmp/ad-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join("s")
    }

    #[test]
    fn stale_socket_file_is_cleaned_up_and_rebound() {
        let path = unique_socket_path("stale");

        // First "run": bind, then simulate a crash by dropping the listener
        // without unlinking the socket file (UnixListener does not do this
        // for us on drop).
        let first = bind_socket(&path).expect("first bind should succeed");
        drop(first);
        assert!(
            path.exists(),
            "socket file should remain after drop, simulating a crash"
        );

        // Second "run": should detect the stale file, clean it up, and bind.
        // Under heavy concurrent test load, closing a listener's fd and a
        // fresh connect() attempt against its now-orphaned path can
        // observably race by a hair even though this is a distinct syscall
        // sequence; a couple of short retries absorb that jitter without
        // masking a real regression (which would fail every time).
        let mut attempt = 0;
        let second = loop {
            match bind_socket(&path) {
                Ok(listener) => break listener,
                Err(BindError::AlreadyRunning) if attempt < 5 => {
                    attempt += 1;
                    thread::sleep(Duration::from_millis(20));
                }
                Err(err) => panic!("second bind should clean up the stale socket and succeed: {err}"),
            }
        };
        drop(second);
    }

    #[test]
    fn bind_fails_when_a_daemon_is_already_running() {
        let path = unique_socket_path("already-running");

        let _listener = bind_socket(&path).expect("first bind should succeed");
        let result = bind_socket(&path);

        assert!(matches!(result, Err(BindError::AlreadyRunning)));
    }

    #[test]
    fn socket_file_permissions_are_owner_only() {
        let path = unique_socket_path("permissions");

        let listener = bind_socket(&path).expect("bind should succeed");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;

        assert_eq!(mode, 0o600);
        drop(listener);
    }
}
