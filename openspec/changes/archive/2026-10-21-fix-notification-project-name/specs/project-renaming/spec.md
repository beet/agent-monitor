## MODIFIED Requirements

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
