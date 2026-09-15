use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use agentmon_proto::LogEntry;

/// Global cap on the activity log's size, per the activity-log spec's
/// "Bounded global retention" requirement. Not configurable - see design.md.
pub const MAX_LOG_ENTRIES: usize = 500;

/// Bounded, in-memory record of notification-worthy agent and test-run
/// events, evicting the oldest entry (by insertion order, which matches
/// event order since entries are always recorded in real time) once
/// `MAX_LOG_ENTRIES` is reached - regardless of which project the oldest
/// entry belongs to.
///
/// Cheap to clone: shares state via `Arc`, like `Registry`.
#[derive(Clone, Default)]
pub struct ActivityLog {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl ActivityLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, entry: LogEntry) {
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// Current entries, oldest first (the order they occurred).
    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap().iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmon_proto::LogCategory;
    use std::path::{Path, PathBuf};

    fn entry(working_dir: &str, occurred_at_ms: u64) -> LogEntry {
        LogEntry {
            working_dir: PathBuf::from(working_dir),
            category: LogCategory::Agent,
            status: "done".to_string(),
            occurred_at_ms,
            pid: None,
        }
    }

    #[test]
    fn records_entries_in_order() {
        let log = ActivityLog::new();
        log.record(entry("/tmp/a", 1));
        log.record(entry("/tmp/b", 2));

        let snapshot = log.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].working_dir, PathBuf::from("/tmp/a"));
        assert_eq!(snapshot[1].working_dir, PathBuf::from("/tmp/b"));
    }

    #[test]
    fn evicts_the_oldest_entry_once_the_cap_is_reached() {
        let log = ActivityLog::new();
        for i in 0..MAX_LOG_ENTRIES as u64 {
            log.record(entry("/tmp/project", i));
        }

        log.record(entry("/tmp/project", MAX_LOG_ENTRIES as u64));

        let snapshot = log.snapshot();
        assert_eq!(snapshot.len(), MAX_LOG_ENTRIES, "log must stay capped at {MAX_LOG_ENTRIES}");
        assert_eq!(
            snapshot[0].occurred_at_ms, 1,
            "the single oldest entry (occurred_at_ms 0) must be evicted"
        );
        assert_eq!(snapshot.last().unwrap().occurred_at_ms, MAX_LOG_ENTRIES as u64);
    }

    #[test]
    fn eviction_is_global_across_different_working_directories() {
        let log = ActivityLog::new();
        log.record(entry("/tmp/oldest-project", 0));
        for i in 1..MAX_LOG_ENTRIES as u64 {
            log.record(entry("/tmp/other-project", i));
        }

        // The log is now at capacity, with the single oldest entry belonging
        // to a different project than the one about to report.
        log.record(entry("/tmp/new-event-project", MAX_LOG_ENTRIES as u64));

        let snapshot = log.snapshot();
        assert_eq!(snapshot.len(), MAX_LOG_ENTRIES);
        assert!(
            snapshot.iter().all(|e| e.working_dir.as_path() != Path::new("/tmp/oldest-project")),
            "the oldest entry must be evicted even though it belongs to a different project than the new event"
        );
    }
}
