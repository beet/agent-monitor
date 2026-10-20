# project-renaming Specification

## Purpose

Lets the user attach a custom display name to a project (its working directory), overriding the directory-derived default name everywhere the daemon or its clients refer to that project.

## Requirements

### Requirement: Custom project display name

The daemon SHALL allow a client to assign a custom display name to a project, identified by its working directory. Once assigned, the custom name SHALL be used as that project's display name everywhere the daemon or a connected client refers to it, in place of the directory-derived default name, until the project is renamed again or the daemon restarts. Resubmitting a project's own current display name (whether its custom name or, if never renamed, its default) SHALL succeed as a no-op. This includes desktop notifications the daemon sends about the project, not only client-rendered views.

#### Scenario: A project with no custom name uses its directory-derived default

- **WHEN** a project has never been renamed
- **THEN** its display name is derived from its working directory, as it always has been

#### Scenario: A rename replaces the default name

- **WHEN** a client assigns a custom name to a project
- **THEN** every subsequent reference to that project's display name uses the custom name instead of its directory-derived default

#### Scenario: Renaming again replaces the previous custom name

- **WHEN** a client assigns a new custom name to a project that already has one
- **THEN** the project's display name becomes the new name, and the previous custom name is no longer used anywhere

#### Scenario: Resubmitting the current name is a no-op

- **WHEN** a client assigns a project the display name it already currently has
- **THEN** the daemon accepts it without changing anything observable

#### Scenario: Desktop notifications use the effective display name

- **WHEN** the daemon sends a desktop notification about a project (an agent status change, a test-run start/pass/fail, or a reminder firing)
- **THEN** the notification text names the project using its effective display name (its custom name if one has been assigned, otherwise its directory-derived default), not the raw working-directory path

### Requirement: Rename validation

A requested display name SHALL be rejected if, after trimming leading and trailing whitespace, it is empty, or if it matches - exactly, case-sensitively - the effective display name (custom name if renamed, otherwise directory-derived default) of any other project the daemon currently knows about. "Any other project the daemon currently knows about" includes every project with a tracked agent, test run, log entry, or reminder, as well as every project that has ever been assigned a custom name, regardless of whether it currently has any of those. A rejected rename SHALL NOT change the project's existing display name.

#### Scenario: A blank name is rejected

- **WHEN** a client submits an empty name, or a name that is entirely whitespace, for a project
- **THEN** the daemon rejects the rename and the project's display name is unchanged

#### Scenario: A name colliding with another project's default name is rejected

- **WHEN** a client submits a name for a project that exactly matches another, different project's directory-derived default name
- **THEN** the daemon rejects the rename and the project's display name is unchanged

#### Scenario: A name colliding with another project's custom name is rejected

- **WHEN** a client submits a name for a project that exactly matches another, different project's current custom name
- **THEN** the daemon rejects the rename and the project's display name is unchanged

#### Scenario: A collision with a project known only from past activity is still rejected

- **WHEN** a client submits a name that matches the effective display name of a project that has no currently tracked agent, test run, or reminder, but does have either a past activity-log entry or a previously assigned custom name
- **THEN** the daemon rejects the rename and the project's display name is unchanged

#### Scenario: A name is not rejected for colliding with the project's own current name

- **WHEN** a client submits a name for a project that matches that same project's own current effective display name
- **THEN** the daemon accepts the rename as a no-op, per "Resubmitting the current name is a no-op"

### Requirement: Renames are visible to every connected client

Once a rename is accepted, the daemon SHALL make the project's new display name visible to every currently-connected client without requiring it to reconnect, and SHALL include every project's current custom name in the initial state sent to a client connecting afterward.

#### Scenario: An already-connected client sees a rename applied elsewhere

- **WHEN** a rename is accepted while another client is already connected
- **THEN** that already-connected client's view of the project's display name updates without reconnecting

#### Scenario: A newly-connecting client sees previously-applied renames

- **WHEN** a client connects to the daemon after a project has already been renamed
- **THEN** the project's display name, as seen by the newly-connected client, is the custom name, not the directory-derived default

### Requirement: Custom names do not persist across a daemon restart

Custom project display names SHALL be held only for the daemon's process lifetime, consistent with every other piece of daemon state (tracked agents, test runs, the activity log, reminders). A daemon restart SHALL clear every custom name, after which every project's display name reverts to its directory-derived default.

#### Scenario: A daemon restart clears all custom names

- **WHEN** the daemon process restarts
- **THEN** no previously assigned custom name is present afterward, and every project's display name is its directory-derived default, the same as tracked agents, test runs, and reminders
