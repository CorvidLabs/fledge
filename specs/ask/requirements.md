---
spec: ask.spec.md
---

## User Stories

- As a developer, I want to run `fledge ask how does X work` to get an AI-powered answer about my codebase
- As a developer, I want to type the question naturally without needing quotes

## Acceptance Criteria

### REQ-ask-001

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- `fledge ask <words...>` joins all trailing arguments into a single question
### REQ-ask-002

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- The question is sent to Claude CLI running in the current project directory
### REQ-ask-003

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- The response is streamed to stdout
### REQ-ask-004

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- Missing Claude CLI produces install instructions
### REQ-ask-005

The implementation SHALL satisfy this requirement.

Acceptance Criteria

- No question provided produces a usage hint

## Constraints

- Requires Claude CLI (`claude`) installed and authenticated
- Output is streamed from the Claude process

## Out of Scope

- Conversation history or follow-up questions
- Specifying model or temperature
- File context selection
