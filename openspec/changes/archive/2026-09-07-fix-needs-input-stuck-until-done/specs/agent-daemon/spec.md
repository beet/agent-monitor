## MODIFIED Requirements

### Requirement: Resuming from needs input
The daemon SHALL treat a `PreToolUse` or `PostToolUse` hook event as a "running" status update, so a session currently in "needs input" status (for example, because of an unresolved permission or elicitation prompt) returns to "running" once Claude resumes work - not only when the user submits a new prompt via `UserPromptSubmit`, and not only on the next distinct tool invocation's `PreToolUse`. A `PostToolUse` event covers the case a `PreToolUse` event cannot: a tool call that itself raised the "needs input" prompt (or several, in sequence) has no further `PreToolUse` event until it completes, so `PostToolUse` is the signal that resumption happened as soon as that call finishes.

#### Scenario: Tool use resumes after a needs-input prompt is resolved
- **WHEN** a tracked session whose current status is "needs input" reports a `PreToolUse` event
- **THEN** the daemon updates that session's status to "running"

#### Scenario: A completed tool call resumes a session stuck on its own prompt
- **WHEN** a tracked session whose current status is "needs input" reports a `PostToolUse` event
- **THEN** the daemon updates that session's status to "running"

#### Scenario: A fresh session's first tool invocation is tracked as running
- **WHEN** the daemon receives a `PreToolUse` event for a session id it has not seen before
- **THEN** it registers a new agent entry with status "running"

#### Scenario: Tool use during an already-running session is a no-op status change
- **WHEN** the daemon receives a `PreToolUse` or `PostToolUse` event for a session whose current status is already "running"
- **THEN** the daemon updates the entry's last-updated timestamp and its status remains "running"
