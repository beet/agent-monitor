use std::process::Command;
use std::thread;
use std::time::Duration;

use agentmon_proto::{AgentInfo, AgentStatus};

use crate::registry::Registry;

/// Checks whether a process with the given pid currently exists.
///
/// macOS has no `/proc`, so this shells out to `ps` rather than relying on a
/// raw `kill(pid, 0)` FFI call.
pub fn process_is_alive(pid: u32) -> bool {
    Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Marks any tracked, non-stale agent whose process has exited as stale.
/// Returns the agents that were newly marked, for the caller to broadcast.
pub fn sweep_once(registry: &Registry) -> Vec<AgentInfo> {
    registry
        .snapshot()
        .into_iter()
        .filter(|agent| agent.status != AgentStatus::Stale)
        .filter(|agent| !process_is_alive(agent.pid))
        .filter_map(|agent| registry.mark_stale(&agent.session_id))
        .collect()
}

/// Spawns a background thread that runs `sweep_once` on a fixed interval,
/// invoking `on_stale` for each agent newly marked stale.
pub fn spawn_liveness_sweep(
    registry: Registry,
    interval: Duration,
    on_stale: impl Fn(AgentInfo) + Send + 'static,
) -> thread::JoinHandle<()> {
    thread::spawn(move || loop {
        thread::sleep(interval);
        for agent in sweep_once(&registry) {
            on_stale(agent);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{AgentEvent, HostContext, SessionId};
    use std::path::PathBuf;

    fn register(registry: &Registry, session_id: &str, pid: u32) {
        registry.upsert(AgentEvent {
            session_id: SessionId(session_id.to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid,
            status: AgentStatus::Running,
        });
    }

    #[test]
    fn process_is_alive_reflects_this_process_and_a_bogus_pid() {
        assert!(process_is_alive(std::process::id()));
        assert!(!process_is_alive(999_999));
    }

    #[test]
    fn sweep_marks_agent_stale_after_its_process_exits() {
        let mut child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn a short-lived child process");
        let pid = child.id();
        assert!(process_is_alive(pid), "child should be alive right after spawn");

        let registry = Registry::new();
        register(&registry, "session-1", pid);
        assert!(sweep_once(&registry).is_empty(), "live process must not be marked stale");

        child.kill().expect("kill child");
        child.wait().expect("reap child");

        let mut newly_stale = Vec::new();
        for _ in 0..40 {
            newly_stale = sweep_once(&registry);
            if !newly_stale.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }

        assert_eq!(newly_stale.len(), 1);
        assert_eq!(newly_stale[0].status, AgentStatus::Stale);
    }

    #[test]
    fn sweep_does_not_repeat_already_stale_agents() {
        let registry = Registry::new();
        register(&registry, "session-1", 999_999);

        let first_sweep = sweep_once(&registry);
        assert_eq!(first_sweep.len(), 1);

        let second_sweep = sweep_once(&registry);
        assert!(
            second_sweep.is_empty(),
            "an already-stale agent should not be reported again"
        );
    }
}
