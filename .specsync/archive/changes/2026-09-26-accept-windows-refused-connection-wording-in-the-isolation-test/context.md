---
change: accept-windows-refused-connection-wording-in-the-isolation-test
artifact: context
---

# Context

## What led here

The v1.8.0 release PR (#532) went red on exactly one cell: `test (windows-latest)`,
in `tests/isolation.rs::default_temp_env_points_github_at_a_dead_port`:

```text
expected a refused local connection, got: error: searching github for template repos:
github api request failed: io: no connection could be made because the target machine
actively refused it. (os error 10061)
```

The same test is green on Windows on `main`. The difference is the lockfile: the release
bump moves `ureq` from 3.3.0 to 3.4.2. In 3.3.0, when every resolved address refused,
`ureq`'s TCP connector threw the OS error away and returned a synthesized
`io::Error::new(ConnectionRefused, "Connection refused")`, so every platform printed the
same English words. 3.4 keeps the last per-address error instead (`last_err` in
`unversioned/transport/tcp.rs`), so the message is now the OS's own:

| Platform | stderr on ureq 3.4.2 |
|---|---|
| Linux | `io: Connection refused (os error 111)` |
| macOS | `io: Connection refused (os error 61)` (observed locally) |
| Windows | `io: No connection could be made because the target machine actively refused it. (os error 10061)` (from the CI log) |

The test's two alternatives were `connection refused` and `127.0.0.1`. The Windows text
has neither, and `ureq` never puts the address in the message, so the test failed.
Nothing in fledge's behavior changed: the request was refused locally, as intended.

## Decisions

**Widen the wording, keep both halves of the proof.** The test exists to rule out a real
request to api.github.com. It does that in two halves, and both are kept unchanged in
strength: the base handed to the child starts with `http://127.0.0.1:`, and the failure is
a refusal. Only the set of words that count as "a refusal" grows, by the two stable parts
of WSAECONNREFUSED's message: `actively refused` and `os error 10061`. A generic
`contains("failed")` stays excluded, because a genuine api.github.com failure satisfies it.

**Match the message, not the error kind.** The test sees a spawned binary's stderr, not
an `io::Error`, so `ErrorKind::ConnectionRefused` is not observable here. Matching the OS
text is the only option at this boundary.

**Land on `main` first.** `main` is green today only because it still pins ureq 3.3.0.
Any lockfile refresh, the release's included, brings in 3.4 and the failure. Fixing it
on `main` lets the release PR rebase onto a test that is already correct, instead of
carrying a test change inside a release commit.

## Out of scope

- Dropping or skipping the Windows matrix cell. corvid-agent offered it as an
  alternative. It would hide the signal instead of fixing a test that is wrong.
- The `127.0.0.1` alternative in the stderr assertion. It predates this change and is
  unreachable with ureq, but removing it is a separate tightening.
