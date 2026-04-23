---
spec: watch.spec.md
---

## Context

The watch module was added in v0.11.0 to turn fledge from a manually-invoked CLI into a persistent development companion. It uses the `notify` crate for cross-platform filesystem event monitoring and supports both tasks and lanes as re-run targets.

The debounce implementation extends the deadline on each new event rather than using a fixed window, which handles rapid save bursts (e.g., formatter running after save) without triggering multiple runs. Default debounce was increased from 200ms to 500ms based on real-world testing.
