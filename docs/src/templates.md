# Templates

Templates are how fledge scaffolds projects. They come from three places: built-in, remote repos, and local directories.

## Built-in Templates

These ship with the binary — always there, no setup needed:

| Template | What it is |
|----------|-----------|
| `rust-cli` | Rust CLI with clap |
| `rust-lib` | Rust library crate |
| `go-cli` | Go CLI app |
| `python-cli` | Python CLI with argparse |
| `ts-bun` | TypeScript on Bun |
| `angular-app` | Angular app |
| `swift-pkg` | Swift package |
| `monorepo` | Multi-project monorepo |

```bash
fledge init my-app --template rust-cli
fledge init my-service --template go-cli
```

## Remote Templates

Any GitHub repo can be a template. Just use `owner/repo`:

```bash
fledge init my-app --template CorvidLabs/fledge-templates/deno-cli

# Pin to a specific version
fledge init my-app --template CorvidLabs/fledge-templates/mcp-server@v1.0
```

Templates get cached locally after the first pull. Use `--refresh` to force a re-download:

```bash
fledge init my-app --template CorvidLabs/fledge-templates/deno-cli --refresh
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

Add it to your config so these show up in `fledge list`:

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
fledge init my-app --template ./path/to/template
```

## Finding Templates

### Search GitHub

Templates on GitHub use the `fledge-template` topic:

```bash
fledge search                  # browse everything
fledge search "react"          # filter by keyword
fledge search --limit 50
```

### List What You Have

```bash
fledge list    # built-in + configured repos + local paths
```

## Publishing Your Own

```bash
# Start with the skeleton
fledge create-template my-template

# Edit template files and template.toml

# Ship it
fledge publish --org MyOrg
```

Add the `fledge-template` topic to your GitHub repo so it shows up in search results.

### Validate Before You Ship

```bash
fledge validate-template .
fledge validate-template . --strict    # warnings become errors
fledge validate-template ./templates   # validate a whole directory
fledge validate-template . --json      # machine-readable output
```

The validator checks for:
- Valid `template.toml` with required fields (`name`, `description`)
- Tera syntax errors in template files
- Undefined variables (not built-in and not in `[prompts]`)
- Render globs that don't match any files
- `template.toml` in the ignore list

You can also just test it:

```bash
fledge init test-output --template ./my-template --dry-run
```

For the full format reference, see the [Template Authoring Guide](./template-authoring.md).

## Resolution Order

When you run `fledge init --template <name>`, fledge looks in this order:

1. **Exact path** — starts with `.` or `/`
2. **Built-in templates** — the 8 bundled ones
3. **Configured repos** — `templates.repos` in your config
4. **Local paths** — `templates.paths` in your config
5. **GitHub shorthand** — treats it as `owner/repo` and fetches it

## Security

Hooks from remote templates (`post_create` commands) always ask for confirmation before running. This way random templates can't execute whatever they want on your machine. Pass `--yes` if you trust the source and want to skip the prompt.
