## MODIFIED Requirements

### Requirement: Reminder identity and lifecycle
The daemon SHALL track reminders scoped to a project (working directory), each identified by the daemon, holding a name, a duration in minutes, and a lifecycle status of not-yet-started, running, or done. A newly created reminder SHALL start in the not-yet-started status. Reminders SHALL be held only for the daemon's process lifetime, consistent with how tracked agents, test runs, and the activity log already work; none of them persist across a daemon restart. A reminder's duration SHALL always be a positive number of minutes; the daemon SHALL reject a request to create a reminder whose duration is zero, and that project's existing reminders SHALL be unchanged.

#### Scenario: Creating a reminder registers it as not-yet-started
- **WHEN** a client creates a reminder for a project with a positive duration
- **THEN** the daemon registers it with not-yet-started status and no elapsed run

#### Scenario: A daemon restart clears all reminders
- **WHEN** the daemon process restarts
- **THEN** no previously tracked reminder is present afterward, the same as tracked agents and test runs

#### Scenario: Creating a reminder with a zero duration is rejected
- **WHEN** a client requests creating a reminder for a project with a duration of zero minutes
- **THEN** the daemon rejects the request and that project's existing reminders are unchanged

### Requirement: Editing a reminder
A client SHALL be able to change a reminder's name, its duration, or both, regardless of its current status. An edit SHALL take effect immediately: if the reminder is running, a changed duration SHALL change when it is next due to complete, measured from its current run's start. Editing SHALL NOT itself send a notification. A reminder's duration SHALL always be a positive number of minutes; the daemon SHALL reject a request to edit a reminder's duration to zero, leaving that reminder unchanged.

#### Scenario: Editing a reminder that is not running
- **WHEN** a client edits the name or duration of a not-yet-started or done reminder to a positive duration
- **THEN** the daemon applies the change and the reminder's status is unaffected

#### Scenario: Editing a running reminder's duration
- **WHEN** a client edits the duration of a reminder that is currently running to a positive duration
- **THEN** the daemon applies the new duration immediately, changing when that run is due to complete, measured from its existing start time

#### Scenario: Editing a reminder's duration to zero is rejected
- **WHEN** a client requests editing a reminder's duration to zero minutes
- **THEN** the daemon rejects the request and that reminder is unchanged
