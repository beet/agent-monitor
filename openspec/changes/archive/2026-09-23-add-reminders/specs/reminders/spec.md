## Purpose

Tracks named, duration-based reminders scoped to a project so the user can be pulled back to check on it after an elapsed time, independent of agent or test-run activity.

## ADDED Requirements

### Requirement: Reminder identity and lifecycle
The daemon SHALL track reminders scoped to a project (working directory), each identified by the daemon, holding a name, a duration in minutes, and a lifecycle status of not-yet-started, running, or done. A newly created reminder SHALL start in the not-yet-started status. Reminders SHALL be held only for the daemon's process lifetime, consistent with how tracked agents, test runs, and the activity log already work; none of them persist across a daemon restart.

#### Scenario: Creating a reminder registers it as not-yet-started
- **WHEN** a client creates a reminder for a project
- **THEN** the daemon registers it with not-yet-started status and no elapsed run

#### Scenario: A daemon restart clears all reminders
- **WHEN** the daemon process restarts
- **THEN** no previously tracked reminder is present afterward, the same as tracked agents and test runs

### Requirement: Per-project reminder cap
The daemon SHALL track at most 100 reminders per project. Creating a reminder for a project that already has 100 SHALL be rejected, and SHALL NOT remove or alter any of that project's existing reminders.

#### Scenario: Cap reached
- **WHEN** a client requests creating a 101st reminder for a project that already has 100
- **THEN** the daemon rejects the request and that project's existing 100 reminders are unchanged

### Requirement: Starting and stopping a reminder
Starting a reminder - whether it has never run, or has already completed a previous run - SHALL set its status to running and record the current time as the start of this run, independent of any previous run's timing. Stopping a running reminder SHALL set its status to done and compute the elapsed time of this run as the time between its start and the stop. Stopping SHALL NOT send a notification.

#### Scenario: Starting a reminder that has never run
- **WHEN** a client starts a not-yet-started reminder
- **THEN** the daemon sets its status to running and records the current time as this run's start

#### Scenario: Re-running a done reminder
- **WHEN** a client starts a reminder that is already done
- **THEN** the daemon sets its status to running and records the current time as this run's start, independent of its previous run's duration

#### Scenario: Stopping a running reminder
- **WHEN** a client stops a reminder whose status is running
- **THEN** the daemon sets its status to done, computes the elapsed time from this run's start to now, and sends no notification

### Requirement: A reminder completes automatically when its duration elapses
The daemon SHALL monitor every running reminder and, once the elapsed time since its current run's start reaches its duration, transition it to done and compute its elapsed time as that duration - independent of whether any client is connected at that moment.

#### Scenario: A reminder finishes with no client connected
- **WHEN** a running reminder's elapsed time reaches its duration while no client is connected to the daemon
- **THEN** the daemon still transitions it to done

#### Scenario: A reminder finishes while a client is connected
- **WHEN** a running reminder's elapsed time reaches its duration while a client is connected
- **THEN** the daemon transitions it to done and pushes the updated reminder to that client without requiring it to reconnect

### Requirement: Reminder completion notification
The daemon SHALL send a macOS user notification identifying the project and the reminder's name when a reminder completes on its own (its elapsed time reaches its duration), using a system sound distinct from the sounds used for agent and test-run notifications. Stopping a reminder manually SHALL NOT trigger this notification.

#### Scenario: A reminder finishes on its own
- **WHEN** a running reminder's elapsed time reaches its duration
- **THEN** the daemon sends a macOS notification identifying the project and the reminder's name

#### Scenario: Manually stopping does not notify
- **WHEN** a client stops a running reminder before its duration has elapsed
- **THEN** the daemon does not send a macOS notification for that reminder

### Requirement: Editing a reminder
A client SHALL be able to change a reminder's name, its duration, or both, regardless of its current status. An edit SHALL take effect immediately: if the reminder is running, a changed duration SHALL change when it is next due to complete, measured from its current run's start. Editing SHALL NOT itself send a notification.

#### Scenario: Editing a reminder that is not running
- **WHEN** a client edits the name or duration of a not-yet-started or done reminder
- **THEN** the daemon applies the change and the reminder's status is unaffected

#### Scenario: Editing a running reminder's duration
- **WHEN** a client edits the duration of a reminder that is currently running
- **THEN** the daemon applies the new duration immediately, changing when that run is due to complete, measured from its existing start time

### Requirement: Deleting a reminder
A client SHALL be able to delete a reminder regardless of its current status. Deleting a reminder SHALL remove it from the daemon's tracked reminders immediately, without first requiring it to be stopped, and SHALL NOT affect any activity log entry already recorded for it.

#### Scenario: Deleting a running reminder
- **WHEN** a client deletes a reminder that is currently running
- **THEN** the daemon removes it from its tracked reminders immediately, without completing or stopping it as a separate step

#### Scenario: A deleted reminder's history is unaffected
- **WHEN** a reminder that has previously logged activity is deleted
- **THEN** its existing activity log entries are unchanged

### Requirement: Reminder query and live updates
The daemon SHALL let connected clients retrieve every project's current reminders and receive updates as they are created, edited, started, stopped, completed, or deleted, without polling being the only option - the same live-update model already used for agents and test runs.

#### Scenario: Client requests current reminders on connect
- **WHEN** a client connects to the daemon
- **THEN** the daemon's snapshot to that client includes every currently tracked reminder, across all projects

#### Scenario: Client receives incremental reminder updates
- **WHEN** a reminder is created, edited, started, stopped, or completes while a client is connected
- **THEN** the daemon pushes that reminder's updated state to the connected client without requiring it to reconnect

#### Scenario: Client receives a reminder removal
- **WHEN** a reminder is deleted while a client is connected
- **THEN** the daemon pushes a removal for that reminder to the connected client
