## ADDED Requirements

### Requirement: Stop events carry the agent's final response text
When a Claude Code `Stop` hook fires, the reporting client SHALL read the `last_assistant_message` field from the hook's payload, if present and non-blank after trimming, and include it as the reported event's response text. Response text longer than 4,000 characters SHALL be truncated to that length with a trailing truncation marker, the same bound applied to last-prompt text, so one event cannot carry unbounded text. Any other hook event (not `Stop`) SHALL always report no response text, regardless of whether its payload happens to include a `last_assistant_message` field. A `Stop` event whose payload carries no `last_assistant_message` field, or whose value is empty or only whitespace, SHALL report no response text rather than an error or placeholder text.

#### Scenario: A Stop event's payload yields a response
- **WHEN** a `Stop` hook fires with a non-blank `last_assistant_message` field in its payload
- **THEN** the reported event's response text is set to that field's value (trimmed)

#### Scenario: A missing response field reports no response
- **WHEN** a `Stop` hook's payload carries no `last_assistant_message` field
- **THEN** the reported event carries no response text, and the hook completes normally

#### Scenario: A blank response field reports no response
- **WHEN** a `Stop` hook's payload carries a `last_assistant_message` field that is empty or only whitespace
- **THEN** the reported event carries no response text

#### Scenario: Non-Stop events never carry a response
- **WHEN** any hook event other than `Stop` fires (e.g. `UserPromptSubmit`, `PreToolUse`, `Notification`)
- **THEN** the reported event carries no response text, regardless of whether its payload happens to include a `last_assistant_message` field

#### Scenario: An overlong response is truncated
- **WHEN** a `Stop` event's `last_assistant_message` field exceeds 4,000 characters
- **THEN** the reported event's response text is truncated to 4,000 characters with a trailing truncation marker
