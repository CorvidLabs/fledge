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

### Search GitHub

Templates on GitHub use the `fledge-template` topic:

```bash
fledge templates search                  # browse everything
fledge templates search "react"          # filter by keyword
fledge templates search --limit 50
fledge templates search --author CorvidLabs
```

### List What You Have

```bash
fledge templates list    # built-in + configured repos + local paths
```

## Updating Projects

When a template gets updated, you can re-apply it to an existing project:

```bash
fledge templates update              # re-apply source template
fledge templates update --dry-run    # preview what would change
fledge templates update --refresh    # force re-clone remote template
```

fledge tracks file hashes in `.fledge/meta.toml` to detect changes:
- **New files** from the template are always added.
- **User-modified files** are skipped with a warning (your changes are preserved).
- **Unmodified files** are updated to the latest template version.
- **Removed files** in the new template version trigger a warning.

Projects created with older fledge versions using `.fledge.toml` are automatically migrated to `.fledge/meta.toml` on the first update.

## Publishing Your Own

```bash
# Start with the skeleton
fledge templates create my-template

# Edit template files and template.toml

# Ship it
fledge templates publish --org MyOrg
```

Add the `fledge-template` topic to your GitHub repo so it shows up in search results.

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
2. **Built-in templates** - the 6 bundled ones
3. **Configured repos** - `templates.repos` in your config
4. **Local paths** - `templates.paths` in your config
5. **GitHub shorthand** - treats it as `owner/repo` and fetches it

## Security

Hooks from remote templates (`post_create` commands) always ask for confirmation before running. This way random templates can't execute whatever they want on your machine. Pass `--yes` if you trust the source and want to skip the prompt.
