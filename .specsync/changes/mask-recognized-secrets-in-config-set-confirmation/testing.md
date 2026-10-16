---
change: mask-recognized-secrets-in-config-set-confirmation
artifact: testing
---

# Testing

## Actual prior evidence

The worker executed these through verification_gate.py F1 and f1_env.py. This metadata preparation does not rerun tests/builds or create lifecycle verification/acceptance records.

| Receipt | Actual result |
|---|---|
| 20261016T070322Z-F1-cfcda0 | Native Fledge offline locked config_secret_output target; gate1/cargo101; seven executed: five real raw-secret-marker failures and two controls pass. |
| 20261016T070640Z-F1-9eb793 | Same target after patch; gate/envelope0; seven pass/zero fail. Passing cases prove exact fake values persist plus masked Get/List. |
| 20261016T070813Z-F1-013eed | pre-commit exit0; fmt/lint, test and spec-check steps success. Envelope omits full-suite count. |
| 20261016T071041Z-F1-cd1d38 | verify-native exit0; all five configured steps success. Three pre-existing recursive self-check skips unchanged. No complete test total exposed. |
| 20261016T070751Z-F1-7ecce5 | SpecSync6.1.1 strict forced coverage exit0:33specs; zero warnings/failures;66/66files and36020/36020LOC. Not lifecycle acceptance. |
| 20261016T070751Z-F1-13c77e | Checkout debug binary main spec lint exit0/version17/zero findings; model pass not requested. Earlier installed1.7.2 unsupported lint exit2. |
| 20261016T071010Z-F1-b88f79 | Cached cargo audit --no-fetch --no-yanked --json exit1:11vulnerabilities and two unmaintained warnings on unchanged Cargo files. No ignore or upgrade. |

## Fixture safety and sensitivity

Tests run the actual Cargo-provided binary, separate TempDirs for HOME/XDG/config/cwd and relevant credential removals, with distinct synthetic markers. They assert exit status/both streams, exact owned TOML persistence, masked Get/List, visible non-secret value and unknown-key non-write. Deprecated Claude's absent List row is disclosed.

Recorded wrapper uses task-local operator/plugin directories, offline cached toolchains and an OS profile denying non-loopback outbound traffic. This is not filesystem confinement or a ban on every loopback service. Config tests require no real credentials, model or live API.

## Pending

Actual cached Trust/risk/provenance, exact nonempty candidate range, live ownership, lifecycle approval/acceptance and hosted checks remain pending. Evidence does not prove argv/history protection, comprehensive credential security, dependency acceptance or release readiness.
