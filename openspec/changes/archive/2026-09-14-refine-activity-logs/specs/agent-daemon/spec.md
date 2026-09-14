## MODIFIED Requirements

### Requirement: Agent registry
The daemon SHALL maintain a registry of known agents keyed by session identifier, recording working directory, host context (nvim, standalone terminal, or desktop app), process id, current status, last-updated timestamp, and a status-since timestamp marking when the agent most recently entered its current status. The registry SHALL reject a "needs input" event for a session whose current status is already "done", since a finished session cannot legitimately need input again until a new "running" event is reported for it. The registry SHALL hold at most one entry per pid: when an event's session id is new but its pid matches an existing entry under a different session id, the registry SHALL remove that existing entry before registering the new one, since the same pid starting a new session id (for example, via `/clear`) means the old session is gone, not merely quiet. The status-since timestamp SHALL only be set to the current time when an update actually changes the agent's status; an event or internal transition (e.g. marking an agent stale) that leaves the status unchanged SHALL leave the status-since timestamp untouched. Replacing a pid's entry this way SHALL permanently supersede the session id it replaced: a later event that names that superseded session id SHALL be treated as unknown rather than as a session the registry has not seen before, so it cannot be re-registered as a new entry and, by the same-pid rule above, cannot evict the live session that replaced it.

#### Scenario: First event for a session registers a new agent
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid does not match any existing entry
- **THEN** it creates a new registry entry with the reported working directory, host context, pid, and status, and sets both the last-updated and status-since timestamps to the current time

#### Scenario: Subsequent event updates the existing agent
- **WHEN** the daemon receives an event for a session id already in the registry
- **THEN** it updates that entry's status and last-updated timestamp instead of creating a duplicate

#### Scenario: A same-status event does not reset the status-since timestamp
- **WHEN** the daemon receives an event for a session already in the registry and the event's status matches the agent's current status
- **THEN** the daemon updates the last-updated timestamp but leaves the status-since timestamp unchanged

#### Scenario: A status transition resets the status-since timestamp
- **WHEN** the daemon receives an event for a session already in the registry and the event's status differs from the agent's current status
- **THEN** the daemon sets the status-since timestamp to the current time in addition to updating the status and last-updated timestamp

#### Scenario: A late needs-input event cannot override a completed session
- **WHEN** the daemon receives a "needs input" event for a session whose current status is already "done"
- **THEN** the daemon leaves that agent's status as "done" and does not apply the "needs input" event, and does not change its status-since timestamp

#### Scenario: A new running event still clears a completed session's status
- **WHEN** the daemon receives a "running" event for a session whose current status is "done"
- **THEN** it updates that entry's status to "running" as normal and resets the status-since timestamp to the current time

#### Scenario: A new session id for an already-tracked pid replaces the old entry
- **WHEN** the daemon receives an event for a session id it has not seen before, and its pid matches an existing entry registered under a different session id
- **THEN** the daemon removes the existing entry for that pid and registers the new session id as the sole entry for that pid, instead of the two entries coexisting

#### Scenario: A late event for an already-superseded session id is ignored
- **WHEN** the daemon receives an event naming a session id that was previously removed because another session id took over its pid, and that replacement session id is still the pid's tracked entry
- **THEN** the daemon does not register the superseded session id as a new entry and does not remove or otherwise alter the pid's current entry

#### Scenario: A late event for a superseded session id whose pid has since moved on again
- **WHEN** the daemon receives an event naming a session id that was superseded, and the pid it belonged to is now tracked under yet another, different session id
- **THEN** the daemon still does not register the superseded session id as a new entry, and the pid's currently tracked entry is unaffected
