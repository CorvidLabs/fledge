# fledge-hello

Example plugin demonstrating the **fledge-v1** protocol. Exercises every message type in a single interactive walkthrough.

## Message types demonstrated

| # | Type | Direction | What happens |
|---|------|-----------|-------------|
| 1 | `log` | plugin → fledge | Colored structured logging |
| 2 | `output` | plugin → fledge | Raw text passthrough |
| 3 | `prompt` | plugin → fledge → plugin | Ask for text input with validation |
| 4 | `confirm` | plugin → fledge → plugin | Yes/no dialog |
| 5 | `select` | plugin → fledge → plugin | Pick one from a list |
| 6 | `multi_select` | plugin → fledge → plugin | Pick multiple from a list |
| 7 | `progress` | plugin → fledge | Determinate progress bar |
| 8 | `store` / `load` | plugin → fledge | Key-value persistence roundtrip |
| 9 | `exec` | plugin → fledge → plugin | Sandboxed shell command |
| 10 | `metadata` | plugin → fledge → plugin | Project context query |
| 11 | `progress` (spinner) | plugin → fledge | Indeterminate spinner |

## Running it

```bash
# From the fledge repo root:
cargo run -- plugin install ./examples/fledge-hello
fledge hello
```

Or test the script directly (it reads JSON from stdin, writes JSON to stdout):

```bash
echo '{"type":"init","protocol":"fledge-v1","args":[],"project":null,"plugin":{"name":"fledge-hello","version":"0.1.0","dir":"/tmp"},"fledge":{"version":"0.9.0"}}' | ./examples/fledge-hello/hello.sh
```

## Writing your own plugin

1. Create a directory with a `plugin.toml` manifest
2. Set `protocol = "fledge-v1"` to opt into structured IPC
3. Your binary reads the `init` message from stdin (one JSON line)
4. Send messages to stdout as JSON lines — fledge handles UI rendering
5. For request messages (`prompt`, `confirm`, `select`, `multi_select`, `load`, `exec`, `metadata`), read the response from stdin
6. Exit with code 0 on success, non-zero on failure

Any language works — the protocol is just newline-delimited JSON over stdio.
