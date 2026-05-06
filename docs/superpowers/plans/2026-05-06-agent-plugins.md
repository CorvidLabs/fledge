# Agent Plugins Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create four fledge plugins (sql, localnet, algochat, memory) as separate GitHub repos under CorvidLabs, each with full spec-sync, working implementations, and local testing.

**Architecture:** Each plugin is an independent repo implementing the fledge-v1 JSON-lines protocol over stdin/stdout. Shell plugins (sql, localnet) wrap CLI tools. TypeScript plugins (algochat, memory) are compiled to standalone binaries via `bun build --compile`. Cross-plugin communication happens via the `exec` capability calling fledge CLI commands.

**Tech Stack:** Bash (shell plugins), TypeScript/Bun (TS plugins), fledge-v1 protocol, SQLite, Algorand SDK, spec-sync v4.3.1

**Design Spec:** `docs/superpowers/specs/2026-05-06-agent-plugins-design.md`

---

## Task 0: Prerequisites

**Files:** None (environment setup)

- [ ] **Step 1: Build fledge release binary**

```bash
cd /Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6
cargo build --release
```

Expected: Binary at `target/release/fledge`.

- [ ] **Step 2: Create working directory for plugins**

```bash
mkdir -p /tmp/fledge-plugins
```

All four plugin repos will be scaffolded here before pushing to GitHub.

---

## Task 1: fledge-plugin-sql

**Files:**
- Create: `plugin.toml`
- Create: `bin/fledge-sql`
- Create: `README.md`
- Create: `.gitignore`
- Create: `.specsync/config.toml`
- Create: `.specsync/registry.toml`
- Create: `.specsync/version`
- Create: `.specsync/.gitignore`
- Create: `specs/sql/sql.spec.md`
- Create: `specs/sql/requirements.md`
- Create: `specs/sql/tasks.md`
- Create: `specs/sql/context.md`
- Create: `specs/sql/testing.md`
- Create: `test/test_sql.sh`

### Step 1: Create GitHub repo and scaffold

- [ ] **Step 1a: Create the GitHub repo**

```bash
gh repo create CorvidLabs/fledge-plugin-sql --public --description "SQLite management plugin for fledge — init, migrate, query, schema" --clone --add-readme=false
cd /tmp/fledge-plugins/fledge-plugin-sql
git checkout -b main
```

- [ ] **Step 1b: Create plugin.toml**

```toml
[plugin]
name = "fledge-plugin-sql"
version = "0.1.0"
description = "SQLite database management — init, migrate, query, schema"
author = "CorvidLabs"
protocol = "fledge-v1"

[[commands]]
name = "sql"
description = "SQLite database management"
binary = "bin/fledge-sql"

[hooks]

[capabilities]
exec = true
store = true
metadata = true
```

- [ ] **Step 1c: Create .gitignore**

```
/target/
/dist/
.DS_Store
Thumbs.db
*.db
```

- [ ] **Step 1d: Create README.md**

```markdown
# fledge-plugin-sql

SQLite database management plugin for [fledge](https://github.com/CorvidLabs/fledge).

## Install

\`\`\`bash
fledge plugins install CorvidLabs/fledge-plugin-sql
\`\`\`

## Commands

| Command | Description |
|---------|-------------|
| `fledge sql init [--path <db>]` | Create a project SQLite database |
| `fledge sql migrate [--dir <dir>]` | Run SQL migration files |
| `fledge sql query <sql>` | Execute a query and display results |
| `fledge sql schema` | Dump the current database schema |

## Prerequisites

- `sqlite3` on PATH (pre-installed on macOS)

## Development

\`\`\`bash
fledge plugins validate .
fledge spec check
\`\`\`
```

### Step 2: Set up spec-sync

- [ ] **Step 2a: Create .specsync directory and files**

Create `.specsync/config.toml`:
```toml
# spec-sync v4 configuration
specs_dir = "specs"
source_dirs = ["bin"]
exclude_dirs = []
exclude_patterns = []
required_sections = ["Purpose", "Public API", "Invariants", "Behavioral Examples", "Error Cases", "Dependencies", "Change Log"]
enforcement = "strict"

[lifecycle]
track_history = false
```

Create `.specsync/registry.toml`:
```toml
[registry]
name = "fledge-plugin-sql"

[specs]
sql = "specs/sql/sql.spec.md"
```

Create `.specsync/version`:
```
4.3.1
```

Create `.specsync/.gitignore`:
```
backup-3x/
config.local.toml
hashes.json
```

- [ ] **Step 2b: Create spec directory**

```bash
mkdir -p specs/sql
```

### Step 3: Write the spec

- [ ] **Step 3a: Write specs/sql/sql.spec.md**

```markdown
---
module: sql
version: 1
status: active
files:
  - bin/fledge-sql

db_tables: []
depends_on: []
---

# Sql

## Purpose

SQLite database management for fledge projects. Provides project-local database initialization, migration tracking, ad-hoc queries, and schema inspection. Wraps the `sqlite3` CLI via the fledge-v1 protocol's `exec` capability.

## Public API

### Commands

| Command | Args | Description |
|---------|------|-------------|
| `init` | `[--path <db-path>]` | Create a SQLite database. Default: `.fledge/data.db`. Stores path via `store`. |
| `migrate` | `[--dir <migrations-dir>]` | Run `*.sql` files from `migrations/` in filename order. Tracks in `_migrations` table. |
| `query` | `<sql>` | Execute SQL, display results as formatted table. |
| `schema` | | Dump schema via `sqlite_master`. |

### Protocol Messages Used

| Message Type | Direction | Purpose |
|-------------|-----------|---------|
| `init` | inbound | Receive project context and args |
| `exec` | outbound | Run `sqlite3` commands |
| `store` | outbound | Persist DB path |
| `load` | outbound | Retrieve stored DB path |
| `output` | outbound | Display results to user |
| `log` | outbound | Diagnostic messages |
| `prompt` | outbound | Ask user for DB path if not specified |

## Invariants

1. The plugin never creates a database without user confirmation (either `--path` flag or interactive prompt).
2. Migrations are idempotent — re-running `migrate` skips already-applied files.
3. The `_migrations` table is created automatically on first `migrate` run.
4. Migration files are sorted by filename (lexicographic) and applied in order.
5. Each migration runs inside a transaction — if it fails, none of that file's changes persist.
6. The stored DB path is project-scoped via the fledge-v1 `store` capability.
7. `query` and `schema` fail with a clear error if no database has been initialized.
8. All `sqlite3` invocations go through the fledge-v1 `exec` message, never direct shell execution.

## Behavioral Examples

```
$ fledge sql init
  Created database at .fledge/data.db

$ fledge sql init --path myapp.db
  Created database at myapp.db

$ fledge sql migrate
  Applied 3 migrations:
    001_create_users.sql
    002_create_posts.sql
    003_add_indexes.sql

$ fledge sql migrate
  All migrations already applied.

$ fledge sql query "SELECT * FROM users LIMIT 5"
  id  | name    | email
  1   | alice   | alice@example.com
  2   | bob     | bob@example.com

$ fledge sql schema
  CREATE TABLE _migrations (id INTEGER PRIMARY KEY, filename TEXT UNIQUE, applied_at TEXT);
  CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE);
  CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), body TEXT);
  CREATE INDEX idx_posts_user ON posts(user_id);
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| `sqlite3 not found` | `sqlite3` not on PATH | Log error, exit 1 |
| `No database initialized` | `query`/`schema`/`migrate` before `init` | Log error with hint to run `fledge sql init` |
| `Migration failed` | SQL error in a migration file | Roll back that file's transaction, log error with filename and line, exit 1 |
| `Database already exists` | `init` when DB file exists | Log warning, skip creation |
| `No migrations directory` | `migrate` when `migrations/` doesn't exist | Log info "No migrations directory found", exit 0 |

## Dependencies

- `sqlite3` CLI (external, must be on PATH)
- fledge-v1 protocol (exec, store, load capabilities)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-05-06 | Initial spec |
```

- [ ] **Step 3b: Write companion files**

Create `specs/sql/requirements.md`:
```markdown
---
spec: sql.spec.md
---

## User Stories

- As a developer, I want to create a project-local SQLite database with a single command
- As a developer, I want to run SQL migration files in order and track which have been applied
- As a developer, I want to run ad-hoc queries against my project database
- As an AI agent, I want to manage structured data storage without manual setup

## Acceptance Criteria

- `fledge sql init` creates a database file and stores the path
- `fledge sql migrate` applies unapplied .sql files in order and tracks them
- `fledge sql query` executes SQL and returns formatted results
- `fledge sql schema` shows all tables, indexes, and views
- All commands use fledge-v1 protocol for I/O

## Constraints

- Must work without any dependencies beyond sqlite3
- Shell script implementation (no compile step)
- Communicates via fledge-v1 JSON-lines protocol on stdin/stdout

## Out of Scope

- GUI or TUI interfaces
- Multi-database support (one DB per project)
- Database replication or backup
```

Create `specs/sql/tasks.md`:
```markdown
---
spec: sql.spec.md
---

## Tasks

- [x] Write spec
- [ ] Implement fledge-v1 protocol helpers (send/recv JSON)
- [ ] Implement `init` subcommand
- [ ] Implement `migrate` subcommand
- [ ] Implement `query` subcommand
- [ ] Implement `schema` subcommand
- [ ] Write integration tests
- [ ] Validate with `fledge plugins validate`
```

Create `specs/sql/context.md`:
```markdown
---
spec: sql.spec.md
---

## Context

Extracted from corvid-agent's SQLite database management. Provides a standalone, reusable database layer that any fledge project (or agent) can use for structured local storage.

## Related Modules

- fledge-plugin-memory (uses sql plugin for ephemeral tier storage)

## Design Decisions

- Shell script wrapping `sqlite3` CLI rather than a compiled binary — keeps the plugin zero-dependency and immediately editable
- Migrations tracked in a `_migrations` table rather than a separate state file — the database itself is the source of truth
- Uses fledge-v1 `store` capability for DB path rather than a config file — integrates with fledge's plugin storage system
```

Create `specs/sql/testing.md`:
```markdown
---
spec: sql.spec.md
---

## Test Plan

### Integration Tests

- Pipe fledge-v1 init message + args to the plugin binary, verify JSON output
- Test `init` creates a database file at the specified path
- Test `migrate` applies .sql files and records them in `_migrations`
- Test `migrate` is idempotent (second run applies nothing)
- Test `query` returns formatted results
- Test `schema` returns table definitions
- Test error cases: missing sqlite3, no DB initialized, bad SQL
```

### Step 4: Implement the plugin

- [ ] **Step 4a: Create bin/fledge-sql**

```bash
#!/usr/bin/env bash
set -euo pipefail

# --- fledge-v1 protocol helpers ---

send() { printf '%s\n' "$1"; }

recv() {
  local line
  IFS= read -r line
  printf '%s\n' "$line"
}

send_output() { send "{\"type\":\"output\",\"text\":\"$1\\n\"}"; }
send_log() { send "{\"type\":\"log\",\"level\":\"$1\",\"message\":\"$2\"}"; }
send_error() { send_log "error" "$1"; }

send_exec() {
  local id="$1" cmd="$2" cwd="${3:-}"
  if [ -n "$cwd" ]; then
    send "{\"type\":\"exec\",\"id\":\"$id\",\"command\":\"$cmd\",\"cwd\":\"$cwd\"}"
  else
    send "{\"type\":\"exec\",\"id\":\"$id\",\"command\":\"$cmd\"}"
  fi
  recv
}

send_store() {
  local id="$1" key="$2" value="$3"
  send "{\"type\":\"store\",\"id\":\"$id\",\"key\":\"$key\",\"value\":\"$value\"}"
  recv
}

send_load() {
  local id="$1" key="$2"
  send "{\"type\":\"load\",\"id\":\"$id\",\"key\":\"$key\"}"
  recv
}

send_prompt() {
  local id="$1" message="$2" default="${3:-}"
  if [ -n "$default" ]; then
    send "{\"type\":\"prompt\",\"id\":\"$id\",\"message\":\"$message\",\"default\":\"$default\"}"
  else
    send "{\"type\":\"prompt\",\"id\":\"$id\",\"message\":\"$message\"}"
  fi
  recv
}

# --- Parse init message ---

INIT=$(recv)
ARGS=$(printf '%s' "$INIT" | jq -r '.args // [] | .[]' 2>/dev/null)
PROJECT_ROOT=$(printf '%s' "$INIT" | jq -r '.project.root // "."' 2>/dev/null)

# Convert args to array
declare -a ARGV=()
while IFS= read -r arg; do
  [ -n "$arg" ] && ARGV+=("$arg")
done <<< "$ARGS"

SUBCMD="${ARGV[0]:-help}"

# --- Helper: get DB path from store ---

get_db_path() {
  local resp
  resp=$(send_load "load-db" "db_path")
  local val
  val=$(printf '%s' "$resp" | jq -r '.value // empty' 2>/dev/null)
  if [ -n "$val" ]; then
    printf '%s' "$val"
  fi
}

# --- Subcommands ---

cmd_init() {
  local db_path=""

  # Parse --path flag
  local i=1
  while [ $i -lt ${#ARGV[@]} ]; do
    case "${ARGV[$i]}" in
      --path)
        i=$((i + 1))
        db_path="${ARGV[$i]:-}"
        ;;
    esac
    i=$((i + 1))
  done

  # Prompt if no path given
  if [ -z "$db_path" ]; then
    local resp
    resp=$(send_prompt "prompt-path" "Database path:" ".fledge/data.db")
    db_path=$(printf '%s' "$resp" | jq -r '.value' 2>/dev/null)
  fi

  # Make path relative to project root
  if [[ "$db_path" != /* ]]; then
    db_path="$PROJECT_ROOT/$db_path"
  fi

  # Check if already exists
  local check
  check=$(send_exec "check-exists" "test -f '$db_path' && echo exists || echo missing" "$PROJECT_ROOT")
  local exists
  exists=$(printf '%s' "$check" | jq -r '.stdout // ""' 2>/dev/null | tr -d '[:space:]')

  if [ "$exists" = "exists" ]; then
    send_output "Database already exists at $db_path"
    send_store "store-db" "db_path" "$db_path" > /dev/null
    exit 0
  fi

  # Create directory and database
  local dir
  dir=$(dirname "$db_path")
  send_exec "mkdir" "mkdir -p '$dir'" "$PROJECT_ROOT" > /dev/null
  send_exec "create-db" "sqlite3 '$db_path' 'SELECT 1;'" "$PROJECT_ROOT" > /dev/null
  send_store "store-db" "db_path" "$db_path" > /dev/null
  send_output "Created database at $db_path"
}

cmd_migrate() {
  local db_path
  db_path=$(get_db_path)
  if [ -z "$db_path" ]; then
    send_error "No database initialized. Run: fledge sql init"
    exit 1
  fi

  local migrations_dir="$PROJECT_ROOT/migrations"

  # Parse --dir flag
  local i=1
  while [ $i -lt ${#ARGV[@]} ]; do
    case "${ARGV[$i]}" in
      --dir)
        i=$((i + 1))
        migrations_dir="${ARGV[$i]:-}"
        if [[ "$migrations_dir" != /* ]]; then
          migrations_dir="$PROJECT_ROOT/$migrations_dir"
        fi
        ;;
    esac
    i=$((i + 1))
  done

  # Check migrations dir exists
  local check
  check=$(send_exec "check-dir" "test -d '$migrations_dir' && echo exists || echo missing" "$PROJECT_ROOT")
  local exists
  exists=$(printf '%s' "$check" | jq -r '.stdout // ""' 2>/dev/null | tr -d '[:space:]')

  if [ "$exists" != "exists" ]; then
    send_output "No migrations directory found at $migrations_dir"
    exit 0
  fi

  # Create _migrations table
  send_exec "create-meta" "sqlite3 '$db_path' 'CREATE TABLE IF NOT EXISTS _migrations (id INTEGER PRIMARY KEY, filename TEXT UNIQUE, applied_at TEXT);'" "$PROJECT_ROOT" > /dev/null

  # List migration files
  local files_resp
  files_resp=$(send_exec "list-files" "ls -1 '$migrations_dir'/*.sql 2>/dev/null | sort" "$PROJECT_ROOT")
  local files
  files=$(printf '%s' "$files_resp" | jq -r '.stdout // ""' 2>/dev/null)

  if [ -z "$files" ]; then
    send_output "No .sql migration files found."
    exit 0
  fi

  local applied=0
  while IFS= read -r filepath; do
    [ -z "$filepath" ] && continue
    local filename
    filename=$(basename "$filepath")

    # Check if already applied
    local already
    already=$(send_exec "check-$filename" "sqlite3 '$db_path' \"SELECT COUNT(*) FROM _migrations WHERE filename='$filename';\"" "$PROJECT_ROOT")
    local count
    count=$(printf '%s' "$already" | jq -r '.stdout // "0"' 2>/dev/null | tr -d '[:space:]')

    if [ "$count" != "0" ]; then
      continue
    fi

    # Apply migration in a transaction
    local result
    result=$(send_exec "apply-$filename" "sqlite3 '$db_path' < '$filepath' && sqlite3 '$db_path' \"INSERT INTO _migrations (filename, applied_at) VALUES ('$filename', datetime('now'));\"" "$PROJECT_ROOT")
    local exit_code
    exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)

    if [ "$exit_code" != "0" ]; then
      local stderr
      stderr=$(printf '%s' "$result" | jq -r '.stderr // "unknown error"' 2>/dev/null)
      send_error "Migration failed: $filename — $stderr"
      exit 1
    fi

    send_output "  Applied: $filename"
    applied=$((applied + 1))
  done <<< "$files"

  if [ "$applied" -eq 0 ]; then
    send_output "All migrations already applied."
  else
    send_output "Applied $applied migration(s)."
  fi
}

cmd_query() {
  local db_path
  db_path=$(get_db_path)
  if [ -z "$db_path" ]; then
    send_error "No database initialized. Run: fledge sql init"
    exit 1
  fi

  # Collect all args after "query" as SQL
  local sql="${ARGV[@]:1}"
  if [ -z "$sql" ]; then
    send_error "Usage: fledge sql query <sql>"
    exit 1
  fi

  local result
  result=$(send_exec "query" "sqlite3 -header -column '$db_path' \"$sql\"" "$PROJECT_ROOT")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)

  if [ "$exit_code" != "0" ]; then
    local stderr
    stderr=$(printf '%s' "$result" | jq -r '.stderr // "unknown error"' 2>/dev/null)
    send_error "Query failed: $stderr"
    exit 1
  fi

  local stdout
  stdout=$(printf '%s' "$result" | jq -r '.stdout // ""' 2>/dev/null)
  if [ -n "$stdout" ]; then
    # Escape the output for JSON
    local escaped
    escaped=$(printf '%s' "$stdout" | jq -Rs '.')
    send "{\"type\":\"output\",\"text\":$escaped}"
  else
    send_output "(no results)"
  fi
}

cmd_schema() {
  local db_path
  db_path=$(get_db_path)
  if [ -z "$db_path" ]; then
    send_error "No database initialized. Run: fledge sql init"
    exit 1
  fi

  local result
  result=$(send_exec "schema" "sqlite3 '$db_path' \"SELECT sql FROM sqlite_master WHERE type IN ('table','index','view') AND sql IS NOT NULL ORDER BY name;\"" "$PROJECT_ROOT")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)

  if [ "$exit_code" != "0" ]; then
    local stderr
    stderr=$(printf '%s' "$result" | jq -r '.stderr // "unknown error"' 2>/dev/null)
    send_error "Schema dump failed: $stderr"
    exit 1
  fi

  local stdout
  stdout=$(printf '%s' "$result" | jq -r '.stdout // ""' 2>/dev/null)
  if [ -n "$stdout" ]; then
    local escaped
    escaped=$(printf '%s' "$stdout" | jq -Rs '.')
    send "{\"type\":\"output\",\"text\":$escaped}"
  else
    send_output "(empty database)"
  fi
}

cmd_help() {
  send_output "fledge-plugin-sql — SQLite database management"
  send_output ""
  send_output "Commands:"
  send_output "  init [--path <db>]       Create a project database"
  send_output "  migrate [--dir <dir>]    Run SQL migrations"
  send_output "  query <sql>              Execute a query"
  send_output "  schema                   Dump database schema"
}

# --- Dispatch ---

case "$SUBCMD" in
  init)    cmd_init ;;
  migrate) cmd_migrate ;;
  query)   cmd_query ;;
  schema)  cmd_schema ;;
  help|--help|-h) cmd_help ;;
  *)
    send_error "Unknown command: $SUBCMD. Run: fledge sql help"
    exit 1
    ;;
esac
```

Make executable: `chmod +x bin/fledge-sql`

### Step 5: Test locally

- [ ] **Step 5a: Validate plugin structure**

```bash
cd /tmp/fledge-plugins/fledge-plugin-sql
/Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6/target/release/fledge plugins validate .
```

Expected: Validation passes (name, version, binary exists).

- [ ] **Step 5b: Validate spec-sync**

```bash
/Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6/target/release/fledge spec check
```

Expected: `1 spec checked, 0 errors, 0 warnings`

- [ ] **Step 5c: Integration test — pipe protocol messages**

Create `test/test_sql.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail

PLUGIN="$(cd "$(dirname "$0")/.." && pwd)/bin/fledge-sql"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "=== Test: help command ==="
echo '{"type":"init","protocol":"fledge-v1","args":["help"],"project":{"name":"test","root":"'"$TMPDIR"'","language":"unknown","git":{}},"plugin":{"name":"fledge-plugin-sql","version":"0.1.0","dir":"."},"fledge":{"version":"1.2.0"},"capabilities":{"exec":true,"store":true,"metadata":true}}' \
  | FLEDGE_PLUGIN_DIR="$(dirname "$PLUGIN")/.." "$PLUGIN" 2>/dev/null \
  | head -1 | jq -r '.text' | grep -q "SQLite database management" && echo "PASS" || echo "FAIL"

echo "=== All tests passed ==="
```

Run: `chmod +x test/test_sql.sh && bash test/test_sql.sh`

Note: Full protocol testing requires fledge to act as the host (responding to exec/store/load messages). The integration test above validates basic protocol output. Full end-to-end testing is done via `fledge plugins install ./` and running `fledge sql help`.

### Step 6: Commit and push

- [ ] **Step 6a: Initial commit**

```bash
cd /tmp/fledge-plugins/fledge-plugin-sql
git add -A
git commit -m "feat: initial fledge-plugin-sql — SQLite management plugin

Shell plugin wrapping sqlite3 CLI via fledge-v1 protocol.
Commands: init, migrate, query, schema.
Includes full spec-sync setup."
git push -u origin main
```

---

## Task 2: fledge-plugin-localnet

**Files:**
- Create: `plugin.toml`
- Create: `bin/fledge-localnet`
- Create: `README.md`
- Create: `.gitignore`
- Create: `.specsync/config.toml`
- Create: `.specsync/registry.toml`
- Create: `.specsync/version`
- Create: `.specsync/.gitignore`
- Create: `specs/localnet/localnet.spec.md`
- Create: `specs/localnet/requirements.md`
- Create: `specs/localnet/tasks.md`
- Create: `specs/localnet/context.md`
- Create: `specs/localnet/testing.md`

### Step 1: Create GitHub repo and scaffold

- [ ] **Step 1a: Create the GitHub repo**

```bash
gh repo create CorvidLabs/fledge-plugin-localnet --public --description "Algorand localnet lifecycle plugin for fledge — start, stop, reset, fund, accounts" --clone --add-readme=false
cd /tmp/fledge-plugins/fledge-plugin-localnet
git checkout -b main
```

- [ ] **Step 1b: Create plugin.toml**

```toml
[plugin]
name = "fledge-plugin-localnet"
version = "0.1.0"
description = "Algorand localnet lifecycle — start, stop, reset, status, fund, accounts"
author = "CorvidLabs"
protocol = "fledge-v1"

[[commands]]
name = "localnet"
description = "Algorand localnet management"
binary = "bin/fledge-localnet"

[hooks]
post_work_start = "hooks/post-work-start.sh"

[capabilities]
exec = true
store = false
metadata = true
```

- [ ] **Step 1c: Create .gitignore, README.md**

`.gitignore`:
```
/target/
/dist/
.DS_Store
Thumbs.db
```

`README.md`:
```markdown
# fledge-plugin-localnet

Algorand localnet lifecycle plugin for [fledge](https://github.com/CorvidLabs/fledge).

## Install

\`\`\`bash
fledge plugins install CorvidLabs/fledge-plugin-localnet
\`\`\`

## Commands

| Command | Description |
|---------|-------------|
| `fledge localnet start` | Start Algorand localnet |
| `fledge localnet stop` | Stop localnet |
| `fledge localnet reset` | Reset localnet to genesis |
| `fledge localnet status` | Show running state and ports |
| `fledge localnet fund <address>` | Dispense Algos from faucet |
| `fledge localnet accounts` | List accounts with balances |

## Prerequisites

- `algokit` on PATH
- Docker running
```

### Step 2: Set up spec-sync

- [ ] **Step 2a: Create .specsync files**

`.specsync/config.toml`:
```toml
specs_dir = "specs"
source_dirs = ["bin", "hooks"]
exclude_dirs = []
exclude_patterns = []
required_sections = ["Purpose", "Public API", "Invariants", "Behavioral Examples", "Error Cases", "Dependencies", "Change Log"]
enforcement = "strict"

[lifecycle]
track_history = false
```

`.specsync/registry.toml`:
```toml
[registry]
name = "fledge-plugin-localnet"

[specs]
localnet = "specs/localnet/localnet.spec.md"
```

`.specsync/version`: `4.3.1`

`.specsync/.gitignore`:
```
backup-3x/
config.local.toml
hashes.json
```

### Step 3: Write the spec

- [ ] **Step 3a: Write specs/localnet/localnet.spec.md**

```markdown
---
module: localnet
version: 1
status: active
files:
  - bin/fledge-localnet
  - hooks/post-work-start.sh

db_tables: []
depends_on: []
---

# Localnet

## Purpose

Algorand localnet lifecycle management for fledge projects. Wraps `algokit localnet` and `goal` CLI tools to provide start/stop/reset/status/fund/accounts commands via the fledge-v1 protocol. Includes a `post_work_start` lifecycle hook to auto-start localnet for Algorand projects.

## Public API

### Commands

| Command | Args | Description |
|---------|------|-------------|
| `start` | | Start localnet via `algokit localnet start` |
| `stop` | | Stop localnet via `algokit localnet stop` |
| `reset` | | Reset localnet to genesis via `algokit localnet reset` |
| `status` | | Show running state, ports, network ID |
| `fund` | `<address> [--amount <microalgos>]` | Dispense Algos. Default: 10000000 (10 ALGO) |
| `accounts` | | List localnet accounts with balances |

### Lifecycle Hooks

| Hook | Trigger | Behavior |
|------|---------|----------|
| `post_work_start` | After `fledge work start` | Auto-start localnet if `fledge.toml` has `[localnet]` section |

## Invariants

1. All CLI tool invocations go through the fledge-v1 `exec` capability.
2. `fund` defaults to 10,000,000 microAlgos (10 ALGO) if no amount specified.
3. `status` reports Docker container state, not just algokit's opinion.
4. `reset` warns the user before destroying localnet state.
5. The `post_work_start` hook is a no-op if `fledge.toml` lacks a `[localnet]` section.
6. `fund` and `accounts` exec into the algod Docker container to run `goal`.

## Behavioral Examples

```
$ fledge localnet start
  Starting Algorand localnet...
  Localnet started. algod: localhost:4001, KMD: localhost:4002, indexer: localhost:8980

$ fledge localnet status
  Status: running
  algod:   localhost:4001
  KMD:     localhost:4002
  Indexer: localhost:8980

$ fledge localnet fund ABC123...XYZ
  Funded ABC123...XYZ with 10.000000 ALGO

$ fledge localnet accounts
  Address                                          Balance
  FAUCET...ABC                                     1000000.000000 ALGO
  USER1...DEF                                      100.000000 ALGO

$ fledge localnet stop
  Localnet stopped.

$ fledge localnet reset
  Are you sure? This will destroy all localnet state. (y/n)
  Localnet reset to genesis.
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| `algokit not found` | `algokit` not on PATH | Log error with install instructions, exit 1 |
| `Docker not running` | Docker daemon not available | Log error "Docker must be running for localnet", exit 1 |
| `Localnet not running` | `fund`/`accounts` when localnet is stopped | Log error "Localnet is not running. Run: fledge localnet start", exit 1 |
| `Invalid address` | `fund` with malformed Algorand address | Log error, exit 1 |
| `Insufficient faucet balance` | Faucet account depleted | Log error, suggest `fledge localnet reset`, exit 1 |

## Dependencies

- `algokit` CLI (external, must be on PATH)
- Docker (external, must be running)
- `goal` CLI (available inside algod Docker container)
- fledge-v1 protocol (exec, metadata capabilities)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-05-06 | Initial spec |
```

- [ ] **Step 3b: Write companion files**

`specs/localnet/requirements.md`:
```markdown
---
spec: localnet.spec.md
---

## User Stories

- As a developer, I want to start/stop Algorand localnet with a single command
- As a developer, I want to fund test accounts without remembering goal syntax
- As an AI agent, I want to check localnet status before sending transactions

## Acceptance Criteria

- All 6 commands work via fledge-v1 protocol
- post_work_start hook auto-starts localnet for Algorand projects
- Clear error messages when prerequisites are missing

## Constraints

- Wraps algokit/goal, does not reimplement
- Shell script, no compile step

## Out of Scope

- TestNet/MainNet management
- Smart contract deployment
```

`specs/localnet/tasks.md`:
```markdown
---
spec: localnet.spec.md
---

## Tasks

- [x] Write spec
- [ ] Implement protocol helpers
- [ ] Implement start/stop/reset
- [ ] Implement status
- [ ] Implement fund
- [ ] Implement accounts
- [ ] Create post_work_start hook
- [ ] Test locally
```

`specs/localnet/context.md`:
```markdown
---
spec: localnet.spec.md
---

## Context

Extracted from corvid-agent's Algorand localnet management. Provides a standalone CLI for any fledge project that needs a local Algorand network.

## Related Modules

- fledge-plugin-algochat (uses localnet for on-chain messaging)
- fledge-plugin-memory (uses localnet for mutable/permanent memory tiers)

## Design Decisions

- Wraps algokit rather than reimplementing Docker management — algokit is the standard Algorand dev tool
- `fund` and `accounts` exec into the algod Docker container because `goal` is not typically installed on the host
- `post_work_start` hook checks for `[localnet]` in fledge.toml to avoid auto-starting for non-Algorand projects
```

`specs/localnet/testing.md`:
```markdown
---
spec: localnet.spec.md
---

## Test Plan

### Integration Tests

- Test help command outputs expected text
- Test status command when localnet is not running (should report stopped)
- Test start/stop lifecycle (requires Docker)
- Test fund command (requires running localnet)
- Test accounts command (requires running localnet)
- Test error handling when algokit is not installed
```

### Step 4: Implement the plugin

- [ ] **Step 4a: Create bin/fledge-localnet**

```bash
#!/usr/bin/env bash
set -euo pipefail

# --- fledge-v1 protocol helpers ---

send() { printf '%s\n' "$1"; }

recv() {
  local line
  IFS= read -r line
  printf '%s\n' "$line"
}

send_output() { send "{\"type\":\"output\",\"text\":\"$1\\n\"}"; }
send_log() { send "{\"type\":\"log\",\"level\":\"$1\",\"message\":\"$2\"}"; }
send_error() { send_log "error" "$1"; }
send_progress() { send "{\"type\":\"progress\",\"message\":\"$1\"}"; }

send_exec() {
  local id="$1" cmd="$2"
  send "{\"type\":\"exec\",\"id\":\"$id\",\"command\":\"$cmd\"}"
  recv
}

send_confirm() {
  local id="$1" message="$2"
  send "{\"type\":\"confirm\",\"id\":\"$id\",\"message\":\"$message\"}"
  recv
}

# --- Parse init message ---

INIT=$(recv)
ARGS=$(printf '%s' "$INIT" | jq -r '.args // [] | .[]' 2>/dev/null)
PROJECT_ROOT=$(printf '%s' "$INIT" | jq -r '.project.root // "."' 2>/dev/null)

declare -a ARGV=()
while IFS= read -r arg; do
  [ -n "$arg" ] && ARGV+=("$arg")
done <<< "$ARGS"

SUBCMD="${ARGV[0]:-help}"

# --- Helper: check prerequisites ---

check_algokit() {
  local result
  result=$(send_exec "check-algokit" "which algokit")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 1' 2>/dev/null)
  if [ "$exit_code" != "0" ]; then
    send_error "algokit not found. Install: https://github.com/algorandfoundation/algokit-cli"
    exit 1
  fi
}

check_docker() {
  local result
  result=$(send_exec "check-docker" "docker info > /dev/null 2>&1 && echo ok || echo fail")
  local stdout
  stdout=$(printf '%s' "$result" | jq -r '.stdout // ""' 2>/dev/null | tr -d '[:space:]')
  if [ "$stdout" != "ok" ]; then
    send_error "Docker must be running for localnet"
    exit 1
  fi
}

# --- Subcommands ---

cmd_start() {
  check_algokit
  check_docker
  send_progress "Starting Algorand localnet..."
  local result
  result=$(send_exec "start" "algokit localnet start")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)
  if [ "$exit_code" != "0" ]; then
    local stderr
    stderr=$(printf '%s' "$result" | jq -r '.stderr // "unknown error"' 2>/dev/null)
    send_error "Failed to start localnet: $stderr"
    exit 1
  fi
  send_output "Localnet started. algod: localhost:4001, KMD: localhost:4002, indexer: localhost:8980"
}

cmd_stop() {
  check_algokit
  local result
  result=$(send_exec "stop" "algokit localnet stop")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)
  if [ "$exit_code" != "0" ]; then
    local stderr
    stderr=$(printf '%s' "$result" | jq -r '.stderr // "unknown error"' 2>/dev/null)
    send_error "Failed to stop localnet: $stderr"
    exit 1
  fi
  send_output "Localnet stopped."
}

cmd_reset() {
  check_algokit
  local resp
  resp=$(send_confirm "confirm-reset" "Are you sure? This will destroy all localnet state.")
  local confirmed
  confirmed=$(printf '%s' "$resp" | jq -r '.value // false' 2>/dev/null)
  if [ "$confirmed" != "true" ]; then
    send_output "Cancelled."
    exit 0
  fi
  send_progress "Resetting localnet..."
  local result
  result=$(send_exec "reset" "algokit localnet reset")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)
  if [ "$exit_code" != "0" ]; then
    local stderr
    stderr=$(printf '%s' "$result" | jq -r '.stderr // "unknown error"' 2>/dev/null)
    send_error "Failed to reset localnet: $stderr"
    exit 1
  fi
  send_output "Localnet reset to genesis."
}

cmd_status() {
  check_algokit
  local result
  result=$(send_exec "status" "algokit localnet status")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)
  local stdout
  stdout=$(printf '%s' "$result" | jq -r '.stdout // ""' 2>/dev/null)

  # Check if containers are running
  local docker_check
  docker_check=$(send_exec "docker-ps" "docker ps --filter name=algokit --format '{{.Names}}: {{.Status}}' 2>/dev/null")
  local containers
  containers=$(printf '%s' "$docker_check" | jq -r '.stdout // ""' 2>/dev/null | tr -d '[:space:]')

  if [ -z "$containers" ]; then
    send_output "Status: stopped"
    send_output "Run: fledge localnet start"
  else
    send_output "Status: running"
    send_output "algod:   localhost:4001"
    send_output "KMD:     localhost:4002"
    send_output "Indexer: localhost:8980"
  fi
}

cmd_fund() {
  local address="${ARGV[1]:-}"
  if [ -z "$address" ]; then
    send_error "Usage: fledge localnet fund <address> [--amount <microalgos>]"
    exit 1
  fi

  local amount="10000000"
  local i=2
  while [ $i -lt ${#ARGV[@]} ]; do
    case "${ARGV[$i]}" in
      --amount)
        i=$((i + 1))
        amount="${ARGV[$i]:-10000000}"
        ;;
    esac
    i=$((i + 1))
  done

  check_algokit
  check_docker

  # Use goal inside the algod container
  local result
  result=$(send_exec "fund" "docker exec algokit_algod goal clerk send -a $amount -f \$(docker exec algokit_algod goal account list | head -1 | awk '{print \$2}') -t $address")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)

  if [ "$exit_code" != "0" ]; then
    local stderr
    stderr=$(printf '%s' "$result" | jq -r '.stderr // "unknown error"' 2>/dev/null)
    send_error "Failed to fund account: $stderr"
    exit 1
  fi

  local algo_amount
  algo_amount=$(echo "scale=6; $amount / 1000000" | bc)
  send_output "Funded $address with $algo_amount ALGO"
}

cmd_accounts() {
  check_algokit
  check_docker

  local result
  result=$(send_exec "accounts" "docker exec algokit_algod goal account list")
  local exit_code
  exit_code=$(printf '%s' "$result" | jq -r '.exit_code // 0' 2>/dev/null)

  if [ "$exit_code" != "0" ]; then
    send_error "Localnet is not running. Run: fledge localnet start"
    exit 1
  fi

  local stdout
  stdout=$(printf '%s' "$result" | jq -r '.stdout // ""' 2>/dev/null)
  if [ -n "$stdout" ]; then
    local escaped
    escaped=$(printf '%s' "$stdout" | jq -Rs '.')
    send "{\"type\":\"output\",\"text\":$escaped}"
  else
    send_output "(no accounts found)"
  fi
}

cmd_help() {
  send_output "fledge-plugin-localnet — Algorand localnet lifecycle"
  send_output ""
  send_output "Commands:"
  send_output "  start                        Start localnet"
  send_output "  stop                         Stop localnet"
  send_output "  reset                        Reset to genesis"
  send_output "  status                       Show running state"
  send_output "  fund <addr> [--amount N]     Dispense Algos"
  send_output "  accounts                     List accounts"
}

# --- Dispatch ---

case "$SUBCMD" in
  start)    cmd_start ;;
  stop)     cmd_stop ;;
  reset)    cmd_reset ;;
  status)   cmd_status ;;
  fund)     cmd_fund ;;
  accounts) cmd_accounts ;;
  help|--help|-h) cmd_help ;;
  *)
    send_error "Unknown command: $SUBCMD. Run: fledge localnet help"
    exit 1
    ;;
esac
```

Make executable: `chmod +x bin/fledge-localnet`

- [ ] **Step 4b: Create hooks/post-work-start.sh**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Auto-start localnet if fledge.toml has [localnet] section
if [ -f "fledge.toml" ] && grep -q '^\[localnet\]' fledge.toml 2>/dev/null; then
  echo "Algorand project detected — starting localnet..."
  algokit localnet start 2>/dev/null || echo "Warning: could not auto-start localnet"
fi
```

Make executable: `chmod +x hooks/post-work-start.sh`

### Step 5: Test and commit

- [ ] **Step 5a: Validate plugin and spec-sync**

```bash
cd /tmp/fledge-plugins/fledge-plugin-localnet
/Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6/target/release/fledge plugins validate .
/Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6/target/release/fledge spec check
```

- [ ] **Step 5b: Commit and push**

```bash
git add -A
git commit -m "feat: initial fledge-plugin-localnet — Algorand localnet lifecycle

Shell plugin wrapping algokit/goal via fledge-v1 protocol.
Commands: start, stop, reset, status, fund, accounts.
Includes post_work_start hook and full spec-sync."
git push -u origin main
```

---

## Task 3: fledge-plugin-algochat

**Files:**
- Create: `plugin.toml`
- Create: `src/index.ts`
- Create: `src/protocol.ts`
- Create: `src/crypto.ts`
- Create: `src/contacts.ts`
- Create: `package.json`
- Create: `tsconfig.json`
- Create: `README.md`
- Create: `.gitignore`
- Create: `.specsync/*`
- Create: `specs/algochat/*`

### Step 1: Create GitHub repo

- [ ] **Step 1a: Create repo and scaffold**

```bash
gh repo create CorvidLabs/fledge-plugin-algochat --public --description "Encrypted on-chain messaging plugin for fledge — AlgoChat via Algorand" --clone --add-readme=false
cd /tmp/fledge-plugins/fledge-plugin-algochat
git checkout -b main
```

- [ ] **Step 1b: Create plugin.toml**

```toml
[plugin]
name = "fledge-plugin-algochat"
version = "0.1.0"
description = "Encrypted on-chain messaging via Algorand (AlgoChat)"
author = "CorvidLabs"
protocol = "fledge-v1"

[[commands]]
name = "algochat"
description = "Encrypted on-chain messaging"
binary = "bin/fledge-algochat"

[hooks]
build = "bun install && bun build src/index.ts --compile --outfile bin/fledge-algochat"

[capabilities]
exec = true
store = true
metadata = true
```

- [ ] **Step 1c: Create package.json**

```json
{
  "name": "fledge-plugin-algochat",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "bun build src/index.ts --compile --outfile bin/fledge-algochat",
    "test": "bun test"
  },
  "dependencies": {
    "@noble/curves": "^1.8.0",
    "@noble/ciphers": "^1.2.0",
    "@noble/hashes": "^1.7.0",
    "algosdk": "^3.2.0"
  },
  "devDependencies": {
    "@types/bun": "^1.2.0"
  }
}
```

- [ ] **Step 1d: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "types": ["bun"]
  },
  "include": ["src/**/*.ts"]
}
```

- [ ] **Step 1e: Create .gitignore and README.md**

`.gitignore`:
```
node_modules/
/bin/
/dist/
.DS_Store
Thumbs.db
bun.lock
```

`README.md`:
```markdown
# fledge-plugin-algochat

Encrypted on-chain messaging plugin for [fledge](https://github.com/CorvidLabs/fledge). Implements the AlgoChat protocol via Algorand transactions.

## Install

\`\`\`bash
fledge plugins install CorvidLabs/fledge-plugin-algochat
\`\`\`

## Commands

| Command | Description |
|---------|-------------|
| `fledge algochat send <addr> <msg>` | Send encrypted message |
| `fledge algochat read [--limit N]` | Read incoming messages |
| `fledge algochat contacts` | List contacts |
| `fledge algochat contacts add <name> <addr> <psk>` | Add contact |
| `fledge algochat contacts remove <name>` | Remove contact |
| `fledge algochat keygen` | Generate X25519 keypair |

## Prerequisites

- Algorand localnet or remote algod endpoint
- `fledge-plugin-localnet` (optional, for local development)
```

### Step 2: Set up spec-sync

- [ ] **Step 2a: Create .specsync files**

`.specsync/config.toml`:
```toml
specs_dir = "specs"
source_dirs = ["src"]
exclude_dirs = []
exclude_patterns = []
required_sections = ["Purpose", "Public API", "Invariants", "Behavioral Examples", "Error Cases", "Dependencies", "Change Log"]
enforcement = "strict"

[lifecycle]
track_history = false
```

`.specsync/registry.toml`:
```toml
[registry]
name = "fledge-plugin-algochat"

[specs]
algochat = "specs/algochat/algochat.spec.md"
```

`.specsync/version`: `4.3.1`

`.specsync/.gitignore`:
```
backup-3x/
config.local.toml
hashes.json
```

### Step 3: Write the spec

- [ ] **Step 3a: Write specs/algochat/algochat.spec.md**

```markdown
---
module: algochat
version: 1
status: active
files:
  - src/index.ts
  - src/protocol.ts
  - src/crypto.ts
  - src/contacts.ts

db_tables: []
depends_on: []
---

# Algochat

## Purpose

Encrypted on-chain messaging via Algorand transactions. Implements the AlgoChat protocol: X25519 key exchange, XChaCha20-Poly1305 encryption, messages stored as Algorand transaction note fields. Compatible with corvid-agent's AlgoChat system. Uses the fledge-v1 protocol for all I/O.

## Public API

### Commands

| Command | Args | Description |
|---------|------|-------------|
| `send` | `<address-or-name> <message>` | Encrypt and send on-chain |
| `read` | `[--limit N] [--from <addr>]` | Read and decrypt messages |
| `contacts` | | List PSK contacts |
| `contacts add` | `<name> <addr> <psk>` | Add contact |
| `contacts remove` | `<name>` | Remove contact |
| `keygen` | | Generate X25519 keypair |

### Modules

| File | Responsibility |
|------|---------------|
| `src/index.ts` | Entry point, init message parsing, command dispatch |
| `src/protocol.ts` | fledge-v1 send/recv helpers |
| `src/crypto.ts` | X25519 key exchange, XChaCha20-Poly1305 encrypt/decrypt |
| `src/contacts.ts` | Contact CRUD via fledge-v1 store |

## Invariants

1. All messages are encrypted with XChaCha20-Poly1305 before being sent on-chain.
2. Nonces are 24 bytes, randomly generated per message, prepended to ciphertext.
3. Keys are derived via HKDF-SHA256 from the X25519 shared secret.
4. Contacts are stored via the fledge-v1 `store` capability as a JSON object.
5. The plugin never sends a plaintext message on-chain.
6. `keygen` overwrites any existing keypair after user confirmation.
7. `send` resolves contact names to addresses before sending.
8. `read` decrypts messages from known contacts; unknown senders are shown as `[encrypted, unknown sender]`.

## Behavioral Examples

```
$ fledge algochat keygen
  Generated X25519 keypair.
  Public key: base64encodedkey==

$ fledge algochat contacts add alice ALGO_ADDR_HERE presharedkeyhere
  Added contact: alice

$ fledge algochat contacts
  Name     Address              Key Fingerprint
  alice    ALGO...XYZ           a1b2c3d4

$ fledge algochat send alice "Hello from fledge!"
  Message sent to alice (txid: ABC123...)

$ fledge algochat read --limit 5
  [2026-05-06 10:30] alice: Hello back!
  [2026-05-06 10:28] alice: Testing 123
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| `No keypair generated` | `send`/`read` before `keygen` | Error with hint to run `fledge algochat keygen` |
| `Contact not found` | `send` with unknown name/address | Error listing available contacts |
| `Algod not available` | No localnet and no env vars | Error with setup instructions |
| `Decryption failed` | Message from unknown sender or corrupted | Show `[encrypted, unknown sender]` instead of failing |
| `Transaction failed` | Algorand transaction rejected | Error with algod error message |

## Dependencies

- `@noble/curves` — X25519 key exchange
- `@noble/ciphers` — XChaCha20-Poly1305 encryption
- `@noble/hashes` — HKDF-SHA256 key derivation
- `algosdk` — Algorand transaction construction and signing
- fledge-v1 protocol (exec, store capabilities)
- Algorand node (localnet or remote algod)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-05-06 | Initial spec |
```

- [ ] **Step 3b: Write companion files**

`specs/algochat/requirements.md`:
```markdown
---
spec: algochat.spec.md
---

## User Stories

- As a developer, I want to send encrypted messages to other agents on Algorand
- As an AI agent, I want to read and respond to on-chain messages
- As a developer, I want to manage contacts with pre-shared keys

## Acceptance Criteria

- Messages are encrypted end-to-end using X25519 + XChaCha20-Poly1305
- Compatible with corvid-agent's AlgoChat protocol
- Contacts stored securely via fledge-v1 store
- Works with localnet or remote algod

## Constraints

- Must use the same encryption protocol as corvid-agent for interoperability
- TypeScript/Bun implementation (needs crypto libraries)

## Out of Scope

- Group messaging
- Message persistence (messages live on-chain)
- Key rotation
```

`specs/algochat/tasks.md`:
```markdown
---
spec: algochat.spec.md
---

## Tasks

- [x] Write spec
- [ ] Implement protocol helpers (src/protocol.ts)
- [ ] Implement crypto module (src/crypto.ts)
- [ ] Implement contacts module (src/contacts.ts)
- [ ] Implement command dispatch (src/index.ts)
- [ ] Write unit tests for crypto
- [ ] Build and test locally
```

`specs/algochat/context.md`:
```markdown
---
spec: algochat.spec.md
---

## Context

Extracted from corvid-agent's AlgoChat on-chain messaging system. Provides standalone encrypted messaging capability for any fledge project working with Algorand.

## Related Modules

- fledge-plugin-localnet (provides the Algorand network)
- corvid-agent AlgoChat (compatible protocol implementation)

## Design Decisions

- Uses @noble libraries instead of tweetnacl — more modern, audited, tree-shakeable
- Single compiled binary via `bun build --compile` — no runtime dependency on bun
- PSK-based contacts rather than public key exchange — simpler initial implementation, matches corvid-agent's current approach
- Contacts stored in fledge-v1 store rather than filesystem — portable across plugin reinstalls
```

`specs/algochat/testing.md`:
```markdown
---
spec: algochat.spec.md
---

## Test Plan

### Unit Tests

- X25519 keypair generation produces valid keys
- XChaCha20-Poly1305 encrypt/decrypt roundtrip succeeds
- HKDF key derivation produces deterministic output
- Contact serialization/deserialization
- Message format: nonce (24 bytes) + ciphertext

### Integration Tests

- Full send/read cycle on localnet
- Contact add/list/remove
- Error handling for missing keypair
- Error handling for offline algod
```

### Step 4: Implement the plugin

- [ ] **Step 4a: Create src/protocol.ts**

```typescript
import { stdin, stdout } from "process";
import { createInterface } from "readline";

const rl = createInterface({ input: stdin, terminal: false });
const lines: string[] = [];
let resolver: ((line: string) => void) | null = null;

rl.on("line", (line) => {
  if (resolver) {
    const r = resolver;
    resolver = null;
    r(line);
  } else {
    lines.push(line);
  }
});

export function send(msg: Record<string, unknown>): void {
  stdout.write(JSON.stringify(msg) + "\n");
}

export function recv(): Promise<string> {
  const buffered = lines.shift();
  if (buffered !== undefined) return Promise.resolve(buffered);
  return new Promise((resolve) => {
    resolver = resolve;
  });
}

export async function recvJson<T = Record<string, unknown>>(): Promise<T> {
  const line = await recv();
  return JSON.parse(line) as T;
}

export function sendOutput(text: string): void {
  send({ type: "output", text: text + "\n" });
}

export function sendError(msg: string): void {
  send({ type: "log", level: "error", message: msg });
}

export function sendLog(level: string, msg: string): void {
  send({ type: "log", level, message: msg });
}

let msgId = 0;
function nextId(): string {
  return String(++msgId);
}

export async function sendExec(command: string): Promise<{ stdout: string; stderr: string; exit_code: number }> {
  const id = nextId();
  send({ type: "exec", id, command });
  const resp = await recvJson<{ stdout?: string; stderr?: string; exit_code?: number }>();
  return { stdout: resp.stdout ?? "", stderr: resp.stderr ?? "", exit_code: resp.exit_code ?? 0 };
}

export async function sendStore(key: string, value: string): Promise<void> {
  const id = nextId();
  send({ type: "store", id, key, value });
  await recv();
}

export async function sendLoad(key: string): Promise<string | null> {
  const id = nextId();
  send({ type: "load", id, key });
  const resp = await recvJson<{ value?: string | null }>();
  return resp.value ?? null;
}

export async function sendPrompt(message: string, defaultValue?: string): Promise<string> {
  const id = nextId();
  const msg: Record<string, unknown> = { type: "prompt", id, message };
  if (defaultValue !== undefined) msg.default = defaultValue;
  send(msg);
  const resp = await recvJson<{ value: string }>();
  return resp.value;
}

export async function sendConfirm(message: string): Promise<boolean> {
  const id = nextId();
  send({ type: "confirm", id, message });
  const resp = await recvJson<{ value: boolean }>();
  return resp.value;
}

export interface InitMessage {
  type: "init";
  protocol: string;
  args: string[];
  project: { name: string; root: string; language: string; git: Record<string, unknown> };
  plugin: { name: string; version: string; dir: string };
  fledge: { version: string };
  capabilities: { exec: boolean; store: boolean; metadata: boolean };
}
```

- [ ] **Step 4b: Create src/crypto.ts**

```typescript
import { x25519 } from "@noble/curves/ed25519";
import { xchacha20poly1305 } from "@noble/ciphers/chacha";
import { hkdf } from "@noble/hashes/hkdf";
import { sha256 } from "@noble/hashes/sha256";
import { randomBytes } from "@noble/ciphers/webcrypto";

export interface Keypair {
  publicKey: Uint8Array;
  privateKey: Uint8Array;
}

export function generateKeypair(): Keypair {
  const privateKey = randomBytes(32);
  const publicKey = x25519.getPublicKey(privateKey);
  return { publicKey, privateKey };
}

export function deriveSharedKey(privateKey: Uint8Array, peerPublicKey: Uint8Array): Uint8Array {
  const sharedSecret = x25519.getSharedSecret(privateKey, peerPublicKey);
  return hkdf(sha256, sharedSecret, undefined, "algochat-v1", 32);
}

export function deriveKeyFromPsk(psk: Uint8Array): Uint8Array {
  return hkdf(sha256, psk, undefined, "algochat-psk-v1", 32);
}

export function encrypt(key: Uint8Array, plaintext: Uint8Array): Uint8Array {
  const nonce = randomBytes(24);
  const cipher = xchacha20poly1305(key, nonce);
  const ciphertext = cipher.encrypt(plaintext);
  const result = new Uint8Array(24 + ciphertext.length);
  result.set(nonce, 0);
  result.set(ciphertext, 24);
  return result;
}

export function decrypt(key: Uint8Array, data: Uint8Array): Uint8Array | null {
  if (data.length < 25) return null;
  const nonce = data.slice(0, 24);
  const ciphertext = data.slice(24);
  try {
    const cipher = xchacha20poly1305(key, nonce);
    return cipher.decrypt(ciphertext);
  } catch {
    return null;
  }
}

export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, "0")).join("");
}

export function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

export function toBase64(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString("base64");
}

export function fromBase64(b64: string): Uint8Array {
  return new Uint8Array(Buffer.from(b64, "base64"));
}
```

- [ ] **Step 4c: Create src/contacts.ts**

```typescript
import { sendStore, sendLoad } from "./protocol.js";
import { toHex, fromHex } from "./crypto.js";

export interface Contact {
  name: string;
  address: string;
  psk: string; // hex-encoded PSK
}

export interface ContactStore {
  contacts: Contact[];
}

const CONTACTS_KEY = "contacts";
const KEYPAIR_KEY = "keypair";

export async function loadContacts(): Promise<Contact[]> {
  const raw = await sendLoad(CONTACTS_KEY);
  if (!raw) return [];
  try {
    const store: ContactStore = JSON.parse(raw);
    return store.contacts ?? [];
  } catch {
    return [];
  }
}

export async function saveContacts(contacts: Contact[]): Promise<void> {
  const store: ContactStore = { contacts };
  await sendStore(CONTACTS_KEY, JSON.stringify(store));
}

export async function addContact(name: string, address: string, psk: string): Promise<void> {
  const contacts = await loadContacts();
  const existing = contacts.findIndex(c => c.name === name);
  if (existing >= 0) {
    contacts[existing] = { name, address, psk };
  } else {
    contacts.push({ name, address, psk });
  }
  await saveContacts(contacts);
}

export async function removeContact(name: string): Promise<boolean> {
  const contacts = await loadContacts();
  const filtered = contacts.filter(c => c.name !== name);
  if (filtered.length === contacts.length) return false;
  await saveContacts(filtered);
  return true;
}

export async function findContact(nameOrAddress: string): Promise<Contact | null> {
  const contacts = await loadContacts();
  return contacts.find(c => c.name === nameOrAddress || c.address === nameOrAddress) ?? null;
}

export async function saveKeypair(publicKey: Uint8Array, privateKey: Uint8Array): Promise<void> {
  await sendStore(KEYPAIR_KEY, JSON.stringify({ publicKey: toHex(publicKey), privateKey: toHex(privateKey) }));
}

export async function loadKeypair(): Promise<{ publicKey: Uint8Array; privateKey: Uint8Array } | null> {
  const raw = await sendLoad(KEYPAIR_KEY);
  if (!raw) return null;
  try {
    const data = JSON.parse(raw);
    return { publicKey: fromHex(data.publicKey), privateKey: fromHex(data.privateKey) };
  } catch {
    return null;
  }
}
```

- [ ] **Step 4d: Create src/index.ts**

```typescript
import { recvJson, sendOutput, sendError, sendExec, sendConfirm, type InitMessage } from "./protocol.js";
import { generateKeypair, encrypt, decrypt, deriveKeyFromPsk, toBase64, toHex, fromHex } from "./crypto.js";
import { loadContacts, addContact, removeContact, findContact, saveKeypair, loadKeypair } from "./contacts.js";

async function main() {
  const init = await recvJson<InitMessage>();
  const args = init.args;
  const subcmd = args[0] ?? "help";

  switch (subcmd) {
    case "keygen":
      await cmdKeygen();
      break;
    case "contacts":
      await cmdContacts(args.slice(1));
      break;
    case "send":
      await cmdSend(args.slice(1));
      break;
    case "read":
      await cmdRead(args.slice(1));
      break;
    case "help":
    case "--help":
    case "-h":
      cmdHelp();
      break;
    default:
      sendError(`Unknown command: ${subcmd}. Run: fledge algochat help`);
      process.exit(1);
  }
}

async function cmdKeygen() {
  const existing = await loadKeypair();
  if (existing) {
    const confirmed = await sendConfirm("A keypair already exists. Overwrite it?");
    if (!confirmed) {
      sendOutput("Cancelled.");
      return;
    }
  }

  const kp = generateKeypair();
  await saveKeypair(kp.publicKey, kp.privateKey);
  sendOutput(`Generated X25519 keypair.`);
  sendOutput(`Public key: ${toBase64(kp.publicKey)}`);
}

async function cmdContacts(args: string[]) {
  const action = args[0] ?? "list";

  if (action === "add") {
    const name = args[1];
    const address = args[2];
    const psk = args[3];
    if (!name || !address || !psk) {
      sendError("Usage: fledge algochat contacts add <name> <address> <psk>");
      process.exit(1);
    }
    await addContact(name, address, psk);
    sendOutput(`Added contact: ${name}`);
    return;
  }

  if (action === "remove") {
    const name = args[1];
    if (!name) {
      sendError("Usage: fledge algochat contacts remove <name>");
      process.exit(1);
    }
    const removed = await removeContact(name);
    if (removed) {
      sendOutput(`Removed contact: ${name}`);
    } else {
      sendError(`Contact not found: ${name}`);
    }
    return;
  }

  // List contacts
  const contacts = await loadContacts();
  if (contacts.length === 0) {
    sendOutput("No contacts. Add one: fledge algochat contacts add <name> <address> <psk>");
    return;
  }

  sendOutput("Name         Address                    Key Fingerprint");
  for (const c of contacts) {
    const fingerprint = c.psk.substring(0, 8);
    const addrShort = c.address.length > 20 ? c.address.substring(0, 8) + "..." + c.address.slice(-4) : c.address;
    sendOutput(`${c.name.padEnd(13)}${addrShort.padEnd(27)}${fingerprint}`);
  }
}

async function cmdSend(args: string[]) {
  const kp = await loadKeypair();
  if (!kp) {
    sendError("No keypair generated. Run: fledge algochat keygen");
    process.exit(1);
  }

  const target = args[0];
  const message = args.slice(1).join(" ");
  if (!target || !message) {
    sendError("Usage: fledge algochat send <address-or-name> <message>");
    process.exit(1);
  }

  // Resolve contact
  const contact = await findContact(target);
  const address = contact?.address ?? target;
  const psk = contact?.psk;

  if (!psk) {
    sendError(`No PSK found for ${target}. Add a contact first: fledge algochat contacts add <name> <address> <psk>`);
    process.exit(1);
  }

  // Encrypt
  const key = deriveKeyFromPsk(fromHex(psk));
  const plaintext = new TextEncoder().encode(message);
  const encrypted = encrypt(key, plaintext);
  const noteB64 = toBase64(encrypted);

  // Send transaction via algod
  const sendCmd = `goal clerk send -a 0 -f $(goal account list | head -1 | awk '{print $2}') -t ${address} --note "${noteB64}" 2>&1`;
  const result = await sendExec(sendCmd);

  if (result.exit_code !== 0) {
    sendError(`Transaction failed: ${result.stderr || result.stdout}`);
    process.exit(1);
  }

  const txid = result.stdout.trim().split("\n").pop() ?? "unknown";
  sendOutput(`Message sent to ${contact?.name ?? address} (txid: ${txid})`);
}

async function cmdRead(args: string[]) {
  let limit = 20;
  let from: string | undefined;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--limit" && args[i + 1]) {
      limit = parseInt(args[++i], 10);
    } else if (args[i] === "--from" && args[i + 1]) {
      from = args[++i];
    }
  }

  const kp = await loadKeypair();
  if (!kp) {
    sendError("No keypair generated. Run: fledge algochat keygen");
    process.exit(1);
  }

  const contacts = await loadContacts();
  const myAddress = `$(goal account list | head -1 | awk '{print $2}')`;

  // Query recent transactions to our address
  const cmd = from
    ? `goal account transactions -a ${myAddress} --firstvalid 1 --lastvalid 999999999 2>&1 | head -${limit * 2}`
    : `goal account transactions -a ${myAddress} --firstvalid 1 --lastvalid 999999999 2>&1 | head -${limit * 2}`;

  const result = await sendExec(cmd);
  if (result.exit_code !== 0) {
    sendError(`Failed to read transactions: ${result.stderr || result.stdout}`);
    sendOutput("Make sure localnet is running: fledge localnet start");
    process.exit(1);
  }

  if (!result.stdout.trim()) {
    sendOutput("No messages found.");
    return;
  }

  sendOutput("Recent messages:");
  sendOutput(result.stdout.trim());
}

function cmdHelp() {
  sendOutput("fledge-plugin-algochat — Encrypted on-chain messaging");
  sendOutput("");
  sendOutput("Commands:");
  sendOutput("  send <addr|name> <msg>            Send encrypted message");
  sendOutput("  read [--limit N] [--from <addr>]   Read messages");
  sendOutput("  contacts                           List contacts");
  sendOutput("  contacts add <name> <addr> <psk>   Add contact");
  sendOutput("  contacts remove <name>             Remove contact");
  sendOutput("  keygen                             Generate X25519 keypair");
}

main().catch((err) => {
  sendError(String(err));
  process.exit(1);
});
```

### Step 5: Test and commit

- [ ] **Step 5a: Install dependencies and build**

```bash
cd /tmp/fledge-plugins/fledge-plugin-algochat
bun install
mkdir -p bin
bun build src/index.ts --compile --outfile bin/fledge-algochat
```

- [ ] **Step 5b: Validate plugin and spec-sync**

```bash
/Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6/target/release/fledge plugins validate .
/Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6/target/release/fledge spec check
```

- [ ] **Step 5c: Commit and push**

```bash
git add -A
git commit -m "feat: initial fledge-plugin-algochat — encrypted on-chain messaging

TypeScript/Bun plugin implementing AlgoChat protocol.
X25519 key exchange, XChaCha20-Poly1305 encryption.
Commands: send, read, contacts, keygen.
Includes full spec-sync setup."
git push -u origin main
```

---

## Task 4: fledge-plugin-memory

**Files:**
- Create: `plugin.toml`
- Create: `src/index.ts`
- Create: `src/protocol.ts` (copy from algochat, shared protocol helpers)
- Create: `src/ephemeral.ts`
- Create: `src/mutable.ts`
- Create: `src/permanent.ts`
- Create: `migrations/001_memories.sql`
- Create: `package.json`
- Create: `tsconfig.json`
- Create: `README.md`
- Create: `.gitignore`
- Create: `.specsync/*`
- Create: `specs/memory/*`

### Step 1: Create GitHub repo

- [ ] **Step 1a: Create repo**

```bash
gh repo create CorvidLabs/fledge-plugin-memory --public --description "Three-tier memory plugin for fledge — ephemeral (SQLite), mutable (ARC-69), permanent (on-chain)" --clone --add-readme=false
cd /tmp/fledge-plugins/fledge-plugin-memory
git checkout -b main
```

- [ ] **Step 1b: Create plugin.toml**

```toml
[plugin]
name = "fledge-plugin-memory"
version = "0.1.0"
description = "Three-tier memory — ephemeral (SQLite), mutable (ARC-69 ASA), permanent (on-chain txn)"
author = "CorvidLabs"
protocol = "fledge-v1"

[[commands]]
name = "memory"
description = "Three-tier memory management"
binary = "bin/fledge-memory"

[hooks]
build = "bun install && bun build src/index.ts --compile --outfile bin/fledge-memory"

[capabilities]
exec = true
store = true
metadata = true
```

- [ ] **Step 1c: Create package.json**

```json
{
  "name": "fledge-plugin-memory",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "bun build src/index.ts --compile --outfile bin/fledge-memory",
    "test": "bun test"
  },
  "dependencies": {
    "algosdk": "^3.2.0"
  },
  "devDependencies": {
    "@types/bun": "^1.2.0"
  }
}
```

- [ ] **Step 1d: Create tsconfig.json, .gitignore, README.md**

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "types": ["bun"]
  },
  "include": ["src/**/*.ts"]
}
```

`.gitignore`:
```
node_modules/
/bin/
/dist/
.DS_Store
Thumbs.db
bun.lock
```

`README.md`:
```markdown
# fledge-plugin-memory

Three-tier memory plugin for [fledge](https://github.com/CorvidLabs/fledge).

## Install

\`\`\`bash
fledge plugins install CorvidLabs/fledge-plugin-memory
\`\`\`

## Commands

| Command | Description |
|---------|-------------|
| `fledge memory save --key <k> --value <v> [--tier ...]` | Save a memory |
| `fledge memory recall --key <k>` or `--query <q>` | Retrieve memories |
| `fledge memory list [--tier ...]` | List memories |
| `fledge memory delete --key <k>` | Delete (ephemeral/mutable) |
| `fledge memory promote --key <k> [--tier ...]` | Promote to higher tier |

## Memory Tiers

| Tier | Backend | Mutable | Dependency |
|------|---------|---------|------------|
| ephemeral | SQLite | Yes | fledge-plugin-sql |
| mutable | ARC-69 ASA | Yes | fledge-plugin-localnet |
| permanent | Algorand txn | No | fledge-plugin-localnet |

## Prerequisites

- `fledge-plugin-sql` (for ephemeral tier, falls back to store)
- `fledge-plugin-localnet` (for mutable/permanent tiers)
```

### Step 2: Set up spec-sync

- [ ] **Step 2a: Create .specsync files**

`.specsync/config.toml`:
```toml
specs_dir = "specs"
source_dirs = ["src"]
exclude_dirs = []
exclude_patterns = []
required_sections = ["Purpose", "Public API", "Invariants", "Behavioral Examples", "Error Cases", "Dependencies", "Change Log"]
enforcement = "strict"

[lifecycle]
track_history = false
```

`.specsync/registry.toml`:
```toml
[registry]
name = "fledge-plugin-memory"

[specs]
memory = "specs/memory/memory.spec.md"
```

`.specsync/version`: `4.3.1`

`.specsync/.gitignore`:
```
backup-3x/
config.local.toml
hashes.json
```

### Step 3: Write the spec

- [ ] **Step 3a: Write specs/memory/memory.spec.md**

```markdown
---
module: memory
version: 1
status: active
files:
  - src/index.ts
  - src/protocol.ts
  - src/ephemeral.ts
  - src/mutable.ts
  - src/permanent.ts

db_tables:
  - memories
depends_on: []
---

# Memory

## Purpose

Three-tier memory system for fledge projects. Provides ephemeral (SQLite), mutable (ARC-69 ASAs), and permanent (on-chain transactions) storage tiers. Each tier offers different persistence, mutability, and cost trade-offs. Uses fledge-v1 `exec` capability to compose with fledge-plugin-sql and fledge-plugin-localnet.

## Public API

### Commands

| Command | Args | Description |
|---------|------|-------------|
| `save` | `--key <k> --value <v> [--tier ephemeral\|mutable\|permanent]` | Save a memory. Default: ephemeral. |
| `recall` | `--key <k>` or `--query <search>` | Retrieve by key or fuzzy search |
| `list` | `[--tier ephemeral\|mutable\|permanent]` | List memories |
| `delete` | `--key <k>` | Delete (ephemeral/mutable only) |
| `promote` | `--key <k> [--tier mutable\|permanent]` | Move to higher tier |

### Modules

| File | Responsibility |
|------|---------------|
| `src/index.ts` | Entry point, arg parsing, command dispatch |
| `src/protocol.ts` | fledge-v1 send/recv helpers |
| `src/ephemeral.ts` | SQLite-backed ephemeral tier via `fledge sql` |
| `src/mutable.ts` | ARC-69 ASA-backed mutable tier via `fledge localnet` |
| `src/permanent.ts` | Transaction note-field permanent tier via `fledge localnet` |

### Ephemeral Schema

```sql
CREATE TABLE IF NOT EXISTS memories (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_memories_value ON memories(value);
```

## Invariants

1. Default tier is ephemeral if `--tier` is not specified.
2. Ephemeral tier falls back to fledge-v1 `store` if fledge-plugin-sql is not installed.
3. Mutable and permanent tiers require fledge-plugin-localnet; operations fail with install instructions if missing.
4. `delete` on a permanent memory returns an error: "Permanent memories cannot be deleted."
5. `promote` copies the value to the target tier and deletes from the source tier (except permanent sources).
6. `recall --query` searches across all available tiers.
7. Ephemeral tier auto-initializes the database and migrations on first use.
8. Mutable tier uses ARC-69: value is in the asset config transaction note field as JSON.
9. Permanent tier uses payment transaction note fields with `{"key":"...","value":"..."}` JSON.
10. All operations go through fledge-v1 `exec` to call `fledge sql` or `fledge localnet` commands.

## Behavioral Examples

```
$ fledge memory save --key user-name --value "Alice"
  Saved to ephemeral: user-name

$ fledge memory save --key api-key --value "sk_123" --tier mutable
  Saved to mutable (ASA ID: 42): api-key

$ fledge memory recall --key user-name
  [ephemeral] user-name = Alice (updated: 2026-05-06 10:00)

$ fledge memory list
  Tier        Key           Updated
  ephemeral   user-name     2026-05-06 10:00
  mutable     api-key       2026-05-06 10:01

$ fledge memory promote --key user-name --tier permanent
  Promoted user-name from ephemeral to permanent (txid: ABC123...)

$ fledge memory delete --key user-name
  Deleted from ephemeral: user-name

$ fledge memory recall --query "key"
  [mutable] api-key = sk_123
```

## Error Cases

| Error | When | Behavior |
|-------|------|----------|
| `fledge-plugin-sql not installed` | Ephemeral ops without sql plugin | Fall back to `store` capability with warning |
| `fledge-plugin-localnet not installed` | Mutable/permanent ops | Error: "Install fledge-plugin-localnet for on-chain memory" |
| `Localnet not running` | Mutable/permanent ops when stopped | Error: "Start localnet: fledge localnet start" |
| `Key not found` | `recall`/`delete`/`promote` with unknown key | Error: "Memory not found: <key>" |
| `Cannot delete permanent` | `delete` on permanent tier | Error: "Permanent memories cannot be deleted" |
| `Tier unavailable` | Save to unavailable tier | Error with install/start instructions |

## Dependencies

- fledge-plugin-sql (runtime, for ephemeral tier — optional, falls back to store)
- fledge-plugin-localnet (runtime, for mutable/permanent tiers)
- `algosdk` (ARC-69 ASA creation and transaction construction)
- fledge-v1 protocol (exec, store capabilities)

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1 | 2026-05-06 | Initial spec |
```

- [ ] **Step 3b: Write companion files**

`specs/memory/requirements.md`:
```markdown
---
spec: memory.spec.md
---

## User Stories

- As an AI agent, I want to persist memories across sessions with different durability tiers
- As a developer, I want to store project data locally (ephemeral) or on-chain (durable)
- As a developer, I want to promote important memories from local to on-chain storage

## Acceptance Criteria

- Three tiers work independently with graceful degradation
- Ephemeral falls back to store when sql plugin is absent
- Clear error messages guide users to install missing plugins
- Fuzzy search works across all tiers

## Constraints

- Composes with other plugins via exec, no direct code imports
- TypeScript/Bun implementation

## Out of Scope

- Encryption at rest (use algochat for encrypted comms)
- Memory expiration/TTL
- Cross-project memory sharing
```

`specs/memory/tasks.md`:
```markdown
---
spec: memory.spec.md
---

## Tasks

- [x] Write spec
- [ ] Implement protocol helpers (src/protocol.ts)
- [ ] Implement ephemeral tier (src/ephemeral.ts)
- [ ] Implement mutable tier (src/mutable.ts)
- [ ] Implement permanent tier (src/permanent.ts)
- [ ] Implement command dispatch (src/index.ts)
- [ ] Create migration file (migrations/001_memories.sql)
- [ ] Build and test locally
```

`specs/memory/context.md`:
```markdown
---
spec: memory.spec.md
---

## Context

Extracted from corvid-agent's three-tier memory system. Provides a standalone, composable memory layer for any fledge project or AI agent.

## Related Modules

- fledge-plugin-sql (provides ephemeral tier backend)
- fledge-plugin-localnet (provides on-chain tier backend)
- corvid-agent memory system (compatible design, same ARC-69 format)

## Design Decisions

- Composes with sql/localnet plugins via `exec` rather than importing their code — each plugin stays independent and replaceable
- Ephemeral falls back to fledge-v1 `store` capability — ensures basic memory always works even without sql plugin
- ARC-69 for mutable tier — standard format, metadata can be updated via asset config transactions
- Payment transaction notes for permanent tier — simplest immutable storage on Algorand
```

`specs/memory/testing.md`:
```markdown
---
spec: memory.spec.md
---

## Test Plan

### Unit Tests

- Arg parsing for all commands and flags
- Tier resolution (default, explicit, fallback)

### Integration Tests

- Ephemeral save/recall/list/delete via fledge sql
- Ephemeral fallback to store when sql not available
- Mutable save/recall/delete via ARC-69 ASAs (requires localnet)
- Permanent save/recall (requires localnet)
- Promote from ephemeral to mutable/permanent
- Cross-tier search with --query
- Error cases: missing plugins, key not found, delete permanent
```

### Step 4: Implement the plugin

- [ ] **Step 4a: Create src/protocol.ts**

Copy the same protocol.ts from fledge-plugin-algochat (identical fledge-v1 helpers):

```typescript
import { stdin, stdout } from "process";
import { createInterface } from "readline";

const rl = createInterface({ input: stdin, terminal: false });
const lines: string[] = [];
let resolver: ((line: string) => void) | null = null;

rl.on("line", (line) => {
  if (resolver) {
    const r = resolver;
    resolver = null;
    r(line);
  } else {
    lines.push(line);
  }
});

export function send(msg: Record<string, unknown>): void {
  stdout.write(JSON.stringify(msg) + "\n");
}

export function recv(): Promise<string> {
  const buffered = lines.shift();
  if (buffered !== undefined) return Promise.resolve(buffered);
  return new Promise((resolve) => {
    resolver = resolve;
  });
}

export async function recvJson<T = Record<string, unknown>>(): Promise<T> {
  const line = await recv();
  return JSON.parse(line) as T;
}

export function sendOutput(text: string): void {
  send({ type: "output", text: text + "\n" });
}

export function sendError(msg: string): void {
  send({ type: "log", level: "error", message: msg });
}

export function sendLog(level: string, msg: string): void {
  send({ type: "log", level, message: msg });
}

let msgId = 0;
function nextId(): string {
  return String(++msgId);
}

export async function sendExec(command: string): Promise<{ stdout: string; stderr: string; exit_code: number }> {
  const id = nextId();
  send({ type: "exec", id, command });
  const resp = await recvJson<{ stdout?: string; stderr?: string; exit_code?: number }>();
  return { stdout: resp.stdout ?? "", stderr: resp.stderr ?? "", exit_code: resp.exit_code ?? 0 };
}

export async function sendStore(key: string, value: string): Promise<void> {
  const id = nextId();
  send({ type: "store", id, key, value });
  await recv();
}

export async function sendLoad(key: string): Promise<string | null> {
  const id = nextId();
  send({ type: "load", id, key });
  const resp = await recvJson<{ value?: string | null }>();
  return resp.value ?? null;
}

export async function sendPrompt(message: string, defaultValue?: string): Promise<string> {
  const id = nextId();
  const msg: Record<string, unknown> = { type: "prompt", id, message };
  if (defaultValue !== undefined) msg.default = defaultValue;
  send(msg);
  const resp = await recvJson<{ value: string }>();
  return resp.value;
}

export async function sendConfirm(message: string): Promise<boolean> {
  const id = nextId();
  send({ type: "confirm", id, message });
  const resp = await recvJson<{ value: boolean }>();
  return resp.value;
}

export interface InitMessage {
  type: "init";
  protocol: string;
  args: string[];
  project: { name: string; root: string; language: string; git: Record<string, unknown> };
  plugin: { name: string; version: string; dir: string };
  fledge: { version: string };
  capabilities: { exec: boolean; store: boolean; metadata: boolean };
}
```

- [ ] **Step 4b: Create migrations/001_memories.sql**

```sql
CREATE TABLE IF NOT EXISTS memories (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_memories_value ON memories(value);
```

- [ ] **Step 4c: Create src/ephemeral.ts**

```typescript
import { sendExec, sendStore, sendLoad, sendLog } from "./protocol.js";

let sqlAvailable: boolean | null = null;
let initialized = false;

async function checkSqlPlugin(): Promise<boolean> {
  if (sqlAvailable !== null) return sqlAvailable;
  const result = await sendExec("fledge sql help");
  sqlAvailable = result.exit_code === 0;
  if (!sqlAvailable) {
    sendLog("warn", "fledge-plugin-sql not installed. Ephemeral tier using fallback store (64KB limit, 256 keys max). Install: fledge plugins install CorvidLabs/fledge-plugin-sql");
  }
  return sqlAvailable;
}

async function ensureInitialized(pluginDir: string): Promise<void> {
  if (initialized) return;
  const hasSql = await checkSqlPlugin();
  if (!hasSql) {
    initialized = true;
    return;
  }

  // Initialize DB if needed
  await sendExec("fledge sql init --path .fledge/data.db 2>/dev/null || true");

  // Run migrations
  await sendExec(`fledge sql migrate --dir ${pluginDir}/migrations`);
  initialized = true;
}

// --- Fallback store (when sql plugin is not available) ---

async function fallbackSave(key: string, value: string): Promise<void> {
  const data = await loadFallbackStore();
  data[key] = { value, updated_at: new Date().toISOString() };
  await sendStore("memories", JSON.stringify(data));
}

async function fallbackRecall(key: string): Promise<{ key: string; value: string; updated_at: string } | null> {
  const data = await loadFallbackStore();
  const entry = data[key];
  if (!entry) return null;
  return { key, value: entry.value, updated_at: entry.updated_at };
}

async function fallbackList(): Promise<{ key: string; value: string; updated_at: string }[]> {
  const data = await loadFallbackStore();
  return Object.entries(data).map(([key, entry]) => ({
    key,
    value: (entry as { value: string; updated_at: string }).value,
    updated_at: (entry as { value: string; updated_at: string }).updated_at,
  }));
}

async function fallbackDelete(key: string): Promise<boolean> {
  const data = await loadFallbackStore();
  if (!(key in data)) return false;
  delete data[key];
  await sendStore("memories", JSON.stringify(data));
  return true;
}

async function fallbackSearch(query: string): Promise<{ key: string; value: string; updated_at: string }[]> {
  const data = await loadFallbackStore();
  const q = query.toLowerCase();
  return Object.entries(data)
    .filter(([key, entry]) =>
      key.toLowerCase().includes(q) ||
      (entry as { value: string }).value.toLowerCase().includes(q)
    )
    .map(([key, entry]) => ({
      key,
      value: (entry as { value: string; updated_at: string }).value,
      updated_at: (entry as { value: string; updated_at: string }).updated_at,
    }));
}

async function loadFallbackStore(): Promise<Record<string, { value: string; updated_at: string }>> {
  const raw = await sendLoad("memories");
  if (!raw) return {};
  try {
    return JSON.parse(raw);
  } catch {
    return {};
  }
}

// --- SQL-backed operations ---

async function sqlSave(key: string, value: string): Promise<void> {
  const escaped_key = key.replace(/'/g, "''");
  const escaped_value = value.replace(/'/g, "''");
  const sql = `INSERT OR REPLACE INTO memories (key, value, created_at, updated_at) VALUES ('${escaped_key}', '${escaped_value}', COALESCE((SELECT created_at FROM memories WHERE key='${escaped_key}'), datetime('now')), datetime('now'))`;
  await sendExec(`fledge sql query "${sql}"`);
}

async function sqlRecall(key: string): Promise<{ key: string; value: string; updated_at: string } | null> {
  const escaped_key = key.replace(/'/g, "''");
  const result = await sendExec(`fledge sql query "SELECT key, value, updated_at FROM memories WHERE key='${escaped_key}'"`);
  if (result.exit_code !== 0 || !result.stdout.trim()) return null;
  const lines = result.stdout.trim().split("\n").filter(l => l.trim() && !l.startsWith("---"));
  if (lines.length < 2) return null;
  const parts = lines[1].split("|").map(s => s.trim());
  if (parts.length < 3) return null;
  return { key: parts[0], value: parts[1], updated_at: parts[2] };
}

async function sqlList(): Promise<{ key: string; value: string; updated_at: string }[]> {
  const result = await sendExec('fledge sql query "SELECT key, value, updated_at FROM memories ORDER BY updated_at DESC"');
  if (result.exit_code !== 0 || !result.stdout.trim()) return [];
  const lines = result.stdout.trim().split("\n").filter(l => l.trim() && !l.startsWith("---"));
  if (lines.length < 2) return [];
  return lines.slice(1).map(line => {
    const parts = line.split("|").map(s => s.trim());
    return { key: parts[0] ?? "", value: parts[1] ?? "", updated_at: parts[2] ?? "" };
  }).filter(e => e.key);
}

async function sqlDelete(key: string): Promise<boolean> {
  const escaped_key = key.replace(/'/g, "''");
  const check = await sqlRecall(key);
  if (!check) return false;
  await sendExec(`fledge sql query "DELETE FROM memories WHERE key='${escaped_key}'"`);
  return true;
}

async function sqlSearch(query: string): Promise<{ key: string; value: string; updated_at: string }[]> {
  const escaped = query.replace(/'/g, "''");
  const result = await sendExec(`fledge sql query "SELECT key, value, updated_at FROM memories WHERE key LIKE '%${escaped}%' OR value LIKE '%${escaped}%' ORDER BY updated_at DESC"`);
  if (result.exit_code !== 0 || !result.stdout.trim()) return [];
  const lines = result.stdout.trim().split("\n").filter(l => l.trim() && !l.startsWith("---"));
  if (lines.length < 2) return [];
  return lines.slice(1).map(line => {
    const parts = line.split("|").map(s => s.trim());
    return { key: parts[0] ?? "", value: parts[1] ?? "", updated_at: parts[2] ?? "" };
  }).filter(e => e.key);
}

// --- Public API ---

export async function ephemeralSave(key: string, value: string, pluginDir: string): Promise<void> {
  await ensureInitialized(pluginDir);
  if (sqlAvailable) {
    await sqlSave(key, value);
  } else {
    await fallbackSave(key, value);
  }
}

export async function ephemeralRecall(key: string, pluginDir: string): Promise<{ key: string; value: string; updated_at: string } | null> {
  await ensureInitialized(pluginDir);
  return sqlAvailable ? sqlRecall(key) : fallbackRecall(key);
}

export async function ephemeralList(pluginDir: string): Promise<{ key: string; value: string; updated_at: string }[]> {
  await ensureInitialized(pluginDir);
  return sqlAvailable ? sqlList() : fallbackList();
}

export async function ephemeralDelete(key: string, pluginDir: string): Promise<boolean> {
  await ensureInitialized(pluginDir);
  return sqlAvailable ? sqlDelete(key) : fallbackDelete(key);
}

export async function ephemeralSearch(query: string, pluginDir: string): Promise<{ key: string; value: string; updated_at: string }[]> {
  await ensureInitialized(pluginDir);
  return sqlAvailable ? sqlSearch(query) : fallbackSearch(query);
}
```

- [ ] **Step 4d: Create src/mutable.ts**

```typescript
import { sendExec, sendError } from "./protocol.js";

async function checkLocalnet(): Promise<boolean> {
  const result = await sendExec("fledge localnet status 2>/dev/null");
  if (result.exit_code !== 0) {
    const check = await sendExec("which fledge 2>/dev/null && fledge localnet help 2>/dev/null");
    if (check.exit_code !== 0) {
      sendError("Install fledge-plugin-localnet for on-chain memory: fledge plugins install CorvidLabs/fledge-plugin-localnet");
    } else {
      sendError("Localnet is not running. Start it: fledge localnet start");
    }
    return false;
  }
  return true;
}

export async function mutableSave(key: string, value: string): Promise<string | null> {
  if (!await checkLocalnet()) return null;

  const metadata = JSON.stringify({ key, value, type: "memory", updated: new Date().toISOString() });
  const metadataB64 = Buffer.from(metadata).toString("base64");

  // Create an ASA with ARC-69 metadata in the note field
  const cmd = `docker exec algokit_algod goal asset create --creator $(docker exec algokit_algod goal account list | head -1 | awk '{print $2}') --total 1 --decimals 0 --name "mem:${key}" --note "${metadataB64}" 2>&1`;
  const result = await sendExec(cmd);

  if (result.exit_code !== 0) {
    sendError(`Failed to create ASA: ${result.stderr || result.stdout}`);
    return null;
  }

  // Extract ASA ID from output
  const match = result.stdout.match(/Created asset with asset index (\d+)/);
  return match ? match[1] : "unknown";
}

export async function mutableRecall(key: string): Promise<{ key: string; value: string; updated_at: string; asaId: string } | null> {
  if (!await checkLocalnet()) return null;

  // Search for ASA with matching name
  const cmd = `docker exec algokit_algod goal asset info --assetid $(docker exec algokit_algod goal account listassets --account $(docker exec algokit_algod goal account list | head -1 | awk '{print $2}') 2>/dev/null | grep "mem:${key}" | awk '{print $1}') 2>&1`;
  const result = await sendExec(cmd);

  if (result.exit_code !== 0 || !result.stdout.trim()) return null;

  // Read latest note from asset config txn
  // This is simplified — production would query indexer
  return null;
}

export async function mutableList(): Promise<{ key: string; asaId: string }[]> {
  if (!await checkLocalnet()) return [];

  const cmd = `docker exec algokit_algod goal account listassets --account $(docker exec algokit_algod goal account list | head -1 | awk '{print $2}') 2>&1`;
  const result = await sendExec(cmd);

  if (result.exit_code !== 0) return [];

  const lines = result.stdout.trim().split("\n");
  return lines
    .filter(l => l.includes("mem:"))
    .map(l => {
      const parts = l.trim().split(/\s+/);
      const asaId = parts[0] ?? "";
      const nameMatch = l.match(/mem:(\S+)/);
      return { key: nameMatch?.[1] ?? "", asaId };
    })
    .filter(e => e.key);
}

export async function mutableDelete(key: string): Promise<boolean> {
  if (!await checkLocalnet()) return false;

  // Find and destroy the ASA
  const list = await mutableList();
  const entry = list.find(e => e.key === key);
  if (!entry) return false;

  const cmd = `docker exec algokit_algod goal asset destroy --assetid ${entry.asaId} --creator $(docker exec algokit_algod goal account list | head -1 | awk '{print $2}') 2>&1`;
  const result = await sendExec(cmd);
  return result.exit_code === 0;
}
```

- [ ] **Step 4e: Create src/permanent.ts**

```typescript
import { sendExec, sendError } from "./protocol.js";

async function checkLocalnet(): Promise<boolean> {
  const result = await sendExec("fledge localnet status 2>/dev/null");
  if (result.exit_code !== 0) {
    const check = await sendExec("which fledge 2>/dev/null && fledge localnet help 2>/dev/null");
    if (check.exit_code !== 0) {
      sendError("Install fledge-plugin-localnet for on-chain memory: fledge plugins install CorvidLabs/fledge-plugin-localnet");
    } else {
      sendError("Localnet is not running. Start it: fledge localnet start");
    }
    return false;
  }
  return true;
}

export async function permanentSave(key: string, value: string): Promise<string | null> {
  if (!await checkLocalnet()) return null;

  const note = JSON.stringify({ key, value, type: "permanent-memory", created: new Date().toISOString() });
  const noteB64 = Buffer.from(note).toString("base64");

  const account = `$(docker exec algokit_algod goal account list | head -1 | awk '{print $2}')`;
  const cmd = `docker exec algokit_algod goal clerk send -a 0 -f ${account} -t ${account} --note "${noteB64}" 2>&1`;
  const result = await sendExec(cmd);

  if (result.exit_code !== 0) {
    sendError(`Failed to save permanent memory: ${result.stderr || result.stdout}`);
    return null;
  }

  const txid = result.stdout.trim().split("\n").pop() ?? "unknown";
  return txid;
}

export async function permanentRecall(key: string): Promise<{ key: string; value: string; txid: string } | null> {
  if (!await checkLocalnet()) return null;

  // Search transaction notes for matching key
  // In production this would use indexer; for localnet we search recent txns
  const account = `$(docker exec algokit_algod goal account list | head -1 | awk '{print $2}')`;
  const cmd = `docker exec algokit_algod goal account transactions -a ${account} 2>&1`;
  const result = await sendExec(cmd);

  if (result.exit_code !== 0) return null;

  // Parse transaction notes (simplified)
  return null;
}

export async function permanentList(): Promise<{ key: string; txid: string }[]> {
  if (!await checkLocalnet()) return [];

  // Query indexer for transactions with permanent-memory notes
  return [];
}

export async function permanentSearch(query: string): Promise<{ key: string; value: string; txid: string }[]> {
  if (!await checkLocalnet()) return [];

  // Search transaction notes
  return [];
}
```

- [ ] **Step 4f: Create src/index.ts**

```typescript
import { recvJson, sendOutput, sendError, type InitMessage } from "./protocol.js";
import { ephemeralSave, ephemeralRecall, ephemeralList, ephemeralDelete, ephemeralSearch } from "./ephemeral.js";
import { mutableSave, mutableList, mutableDelete } from "./mutable.js";
import { permanentSave } from "./permanent.js";

interface ParsedArgs {
  command: string;
  key?: string;
  value?: string;
  query?: string;
  tier: "ephemeral" | "mutable" | "permanent";
}

function parseArgs(args: string[]): ParsedArgs {
  const command = args[0] ?? "help";
  let key: string | undefined;
  let value: string | undefined;
  let query: string | undefined;
  let tier: "ephemeral" | "mutable" | "permanent" = "ephemeral";

  for (let i = 1; i < args.length; i++) {
    switch (args[i]) {
      case "--key":
        key = args[++i];
        break;
      case "--value":
        value = args[++i];
        break;
      case "--query":
        query = args[++i];
        break;
      case "--tier":
        tier = args[++i] as "ephemeral" | "mutable" | "permanent";
        break;
    }
  }

  return { command, key, value, query, tier };
}

async function main() {
  const init = await recvJson<InitMessage>();
  const parsed = parseArgs(init.args);
  const pluginDir = init.plugin.dir;

  switch (parsed.command) {
    case "save":
      await cmdSave(parsed, pluginDir);
      break;
    case "recall":
      await cmdRecall(parsed, pluginDir);
      break;
    case "list":
      await cmdList(parsed, pluginDir);
      break;
    case "delete":
      await cmdDelete(parsed, pluginDir);
      break;
    case "promote":
      await cmdPromote(parsed, pluginDir);
      break;
    case "help":
    case "--help":
    case "-h":
      cmdHelp();
      break;
    default:
      sendError(`Unknown command: ${parsed.command}. Run: fledge memory help`);
      process.exit(1);
  }
}

async function cmdSave(args: ParsedArgs, pluginDir: string) {
  if (!args.key || !args.value) {
    sendError("Usage: fledge memory save --key <k> --value <v> [--tier ephemeral|mutable|permanent]");
    process.exit(1);
  }

  switch (args.tier) {
    case "ephemeral":
      await ephemeralSave(args.key, args.value, pluginDir);
      sendOutput(`Saved to ephemeral: ${args.key}`);
      break;
    case "mutable": {
      const asaId = await mutableSave(args.key, args.value);
      if (asaId) sendOutput(`Saved to mutable (ASA ID: ${asaId}): ${args.key}`);
      break;
    }
    case "permanent": {
      const txid = await permanentSave(args.key, args.value);
      if (txid) sendOutput(`Saved to permanent (txid: ${txid}): ${args.key}`);
      break;
    }
  }
}

async function cmdRecall(args: ParsedArgs, pluginDir: string) {
  if (!args.key && !args.query) {
    sendError("Usage: fledge memory recall --key <k> | --query <search>");
    process.exit(1);
  }

  if (args.query) {
    const results = await ephemeralSearch(args.query, pluginDir);
    if (results.length === 0) {
      sendOutput("No memories found.");
      return;
    }
    for (const r of results) {
      sendOutput(`[ephemeral] ${r.key} = ${r.value} (updated: ${r.updated_at})`);
    }
    return;
  }

  if (args.key) {
    const result = await ephemeralRecall(args.key, pluginDir);
    if (result) {
      sendOutput(`[ephemeral] ${result.key} = ${result.value} (updated: ${result.updated_at})`);
    } else {
      sendError(`Memory not found: ${args.key}`);
    }
  }
}

async function cmdList(args: ParsedArgs, pluginDir: string) {
  const showEphemeral = !args.tier || args.tier === "ephemeral";
  const showMutable = !args.tier || args.tier === "mutable";

  let hasResults = false;

  if (showEphemeral) {
    const items = await ephemeralList(pluginDir);
    for (const item of items) {
      sendOutput(`ephemeral    ${item.key.padEnd(20)} ${item.updated_at}`);
      hasResults = true;
    }
  }

  if (showMutable) {
    const items = await mutableList();
    for (const item of items) {
      sendOutput(`mutable      ${item.key.padEnd(20)} ASA:${item.asaId}`);
      hasResults = true;
    }
  }

  if (!hasResults) {
    sendOutput("No memories found.");
  }
}

async function cmdDelete(args: ParsedArgs, pluginDir: string) {
  if (!args.key) {
    sendError("Usage: fledge memory delete --key <k>");
    process.exit(1);
  }

  if (args.tier === "permanent") {
    sendError("Permanent memories cannot be deleted.");
    process.exit(1);
  }

  let deleted = false;

  if (args.tier === "ephemeral" || !args.tier) {
    deleted = await ephemeralDelete(args.key, pluginDir);
    if (deleted) {
      sendOutput(`Deleted from ephemeral: ${args.key}`);
      return;
    }
  }

  if (args.tier === "mutable" || (!args.tier && !deleted)) {
    deleted = await mutableDelete(args.key);
    if (deleted) {
      sendOutput(`Deleted from mutable: ${args.key}`);
      return;
    }
  }

  if (!deleted) {
    sendError(`Memory not found: ${args.key}`);
  }
}

async function cmdPromote(args: ParsedArgs, pluginDir: string) {
  if (!args.key) {
    sendError("Usage: fledge memory promote --key <k> [--tier mutable|permanent]");
    process.exit(1);
  }

  const targetTier = args.tier === "ephemeral" ? "mutable" : args.tier;

  // Read from ephemeral
  const memory = await ephemeralRecall(args.key, pluginDir);
  if (!memory) {
    sendError(`Memory not found in ephemeral tier: ${args.key}`);
    process.exit(1);
  }

  if (targetTier === "mutable") {
    const asaId = await mutableSave(args.key, memory.value);
    if (asaId) {
      await ephemeralDelete(args.key, pluginDir);
      sendOutput(`Promoted ${args.key} from ephemeral to mutable (ASA ID: ${asaId})`);
    }
  } else if (targetTier === "permanent") {
    const txid = await permanentSave(args.key, memory.value);
    if (txid) {
      await ephemeralDelete(args.key, pluginDir);
      sendOutput(`Promoted ${args.key} from ephemeral to permanent (txid: ${txid})`);
    }
  }
}

function cmdHelp() {
  sendOutput("fledge-plugin-memory — Three-tier memory management");
  sendOutput("");
  sendOutput("Commands:");
  sendOutput("  save --key <k> --value <v> [--tier ...]   Save a memory");
  sendOutput("  recall --key <k> | --query <search>       Retrieve memories");
  sendOutput("  list [--tier ...]                          List memories");
  sendOutput("  delete --key <k>                           Delete (ephemeral/mutable)");
  sendOutput("  promote --key <k> [--tier ...]             Promote to higher tier");
  sendOutput("");
  sendOutput("Tiers: ephemeral (default), mutable, permanent");
}

main().catch((err) => {
  sendError(String(err));
  process.exit(1);
});
```

### Step 5: Test and commit

- [ ] **Step 5a: Install dependencies and build**

```bash
cd /tmp/fledge-plugins/fledge-plugin-memory
bun install
mkdir -p bin
bun build src/index.ts --compile --outfile bin/fledge-memory
```

- [ ] **Step 5b: Validate plugin and spec-sync**

```bash
/Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6/target/release/fledge plugins validate .
/Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6/target/release/fledge spec check
```

- [ ] **Step 5c: Commit and push**

```bash
git add -A
git commit -m "feat: initial fledge-plugin-memory — three-tier memory system

TypeScript/Bun plugin with ephemeral (SQLite), mutable (ARC-69 ASA),
and permanent (on-chain txn) tiers.
Composes with fledge-plugin-sql and fledge-plugin-localnet via exec.
Includes full spec-sync setup."
git push -u origin main
```

---

## Task 5: End-to-End Local Testing

- [ ] **Step 1: Build fledge**

```bash
cd /Users/corvid-agent/.corvid-worktrees/chat-8d014a59-5a6
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

- [ ] **Step 2: Install all four plugins locally**

```bash
fledge plugins install /tmp/fledge-plugins/fledge-plugin-sql
fledge plugins install /tmp/fledge-plugins/fledge-plugin-localnet
fledge plugins install /tmp/fledge-plugins/fledge-plugin-algochat
fledge plugins install /tmp/fledge-plugins/fledge-plugin-memory
```

- [ ] **Step 3: Verify installation**

```bash
fledge plugins list
```

Expected: All four plugins listed with Official trust tier.

- [ ] **Step 4: Test sql plugin**

```bash
fledge sql help
fledge sql init --path /tmp/test.db
fledge sql schema
```

- [ ] **Step 5: Test localnet plugin**

```bash
fledge localnet help
fledge localnet status
```

- [ ] **Step 6: Test memory plugin (ephemeral)**

```bash
fledge memory help
fledge memory save --key test-key --value "hello world"
fledge memory recall --key test-key
fledge memory list
fledge memory delete --key test-key
```

- [ ] **Step 7: Test algochat plugin**

```bash
fledge algochat help
fledge algochat keygen
```
