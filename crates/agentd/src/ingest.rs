use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use agentmon_proto::{
    AgentEvent, AgentInfo, AgentStatus, LogCategory, LogEntry, SessionId, TestRunInfo, TestRunStatus,
};

use crate::activity_log::ActivityLog;
use crate::notify::Notifier;
use crate::registry::Registry;

/// Invoked with every entry as it's recorded, so a caller (e.g. the server's
/// socket broadcaster) can react without `Ingestor` needing to know about
/// subscribers. Mirrors how `spawn_liveness_sweep` takes a callback for
/// its own updates.
type LogListener = Arc<dyn Fn(LogEntry) + Send + Sync>;

/// Applies incoming agent events to the registry and dispatches
/// notifications on transitions that need the user's attention.
#[derive(Clone)]
pub struct Ingestor {
    registry: Registry,
    notifier: Arc<dyn Notifier>,
    activity_log: ActivityLog,
    log_listener: Arc<Mutex<Option<LogListener>>>,
}

impl Ingestor {
    pub fn new(registry: Registry, notifier: Arc<dyn Notifier>) -> Self {
        Self {
            registry,
            notifier,
            activity_log: ActivityLog::new(),
            log_listener: Arc::new(Mutex::new(None)),
        }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The daemon's bounded activity log - see the activity-log spec.
    pub fn activity_log(&self) -> &ActivityLog {
        &self.activity_log
    }

    /// Registers a callback invoked with every log entry as it's recorded.
    /// Used by `server::serve` to broadcast new entries to subscribers.
    pub fn set_log_listener(&self, listener: impl Fn(LogEntry) + Send + Sync + 'static) {
        *self.log_listener.lock().unwrap() = Some(Arc::new(listener));
    }

    /// Updates the registry from a hook-reported event, notifying the user
    /// and recording an activity-log entry if the resulting status is a new
    /// transition into "done" or "needs input". Independent of any
    /// notification, also records a "started" entry whenever the transition
    /// moves status into "running" from something else (or registers a
    /// brand-new entry already "running") - see the activity-log spec's
    /// "Activity log captures notification-worthy events" requirement.
    ///
    /// Also returns any session ids the registry retired while applying this
    /// event (see `UpsertOutcome::retired_session_ids`), so the caller can
    /// broadcast their removal to already-connected clients.
    pub fn ingest_event(&self, event: AgentEvent) -> (AgentInfo, Vec<SessionId>) {
        let outcome = self.registry.upsert(event);
        if outcome.stale_event_ignored {
            // A late event for an already-superseded session id: `agent` and
            // `previous_status` describe the pid's unaffected live entry,
            // not a real transition, so notifying/logging off them would be
            // spurious (see `UpsertOutcome::stale_event_ignored`'s doc
            // comment) - most concretely, a live "needs input" status always
            // notifies regardless of whether it changed.
            return (outcome.agent, outcome.retired_session_ids);
        }
        if is_run_start(outcome.previous_status, outcome.agent.status) {
            // Uses `run_started_ms` (rather than a fresh `now_ms()` call) so
            // this entry's timestamp exactly matches the value the registry
            // just set/reset it to - the same value a "done" entry's
            // Logs-tab duration will later be measured from.
            self.record_log(LogEntry {
                working_dir: outcome.agent.cwd.clone(),
                category: LogCategory::Agent,
                status: "started".to_string(),
                occurred_at_ms: outcome.agent.run_started_ms,
                pid: Some(outcome.agent.pid),
            });
        }
        if should_notify(outcome.previous_status, outcome.agent.status) {
            self.notifier.notify(&outcome.agent);
            self.record_log(LogEntry {
                working_dir: outcome.agent.cwd.clone(),
                category: LogCategory::Agent,
                status: agent_log_status(outcome.agent.status).to_string(),
                occurred_at_ms: now_ms(),
                pid: Some(outcome.agent.pid),
            });
        }
        (outcome.agent, outcome.retired_session_ids)
    }

    /// Updates the registry from a reported test-run event, notifying the
    /// user and recording an activity-log entry for every status - "started",
    /// "passed", or "failed" - independent of whether its working directory
    /// has a tracked agent, so each event over the life of a long agent
    /// session is heard on its own.
    pub fn ingest_test_run(&self, cwd: PathBuf, pid: u32, status: TestRunStatus) -> TestRunInfo {
        let test_run = self.registry.upsert_test_run(cwd, pid, status);
        self.notifier.notify_test_run(&test_run.cwd, status);
        self.record_log(LogEntry {
            working_dir: test_run.cwd.clone(),
            category: LogCategory::TestRun,
            status: test_run_log_status(status).to_string(),
            occurred_at_ms: now_ms(),
            pid: Some(test_run.pid),
        });
        test_run
    }

    fn record_log(&self, entry: LogEntry) {
        self.activity_log.record(entry.clone());
        if let Some(listener) = self.log_listener.lock().unwrap().as_ref() {
            listener(entry);
        }
    }
}

/// Maps an agent status that warrants a notification to the status string
/// recorded in the activity log - matching the wire format's snake_case
/// spelling (see agentmon-proto's `AgentStatus`).
fn agent_log_status(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Done => "done",
        AgentStatus::NeedsInput => "needs_input",
        _ => unreachable!("only done/needs_input transitions are logged, guarded by should_notify"),
    }
}

/// Maps a test-run status to the status string recorded in the activity log,
/// matching the wire format's snake_case spelling.
fn test_run_log_status(status: TestRunStatus) -> &'static str {
    match status {
        TestRunStatus::Started => "started",
        TestRunStatus::Passed => "passed",
        TestRunStatus::Failed => "failed",
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
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

/// Whether this transition begins a new run of work, warranting a "started"
/// activity-log entry: entering "running" from any other status (including
/// no previous status at all, i.e. a brand-new registration), but not a
/// same-status "running" event continuing an already-running turn. Mirrors
/// exactly the condition the registry uses to reset `run_started_ms`.
fn is_run_start(previous: Option<AgentStatus>, current: AgentStatus) -> bool {
    current == AgentStatus::Running && previous != Some(AgentStatus::Running)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::HostContext;
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

        let (agent, _) = ingestor.ingest_event(event(AgentStatus::Running));

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
        let (agent, _) = ingestor.ingest_event(event(AgentStatus::NeedsInput));

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
        let (agent, _) = ingestor.ingest_event(event(AgentStatus::Running));

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

    #[test]
    fn done_transition_appends_a_log_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Done));

        let logs = ingestor.activity_log().snapshot();
        assert_eq!(
            logs.len(),
            2,
            "the initial run-start is logged in addition to the done entry"
        );
        assert_eq!(logs[0].status, "started");
        assert_eq!(logs[1].category, agentmon_proto::LogCategory::Agent);
        assert_eq!(logs[1].status, "done");
        assert_eq!(logs[1].working_dir, PathBuf::from("/tmp/project"));
    }

    #[test]
    fn needs_input_transition_appends_a_log_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::NeedsInput));

        let logs = ingestor.activity_log().snapshot();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].status, "needs_input");
    }

    #[test]
    fn repeated_needs_input_appends_a_log_entry_each_time() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::NeedsInput));
        ingestor.ingest_event(event(AgentStatus::NeedsInput));

        assert_eq!(
            ingestor.activity_log().snapshot().len(),
            2,
            "each needs-input event is logged, matching notification behavior"
        );
    }

    #[test]
    fn repeated_done_appends_only_one_log_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_event(event(AgentStatus::Done));

        assert_eq!(
            ingestor.activity_log().snapshot().len(),
            1,
            "repeated same-status done events must not log a duplicate entry"
        );
    }

    #[test]
    fn declined_does_not_append_a_log_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Declined));

        let logs = ingestor.activity_log().snapshot();
        assert!(
            logs.iter().all(|entry| entry.status != "declined"),
            "the declined transition itself must not produce a log entry, got: {logs:?}"
        );
        assert_eq!(
            logs.len(),
            1,
            "only the preceding run-start (from the Running event) should be logged"
        );
        assert_eq!(logs[0].status, "started");
    }

    #[test]
    fn idle_transitions_do_not_append_log_entries() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Idle));

        let logs = ingestor.activity_log().snapshot();
        assert_eq!(
            logs.len(),
            1,
            "only the run-start from the Running event should be logged, not the Idle transition"
        );
        assert_eq!(logs[0].status, "started");
    }

    #[test]
    fn a_new_agents_first_running_event_logs_started_with_its_pid() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::Running));

        let logs = ingestor.activity_log().snapshot();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].category, agentmon_proto::LogCategory::Agent);
        assert_eq!(logs[0].status, "started");
        assert_eq!(
            logs[0].pid,
            Some(1),
            "a started entry must carry the reporting agent's process id"
        );
    }

    #[test]
    fn repeated_running_status_does_not_append_a_started_log_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::Running));
        // Models repeated PreToolUse/PostToolUse events during the same
        // running turn - no additional transition, so no additional entry.
        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Running));

        assert_eq!(
            ingestor.activity_log().snapshot().len(),
            1,
            "continuing an already-running turn must not append another started entry"
        );
    }

    #[test]
    fn resuming_into_running_from_each_status_logs_exactly_one_started_entry() {
        for from in [
            AgentStatus::Idle,
            AgentStatus::NeedsInput,
            AgentStatus::Done,
            AgentStatus::Declined,
        ] {
            let notifier = Arc::new(RecordingNotifier::default());
            let ingestor = Ingestor::new(Registry::new(), notifier);

            ingestor.ingest_event(event(AgentStatus::Running));
            ingestor.ingest_event(event(from));
            let logs_before_resume = ingestor.activity_log().snapshot().len();

            ingestor.ingest_event(event(AgentStatus::Running));

            let logs = ingestor.activity_log().snapshot();
            assert_eq!(
                logs.len(),
                logs_before_resume + 1,
                "resuming into running from {from:?} must append exactly one more entry"
            );
            assert_eq!(logs.last().unwrap().status, "started");
        }
    }

    #[test]
    fn resuming_into_running_from_stale_logs_a_started_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let registry = Registry::new();
        let ingestor = Ingestor::new(registry.clone(), notifier);

        ingestor.ingest_event(event(AgentStatus::Running));
        registry
            .mark_stale(&SessionId("session-1".to_string()))
            .expect("agent should be marked stale");
        let logs_before_resume = ingestor.activity_log().snapshot().len();

        ingestor.ingest_event(event(AgentStatus::Running));

        let logs = ingestor.activity_log().snapshot();
        assert_eq!(logs.len(), logs_before_resume + 1);
        assert_eq!(logs.last().unwrap().status, "started");
    }

    #[test]
    fn a_session_cycling_through_several_turns_logs_one_started_entry_per_turn() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        // Three full started -> running -> done turns under the same pid.
        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Done));

        let logs = ingestor.activity_log().snapshot();
        let started_count = logs.iter().filter(|entry| entry.status == "started").count();
        let done_count = logs.iter().filter(|entry| entry.status == "done").count();
        assert_eq!(
            started_count, 3,
            "each of the three turns must log its own started entry, not just the first"
        );
        assert_eq!(done_count, 3);
    }

    #[test]
    fn done_and_needs_input_log_entries_carry_the_agents_pid() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::NeedsInput));
        ingestor.ingest_event(event(AgentStatus::Running));
        ingestor.ingest_event(event(AgentStatus::Done));

        let logs = ingestor.activity_log().snapshot();
        let needs_input_entry = logs.iter().find(|entry| entry.status == "needs_input").unwrap();
        let done_entry = logs.iter().find(|entry| entry.status == "done").unwrap();
        assert_eq!(needs_input_entry.pid, Some(1));
        assert_eq!(done_entry.pid, Some(1));
    }

    #[test]
    fn a_late_event_for_a_superseded_session_does_not_notify_or_log_even_if_the_live_session_needs_input() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier.clone());

        // session-1 is superseded by session-2 on the same pid (e.g.
        // `/clear`), and session-2 then needs input - a status that always
        // notifies on its own, regardless of whether it just changed.
        ingestor.ingest_event(AgentEvent {
            session_id: SessionId("session-1".to_string()),
            pid: 1,
            ..event(AgentStatus::Running)
        });
        ingestor.ingest_event(AgentEvent {
            session_id: SessionId("session-2".to_string()),
            pid: 1,
            ..event(AgentStatus::Running)
        });
        ingestor.ingest_event(AgentEvent {
            session_id: SessionId("session-2".to_string()),
            pid: 1,
            ..event(AgentStatus::NeedsInput)
        });
        let calls_before_late_event = notifier.calls.lock().unwrap().len();
        let logs_before_late_event = ingestor.activity_log().snapshot().len();

        // A late hook event for session-1 - already superseded - finally
        // arrives. Naively re-checking should_notify against the returned
        // (unchanged) session-2 entry would wrongly notify again here,
        // since session-2's live status is "needs input".
        let (agent, _) = ingestor.ingest_event(AgentEvent {
            session_id: SessionId("session-1".to_string()),
            pid: 1,
            ..event(AgentStatus::Done)
        });

        assert_eq!(
            agent.session_id,
            SessionId("session-2".to_string()),
            "the late event must resolve to session-2, the pid's live entry"
        );
        assert_eq!(
            notifier.calls.lock().unwrap().len(),
            calls_before_late_event,
            "a stale, dropped event must not trigger a notification"
        );
        assert_eq!(
            ingestor.activity_log().snapshot().len(),
            logs_before_late_event,
            "a stale, dropped event must not append an activity-log entry"
        );
    }

    #[test]
    fn test_run_lifecycle_events_each_append_a_log_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 1, TestRunStatus::Started);
        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 1, TestRunStatus::Passed);

        let logs = ingestor.activity_log().snapshot();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].category, agentmon_proto::LogCategory::TestRun);
        assert_eq!(logs[0].status, "started");
        assert_eq!(logs[1].status, "passed");
    }

    #[test]
    fn a_failed_test_run_appends_a_log_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);

        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 1, TestRunStatus::Failed);

        let logs = ingestor.activity_log().snapshot();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].status, "failed");
    }

    #[test]
    fn set_log_listener_is_invoked_for_every_recorded_entry() {
        let notifier = Arc::new(RecordingNotifier::default());
        let ingestor = Ingestor::new(Registry::new(), notifier);
        let received: Arc<Mutex<Vec<LogEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();
        ingestor.set_log_listener(move |entry| received_clone.lock().unwrap().push(entry));

        ingestor.ingest_event(event(AgentStatus::Done));
        ingestor.ingest_test_run(PathBuf::from("/tmp/project"), 1, TestRunStatus::Started);

        assert_eq!(received.lock().unwrap().len(), 2);
    }
}
