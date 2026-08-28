use std::sync::Arc;

use agentmon_proto::{AgentEvent, AgentInfo, AgentStatus};

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
}

/// A transition warrants a notification only when it lands on "done" or
/// "needs input" and actually changes the status - repeated events
/// reporting the same status must not re-notify.
fn should_notify(previous: Option<AgentStatus>, current: AgentStatus) -> bool {
    matches!(current, AgentStatus::Done | AgentStatus::NeedsInput) && previous != Some(current)
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
    }

    impl Notifier for RecordingNotifier {
        fn notify(&self, agent: &AgentInfo) {
            self.calls.lock().unwrap().push(agent.clone());
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
    fn repeated_identical_status_does_not_renotify() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_event(event(AgentStatus::Done));

        assert_eq!(
            notifier.calls.lock().unwrap().len(),
            1,
            "repeated same-status events must not trigger duplicate notifications"
        );
    }

    #[test]
    fn switching_between_attention_states_notifies_again() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_event(event(AgentStatus::NeedsInput));

        assert_eq!(notifier.calls.lock().unwrap().len(), 2);
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
}
