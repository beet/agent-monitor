use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agentmon_proto::{ReminderId, ReminderInfo, ReminderStatus};

/// Cap on how many reminders a single project (working directory) may hold,
/// per the reminders spec's "Per-project reminder cap" requirement. Storage
/// for reminders is bounded separately from the activity log's own cap.
pub const MAX_REMINDERS_PER_PROJECT: usize = 100;

#[derive(Default)]
struct ReminderRegistryState {
    reminders: HashMap<ReminderId, ReminderInfo>,
    next_id: u64,
}

/// In-memory registry of tracked reminders, keyed by daemon-generated id.
///
/// Cheap to clone: internally shares state via `Arc`, like `Registry`.
/// Reminders live only for the daemon's process lifetime - see design.md.
#[derive(Clone, Default)]
pub struct ReminderRegistry {
    state: Arc<Mutex<ReminderRegistryState>>,
}

impl ReminderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new reminder as not-yet-started. Returns `None` without
    /// creating anything if `cwd` already holds `MAX_REMINDERS_PER_PROJECT`
    /// reminders.
    pub fn create(&self, cwd: PathBuf, name: String, duration_minutes: u32) -> Option<ReminderInfo> {
        let mut state = self.state.lock().unwrap();
        let existing_for_project = state.reminders.values().filter(|r| r.cwd == cwd).count();
        if existing_for_project >= MAX_REMINDERS_PER_PROJECT {
            return None;
        }

        state.next_id += 1;
        let id = ReminderId(format!("reminder-{}", state.next_id));
        let now = now_ms();
        let reminder = ReminderInfo {
            id: id.clone(),
            cwd,
            name,
            duration_minutes,
            status: ReminderStatus::NotYetStarted,
            created_at_ms: now,
            run_started_ms: None,
            last_updated_ms: now,
        };
        state.reminders.insert(id, reminder.clone());
        Some(reminder)
    }

    /// Changes a reminder's name and/or duration, regardless of its current
    /// status. Returns `None` if `id` is unknown.
    pub fn update(&self, id: &ReminderId, name: String, duration_minutes: u32) -> Option<ReminderInfo> {
        let mut state = self.state.lock().unwrap();
        let reminder = state.reminders.get_mut(id)?;
        reminder.name = name;
        reminder.duration_minutes = duration_minutes;
        Some(reminder.clone())
    }

    /// Removes a reminder regardless of its current status. Returns `true` if
    /// a reminder was removed.
    pub fn delete(&self, id: &ReminderId) -> bool {
        let mut state = self.state.lock().unwrap();
        state.reminders.remove(id).is_some()
    }

    /// Starts a reminder - fresh, or as a re-run of one already done. Always
    /// records the current time as this run's start, independent of any
    /// previous run, so re-running never accumulates from the prior run.
    /// Returns `None` if `id` is unknown.
    pub fn start(&self, id: &ReminderId) -> Option<ReminderInfo> {
        let mut state = self.state.lock().unwrap();
        let reminder = state.reminders.get_mut(id)?;
        let now = now_ms();
        reminder.status = ReminderStatus::Running;
        reminder.run_started_ms = Some(now);
        reminder.last_updated_ms = now;
        Some(reminder.clone())
    }

    /// Stops a running reminder, computing its elapsed time from this run's
    /// start to now. Returns `None` if `id` is unknown or the reminder is not
    /// currently running (stopping is only meaningful for a running
    /// reminder).
    pub fn stop(&self, id: &ReminderId) -> Option<ReminderInfo> {
        let mut state = self.state.lock().unwrap();
        let reminder = state.reminders.get_mut(id)?;
        if reminder.status != ReminderStatus::Running {
            return None;
        }
        reminder.status = ReminderStatus::Done;
        reminder.last_updated_ms = now_ms();
        Some(reminder.clone())
    }

    pub fn get(&self, id: &ReminderId) -> Option<ReminderInfo> {
        self.state.lock().unwrap().reminders.get(id).cloned()
    }

    pub fn snapshot(&self) -> Vec<ReminderInfo> {
        self.state.lock().unwrap().reminders.values().cloned().collect()
    }

    /// Transitions to `Done` every running reminder whose elapsed time (now -
    /// its current run's start) has reached its duration, computing its
    /// elapsed time as exactly that duration. Returns the reminders that
    /// changed, for the caller to broadcast/notify/log.
    pub fn sweep_completed(&self) -> Vec<ReminderInfo> {
        let now = now_ms();
        let mut state = self.state.lock().unwrap();
        let mut changed = Vec::new();
        for reminder in state.reminders.values_mut() {
            if reminder.status != ReminderStatus::Running {
                continue;
            }
            let Some(started) = reminder.run_started_ms else {
                continue;
            };
            let due_at_ms = started + reminder.duration_minutes as u64 * 60_000;
            if now >= due_at_ms {
                reminder.status = ReminderStatus::Done;
                reminder.last_updated_ms = due_at_ms;
                changed.push(reminder.clone());
            }
        }
        changed
    }
}

/// Spawns a background thread that runs `sweep_completed` on a fixed
/// interval, invoking `on_completed` for each reminder newly transitioned to
/// `Done`. Structurally identical to `liveness::spawn_liveness_sweep`.
pub fn spawn_reminder_sweep(
    registry: ReminderRegistry,
    interval: Duration,
    on_completed: impl Fn(ReminderInfo) + Send + 'static,
) -> thread::JoinHandle<()> {
    thread::spawn(move || loop {
        thread::sleep(interval);
        for reminder in registry.sweep_completed() {
            on_completed(reminder);
        }
    })
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

    fn project() -> PathBuf {
        PathBuf::from("/tmp/project")
    }

    #[test]
    fn creating_a_reminder_registers_it_as_not_yet_started() {
        let registry = ReminderRegistry::new();

        let reminder = registry.create(project(), "Check the build".to_string(), 10).unwrap();

        assert_eq!(reminder.status, ReminderStatus::NotYetStarted);
        assert_eq!(reminder.run_started_ms, None);
        assert_eq!(registry.snapshot().len(), 1);
    }

    #[test]
    fn cap_reached_rejects_creation_and_leaves_existing_reminders_unchanged() {
        let registry = ReminderRegistry::new();
        for i in 0..MAX_REMINDERS_PER_PROJECT {
            registry.create(project(), format!("reminder-{i}"), 5).unwrap();
        }

        let rejected = registry.create(project(), "one too many".to_string(), 5);

        assert!(rejected.is_none(), "the 101st reminder for a project must be rejected");
        assert_eq!(registry.snapshot().len(), MAX_REMINDERS_PER_PROJECT);
    }

    #[test]
    fn cap_is_scoped_per_project() {
        let registry = ReminderRegistry::new();
        for i in 0..MAX_REMINDERS_PER_PROJECT {
            registry.create(project(), format!("reminder-{i}"), 5).unwrap();
        }

        let other_project = registry.create(PathBuf::from("/tmp/other"), "fine".to_string(), 5);

        assert!(
            other_project.is_some(),
            "a different project's cap must be independent"
        );
    }

    #[test]
    fn starting_a_never_run_reminder_sets_it_running() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 10).unwrap();

        let started = registry.start(&created.id).unwrap();

        assert_eq!(started.status, ReminderStatus::Running);
        assert!(started.run_started_ms.is_some());
    }

    #[test]
    fn re_running_a_done_reminder_resets_its_start_without_accumulating() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 10).unwrap();
        registry.start(&created.id);
        let first_run_started_ms = registry.get(&created.id).unwrap().run_started_ms;
        registry.stop(&created.id);

        std::thread::sleep(Duration::from_millis(5));
        let restarted = registry.start(&created.id).unwrap();

        assert_eq!(restarted.status, ReminderStatus::Running);
        assert!(
            restarted.run_started_ms > first_run_started_ms,
            "re-running must record a fresh start time, not accumulate from the prior run"
        );
    }

    #[test]
    fn stopping_a_running_reminder_transitions_to_done_with_no_notification_side_effect() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 10).unwrap();
        registry.start(&created.id);

        let stopped = registry.stop(&created.id).unwrap();

        assert_eq!(stopped.status, ReminderStatus::Done);
        assert!(stopped.last_updated_ms >= stopped.run_started_ms.unwrap());
    }

    #[test]
    fn stopping_a_reminder_that_is_not_running_is_a_no_op() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 10).unwrap();

        assert!(registry.stop(&created.id).is_none());
        assert_eq!(registry.get(&created.id).unwrap().status, ReminderStatus::NotYetStarted);
    }

    #[test]
    fn editing_a_reminder_that_is_not_running_updates_name_and_duration() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 10).unwrap();

        let updated = registry
            .update(&created.id, "Check the deploy".to_string(), 20)
            .unwrap();

        assert_eq!(updated.name, "Check the deploy");
        assert_eq!(updated.duration_minutes, 20);
        assert_eq!(updated.status, ReminderStatus::NotYetStarted);
    }

    #[test]
    fn editing_a_running_reminders_duration_changes_its_due_time_without_resetting_run_started_ms() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 10).unwrap();
        let started = registry.start(&created.id).unwrap();

        let updated = registry
            .update(&created.id, "Check the build".to_string(), 30)
            .unwrap();

        assert_eq!(updated.duration_minutes, 30);
        assert_eq!(
            updated.run_started_ms, started.run_started_ms,
            "editing duration must not reset the existing run's start time"
        );
    }

    #[test]
    fn deleting_a_running_reminder_removes_it_immediately() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 10).unwrap();
        registry.start(&created.id);

        let deleted = registry.delete(&created.id);

        assert!(deleted);
        assert!(registry.get(&created.id).is_none());
    }

    #[test]
    fn deleting_an_unknown_reminder_returns_false() {
        let registry = ReminderRegistry::new();

        assert!(!registry.delete(&ReminderId("unknown".to_string())));
    }

    #[test]
    fn sweep_transitions_a_reminder_whose_duration_has_elapsed() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 0).unwrap();
        registry.start(&created.id);

        let changed = registry.sweep_completed();

        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].status, ReminderStatus::Done);
        let done = registry.get(&created.id).unwrap();
        assert_eq!(
            done.last_updated_ms - done.run_started_ms.unwrap(),
            0,
            "a 0-minute reminder's elapsed time must equal its duration exactly"
        );
    }

    #[test]
    fn sweep_does_not_repeat_an_already_done_reminder() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 0).unwrap();
        registry.start(&created.id);

        let first_sweep = registry.sweep_completed();
        assert_eq!(first_sweep.len(), 1);

        let second_sweep = registry.sweep_completed();
        assert!(
            second_sweep.is_empty(),
            "an already-done reminder must not be reported again"
        );
    }

    #[test]
    fn sweep_ignores_a_reminder_whose_duration_has_not_yet_elapsed() {
        let registry = ReminderRegistry::new();
        let created = registry.create(project(), "Check the build".to_string(), 60).unwrap();
        registry.start(&created.id);

        let changed = registry.sweep_completed();

        assert!(changed.is_empty());
        assert_eq!(registry.get(&created.id).unwrap().status, ReminderStatus::Running);
    }
}
