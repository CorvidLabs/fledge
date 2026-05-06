# Agent Plugins Design Spec

**Date:** 2026-05-06
**Author:** CorvidAgent + Leif
**Status:** Approved

## Overview

Four fledge plugins that extract agent-facing capabilities from corvid-agent into reusable, standalone tools. Each plugin is a separate GitHub repo under `CorvidLabs/`, installable via `fledge plugins install`, and usable by both humans and AI agents. All four use the fledge-v1 protocol and ship with full spec-sync.

## Plugins

### 1. fledge-plugin-sql

**Repo:** `CorvidLabs/fledge-plugin-sql`
**Language:** Shell (wraps `sqlite3` CLI)
**Protocol:** fledge-v1
**Capabilities:** exec, store, metadata

#### Commands

| Command | Args | Description |
|---------|------|-------------|
| `fledge sql init` | `[--path <db-path>]` | Create a project SQLite DB. Default: `.fledge/data.db`. Stores path via `store` capability. |
| `fledge sql migrate` | `[--dir <migrations-dir>]` | Run `.sql` files from `migrations/` in filename order. Tracks applied migrations in a `_migrations` table. Skips already-applied. |
| `fledge sql query` | `<sql>` | Execute SQL and display results as a formatted table via `output` messages. |
| `fledge sql schema` | | Dump current schema (`SELECT sql FROM sqlite_master`). |

#### Behavior

- `init` creates the directory and database file, then stores the path in plugin-local storage so subsequent commands find it automatically.
- `migrate` creates a `_migrations` table on first run: `(id INTEGER PRIMARY KEY, filename TEXT UNIQUE, applied_at TEXT)`. Scans the migrations directory for `*.sql` files, sorts by filename, skips any already recorded, and executes the rest in a transaction.
- `query` passes SQL to `sqlite3 -header -column <db>` via the `exec` capability and returns output.
- `schema` runs `SELECT sql FROM sqlite_master WHERE type IN ('table','index','view') ORDER BY name` and outputs the result.

#### Prerequisites

- `sqlite3` on PATH (pre-installed on macOS, available via package managers on Linux).

#### Lifecycle Hooks

None.

---

### 2. fledge-plugin-localnet

**Repo:** `CorvidLabs/fledge-plugin-localnet`
**Language:** Shell (wraps `algokit` CLI)
**Protocol:** fledge-v1
**Capabilities:** exec, metadata

#### Commands

| Command | Args | Description |
|---------|------|-------------|
| `fledge localnet start` | | Start Algorand localnet via `algokit localnet start`. |
| `fledge localnet stop` | | Stop localnet via `algokit localnet stop`. |
| `fledge localnet reset` | | Reset localnet to genesis via `algokit localnet reset`. |
| `fledge localnet status` | | Show running state, ports, and network ID. |
| `fledge localnet fund` | `<address> [--amount <microalgos>]` | Dispense Algos from the localnet faucet. Default: 10,000,000 microAlgos (10 ALGO). |
| `fledge localnet accounts` | | List available localnet accounts with balances. |

#### Behavior

- `start`/`stop`/`reset` delegate directly to `algokit localnet` subcommands, streaming output via `progress` messages.
- `status` checks if Docker containers are running, queries algod for network info, and reports ports (algod: 4001, KMD: 4002, indexer: 8980).
- `fund` uses `goal clerk send` from the default localnet faucet account to the target address.
- `accounts` lists KMD wallet accounts with their ALGO balances via `goal account list`.

#### Prerequisites

- `algokit` on PATH (provides localnet management).
- Docker running (algokit localnet uses Docker containers).
- `goal` is available inside the algod Docker container; the plugin execs into the container.

#### Lifecycle Hooks

| Hook | Behavior |
|------|----------|
| `post_work_start` | If `fledge.toml` contains `[localnet]` section, auto-start localnet. |

---

### 3. fledge-plugin-algochat

**Repo:** `CorvidLabs/fledge-plugin-algochat`
**Language:** TypeScript (Bun runtime)
**Protocol:** fledge-v1
**Capabilities:** exec, store, metadata

#### Commands

| Command | Args | Description |
|---------|------|-------------|
| `fledge algochat send` | `<address-or-name> <message>` | Encrypt and send a message on-chain via Algorand transaction. Accepts contact name or raw address. |
| `fledge algochat read` | `[--limit N] [--from <address>]` | Read and decrypt incoming messages. Default limit: 20. |
| `fledge algochat contacts` | | List all PSK contacts (name, address, key fingerprint). |
| `fledge algochat contacts add` | `<name> <address> <psk>` | Add a contact with a pre-shared key. |
| `fledge algochat contacts remove` | `<name>` | Remove a contact. |
| `fledge algochat keygen` | | Generate an X25519 keypair for message encryption. Stores in plugin-local storage. |

#### Encryption Protocol

Matches corvid-agent's AlgoChat protocol:

1. **Key exchange:** X25519 (Curve25519 Diffie-Hellman)
2. **Encryption:** XChaCha20-Poly1305 (AEAD)
3. **Message format:** `[nonce (24 bytes)][ciphertext]` stored in Algorand transaction note field
4. **PSK contacts:** Pre-shared symmetric keys for agents/users who have exchanged keys out-of-band
5. **Key derivation:** HKDF-SHA256 from shared secret to produce encryption key

#### Dependencies

- npm: `@noble/curves` (X25519), `@noble/ciphers` (XChaCha20-Poly1305), `algosdk` (Algorand transactions)
- Requires either `fledge localnet status` (checks for running localnet) or environment variables (`ALGOD_SERVER`, `ALGOD_TOKEN`, `ALGOD_PORT`) for a remote node.

#### Storage

- Contacts and keys stored via fledge-v1 `store` capability. PSK values are stored as hex strings; the store is plugin-local (not shared with other plugins).
- Messages live on-chain; `read` queries the algod/indexer API.
- The plugin's own X25519 keypair (generated by `keygen`) is also stored via `store`. Users should treat the plugin storage directory as sensitive.

#### Lifecycle Hooks

None.

---

### 4. fledge-plugin-memory

**Repo:** `CorvidLabs/fledge-plugin-memory`
**Language:** TypeScript (Bun runtime)
**Protocol:** fledge-v1
**Capabilities:** exec, store, metadata

#### Commands

| Command | Args | Description |
|---------|------|-------------|
| `fledge memory save` | `--key <k> --value <v> [--tier ephemeral\|mutable\|permanent]` | Save a memory. Default tier: ephemeral. |
| `fledge memory recall` | `--key <k>` or `--query <search>` | Retrieve by exact key or fuzzy text search. |
| `fledge memory list` | `[--tier ephemeral\|mutable\|permanent]` | List all memories, optionally filtered by tier. |
| `fledge memory delete` | `--key <k>` | Delete a memory. Ephemeral and mutable tiers only. |
| `fledge memory promote` | `--key <k> [--tier mutable\|permanent]` | Promote a memory from a lower tier to a higher one. Default target: mutable. |

#### Three-Tier Architecture

| Tier | Backend | Mutable | Persistence | Plugin Dependency |
|------|---------|---------|-------------|-------------------|
| **Ephemeral** | SQLite (via `fledge sql`) | Full CRUD | Per-project, local disk | `fledge-plugin-sql` |
| **Mutable** | ARC-69 ASAs on Algorand | Yes (asset config txn updates metadata) | On-chain, survives reinstalls | `fledge-plugin-localnet` |
| **Permanent** | Plain Algorand note-field txns | No (immutable once sent) | On-chain, forever | `fledge-plugin-localnet` |

#### Graceful Degradation

- If `fledge-plugin-sql` is not installed: ephemeral tier falls back to the fledge-v1 `store` capability (limited to 64KB values, 256 keys max). A warning is shown recommending sql plugin installation.
- If `fledge-plugin-localnet` is not installed: mutable and permanent tiers return an error with install instructions. Ephemeral still works.
- If localnet is installed but not running: mutable/permanent operations prompt to start it.

#### Ephemeral Schema

```sql
CREATE TABLE IF NOT EXISTS memories (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_memories_value ON memories(value);
```

The plugin ships a `migrations/001_memories.sql` file. On first use of the ephemeral tier, it runs `fledge sql init` (if no DB exists) followed by `fledge sql migrate --dir <plugin-dir>/migrations` via the `exec` capability. The DB path is stored/loaded via `fledge sql`'s own store.

#### Mutable Tier (ARC-69 ASAs)

- `save --tier mutable` creates an ASA where the note field contains the key and the ARC-69 metadata JSON contains the value.
- `recall` reads the latest asset config transaction's ARC-69 metadata.
- `delete` destroys the ASA (clawback to creator, then destroy).
- Follows the ARC-69 standard: metadata is a JSON object in the most recent asset config transaction note field.

#### Permanent Tier (Note-Field Transactions)

- `save --tier permanent` sends a zero-value payment transaction with `{"key": "<k>", "value": "<v>"}` JSON in the note field.
- `recall` searches transaction history for matching key via indexer.
- `delete` returns an error: "Permanent memories cannot be deleted."
- `promote` from ephemeral to permanent sends the transaction and removes the ephemeral record.

#### Fuzzy Search (`--query`)

- Ephemeral: `SELECT * FROM memories WHERE key LIKE '%query%' OR value LIKE '%query%'`
- Mutable: Iterates created ASAs and matches against ARC-69 metadata
- Permanent: Searches transaction notes via indexer API

#### Lifecycle Hooks

None.

---

## Cross-Plugin Dependencies

```
fledge-plugin-memory
├── fledge-plugin-sql       (ephemeral tier)
└── fledge-plugin-localnet  (mutable + permanent tiers)

fledge-plugin-algochat
└── fledge-plugin-localnet  (or direct algod config)
```

Dependencies are runtime, not build-time. Memory and algochat use fledge-v1 `exec` to invoke `fledge sql` / `fledge localnet` commands. No plugin imports another's code.

## Spec-Sync Structure

Each plugin repo ships with its own spec-sync setup:

```
fledge-plugin-<name>/
├── .specsync/
│   ├── config.toml          # specs_dir = "specs", required_sections, enforcement = "strict"
│   ├── registry.toml        # single spec entry
│   └── version              # spec-sync version
├── specs/
│   └── <name>/
│       ├── <name>.spec.md   # Main spec (7 required sections)
│       ├── requirements.md  # User stories, acceptance criteria
│       ├── tasks.md         # Implementation tasks
│       ├── context.md       # Design decisions and motivation
│       └── testing.md       # Test plan
├── plugin.toml
├── bin/ or src/
├── README.md
└── .gitignore
```

Each spec follows fledge's standard format: YAML frontmatter (`module`, `version`, `status`, `files`, `db_tables`, `depends_on`) plus the 7 required markdown sections (Purpose, Public API, Invariants, Behavioral Examples, Error Cases, Dependencies, Change Log).

## Build & Test Strategy

### Shell plugins (sql, localnet)

- No build step needed — shell scripts are directly executable.
- Test locally: `fledge plugins create fledge-plugin-sql`, populate, then `fledge plugins validate .` and manual CLI testing.
- Validation: `fledge spec check` in each repo.

### TypeScript plugins (algochat, memory)

- Each TS plugin is a single binary that dispatches subcommands based on the `args` array in the fledge-v1 `init` message (e.g., `args: ["save", "--key", "foo"]`).
- Build: `bun install && bun build src/index.ts --compile --outfile bin/fledge-<name>`
- `plugin.toml` build hook: `bun install && bun build src/index.ts --compile --outfile bin/fledge-<name>`
- Test: `bun test` for unit tests, manual CLI testing for integration.
- Validation: `fledge plugins validate .` and `fledge spec check`.

## Installation

```bash
# Install individually
fledge plugins install CorvidLabs/fledge-plugin-sql
fledge plugins install CorvidLabs/fledge-plugin-localnet
fledge plugins install CorvidLabs/fledge-plugin-algochat
fledge plugins install CorvidLabs/fledge-plugin-memory

# Or as a bundle (future: add to DEFAULT_PLUGINS)
fledge plugins install CorvidLabs/fledge-plugin-sql CorvidLabs/fledge-plugin-localnet CorvidLabs/fledge-plugin-algochat CorvidLabs/fledge-plugin-memory
```

## Trust Tier

All four plugins are under `CorvidLabs` org, so they automatically receive **Official** trust tier. No capability restrictions.

## Implementation Order

1. **fledge-plugin-sql** — standalone, no deps, simplest
2. **fledge-plugin-localnet** — standalone, wraps existing CLI
3. **fledge-plugin-algochat** — needs localnet or algod, has crypto
4. **fledge-plugin-memory** — depends on sql + localnet, most complex
