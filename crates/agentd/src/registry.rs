use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use agentmon_proto::{AgentEvent, AgentInfo, AgentStatus, SessionId};

#[derive(Default)]
struct RegistryState {
    agents: HashMap<SessionId, AgentInfo>,
}

/// In-memory registry of tracked agents, keyed by session id.
///
/// Cheap to clone: internally shares state via `Arc`, so every clone reads
/// and writes the same underlying registry.
#[derive(Clone, Default)]
pub struct Registry {
    state: Arc<Mutex<RegistryState>>,
}

pub struct UpsertOutcome {
    pub agent: AgentInfo,
    pub is_new: bool,
    pub previous_status: Option<AgentStatus>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a newly seen session, or updates an existing one in place.
    ///
    /// A "needs input" event is dropped when the session is already "done":
    /// a finished session can't legitimately need input again until a new
    /// "running" event starts its next turn, so this guards against a
    /// needs-input hook event (e.g. Claude Code's idle-prompt notification)
    /// arriving after that session's Stop event and flipping it back.
    pub fn upsert(&self, event: AgentEvent) -> UpsertOutcome {
        let mut state = self.state.lock().unwrap();
        let previous_status = state.agents.get(&event.session_id).map(|a| a.status);

        if previous_status == Some(AgentStatus::Done) && event.status == AgentStatus::NeedsInput {
            let agent = state.agents.get(&event.session_id).unwrap().clone();
            return UpsertOutcome {
                agent,
                is_new: false,
                previous_status,
            };
        }

        let agent = AgentInfo {
            session_id: event.session_id.clone(),
            cwd: event.cwd,
            host_context: event.host_context,
            pid: event.pid,
            status: event.status,
            last_updated_ms: now_ms(),
        };
        state.agents.insert(agent.session_id.clone(), agent.clone());

        UpsertOutcome {
            agent,
            is_new: previous_status.is_none(),
            previous_status,
        }
    }

    /// Marks a tracked agent as stale. Returns `None` if the session is
    /// unknown or already marked stale.
    pub fn mark_stale(&self, session_id: &SessionId) -> Option<AgentInfo> {
        let mut state = self.state.lock().unwrap();
        let agent = state.agents.get_mut(session_id)?;
        if agent.status == AgentStatus::Stale {
            return None;
        }
        agent.status = AgentStatus::Stale;
        agent.last_updated_ms = now_ms();
        Some(agent.clone())
    }

    pub fn snapshot(&self) -> Vec<AgentInfo> {
        let state = self.state.lock().unwrap();
        state.agents.values().cloned().collect()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::HostContext;
    use std::path::PathBuf;

    fn sample_event(status: AgentStatus) -> AgentEvent {
        AgentEvent {
            session_id: SessionId("session-1".to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 123,
            status,
        }
    }

    #[test]
    fn first_event_registers_a_new_agent() {
        let registry = Registry::new();

        let outcome = registry.upsert(sample_event(AgentStatus::Running));

        assert!(outcome.is_new);
        assert_eq!(outcome.previous_status, None);
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].status, AgentStatus::Running);
    }

    #[test]
    fn subsequent_event_updates_the_existing_agent_in_place() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));

        let outcome = registry.upsert(sample_event(AgentStatus::Done));

        assert!(!outcome.is_new);
        assert_eq!(outcome.previous_status, Some(AgentStatus::Running));
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1, "must update in place, not duplicate");
        assert_eq!(snapshot[0].status, AgentStatus::Done);
    }

    #[test]
    fn needs_input_event_is_dropped_when_session_already_done() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::Done));
        let done_snapshot = registry.snapshot();
        let done_last_updated_ms = done_snapshot[0].last_updated_ms;

        let outcome = registry.upsert(sample_event(AgentStatus::NeedsInput));

        assert_eq!(outcome.agent.status, AgentStatus::Done);
        assert_eq!(outcome.previous_status, Some(AgentStatus::Done));
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].status, AgentStatus::Done);
        assert_eq!(
            snapshot[0].last_updated_ms, done_last_updated_ms,
            "a dropped needs-input event must not update last_updated_ms"
        );
    }

    #[test]
    fn running_event_still_clears_a_done_session() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::Done));

        let outcome = registry.upsert(sample_event(AgentStatus::Running));

        assert_eq!(outcome.agent.status, AgentStatus::Running);
        assert_eq!(outcome.previous_status, Some(AgentStatus::Done));
        assert_eq!(registry.snapshot()[0].status, AgentStatus::Running);
    }

    #[test]
    fn mark_stale_transitions_a_known_agent() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        let session_id = SessionId("session-1".to_string());

        let updated = registry.mark_stale(&session_id);

        assert_eq!(updated.map(|a| a.status), Some(AgentStatus::Stale));
        assert_eq!(registry.snapshot()[0].status, AgentStatus::Stale);
    }

    #[test]
    fn mark_stale_is_a_no_op_for_unknown_session() {
        let registry = Registry::new();

        let result = registry.mark_stale(&SessionId("unknown".to_string()));

        assert!(result.is_none());
    }
}
