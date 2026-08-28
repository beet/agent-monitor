use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The label used by the real, user-facing install. Tests use a distinct
/// throwaway label so a failed cleanup never leaves anything behind under
/// this identifier.
pub const LABEL: &str = "com.agentmon.agentd";

pub fn plist_path(label: &str) -> PathBuf {
    home_dir()
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{label}.plist"))
}

fn log_dir() -> PathBuf {
    home_dir().join("Library").join("Logs").join("agentmon")
}

fn home_dir() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME environment variable must be set");
    PathBuf::from(home)
}

/// Renders the launchd user-agent plist that runs `binary_path --foreground`
/// at login and restarts it if it exits, throttled so a crash loop can't
/// spin - per design.md's "Daemon as a launchd user agent" decision.
pub fn render_plist(label: &str, binary_path: &str, log_dir: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary_path}</string>
        <string>--foreground</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ThrottleInterval</key>
    <integer>5</integer>
    <key>StandardOutPath</key>
    <string>{log_dir}/agentd.out.log</string>
    <key>StandardErrorPath</key>
    <string>{log_dir}/agentd.err.log</string>
</dict>
</plist>
"#
    )
}

/// Writes the plist for `label` and loads it via `launchctl`, so the daemon
/// starts now and on every future login.
pub fn install(label: &str, binary_path: &Path) -> io::Result<PathBuf> {
    let path = plist_path(label);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let logs = log_dir();
    fs::create_dir_all(&logs)?;

    let xml = render_plist(label, &binary_path.to_string_lossy(), &logs.to_string_lossy());
    fs::write(&path, xml)?;

    let status = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&path)
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "launchctl load exited with {status}"
        )));
    }
    Ok(path)
}

/// Unloads the service via `launchctl` and removes its plist. Safe to call
/// even if it was never installed.
pub fn uninstall(label: &str) -> io::Result<()> {
    let path = plist_path(label);
    if !path.exists() {
        return Ok(());
    }
    // Best-effort: if it's already unloaded (e.g. crashed and launchd gave
    // up), still proceed to remove the plist.
    let _ = Command::new("launchctl").arg("unload").arg(&path).status();
    fs::remove_file(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_plist_includes_the_expected_keys_and_values() {
        let xml = render_plist(
            "com.example.agentd",
            "/usr/local/bin/agentd",
            "/Users/beet/Library/Logs/agentmon",
        );

        assert!(xml.contains("<string>com.example.agentd</string>"));
        assert!(xml.contains("<string>/usr/local/bin/agentd</string>"));
        assert!(xml.contains("<string>--foreground</string>"));
        assert!(xml.contains("<key>KeepAlive</key>\n    <true/>"));
        assert!(xml.contains("<key>ThrottleInterval</key>\n    <integer>5</integer>"));
        assert!(xml.contains("<key>RunAtLoad</key>\n    <true/>"));
        assert!(xml.contains("/Users/beet/Library/Logs/agentmon/agentd.out.log"));
        assert!(xml.contains("/Users/beet/Library/Logs/agentmon/agentd.err.log"));
    }

    #[test]
    fn plist_path_is_under_launch_agents_and_named_by_label() {
        let path = plist_path("com.example.agentd");

        assert!(path.starts_with(home_dir().join("Library").join("LaunchAgents")));
        assert_eq!(path.file_name().unwrap(), "com.example.agentd.plist");
    }

    #[test]
    fn uninstall_is_a_no_op_when_nothing_is_installed() {
        // A label that (almost certainly) has never been installed.
        uninstall("com.agentmon.agentd.never-installed-test").unwrap();
    }
}
