# Templates

Templates are how fledge scaffolds projects. They come from three places: built-in, remote repos, and local directories.

## Built-in Templates

These ship with the binary. Always there, no setup needed:

| Template | What it is |
|----------|-----------|
| `rust-cli` | Rust CLI with clap, CI, release automation |
| `ts-bun` | TypeScript on Bun with Biome |
| `python-cli` | Python CLI with Click and Ruff |
| `go-cli` | Go CLI with Cobra |
| `ts-node` | TypeScript on Node with tsx and Biome |
| `static-site` | Vanilla HTML/CSS/JS, no dependencies |
| `kotlin-kmp` | Kotlin Multiplatform library |
| `kotlin-ktor-api` | Kotlin Ktor HTTP API |

For Angular, MCP server, Deno, Swift, monorepo, and more, check the [official template repo](https://github.com/CorvidLabs/fledge-templates):

```bash
fledge templates init my-app --template rust-cli
fledge templates init my-service --template CorvidLabs/fledge-templates/go-cli
```

## Remote Templates

Any GitHub repo can be a template. Just use `owner/repo`:

```bash
fledge templates init my-app --template CorvidLabs/fledge-templates/deno-cli

# Pin to a specific version
fledge templates init my-app --template CorvidLabs/fledge-templates/mcp-server@v1.0
```

Templates get cached locally after the first pull. Use `--refresh` to force a re-download:

```bash
fledge templates init my-app --template CorvidLabs/fledge-templates/deno-cli --refresh
```

### Official Template Collection

[CorvidLabs/fledge-templates](https://github.com/CorvidLabs/fledge-templates) has a growing set of community templates:

| Template | What it is |
|----------|-----------|
| `corvid-agent-skill` | CorvidAgent skill module |
| `deno-cli` | Deno CLI app |
| `mcp-server` | MCP server project |
| `python-api` | FastAPI app |
| `rust-workspace` | Rust workspace with multiple crates |
| `static-site` | Static site (HTML/CSS/JS) |

Add it to your config so these show up in `fledge templates list`:

```bash
fledge config add templates.repos "CorvidLabs/fledge-templates"
```

Or use the preset which sets everything up:

```bash
fledge config init --preset corvidlabs
```

## Local Templates

Point fledge at a directory on disk:

```bash
fledge config add templates.paths "~/my-templates"
```

Or just pass a path directly:

```bash
fledge templates init my-app --template ./path/to/template
```

## Finding Templates

### List What You Have

```bash
fledge templates list    # built-in + configured repos + local paths
```

### Search GitHub

```bash
fledge templates search                  # browse everything
fledge templates search "react"          # filter by keyword
fledge templates search --limit 50
fledge templates search --author CorvidLabs
```

Templates on GitHub use the `fledge-template` topic, that's what `templates search` filters on. Add `--json` for an array of `{owner, name, description, stars, url, topics, trust_tier}`.

## Project Metadata

`fledge templates init` writes `.fledge/meta.toml` to your project root: template source, variable values used during scaffolding, and per-file SHA hashes. This metadata is informational — a community plugin can ingest it if you want template-update tooling.

## Publishing Your Own

```bash
# Start with the skeleton
fledge templates create my-template

# Edit template files and template.toml

# Validate before publishing
fledge templates validate .

# Publish
fledge templates publish --org MyOrg
```

`templates publish` validates the directory through the same gate `templates validate` uses, then creates (or updates) the GitHub repo, tags it with the `fledge-template` topic so it shows up in `templates search`, and force-pushes the directory contents. `--private` for an unlisted repo, `--description <text>` to override the default, `--yes`/`-y` to skip the confirmation prompt.

### Validate Before You Ship

```bash
fledge templates validate .
fledge templates validate . --strict    # warnings become errors
fledge templates validate ./templates   # validate a whole directory
fledge templates validate . --json      # machine-readable output
```

The validator checks for:
- Valid `template.toml` with required fields (`name`, `description`)
- Tera syntax errors in template files
- Undefined variables (not built-in and not in `[prompts]`)
- Render globs that don't match any files
- `template.toml` in the ignore list

You can also just test it:

```bash
fledge templates init test-output --template ./my-template --dry-run
```

For the full format reference, see the [Template Authoring Guide](./template-authoring.md).

## Resolution Order

When you run `fledge templates init --template <name>`, fledge looks in this order:

1. **Exact path** - starts with `.` or `/`
2. **Built-in templates** - the 8 bundled ones (`go-cli`, `kotlin-kmp`, `kotlin-ktor-api`, `python-cli`, `rust-cli`, `static-site`, `ts-bun`, `ts-node`)
3. **Configured repos** - `templates.repos` in your config
4. **Local paths** - `templates.paths` in your config
5. **GitHub shorthand** - treats it as `owner/repo` and fetches it

## Security

> **Warning:** Remote template hooks execute shell commands on your machine. Always review what a template's `post_create` hooks will run before confirming.

Hook consent is split by template provenance:

- **Local templates** (built-in starters and anything under `templates.paths` in your config) are presumed user-authored. `--yes` (or `FLEDGE_NON_INTERACTIVE=1`) auto-confirms their `post_create` hooks.
- **Remote templates** fetched from GitHub require an explicit trust grant. `--yes` does **not** authorize their hooks — pass `--trust-hooks` (or set `FLEDGE_TRUST_HOOKS=1`) to authorize hook execution for the run. Without it, the prompt fires interactively, or hooks are skipped in non-interactive mode with a hint pointing at the right flag (the rest of init still succeeds; `hooks_run: false` in the JSON envelope).

The `--dry-run` path always lists the hooks that would run regardless of trust, so you can audit before consenting.
