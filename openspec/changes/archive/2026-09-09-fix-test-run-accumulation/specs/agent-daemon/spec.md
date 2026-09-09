## MODIFIED Requirements

### Requirement: Agents and test runs are grouped by working directory
The daemon SHALL group tracked agents and test runs by exact working directory match: each distinct working directory forms a group containing zero or more tracked agents (keyed by session id, as today) and at most one tracked test run. This grouping is additive to agent tracking - it SHALL NOT change how individual agent events are processed, keyed, or deduplicated. A test run SHALL be identified by its working directory alone, independent of any agent: reporting a new test-run event for a directory SHALL replace any previously tracked test run for that directory, regardless of process id, so a directory's test-run row always reflects only the most recently reported run rather than accumulating one entry per invocation.

#### Scenario: A test run in a directory with a tracked agent
- **WHEN** a test-run event's working directory exactly matches a directory with at least one tracked agent
- **THEN** the daemon records the test run in that directory's group alongside its tracked agent(s)

#### Scenario: A test run in a directory with no tracked agent
- **WHEN** a test-run event's working directory does not match any tracked agent's working directory
- **THEN** the daemon still records the test run, in a group with no agent members

#### Scenario: A directory with multiple tracked agents
- **WHEN** two or more agents share the same working directory (for example, two terminal tabs each running their own Claude Code session there)
- **THEN** a test-run event for that directory is recorded in the single group containing all of them, without the daemon selecting or favoring one agent over another

#### Scenario: Grouping does not affect agent identity
- **WHEN** the daemon processes an agent hook event
- **THEN** it applies the existing session-id-keyed registry behavior unchanged, independent of how many test runs share that agent's directory group

#### Scenario: A later test run replaces the previous one in the same directory
- **WHEN** the daemon receives a test-run event for a working directory that already has a tracked test run, reported by a different process id than the one currently tracked
- **THEN** the daemon replaces the tracked test run with the new one instead of tracking both, so repeated invocations in the same directory (for example, an edit/test loop) never accumulate more than one test-run row per directory
