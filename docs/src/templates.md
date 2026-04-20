# Templates

Fledge uses templates to scaffold new projects. Templates come from three sources: built-in, remote repositories, and local directories.

## Built-in Templates

These ship with the fledge binary — always available, no configuration needed:

| Template | Description |
|----------|-------------|
| `rust-cli` | Rust CLI application with clap |
| `rust-lib` | Rust library crate |
| `go-cli` | Go CLI application |
| `python-cli` | Python CLI with argparse |
| `ts-bun` | TypeScript project with Bun runtime |
| `angular-app` | Angular application |
| `swift-pkg` | Swift package |
| `monorepo` | Multi-project monorepo structure |

Use them directly:

```bash
fledge init my-app --template rust-cli
fledge init my-service --template go-cli
```

## Remote Templates

Any GitHub repository can be a template source. Use `owner/repo` syntax:

```bash
# From a specific repo
fledge init my-app --template CorvidLabs/fledge-templates/deno-cli

# Pin to a version tag
fledge init my-app --template CorvidLabs/fledge-templates/mcp-server@v1.0
```

Remote templates are cached locally after the first download. Use `--refresh` to force a re-download:

```bash
fledge init my-app --template CorvidLabs/fledge-templates/deno-cli --refresh
```

### The fledge-templates Repository

[CorvidLabs/fledge-templates](https://github.com/CorvidLabs/fledge-templates) is the official community template collection. It includes:

| Template | Description |
|----------|-------------|
| `corvid-agent-skill` | CorvidAgent skill module |
| `deno-cli` | Deno CLI application |
| `mcp-server` | MCP server project |
| `python-api` | Python FastAPI application |
| `rust-workspace` | Rust workspace with multiple crates |
| `static-site` | Static site with HTML/CSS/JS |

Add it to your config so these templates appear in `fledge list`:

```bash
fledge config add templates.repos "CorvidLabs/fledge-templates"
```

Or use the CorvidLabs preset which sets this up automatically:

```bash
fledge config init --preset corvidlabs
```

## Local Templates

Point fledge at directories on your filesystem:

```bash
fledge config add templates.paths "~/my-templates"
```

Or use a path directly:

```bash
fledge init my-app --template ./path/to/template
```

## Discovering Templates

### Search GitHub

Find templates published by the community using the `fledge-template` topic:

```bash
fledge search                  # browse all
fledge search "react"          # search by keyword
fledge search --limit 50       # more results
```

### List Available

See all templates you have access to (built-in + configured repos + local paths):

```bash
fledge list
```

## Publishing Your Own

### Quick Start

```bash
# Scaffold a template skeleton
fledge create-template my-template

# Edit template files and template.toml

# Publish to GitHub
fledge publish --org MyOrg
```

### Make It Discoverable

Add the `fledge-template` topic to your GitHub repository. This makes it appear in `fledge search` results.

### Validate Before Publishing

```bash
# Basic validation
fledge validate-template .

# Strict mode (warnings are errors)
fledge validate-template . --strict
```

The validator checks:
- `template.toml` exists and parses correctly
- Required fields (`name`, `description`) are present
- All `.tera` files have valid syntax
- Variables used in templates are defined (built-in or via `[prompts]`)
- `files.render` globs match actual files
- `template.toml` is in the ignore list

For details on the template format, see the [Template Authoring Guide](./template-authoring.md).

## Template Resolution Order

When you run `fledge init --template <name>`, fledge searches in this order:

1. **Exact path** — if the name is a file path (starts with `.` or `/`)
2. **Built-in templates** — the 8 templates bundled with fledge
3. **Configured repos** — repositories listed in `templates.repos`
4. **Local paths** — directories listed in `templates.paths`
5. **GitHub shorthand** — treated as `owner/repo` and fetched directly

## Security

Remote template hooks (`post_create` commands) require explicit confirmation before running. This prevents untrusted templates from executing arbitrary commands. Pass `--yes` to skip the confirmation prompt if you trust the source.
