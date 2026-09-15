use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use agentmon_proto::{AgentEvent, AgentInfo, AgentStatus, SessionId, TestRunInfo, TestRunStatus};

/// Cap on how many retired session ids `RegistryState::superseded_sessions`
/// remembers, evicting the oldest once exceeded - the same bounded-eviction
/// shape as `ActivityLog`'s cap, so memory stays flat over a long-running
/// daemon instead of growing with total historical `/clear` churn. Session
/// ids are Claude Code-generated and effectively unique, so this is a global
/// set rather than one bounded per pid: a pid can be superseded more than
/// once (session A -> B -> C), and a late event for the original session A
/// must still be recognized as stale even after B - the entry that directly
/// retired it - has itself since been retired by C.
const MAX_SUPERSEDED_SESSIONS: usize = 256;

#[derive(Default)]
struct RegistryState {
    agents: HashMap<SessionId, AgentInfo>,
    /// Keyed by working directory alone, not by pid or any agent identity: a
    /// test run may be launched outside any tracked agent's process tree
    /// (per the rspec-test-reporting spec), and a directory holds at most
    /// one tracked test run at a time - a later report for the same
    /// directory always replaces the previous one, regardless of pid, so
    /// repeated invocations (e.g. an edit/test loop) don't accumulate one
    /// entry per invocation.
    test_runs: HashMap<PathBuf, TestRunInfo>,
    /// Every session id retired by `upsert`'s same-pid dedup, most recent
    /// last, so a late/out-of-order event naming one of them can be
    /// recognized and dropped instead of being resurrected as a "new" entry
    /// that evicts whichever session currently holds that pid. See
    /// `MAX_SUPERSEDED_SESSIONS`.
    superseded_sessions: VecDeque<SessionId>,
    /// Mirrors `superseded_sessions` for O(1) membership checks; kept in
    /// sync with it on every insert and eviction.
    superseded_session_set: HashSet<SessionId>,
}

impl RegistryState {
    fn record_superseded(&mut self, session_id: SessionId) {
        if self.superseded_session_set.insert(session_id.clone()) {
            self.superseded_sessions.push_back(session_id);
            if self.superseded_sessions.len() > MAX_SUPERSEDED_SESSIONS {
                if let Some(oldest) = self.superseded_sessions.pop_front() {
                    self.superseded_session_set.remove(&oldest);
                }
            }
        }
    }
}

/// Every tracked agent and test run sharing one exact working directory, per
/// the "agents and test runs are grouped by working directory" requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryGroup {
    pub cwd: PathBuf,
    pub agents: Vec<AgentInfo>,
    pub test_runs: Vec<TestRunInfo>,
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
    /// Set when the incoming event was recognized as stale and had no effect
    /// on the registry - currently only a late event naming an
    /// already-superseded session id (see `upsert`'s doc comment). Callers
    /// (`Ingestor::ingest_event`) must treat this as a pure no-op: `agent`
    /// and `previous_status` both describe the pid's unaffected live entry,
    /// not the dropped event, so they must not be fed to notification/log
    /// logic that inspects a status transition - doing so could spuriously
    /// re-notify (e.g. if the live entry's status happens to be "needs
    /// input", which always notifies regardless of whether it changed).
    pub stale_event_ignored: bool,
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
    ///
    /// A pid never has more than one live entry: when this event's session
    /// id is new but its pid matches an existing entry under a different
    /// session id, that existing entry is dropped first. This happens when
    /// the same Claude Code process starts a new session id (e.g. `/clear`)
    /// - the old session is gone, not merely quiet, so it must not linger
    /// as an untouched duplicate row sharing the live pid.
    ///
    /// Retiring an entry this way permanently supersedes its session id:
    /// hook events arrive over independent, one-shot connections with no
    /// ordering guarantee between them, so a late event for the
    /// just-retired session id can still arrive after its replacement has
    /// taken over the pid. Without a record of the supersession, that late
    /// event would look like a brand-new session sharing the live pid and
    /// would itself evict the replacement - producing a flickering
    /// duplicate row that only resolves once events stop naming the dead
    /// session. Such a late event is instead recognized as stale and
    /// dropped as a no-op, returning whichever entry currently holds the
    /// pid unchanged.
    pub fn upsert(&self, event: AgentEvent) -> UpsertOutcome {
        let mut state = self.state.lock().unwrap();
        let previous = state.agents.get(&event.session_id).cloned();
        let previous_status = previous.as_ref().map(|a| a.status);

        if previous_status == Some(AgentStatus::Done) && event.status == AgentStatus::NeedsInput {
            let agent = previous.unwrap();
            return UpsertOutcome {
                agent,
                is_new: false,
                previous_status,
                stale_event_ignored: false,
            };
        }

        if previous_status.is_none() {
            if state.superseded_session_set.contains(&event.session_id) {
                if let Some(current) = state.agents.values().find(|a| a.pid == event.pid).cloned() {
                    return UpsertOutcome {
                        previous_status: Some(current.status),
                        is_new: false,
                        agent: current,
                        stale_event_ignored: true,
                    };
                }
                // The superseded session id's pid isn't tracked under any
                // live entry (should not happen given the invariants above,
                // but falling through registers the event rather than
                // silently discarding it with nothing to return).
            }

            let retiring: Vec<SessionId> = state
                .agents
                .iter()
                .filter(|(session_id, agent)| agent.pid == event.pid && **session_id != event.session_id)
                .map(|(session_id, _)| session_id.clone())
                .collect();
            for session_id in retiring {
                state.agents.remove(&session_id);
                state.record_superseded(session_id);
            }
        }

        let now = now_ms();
        let status_since_ms = match &previous {
            Some(agent) if agent.status == event.status => agent.status_since_ms,
            _ => now,
        };
        // Resets on every transition into "running" - including from "done" -
        // rather than only at entry creation or pid replacement, since one
        // agent's pid persists across many started -> running -> done turns
        // over the life of a session (unlike a test run's pid-scoped
        // `run_started_ms`, where a new pid *is* a new run).
        let run_started_ms = match &previous {
            Some(agent) if agent.status != AgentStatus::Running && event.status == AgentStatus::Running => now,
            Some(agent) => agent.run_started_ms,
            None => now,
        };

        let agent = AgentInfo {
            session_id: event.session_id.clone(),
            cwd: event.cwd,
            host_context: event.host_context,
            pid: event.pid,
            status: event.status,
            last_updated_ms: now,
            status_since_ms,
            run_started_ms,
        };
        state.agents.insert(agent.session_id.clone(), agent.clone());

        UpsertOutcome {
            agent,
            is_new: previous_status.is_none(),
            previous_status,
            stale_event_ignored: false,
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
        let now = now_ms();
        agent.status = AgentStatus::Stale;
        agent.last_updated_ms = now;
        agent.status_since_ms = now;
        Some(agent.clone())
    }

    pub fn snapshot(&self) -> Vec<AgentInfo> {
        let state = self.state.lock().unwrap();
        state.agents.values().cloned().collect()
    }

    /// Records a test-run event, keyed by `cwd` alone. A later event for the
    /// same directory always replaces the previous one - even if it was
    /// reported by a different pid - rather than accumulating one entry per
    /// invocation, mirroring how an agent's row persists until its next
    /// status transition.
    ///
    /// `run_started_ms` marks when the currently tracked process (`pid`)
    /// began: it carries over from the previous entry when this event's pid
    /// matches (the same process moving from "started" to "passed"/"failed"),
    /// and resets to now when the pid differs (a new run), mirroring how
    /// `status_since_ms` tracks an agent's current status.
    pub fn upsert_test_run(&self, cwd: PathBuf, pid: u32, status: TestRunStatus) -> TestRunInfo {
        let mut state = self.state.lock().unwrap();
        let now = now_ms();
        let run_started_ms = match state.test_runs.get(&cwd) {
            Some(existing) if existing.pid == pid => existing.run_started_ms,
            _ => now,
        };
        let test_run = TestRunInfo {
            cwd: cwd.clone(),
            pid,
            status,
            last_updated_ms: now,
            run_started_ms,
        };
        state.test_runs.insert(cwd.clone(), test_run.clone());
        test_run
    }

    pub fn snapshot_test_runs(&self) -> Vec<TestRunInfo> {
        let state = self.state.lock().unwrap();
        state.test_runs.values().cloned().collect()
    }

    /// Groups every tracked agent and test run by exact working-directory
    /// match. A directory with neither never appears - there is nothing to
    /// group. This is a read-side view only: it does not change how agents
    /// or test runs are stored or keyed internally.
    pub fn directory_groups(&self) -> Vec<DirectoryGroup> {
        let state = self.state.lock().unwrap();
        let mut groups: HashMap<PathBuf, DirectoryGroup> = HashMap::new();

        for agent in state.agents.values() {
            groups
                .entry(agent.cwd.clone())
                .or_insert_with(|| DirectoryGroup {
                    cwd: agent.cwd.clone(),
                    agents: Vec::new(),
                    test_runs: Vec::new(),
                })
                .agents
                .push(agent.clone());
        }

        for test_run in state.test_runs.values() {
            groups
                .entry(test_run.cwd.clone())
                .or_insert_with(|| DirectoryGroup {
                    cwd: test_run.cwd.clone(),
                    agents: Vec::new(),
                    test_runs: Vec::new(),
                })
                .test_runs
                .push(test_run.clone());
        }

        groups.into_values().collect()
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
    use std::thread;
    use std::time::Duration;

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
    fn same_status_event_leaves_status_since_ms_unchanged() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        let status_since_ms = registry.snapshot()[0].status_since_ms;
        thread::sleep(Duration::from_millis(10));

        // A `PostToolUse` event during the same running turn reports the
        // same status - it must not look like a fresh transition.
        registry.upsert(sample_event(AgentStatus::Running));

        let snapshot = registry.snapshot();
        assert_eq!(
            snapshot[0].status_since_ms, status_since_ms,
            "a same-status event must not reset status_since_ms"
        );
        assert!(
            snapshot[0].last_updated_ms > status_since_ms,
            "last_updated_ms must still advance even when status is unchanged"
        );
    }

    #[test]
    fn a_new_agent_sets_run_started_ms_to_its_creation_time() {
        let registry = Registry::new();

        let outcome = registry.upsert(sample_event(AgentStatus::Running));

        assert_eq!(outcome.agent.run_started_ms, outcome.agent.last_updated_ms);
        assert_eq!(registry.snapshot()[0].run_started_ms, outcome.agent.run_started_ms);
    }

    #[test]
    fn same_status_event_leaves_run_started_ms_unchanged() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        let run_started_ms = registry.snapshot()[0].run_started_ms;
        thread::sleep(Duration::from_millis(10));

        registry.upsert(sample_event(AgentStatus::Running));

        assert_eq!(
            registry.snapshot()[0].run_started_ms, run_started_ms,
            "a same-status event must not reset run_started_ms"
        );
    }

    #[test]
    fn transitioning_away_from_running_leaves_run_started_ms_unchanged() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        let run_started_ms = registry.snapshot()[0].run_started_ms;
        thread::sleep(Duration::from_millis(10));

        registry.upsert(sample_event(AgentStatus::NeedsInput));

        assert_eq!(
            registry.snapshot()[0].run_started_ms, run_started_ms,
            "leaving running must not change run_started_ms - it still marks when the run that just ended began"
        );
    }

    #[test]
    fn transitioning_between_two_non_running_statuses_leaves_run_started_ms_unchanged() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::NeedsInput));
        let run_started_ms = registry.snapshot()[0].run_started_ms;
        thread::sleep(Duration::from_millis(10));

        registry.upsert(sample_event(AgentStatus::Declined));

        assert_eq!(
            registry.snapshot()[0].run_started_ms, run_started_ms,
            "moving between two non-running statuses must not change run_started_ms"
        );
    }

    #[test]
    fn a_transition_into_running_resets_run_started_ms() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        let first_run_started_ms = registry.snapshot()[0].run_started_ms;
        thread::sleep(Duration::from_millis(10));
        registry.upsert(sample_event(AgentStatus::NeedsInput));

        registry.upsert(sample_event(AgentStatus::Running));

        assert!(
            registry.snapshot()[0].run_started_ms > first_run_started_ms,
            "resuming into running from a different status must reset run_started_ms"
        );
    }

    #[test]
    fn a_done_to_running_transition_resets_run_started_ms_for_the_new_turn() {
        // An agent's pid persists across many started -> running -> done
        // turns, unlike a test run's pid-scoped run_started_ms, so this must
        // reset on every re-entry into running - including from "done" -
        // rather than staying pinned to the session's very first turn.
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::Done));
        let first_turn_run_started_ms = registry.snapshot()[0].run_started_ms;
        thread::sleep(Duration::from_millis(10));

        let outcome = registry.upsert(sample_event(AgentStatus::Running));

        assert!(
            outcome.agent.run_started_ms > first_turn_run_started_ms,
            "done -> running must begin a new turn's run_started_ms, not keep the first turn's"
        );
        assert_eq!(registry.snapshot()[0].run_started_ms, outcome.agent.run_started_ms);
    }

    #[test]
    fn a_new_session_id_for_a_tracked_pid_resets_run_started_ms() {
        let registry = Registry::new();
        registry.upsert(AgentEvent {
            session_id: SessionId("session-1".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });
        let first_run_started_ms = registry.snapshot()[0].run_started_ms;
        thread::sleep(Duration::from_millis(10));

        registry.upsert(AgentEvent {
            session_id: SessionId("session-2".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });

        assert!(
            registry.snapshot()[0].run_started_ms > first_run_started_ms,
            "a pid-replacement entry must get its own creation-time run_started_ms, not inherit the replaced entry's"
        );
    }

    #[test]
    fn status_transition_resets_status_since_ms() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        let running_status_since_ms = registry.snapshot()[0].status_since_ms;
        thread::sleep(Duration::from_millis(10));

        registry.upsert(sample_event(AgentStatus::Done));

        let snapshot = registry.snapshot();
        assert!(
            snapshot[0].status_since_ms > running_status_since_ms,
            "a status transition must reset status_since_ms to the current time"
        );
    }

    #[test]
    fn needs_input_event_is_dropped_when_session_already_done() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::Done));
        let done_snapshot = registry.snapshot();
        let done_last_updated_ms = done_snapshot[0].last_updated_ms;
        let done_status_since_ms = done_snapshot[0].status_since_ms;
        thread::sleep(Duration::from_millis(10));

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
        assert_eq!(
            snapshot[0].status_since_ms, done_status_since_ms,
            "a dropped needs-input event must not update status_since_ms"
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
    fn running_event_clears_a_needs_input_session() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::NeedsInput));

        let outcome = registry.upsert(sample_event(AgentStatus::Running));

        assert_eq!(outcome.agent.status, AgentStatus::Running);
        assert_eq!(outcome.previous_status, Some(AgentStatus::NeedsInput));
        assert_eq!(registry.snapshot()[0].status, AgentStatus::Running);
    }

    #[test]
    fn running_event_clears_a_declined_session() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::Declined));

        let outcome = registry.upsert(sample_event(AgentStatus::Running));

        assert_eq!(outcome.agent.status, AgentStatus::Running);
        assert_eq!(outcome.previous_status, Some(AgentStatus::Declined));
        assert_eq!(registry.snapshot()[0].status, AgentStatus::Running);
    }

    #[test]
    fn done_event_clears_a_declined_session() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::Declined));

        let outcome = registry.upsert(sample_event(AgentStatus::Done));

        assert_eq!(outcome.agent.status, AgentStatus::Done);
        assert_eq!(outcome.previous_status, Some(AgentStatus::Declined));
        assert_eq!(registry.snapshot()[0].status, AgentStatus::Done);
    }

    #[test]
    fn running_event_clears_a_needs_input_session_via_post_tool_use() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert(sample_event(AgentStatus::NeedsInput));

        // Models a `PostToolUse` event, which reports the same `Running`
        // status as `PreToolUse` - the registry has no notion of which hook
        // produced the event, only the status it maps to.
        let outcome = registry.upsert(sample_event(AgentStatus::Running));

        assert_eq!(outcome.agent.status, AgentStatus::Running);
        assert_eq!(outcome.previous_status, Some(AgentStatus::NeedsInput));
        assert_eq!(registry.snapshot()[0].status, AgentStatus::Running);
    }

    #[test]
    fn mark_stale_transitions_a_known_agent() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        let running_status_since_ms = registry.snapshot()[0].status_since_ms;
        let session_id = SessionId("session-1".to_string());
        thread::sleep(Duration::from_millis(10));

        let updated = registry.mark_stale(&session_id);

        assert_eq!(updated.map(|a| a.status), Some(AgentStatus::Stale));
        assert_eq!(registry.snapshot()[0].status, AgentStatus::Stale);
        assert!(
            registry.snapshot()[0].status_since_ms > running_status_since_ms,
            "marking an agent stale must reset its status_since_ms"
        );
    }

    #[test]
    fn mark_stale_is_a_no_op_for_unknown_session() {
        let registry = Registry::new();

        let result = registry.mark_stale(&SessionId("unknown".to_string()));

        assert!(result.is_none());
    }

    #[test]
    fn a_new_session_id_for_a_tracked_pid_replaces_the_old_entry() {
        let registry = Registry::new();
        registry.upsert(AgentEvent {
            session_id: SessionId("session-1".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });

        registry.upsert(AgentEvent {
            session_id: SessionId("session-2".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });

        let snapshot = registry.snapshot();
        assert_eq!(
            snapshot.len(),
            1,
            "the old session-1 entry must be replaced, not left as a duplicate"
        );
        assert_eq!(snapshot[0].session_id, SessionId("session-2".to_string()));
    }

    #[test]
    fn a_late_event_for_an_already_superseded_session_id_is_ignored() {
        let registry = Registry::new();
        registry.upsert(AgentEvent {
            session_id: SessionId("session-1".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });
        registry.upsert(AgentEvent {
            session_id: SessionId("session-2".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });

        // A hook event for session-1, delayed enough that it arrives after
        // session-2 (same pid) already took over - simulating the race
        // between two independently-connecting, one-shot hook reports.
        let outcome = registry.upsert(AgentEvent {
            session_id: SessionId("session-1".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Done)
        });

        assert_eq!(
            outcome.agent.session_id,
            SessionId("session-2".to_string()),
            "the late event must not resurrect session-1 as a new entry"
        );
        assert_eq!(
            outcome.agent.status,
            AgentStatus::Running,
            "session-2's status must be unaffected by the late session-1 event"
        );
        assert!(!outcome.is_new);

        let snapshot = registry.snapshot();
        assert_eq!(
            snapshot.len(),
            1,
            "the late event must not leave session-1 lingering as a duplicate entry"
        );
        assert_eq!(snapshot[0].session_id, SessionId("session-2".to_string()));
        assert_eq!(
            snapshot[0].status,
            AgentStatus::Running,
            "the live session-2 entry must be completely unaffected by the late event"
        );
    }

    #[test]
    fn a_late_event_for_a_session_superseded_twice_over_is_still_ignored() {
        let registry = Registry::new();
        registry.upsert(AgentEvent {
            session_id: SessionId("session-a".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });
        // session-b supersedes session-a on pid 123.
        registry.upsert(AgentEvent {
            session_id: SessionId("session-b".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });
        // session-c supersedes session-b on the same pid - the pid has moved
        // on again since session-a was first retired.
        registry.upsert(AgentEvent {
            session_id: SessionId("session-c".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });

        // A very late event for session-a, the original (twice-superseded)
        // session id, finally arrives.
        let outcome = registry.upsert(AgentEvent {
            session_id: SessionId("session-a".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Done)
        });

        assert_eq!(
            outcome.agent.session_id,
            SessionId("session-c".to_string()),
            "the late session-a event must not disturb session-c, the pid's current session"
        );
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1, "only session-c should remain tracked");
        assert_eq!(snapshot[0].session_id, SessionId("session-c".to_string()));
        assert_eq!(snapshot[0].status, AgentStatus::Running);
    }

    #[test]
    fn unrelated_pids_are_unaffected_by_dedup() {
        let registry = Registry::new();
        registry.upsert(AgentEvent {
            session_id: SessionId("session-1".to_string()),
            pid: 123,
            ..sample_event(AgentStatus::Running)
        });

        registry.upsert(AgentEvent {
            session_id: SessionId("session-2".to_string()),
            pid: 456,
            ..sample_event(AgentStatus::Running)
        });

        let mut snapshot = registry.snapshot();
        snapshot.sort_by(|a, b| a.session_id.0.cmp(&b.session_id.0));
        assert_eq!(snapshot.len(), 2, "different pids must both be tracked");
        assert_eq!(snapshot[0].session_id, SessionId("session-1".to_string()));
        assert_eq!(snapshot[1].session_id, SessionId("session-2".to_string()));
    }

    #[test]
    fn first_started_event_creates_a_test_run_entry() {
        let registry = Registry::new();

        let test_run = registry.upsert_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Started);

        assert_eq!(test_run.cwd, PathBuf::from("/tmp/project"));
        assert_eq!(test_run.pid, 999);
        assert_eq!(test_run.status, TestRunStatus::Started);
        assert_eq!(registry.snapshot_test_runs().len(), 1);
    }

    #[test]
    fn a_later_event_for_the_same_key_updates_in_place() {
        let registry = Registry::new();
        registry.upsert_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Started);

        let test_run = registry.upsert_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Failed);

        assert_eq!(test_run.status, TestRunStatus::Failed);
        let snapshot = registry.snapshot_test_runs();
        assert_eq!(snapshot.len(), 1, "must update in place, not duplicate");
        assert_eq!(snapshot[0].status, TestRunStatus::Failed);
    }

    #[test]
    fn a_same_pid_completion_event_preserves_run_started_ms() {
        let registry = Registry::new();
        let started = registry.upsert_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Started);
        let run_started_ms = started.run_started_ms;
        thread::sleep(Duration::from_millis(10));

        let test_run = registry.upsert_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Passed);

        assert_eq!(
            test_run.run_started_ms, run_started_ms,
            "a same-pid completion event must not reset run_started_ms"
        );
        assert!(
            test_run.last_updated_ms > run_started_ms,
            "last_updated_ms must still advance even though run_started_ms is unchanged"
        );
    }

    #[test]
    fn a_different_pid_event_resets_run_started_ms() {
        let registry = Registry::new();
        let first = registry.upsert_test_run(PathBuf::from("/tmp/project"), 111, TestRunStatus::Started);
        let first_run_started_ms = first.run_started_ms;
        thread::sleep(Duration::from_millis(10));

        let test_run = registry.upsert_test_run(PathBuf::from("/tmp/project"), 222, TestRunStatus::Started);

        assert!(
            test_run.run_started_ms > first_run_started_ms,
            "a different pid must start a fresh run_started_ms rather than inheriting the previous run's"
        );
    }

    #[test]
    fn different_pids_in_the_same_directory_collapse_to_one_entry() {
        let registry = Registry::new();
        registry.upsert_test_run(PathBuf::from("/tmp/project"), 1, TestRunStatus::Started);

        registry.upsert_test_run(PathBuf::from("/tmp/project"), 2, TestRunStatus::Started);

        assert_eq!(
            registry.snapshot_test_runs().len(),
            1,
            "a directory holds at most one test run, regardless of how many distinct pids report to it"
        );
    }

    #[test]
    fn a_later_test_run_from_a_different_pid_replaces_the_previous_one() {
        let registry = Registry::new();
        registry.upsert_test_run(PathBuf::from("/tmp/project"), 111, TestRunStatus::Started);

        let test_run = registry.upsert_test_run(PathBuf::from("/tmp/project"), 222, TestRunStatus::Failed);

        assert_eq!(
            test_run.pid, 222,
            "the surviving entry must reflect the newest report, not the first one"
        );
        let snapshot = registry.snapshot_test_runs();
        assert_eq!(
            snapshot.len(),
            1,
            "sequential invocations in the same directory (e.g. an edit/test loop) must never accumulate"
        );
        assert_eq!(snapshot[0].pid, 222);
        assert_eq!(snapshot[0].status, TestRunStatus::Failed);
    }

    #[test]
    fn different_directories_each_keep_their_own_test_run() {
        let registry = Registry::new();
        registry.upsert_test_run(PathBuf::from("/tmp/project-a"), 1, TestRunStatus::Passed);

        registry.upsert_test_run(PathBuf::from("/tmp/project-b"), 2, TestRunStatus::Failed);

        let mut snapshot = registry.snapshot_test_runs();
        snapshot.sort_by(|a, b| a.cwd.cmp(&b.cwd));
        assert_eq!(snapshot.len(), 2, "unrelated directories must not affect each other");
        assert_eq!(snapshot[0].cwd, PathBuf::from("/tmp/project-a"));
        assert_eq!(snapshot[1].cwd, PathBuf::from("/tmp/project-b"));
    }

    #[test]
    fn directory_groups_includes_a_directory_with_only_agents() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));

        let groups = registry.directory_groups();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].cwd, PathBuf::from("/tmp/project"));
        assert_eq!(groups[0].agents.len(), 1);
        assert!(groups[0].test_runs.is_empty());
    }

    #[test]
    fn directory_groups_includes_a_directory_with_only_test_runs() {
        let registry = Registry::new();
        registry.upsert_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Started);

        let groups = registry.directory_groups();

        assert_eq!(groups.len(), 1);
        assert!(groups[0].agents.is_empty());
        assert_eq!(groups[0].test_runs.len(), 1);
    }

    #[test]
    fn directory_groups_includes_a_directory_with_both() {
        let registry = Registry::new();
        registry.upsert(sample_event(AgentStatus::Running));
        registry.upsert_test_run(PathBuf::from("/tmp/project"), 999, TestRunStatus::Started);

        let groups = registry.directory_groups();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].agents.len(), 1);
        assert_eq!(groups[0].test_runs.len(), 1);
    }

    #[test]
    fn directory_groups_excludes_directories_with_neither() {
        let registry = Registry::new();

        let groups = registry.directory_groups();

        assert!(groups.is_empty());
    }
}
