## MODIFIED Requirements

### Requirement: Status is visually distinguishable
The TUI SHALL visually distinguish agent statuses (e.g. running, idle, needs input, done, stale, declined) from one another so the user can scan the list and immediately identify agents needing attention. Each status SHALL be prefixed with a distinct emoji marker in addition to any color/style distinction: running with 🔧, idle with 💤, needs input with 🔔, done with ✅, stale with 🕸️, and declined with 🚫.

#### Scenario: An agent needs input
- **WHEN** an agent's status is "needs input"
- **THEN** that row displays the 🔔 marker and is visually distinguished (e.g. color) from rows in other states, using bold colored text rather than a solid background fill

#### Scenario: Each status has a distinct emoji marker
- **WHEN** the TUI renders a row for an agent
- **THEN** the status cell is prefixed with the emoji for that status (🔧 running, 💤 idle, 🔔 needs input, ✅ done, 🕸️ stale, 🚫 declined)

#### Scenario: A declined permission is visually distinguished
- **WHEN** an agent's status is "declined"
- **THEN** that row displays the 🚫 marker and is visually distinguished (e.g. color) from rows in other states
