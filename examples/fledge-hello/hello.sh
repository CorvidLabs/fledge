#!/usr/bin/env bash
#
# fledge-hello: example plugin exercising every fledge-v1 protocol message.
# Communicates via JSON-lines on stdin (fledge → plugin) and stdout (plugin → fledge).
# Stderr goes directly to the terminal for debug output.

set -euo pipefail

# Save real stdout (piped to fledge) so subshells can use it.
exec 3>&1

MSG_ID=0
next_id() { MSG_ID=$((MSG_ID + 1)); echo "$MSG_ID"; }

# Send a one-way message to fledge.
send() { printf '%s\n' "$1" >&3; }

# Send a request and read fledge's response.
request() {
    printf '%s\n' "$1" >&3
    read -r REPLY_LINE
    echo "$REPLY_LINE"
}

# ── Step 0: Read the init message ──────────────────────────────────────────
read -r INIT_MSG
echo "init received" >&2

PROJECT_NAME=$(echo "$INIT_MSG" | sed -n 's/.*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)

# ── Step 1: Log ────────────────────────────────────────────────────────────
send '{"type":"log","level":"info","message":"fledge-hello plugin started"}'

# ── Step 2: Output ─────────────────────────────────────────────────────────
send '{"type":"output","text":"\n  Welcome to fledge-hello!\n  This plugin demonstrates every fledge-v1 protocol message.\n\n"}'

# ── Step 3: Prompt — ask for a name ────────────────────────────────────────
ID=$(next_id)
RESP=$(request "{\"type\":\"prompt\",\"id\":\"$ID\",\"message\":\"What is your name?\",\"default\":\"world\",\"validate\":\"non_empty\"}")
NAME=$(echo "$RESP" | sed -n 's/.*"value"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')

send "{\"type\":\"output\",\"text\":\"  Hello, ${NAME}!\n\n\"}"

# ── Step 4: Confirm ────────────────────────────────────────────────────────
ID=$(next_id)
RESP=$(request "{\"type\":\"confirm\",\"id\":\"$ID\",\"message\":\"Run the full demo?\",\"default\":true}")

if echo "$RESP" | grep -q '"value"[[:space:]]*:[[:space:]]*false'; then
    send '{"type":"output","text":"  Okay, exiting early. Bye!\n"}'
    exit 0
fi

# ── Step 5: Select ─────────────────────────────────────────────────────────
ID=$(next_id)
RESP=$(request "{\"type\":\"select\",\"id\":\"$ID\",\"message\":\"Pick a color:\",\"options\":[\"red\",\"green\",\"blue\"],\"default\":1}")
COLOR=$(echo "$RESP" | sed -n 's/.*"value"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
send "{\"type\":\"log\",\"level\":\"info\",\"message\":\"You picked: ${COLOR}\"}"

# ── Step 6: Multi-select ──────────────────────────────────────────────────
ID=$(next_id)
RESP=$(request "{\"type\":\"multi_select\",\"id\":\"$ID\",\"message\":\"Select toppings:\",\"options\":[\"cheese\",\"pepperoni\",\"mushrooms\",\"olives\"],\"defaults\":[0,1]}")
send '{"type":"log","level":"info","message":"Toppings selected"}'

# ── Step 7: Progress bar ──────────────────────────────────────────────────
send '{"type":"output","text":"\n"}'
TOTAL=5
for i in $(seq 1 $TOTAL); do
    send "{\"type\":\"progress\",\"message\":\"Baking pizza\",\"current\":$i,\"total\":$TOTAL}"
    sleep 0.3
done
send '{"type":"progress","done":true}'

# ── Step 8: Store and Load ─────────────────────────────────────────────────
send "{\"type\":\"store\",\"key\":\"last_user\",\"value\":\"${NAME}\"}"
send "{\"type\":\"store\",\"key\":\"favorite_color\",\"value\":\"${COLOR}\"}"

ID=$(next_id)
RESP=$(request "{\"type\":\"load\",\"id\":\"$ID\",\"key\":\"last_user\"}")
LOADED=$(echo "$RESP" | sed -n 's/.*"value"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
send "{\"type\":\"log\",\"level\":\"debug\",\"message\":\"Store/load roundtrip: stored '${NAME}', loaded '${LOADED}'\"}"

# ── Step 9: Exec — run a shell command ─────────────────────────────────────
ID=$(next_id)
RESP=$(request "{\"type\":\"exec\",\"id\":\"$ID\",\"command\":\"date +%Y-%m-%d\",\"timeout\":5}")
DATE=$(echo "$RESP" | sed -n 's/.*"stdout"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | tr -d '\\n')
send "{\"type\":\"log\",\"level\":\"info\",\"message\":\"Today is ${DATE}\"}"

# ── Step 10: Metadata — query project info ─────────────────────────────────
ID=$(next_id)
RESP=$(request "{\"type\":\"metadata\",\"id\":\"$ID\",\"keys\":[\"git_tags\",\"fledge_config\"]}")
send "{\"type\":\"log\",\"level\":\"debug\",\"message\":\"Metadata response received\"}"

# ── Step 11: Spinner progress ─────────────────────────────────────────────
send '{"type":"progress","message":"Finishing up"}'
sleep 1
send '{"type":"progress","done":true}'

# ── Done ──────────────────────────────────────────────────────────────────
send '{"type":"output","text":"\n  All done! Every protocol message exercised successfully.\n\n"}'
send '{"type":"log","level":"info","message":"fledge-hello plugin finished"}'
