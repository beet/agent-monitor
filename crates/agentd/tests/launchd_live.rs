//! Exercises the real install/uninstall flow against this machine's actual
//! `launchctl`. Ignored by default since it touches persistent system state
//! (a LaunchAgents plist, briefly a running background process); run
//! explicitly with:
//!
//!   cargo test -p agentd --test launchd_live -- --ignored --nocapture
//!
//! Uses a throwaway label distinct from `agentd::launchd::LABEL` so a failed
//! cleanup here can never leave anything behind under the real service name.

use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::Command;

const TEST_LABEL: &str = "com.agentmon.agentd.verify-test";

fn launchctl_list_contains(label: &str) -> bool {
    let output = Command::new("launchctl")
        .arg("list")
        .output()
        .expect("run launchctl list");
    String::from_utf8_lossy(&output.stdout).contains(label)
}

#[test]
#[ignore]
fn install_then_uninstall_round_trips_through_real_launchctl() {
    // Always attempt a clean slate first, in case a prior run of this test
    // was interrupted before it could uninstall.
    let _ = agentd::launchd::uninstall(TEST_LABEL);
    assert!(
        !launchctl_list_contains(TEST_LABEL),
        "precondition: {TEST_LABEL} must not already be loaded"
    );

    let binary_path = PathBuf::from(env!("CARGO_BIN_EXE_agentd"));
    let plist_path =
        agentd::launchd::install(TEST_LABEL, &binary_path).expect("install should succeed");
    assert!(plist_path.exists(), "plist file should be written");

    assert!(
        launchctl_list_contains(TEST_LABEL),
        "launchctl list should show {TEST_LABEL} after install"
    );

    agentd::launchd::uninstall(TEST_LABEL).expect("uninstall should succeed");
    assert!(!plist_path.exists(), "plist file should be removed");
    assert!(
        !launchctl_list_contains(TEST_LABEL),
        "launchctl list should not show {TEST_LABEL} after uninstall"
    );
}

/// Confirms the plist ends up referencing the symlink path itself, not its
/// canonicalized target - the property `resolve_binary_path` in `main.rs`
/// relies on to survive a Homebrew upgrade with just a restart. Uses a
/// throwaway label and a `/tmp` symlink so it never touches the real
/// `com.agentmon.agentd` service.
#[test]
#[ignore]
fn install_writes_the_symlink_path_not_its_canonicalized_target() {
    let _ = agentd::launchd::uninstall(TEST_LABEL);
    assert!(
        !launchctl_list_contains(TEST_LABEL),
        "precondition: {TEST_LABEL} must not already be loaded"
    );

    let real_binary = PathBuf::from(env!("CARGO_BIN_EXE_agentd"));
    let symlink_path = std::env::temp_dir().join("agentd-verify-symlink");
    let _ = std::fs::remove_file(&symlink_path);
    symlink(&real_binary, &symlink_path).expect("create symlink to the built agentd binary");
    assert_ne!(
        symlink_path.canonicalize().unwrap(),
        symlink_path,
        "precondition: symlink target must differ from the symlink path itself"
    );

    let plist_path =
        agentd::launchd::install(TEST_LABEL, &symlink_path).expect("install should succeed");
    let plist_contents = std::fs::read_to_string(&plist_path).expect("read installed plist");

    assert!(
        plist_contents.contains(&symlink_path.to_string_lossy().to_string()),
        "plist should reference the symlink path {symlink_path:?}, got:\n{plist_contents}"
    );
    assert!(
        !plist_contents.contains(&real_binary.to_string_lossy().to_string()),
        "plist should not reference the canonicalized target {real_binary:?}, got:\n{plist_contents}"
    );

    agentd::launchd::uninstall(TEST_LABEL).expect("uninstall should succeed");
    let _ = std::fs::remove_file(&symlink_path);
}
