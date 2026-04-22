#!/usr/bin/env bash
# Non-interactive protocol test — only uses messages that don't need user input.
# Tests: log, output, progress, store, load, exec, metadata.
set -euo pipefail

# Save original stdout (fd 1 goes to fledge) so subshells can write to it.
exec 3>&1

read -r INIT_MSG
echo "init received" >&2

send() { printf '%s\n' "$1" >&3; }

# Send a request and read fledge's response. Writes the request on fd 3
# (the real stdout) and reads the JSON response from stdin.
request() {
    printf '%s\n' "$1" >&3
    read -r REPLY_LINE
    echo "$REPLY_LINE"
}

# Log all levels
send '{"type":"log","level":"debug","message":"debug test"}'
send '{"type":"log","level":"info","message":"info test"}'
send '{"type":"log","level":"warn","message":"warn test"}'
send '{"type":"log","level":"error","message":"error test"}'

# Output
send '{"type":"output","text":"hello from plugin\n"}'

# Progress bar
send '{"type":"progress","message":"Working","current":1,"total":3}'
send '{"type":"progress","message":"Working","current":2,"total":3}'
send '{"type":"progress","message":"Working","current":3,"total":3}'
send '{"type":"progress","done":true}'

# Spinner
send '{"type":"progress","message":"Thinking"}'
send '{"type":"progress","done":true}'

# Store and load
send '{"type":"store","key":"test_key","value":"test_value"}'
RESP=$(request '{"type":"load","id":"1","key":"test_key"}')
echo "load response: $RESP" >&2

# Exec
RESP=$(request '{"type":"exec","id":"2","command":"echo protocol-ok","timeout":5}')
echo "exec response: $RESP" >&2

# Metadata
RESP=$(request '{"type":"metadata","id":"3","keys":["git_tags"]}')
echo "metadata response: $RESP" >&2

send '{"type":"output","text":"all done\n"}'
send '{"type":"log","level":"info","message":"test complete"}'
