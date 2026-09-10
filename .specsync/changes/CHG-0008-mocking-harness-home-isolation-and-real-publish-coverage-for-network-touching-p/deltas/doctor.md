---
change: CHG-0008-mocking-harness-home-isolation-and-real-publish-coverage-for-network-touching-p
module: doctor
---

## ADDED

### REQUIREMENT REQ-doctor-020

The `doctor` module's provider reachability probe SHALL be testable against a loopback
endpoint, and doctor's CLI tests SHALL run under an isolated environment rather than the
developer's real configuration.

Acceptance Criteria
- `probe_ollama_host` returns true for a reachable loopback endpoint and false for a closed port.
- Doctor CLI tests execute with `HOME`, `XDG_CONFIG_HOME` and `FLEDGE_CONFIG_DIR` pointed at tempdirs.
- No doctor test contacts a non-loopback host.
