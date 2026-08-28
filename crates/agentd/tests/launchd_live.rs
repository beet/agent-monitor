//! Exercises the real install/uninstall flow against this machine's actual
//! `launchctl`. Ignored by default since it touches persistent system state
//! (a LaunchAgents plist, briefly a running background process); run
//! explicitly with:
//!
//!   cargo test -p agentd --test launchd_live -- --ignored --nocapture
//!
//! Uses a throwaway label distinct from `agentd::launchd::LABEL` so a failed
//! cleanup here can never leave anything behind under the real service name.

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
