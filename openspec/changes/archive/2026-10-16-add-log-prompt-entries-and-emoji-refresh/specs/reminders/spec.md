## MODIFIED Requirements

### Requirement: Reminder completion notification
The daemon SHALL send a macOS user notification identifying the project and the reminder's name when a reminder completes on its own (its elapsed time reaches its duration), using a system sound distinct from the sounds used for agent and test-run notifications. Stopping a reminder manually SHALL NOT trigger this notification. The notification body SHALL be prefixed with ⏰, so the notification is identifiable as a reminder at a glance before reading its text.

#### Scenario: A reminder finishes on its own
- **WHEN** a running reminder's elapsed time reaches its duration
- **THEN** the daemon sends a macOS notification identifying the project and the reminder's name, with its body prefixed by ⏰

#### Scenario: Manually stopping does not notify
- **WHEN** a client stops a running reminder before its duration has elapsed
- **THEN** the daemon does not send a macOS notification for that reminder
