---
module: ask
version: 1
status: active
files:
  - src/ask.rs

db_tables: []
depends_on: []
---

# Ask

## Purpose

Ask questions about your codebase using AI. Sends the question to Claude CLI which has access to the project context, and streams the answer back.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `run` | Entry point for the ask command |
| `AskOptions` | Options struct with the question text |

### Structs & Enums

| Type | Description |
|------|-------------|
| `AskOptions` | `{ question: String }` |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `run` | `(AskOptions) -> Result<()>` | Sends question to Claude CLI and displays answer |

## Invariants

1. Requires Claude CLI (`claude`) to be installed and authenticated
2. Question is joined from multiple args (no quotes required)
3. Claude CLI runs in the current project directory for context

## Behavioral Examples

### ask — simple question
```
$ fledge ask how does template rendering work
● Thinking...

[AI response streamed here]
```

### ask — no question
```
$ fledge ask
error: Please provide a question. Usage: fledge ask <question>
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| Claude CLI not installed | `claude --version` fails | Bail with install instructions |
| No question provided | Empty args | Bail with usage hint |
| Claude CLI error | Non-zero exit | Bail with error |

## Dependencies

- Claude CLI — AI inference (external dependency)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-04-19 | Initial spec |
