use std::path::PathBuf;
use std::sync::Arc;

use agentmon_proto::{AgentEvent, AgentInfo, AgentStatus, TestRunInfo, TestRunStatus};

use crate::notify::Notifier;
use crate::registry::Registry;

/// Applies incoming agent events to the registry and dispatches
/// notifications on transitions that need the user's attention.
#[derive(Clone)]
pub struct Ingestor {
    registry: Registry,
    notifier: Arc<dyn Notifier>,
}

impl Ingestor {
    pub fn new(registry: Registry, notifier: Arc<dyn Notifier>) -> Self {
        Self { registry, notifier }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Updates the registry from a hook-reported event, notifying the user
    /// if the resulting status is a new transition into "done" or "needs
    /// input".
    pub fn ingest_event(&self, event: AgentEvent) -> AgentInfo {
        let outcome = self.registry.upsert(event);
        if should_notify(outcome.previous_status, outcome.agent.status) {
            self.notifier.notify(&outcome.agent);
        }
        outcome.agent
    }

    /// Updates the registry from a reported test-run event, notifying the
    /// user for every status - "started", "passed", or "failed" - independent
    /// of whether its working directory has a tracked agent, so each event
    /// over the life of a long agent session is heard on its own.
    pub fn ingest_test_run(&self, cwd: PathBuf, pid: u32, status: TestRunStatus) -> TestRunInfo {
        let test_run = self.registry.upsert_test_run(cwd, pid, status);
        self.notifier.notify_test_run(&test_run.cwd, status);
        test_run
    }
}

/// Every "needs input" event notifies, since each one represents a distinct
/// blocking prompt and Claude Code reports no event when a prompt is
/// resolved (so two prompts in the same turn look identical to the
/// registry). A "done" event only notifies when it actually changes the
/// status - repeated "done" events for an already-done session must not
/// re-notify.
fn should_notify(previous: Option<AgentStatus>, current: AgentStatus) -> bool {
    match current {
        AgentStatus::NeedsInput => true,
        AgentStatus::Done => previous != Some(current),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::{HostContext, SessionId};
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingNotifier {
        calls: Mutex<Vec<AgentInfo>>,
        test_run_calls: Mutex<Vec<(PathBuf, TestRunStatus)>>,
    }

    impl Notifier for RecordingNotifier {
        fn notify(&self, agent: &AgentInfo) {
            self.calls.lock().unwrap().push(agent.clone());
        }

        fn notify_test_run(&self, cwd: &std::path::Path, status: TestRunStatus) {
            self.test_run_calls
                .lock()
                .unwrap()
                .push((cwd.to_path_buf(), status));
        }
    }

    fn event(status: AgentStatus) -> AgentEvent {
        AgentEvent {
            session_id: SessionId("session-1".to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 1,
            status,
        }
    }

    #[test]
    fn ingest_event_updates_the_registry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        let agent = ingestor.ingest_event(event(AgentStatus::Running));

        assert_eq!(agent.status, AgentStatus::Running);
        assert_eq!(ingestor.registry().snapshot().len(), 1);
    }

    #[test]
    fn transition_to_done_sends_one_notification() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Done));

        assert_eq!(notifier.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn transition_to_needs_input_sends_one_notification() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::NeedsInput));

        assert_eq!(notifier.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn repeated_done_status_does_not_renotify() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_event(event(AgentStatus::Done));

        assert_eq!(
            notifier.calls.lock().unwrap().len(),
            1,
            "repeated same-status done events must not trigger duplicate notifications"
        );
    }

    #[test]
    fn repeated_needs_input_status_notifies_each_time() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::NeedsInput));
        ingestor.ingest_event(event(AgentStatus::NeedsInput));
        ingestor.ingest_event(event(AgentStatus::NeedsInput));

        assert_eq!(
            notifier.calls.lock().unwrap().len(),
            3,
            "each needs-input event is a distinct blocking prompt and must notify"
        );
    }

    #[test]
    fn switching_from_needs_input_to_done_notifies_again() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::NeedsInput));
        ingestor.ingest_event(event(AgentStatus::Done));

        assert_eq!(notifier.calls.lock().unwrap().len(), 2);
    }

    #[test]
    fn needs_input_after_done_is_guarded_and_does_not_notify() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::Done));
        let agent = ingestor.ingest_event(event(AgentStatus::NeedsInput));

        assert_eq!(
            agent.status,
            AgentStatus::Done,
            "a needs-input event for an already-done session must be dropped by the registry"
        );
        assert_eq!(
            notifier.calls.lock().unwrap().len(),
            1,
            "the dropped needs-input event must not trigger a second notification"
        );
    }

    #[test]
    fn repeated_running_status_does_not_renotify_or_change_status() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        // Models repeated PreToolUse/PostToolUse events for a session that is
        // already "running" - each tool call reports "running" again (once
        // before it runs, once after), and this must stay a silent no-op
        // rather than notifying on every tool use.
        ingestor.ingest_event(event(AgentStatus::Running));
        let agent = ingestor.ingest_event(event(AgentStatus::Running));

        assert_eq!(agent.status, AgentStatus::Running);
        assert_eq!(
            notifier.calls.lock().unwrap().len(),
            0,
            "repeated running events must not trigger a notification"
        );
    }

    #[test]
    fn transitions_that_are_not_done_or_needs_input_never_notify() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Idle));
        ingestor.ingest_event(event(AgentStatus::Running));

        assert_eq!(notifier.calls.lock().unwrap().len(), 0);
    }

    #[test]
    fn ingest_test_run_updates_the_registry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        let test_run =
            ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Started);

        assert_eq!(test_run.status, TestRunStatus::Started);
        assert_eq!(ingestor.registry().snapshot_test_runs().len(), 1);
    }

    #[test]
    fn a_failing_run_with_no_tracked_agent_notifies() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Failed);

        assert_eq!(notifier.test_run_calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_failing_run_with_a_tracked_agent_still_notifies() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());
        ingestor.ingest_event(event(AgentStatus::Running));

        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Failed);

        assert_eq!(
            notifier.test_run_calls.lock().unwrap().as_slice(),
            [(PathBuf::from("/tmp/project"), TestRunStatus::Failed)],
            "a tracked agent in the directory must not suppress the test-run notification"
        );
    }

    #[test]
    fn a_passing_run_notifies_regardless_of_a_tracked_agent() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());
        ingestor.ingest_event(event(AgentStatus::Running));

        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Passed);

        assert_eq!(
            notifier.test_run_calls.lock().unwrap().as_slice(),
            [(PathBuf::from("/tmp/project"), TestRunStatus::Passed)]
        );
    }

    #[test]
    fn a_started_run_notifies() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Started);

        assert_eq!(
            notifier.test_run_calls.lock().unwrap().as_slice(),
            [(PathBuf::from("/tmp/project"), TestRunStatus::Started)]
        );
    }

    #[test]
    fn repeated_lifecycle_events_in_the_same_directory_each_notify() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        // Two distinct runs (each its own pid) starting and failing in the
        // same directory over the life of one long agent session - each
        // event must notify on its own, not just the first.
        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 1, TestRunStatus::Started);
        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 1, TestRunStatus::Failed);
        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 2, TestRunStatus::Started);
        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 2, TestRunStatus::Failed);

        let calls = notifier.test_run_calls.lock().unwrap();
        assert_eq!(
            calls.as_slice(),
            [
                (PathBuf::from("/tmp/project"), TestRunStatus::Started),
                (PathBuf::from("/tmp/project"), TestRunStatus::Failed),
                (PathBuf::from("/tmp/project"), TestRunStatus::Started),
                (PathBuf::from("/tmp/project"), TestRunStatus::Failed),
            ],
            "each lifecycle event must notify independently, got: {calls:?}"
        );
    }
}
