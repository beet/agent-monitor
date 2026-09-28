use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod framing;
pub use framing::{read_message, write_message};

/// Default per-user socket path: `~/Library/Application Support/agentmon/agentd.sock`.
///
/// Shared by the daemon (binds it) and every client (the TUI, the hook
/// reporter) so they agree on where to find each other without one
/// depending on the other's crate.
pub fn default_socket_path() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME environment variable must be set");
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("agentmon")
        .join("agentd.sock")
}

/// Identifies a Claude Code session across hook events and client updates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

/// Where a Claude Code session is hosted, per proposal.md's "identity" decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostContext {
    Nvim,
    Terminal,
    Desktop,
}

/// Lifecycle status of a tracked agent, per the agent-daemon and
/// agent-monitor-tui specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Running,
    Idle,
    NeedsInput,
    Done,
    Stale,
    Declined,
}

/// Lifecycle status of a tracked test run, per the rspec-test-reporting and
/// agent-daemon specs. Distinct from `AgentStatus`: a test run has no
/// "declined" or "stale" concept - its last reported status simply stands
/// until superseded by the next run at the same working directory and pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunStatus {
    /// The run is in progress. Named to match `AgentStatus::Running` rather
    /// than the "started" word used for the one-time event that begins a
    /// run (reported by the RSpec formatter, and used for the activity
    /// log's entry and the notification text) - renamed here on purpose so
    /// the ongoing/live status reads "running" wherever it's displayed,
    /// while `#[serde(rename = "started")]` keeps the wire JSON value
    /// unchanged, so the formatter and daemon need no changes.
    #[serde(rename = "started")]
    Running,
    Passed,
    Failed,
}

/// The daemon's view of a tracked test run, as sent to clients. Identified by
/// `(cwd, pid)`, not by any Claude Code session id - a test run may be
/// launched outside any agent's process tree (see rspec-test-reporting spec).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRunInfo {
    pub cwd: PathBuf,
    pub pid: u32,
    pub status: TestRunStatus,
    /// Unix epoch milliseconds of the last update to this test run.
    pub last_updated_ms: u64,
    /// Unix epoch milliseconds of when this run (the tracked process id)
    /// started - unlike `last_updated_ms`, this stays fixed across the
    /// started -> passed/failed lifecycle of the same process, and only
    /// resets when a different pid reports for the same directory.
    pub run_started_ms: u64,
}

/// A status event reported by a Claude Code hook to the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEvent {
    pub session_id: SessionId,
    pub cwd: PathBuf,
    pub host_context: HostContext,
    pub pid: u32,
    pub status: AgentStatus,
}

/// The daemon's view of a tracked agent, as sent to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInfo {
    pub session_id: SessionId,
    pub cwd: PathBuf,
    pub host_context: HostContext,
    pub pid: u32,
    pub status: AgentStatus,
    /// Unix epoch milliseconds of the last update to this agent.
    pub last_updated_ms: u64,
    /// Unix epoch milliseconds of when this agent most recently entered
    /// its current status, unlike `last_updated_ms` which also bumps on
    /// same-status events (e.g. each tool call while running).
    pub status_since_ms: u64,
    /// Unix epoch milliseconds of when this agent most recently transitioned
    /// into "running" (or was registered for the first time already
    /// "running"). Unlike `TestRunInfo::run_started_ms`, which resets on a
    /// new pid because a test run's pid *is* one run, this resets on every
    /// transition into running - including "done" -> "running" - because one
    /// agent's pid persists across many started -> running -> done turns
    /// over the life of a session.
    pub run_started_ms: u64,
}

/// Category of event recorded in the activity log - see the activity-log
/// spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogCategory {
    Agent,
    TestRun,
    Reminder,
}

/// One notification-worthy event recorded in the daemon's bounded activity
/// log, per the activity-log spec's "Activity log captures notification-worthy
/// events" requirement. `status` reuses the same status strings already sent
/// for agents/test-runs (e.g. "done", "needs_input", "started") rather than a
/// parallel enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub working_dir: PathBuf,
    pub category: LogCategory,
    pub status: String,
    /// Unix epoch milliseconds of when this event occurred.
    pub occurred_at_ms: u64,
    /// The reporting process id, when known. Populated for agent-category
    /// entries; test-run and reminder entries leave this `None`.
    pub pid: Option<u32>,
    /// The reminder's name as of this event. Populated only for
    /// reminder-category entries, so a later-deleted reminder's past entries
    /// keep showing the name it had when each was recorded.
    pub reminder_name: Option<String>,
    /// Unix epoch milliseconds of the reminder's due time as of this event.
    /// Populated only for reminder-category "started" entries, computed from
    /// its start time and duration at the moment it started; it does not
    /// change if the reminder's duration is edited later.
    pub reminder_due_at_ms: Option<u64>,
}

/// Identifies a reminder across client commands and daemon updates.
/// Daemon-generated - unlike `SessionId`, nothing gives a client a natural
/// identity for a reminder before the daemon creates it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReminderId(pub String);

/// Lifecycle status of a tracked reminder, per the reminders spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderStatus {
    NotYetStarted,
    Running,
    Done,
}

/// The daemon's view of a tracked reminder, as sent to clients. Mirrors
/// `TestRunInfo`'s `run_started_ms`/`last_updated_ms` pair so elapsed/duration
/// math is identical: running elapsed = now - `run_started_ms`; done elapsed
/// = `last_updated_ms` - `run_started_ms`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderInfo {
    pub id: ReminderId,
    pub cwd: PathBuf,
    pub name: String,
    pub duration_minutes: u32,
    pub status: ReminderStatus,
    /// Unix epoch milliseconds of when this reminder was created.
    pub created_at_ms: u64,
    /// Unix epoch milliseconds of this reminder's most recent start. `None`
    /// until it has been started at least once.
    pub run_started_ms: Option<u64>,
    /// Unix epoch milliseconds of the last started/stopped/finished event,
    /// or `created_at_ms` if it has never been started.
    pub last_updated_ms: u64,
}

/// The first message a connection sends, telling the daemon whether it is a
/// short-lived hook event report or a long-lived subscriber (e.g. the TUI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    ReportEvent { event: AgentEvent },
    /// Reported by a test formatter (e.g. RSpec), not a Claude Code hook -
    /// carries no session id, since a test run has no reliable way to learn
    /// one (see rspec-test-reporting spec).
    ReportTestRun {
        cwd: PathBuf,
        pid: u32,
        status: TestRunStatus,
    },
    Subscribe,
    /// Creates a new reminder for `cwd`. The daemon assigns its `ReminderId`
    /// and broadcasts the result as a `ReminderUpdate` - there is no
    /// synchronous reply, matching `ReportEvent`/`ReportTestRun`'s
    /// fire-and-forget shape.
    CreateReminder {
        cwd: PathBuf,
        name: String,
        duration_minutes: u32,
    },
    /// Changes an existing reminder's name and/or duration, regardless of its
    /// current status.
    UpdateReminder {
        id: ReminderId,
        name: String,
        duration_minutes: u32,
    },
    /// Removes a reminder regardless of its current status. Does not affect
    /// any activity log entry already recorded for it.
    DeleteReminder { id: ReminderId },
    /// Starts a reminder - fresh, or as a re-run of one already done.
    StartReminder { id: ReminderId },
    /// Stops a running reminder. Does not send a notification.
    StopReminder { id: ReminderId },
}

/// A message sent from the daemon to a connected client (e.g. the TUI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Full current state, sent once when a client connects.
    Snapshot {
        agents: Vec<AgentInfo>,
        test_runs: Vec<TestRunInfo>,
        /// The activity log's current entries, in the order they occurred.
        logs: Vec<LogEntry>,
        /// Every currently tracked reminder, across all projects.
        reminders: Vec<ReminderInfo>,
    },
    /// An incremental update to a single agent's state.
    AgentUpdate { agent: AgentInfo },
    /// An incremental update to a single test run's state.
    TestRunUpdate { test_run: TestRunInfo },
    /// A new activity log entry, pushed as it's recorded.
    LogAppended { entry: LogEntry },
    /// A tracked agent's session id has been retired - e.g. a new session id
    /// took over its pid (`/clear`) - and should be dropped from a client's
    /// local state rather than lingering as a frozen duplicate.
    AgentRemoved { session_id: SessionId },
    /// An incremental update to a single reminder's state - created, edited,
    /// started, stopped, or completed.
    ReminderUpdate { reminder: ReminderInfo },
    /// A reminder has been deleted and should be dropped from a client's
    /// local state.
    ReminderRemoved { id: ReminderId },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_test_run() -> TestRunInfo {
        TestRunInfo {
            cwd: PathBuf::from("/Users/beet/Documents/Projects/enclaudinate"),
            pid: 5150,
            status: TestRunStatus::Running,
            last_updated_ms: 1_700_000_000_000,
            run_started_ms: 1_700_000_000_000,
        }
    }

    fn sample_agent() -> AgentInfo {
        AgentInfo {
            session_id: SessionId("session-123".to_string()),
            cwd: PathBuf::from("/Users/beet/Documents/Projects/enclaudinate"),
            host_context: HostContext::Nvim,
            pid: 4242,
            status: AgentStatus::Running,
            last_updated_ms: 1_700_000_000_000,
            status_since_ms: 1_700_000_000_000,
            run_started_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn agent_event_round_trips_through_json() {
        let event = AgentEvent {
            session_id: SessionId("session-123".to_string()),
            cwd: PathBuf::from("/tmp/project"),
            host_context: HostContext::Terminal,
            pid: 99,
            status: AgentStatus::NeedsInput,
        };

        let json = serde_json::to_string(&event).unwrap();
        let decoded: AgentEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event, decoded);
    }

    #[test]
    fn agent_info_round_trips_through_json() {
        let info = sample_agent();

        let json = serde_json::to_string(&info).unwrap();
        let decoded: AgentInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info, decoded);
        assert!(
            json.contains("\"status_since_ms\":1700000000000"),
            "expected status_since_ms field in JSON, got: {json}"
        );
        assert!(
            json.contains("\"run_started_ms\":1700000000000"),
            "expected run_started_ms field in JSON, got: {json}"
        );
    }

    #[test]
    fn test_run_info_round_trips_through_json() {
        let info = sample_test_run();

        let json = serde_json::to_string(&info).unwrap();
        let decoded: TestRunInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info, decoded);
        assert!(
            json.contains("\"run_started_ms\":1700000000000"),
            "expected run_started_ms field in JSON, got: {json}"
        );
    }

    fn sample_log_entry() -> LogEntry {
        LogEntry {
            working_dir: PathBuf::from("/Users/beet/Documents/Projects/enclaudinate"),
            category: LogCategory::Agent,
            status: "done".to_string(),
            occurred_at_ms: 1_700_000_000_000,
            pid: Some(4242),
            reminder_name: None,
            reminder_due_at_ms: None,
        }
    }

    fn sample_reminder() -> ReminderInfo {
        ReminderInfo {
            id: ReminderId("reminder-1".to_string()),
            cwd: PathBuf::from("/Users/beet/Documents/Projects/enclaudinate"),
            name: "Check the build".to_string(),
            duration_minutes: 10,
            status: ReminderStatus::Running,
            created_at_ms: 1_700_000_000_000,
            run_started_ms: Some(1_700_000_000_000),
            last_updated_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn server_message_snapshot_round_trips_through_json() {
        let message = ServerMessage::Snapshot {
            agents: vec![sample_agent()],
            test_runs: vec![sample_test_run()],
            logs: vec![sample_log_entry()],
            reminders: vec![sample_reminder()],
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn reminder_info_round_trips_through_json() {
        let info = sample_reminder();

        let json = serde_json::to_string(&info).unwrap();
        let decoded: ReminderInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info, decoded);
    }

    #[test]
    fn reminder_status_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&ReminderStatus::NotYetStarted).unwrap(),
            "\"not_yet_started\""
        );
        assert_eq!(
            serde_json::to_string(&ReminderStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&ReminderStatus::Done).unwrap(),
            "\"done\""
        );
    }

    #[test]
    fn log_category_reminder_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&LogCategory::Reminder).unwrap(),
            "\"reminder\""
        );
    }

    #[test]
    fn reminder_log_entry_round_trips_with_its_name() {
        let entry = LogEntry {
            category: LogCategory::Reminder,
            status: "finished".to_string(),
            pid: None,
            reminder_name: Some("Check the build".to_string()),
            ..sample_log_entry()
        };

        let json = serde_json::to_string(&entry).unwrap();
        let decoded: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry, decoded);
        assert_eq!(decoded.reminder_name.as_deref(), Some("Check the build"));
    }

    #[test]
    fn started_reminder_log_entry_round_trips_with_its_eta() {
        let entry = LogEntry {
            category: LogCategory::Reminder,
            status: "started".to_string(),
            pid: None,
            reminder_name: Some("Check the build".to_string()),
            reminder_due_at_ms: Some(1_700_000_600_000),
            ..sample_log_entry()
        };

        let json = serde_json::to_string(&entry).unwrap();
        let decoded: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry, decoded);
        assert_eq!(decoded.reminder_due_at_ms, Some(1_700_000_600_000));
        assert!(
            json.contains("\"reminder_due_at_ms\":1700000600000"),
            "expected reminder_due_at_ms field in JSON, got: {json}"
        );
    }

    #[test]
    fn client_message_create_reminder_round_trips_through_json() {
        let message = ClientMessage::CreateReminder {
            cwd: PathBuf::from("/tmp/project"),
            name: "Check the build".to_string(),
            duration_minutes: 10,
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn client_message_update_reminder_round_trips_through_json() {
        let message = ClientMessage::UpdateReminder {
            id: ReminderId("reminder-1".to_string()),
            name: "Check the deploy".to_string(),
            duration_minutes: 15,
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn client_message_delete_start_stop_reminder_round_trip_through_json() {
        for message in [
            ClientMessage::DeleteReminder {
                id: ReminderId("reminder-1".to_string()),
            },
            ClientMessage::StartReminder {
                id: ReminderId("reminder-1".to_string()),
            },
            ClientMessage::StopReminder {
                id: ReminderId("reminder-1".to_string()),
            },
        ] {
            let json = serde_json::to_string(&message).unwrap();
            let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(message, decoded);
        }
    }

    #[test]
    fn server_message_reminder_update_round_trips_through_json() {
        let message = ServerMessage::ReminderUpdate {
            reminder: sample_reminder(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn server_message_reminder_removed_round_trips_through_json() {
        let message = ServerMessage::ReminderRemoved {
            id: ReminderId("reminder-1".to_string()),
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn log_entry_round_trips_through_json() {
        let entry = sample_log_entry();

        let json = serde_json::to_string(&entry).unwrap();
        let decoded: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry, decoded);
        assert!(
            json.contains("\"pid\":4242"),
            "expected pid field in JSON, got: {json}"
        );
    }

    #[test]
    fn log_entry_with_no_pid_round_trips_through_json() {
        let entry = LogEntry {
            pid: None,
            ..sample_log_entry()
        };

        let json = serde_json::to_string(&entry).unwrap();
        let decoded: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry, decoded);
        assert_eq!(decoded.pid, None);
    }

    #[test]
    fn log_category_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&LogCategory::Agent).unwrap(),
            "\"agent\""
        );
        assert_eq!(
            serde_json::to_string(&LogCategory::TestRun).unwrap(),
            "\"test_run\""
        );
    }

    #[test]
    fn server_message_log_appended_round_trips_through_json() {
        let message = ServerMessage::LogAppended {
            entry: sample_log_entry(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn server_message_agent_removed_round_trips_through_json() {
        let message = ServerMessage::AgentRemoved {
            session_id: SessionId("session-123".to_string()),
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn client_message_report_test_run_round_trips_through_json() {
        let message = ClientMessage::ReportTestRun {
            cwd: PathBuf::from("/tmp/project"),
            pid: 321,
            status: TestRunStatus::Failed,
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn server_message_test_run_update_round_trips_through_json() {
        let mut test_run = sample_test_run();
        test_run.status = TestRunStatus::Passed;
        let message = ServerMessage::TestRunUpdate { test_run };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn test_run_status_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&TestRunStatus::Running).unwrap(),
            "\"started\""
        );
        assert_eq!(
            serde_json::to_string(&TestRunStatus::Passed).unwrap(),
            "\"passed\""
        );
        assert_eq!(
            serde_json::to_string(&TestRunStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn test_run_status_running_round_trips_through_the_started_wire_value() {
        let json = serde_json::to_string(&TestRunStatus::Running).unwrap();
        assert_eq!(json, "\"started\"", "the wire value must stay \"started\"");

        let decoded: TestRunStatus = serde_json::from_str("\"started\"").unwrap();
        assert_eq!(decoded, TestRunStatus::Running);
    }

    #[test]
    fn client_message_report_event_round_trips_through_json() {
        let message = ClientMessage::ReportEvent {
            event: AgentEvent {
                session_id: SessionId("session-123".to_string()),
                cwd: PathBuf::from("/tmp/project"),
                host_context: HostContext::Desktop,
                pid: 7,
                status: AgentStatus::Idle,
            },
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn client_message_subscribe_round_trips_through_json() {
        let message = ClientMessage::Subscribe;

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn server_message_agent_update_round_trips_through_json() {
        let mut agent = sample_agent();
        agent.status = AgentStatus::Done;
        let message = ServerMessage::AgentUpdate { agent };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message, decoded);
    }

    #[test]
    fn host_context_and_status_serialize_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&HostContext::Desktop).unwrap(),
            "\"desktop\""
        );
        assert_eq!(
            serde_json::to_string(&AgentStatus::NeedsInput).unwrap(),
            "\"needs_input\""
        );
        assert_eq!(
            serde_json::to_string(&AgentStatus::Declined).unwrap(),
            "\"declined\""
        );
    }
}
