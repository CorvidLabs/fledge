---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
module: llm
---

## ADDED

### REQUIREMENT REQ-llm-020

The `llm` module's Ollama provider SHALL be exercisable against a loopback endpoint so
its request shape and error mapping are covered without contacting a real host.

Acceptance Criteria
- The request body carries `model`, `prompt` and `stream: false`.
- A configured API key is sent as a Bearer header; absent a key, no auth header is sent.
- HTTP status errors, undecodable bodies and connection refusal each map to a distinct, non-panicking error.
