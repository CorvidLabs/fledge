# fledge

Get your projects ready to fly.

A fast, opinionated project scaffolding CLI built in Rust. Create new projects from templates — local or remote — with smart defaults, Tera-powered rendering, and zero boilerplate.

## Why fledge?

- **Fast** — native Rust binary, no runtime dependencies
- **Smart defaults** — pulls author/org from git config, renders dates, computes name variants automatically
- **Remote templates** — use any GitHub repo as a template source with `owner/repo` syntax
- **Extensible** — create your own templates with a simple `template.toml` manifest
- **Safe** — remote template hooks require explicit confirmation before running
- **Optional TUI** — interactive template browser with `--features tui`

## Install

```bash
# From crates.io
cargo install fledge

# With TUI support
cargo install fledge --features tui

# From source
git clone https://github.com/CorvidLabs/fledge.git
cd fledge && cargo install --path .
```

## Quick Start

```bash
# Create a new Rust CLI project
fledge init my-tool --template rust-cli

# Browse templates interactively
fledge init my-project

# Use a remote GitHub template
fledge init my-app --template CorvidLabs/fledge-templates/react-app

# Preview what would be created
fledge init my-tool --template rust-cli --dry-run

# Skip all prompts with defaults
fledge init my-tool --template rust-cli --yes

# List available templates
fledge list
```

## Built-in Templates

| Template | Description |
|----------|-------------|
| `rust-cli` | Rust CLI application with clap, CI, and release automation |
| `rust-lib` | Rust library crate with docs and publishing workflow |
| `swift-pkg` | Swift package with Package.swift, CI, and coding conventions |
| `ts-bun` | TypeScript project with Bun runtime |
| `angular-app` | Angular application with mobile-first setup |

## CLI Reference

### `fledge init <name>`

Create a new project from a template.

```
fledge init <name> [OPTIONS]

Arguments:
  <name>              Project name

Options:
  -t, --template      Template to use (skip interactive selection)
  -o, --output        Parent directory for the project [default: .]
      --no-git        Skip git init and initial commit
      --no-install    Skip dependency installation (post-create hooks)
      --refresh       Force re-clone of cached remote templates
      --dry-run       Show what would be created without writing anything
  -y, --yes           Skip all confirmation prompts (accept defaults)
```

### `fledge list`

List all available templates (built-in + configured).

### `fledge tui` *(requires `--features tui`)*

Interactive terminal UI for browsing templates and scaffolding projects. Navigate with arrow keys, fill in variables with Tab, confirm with Enter.

```
fledge tui [OPTIONS]

Options:
  -o, --output        Parent directory for the project [default: .]
      --no-git        Skip git init and initial commit
```

### `fledge completions <shell>`

Generate shell completions for your shell. Supported: `bash`, `zsh`, `fish`, `powershell`.

```bash
# Bash
fledge completions bash >> ~/.bashrc

# Zsh
fledge completions zsh > ~/.zfunc/_fledge

# Fish
fledge completions fish > ~/.config/fish/completions/fledge.fish
```

## Remote Templates

Any GitHub repository can be a template source. Use `owner/repo` syntax:

```bash
# Use a single-template repo
fledge init my-app --template user/my-template

# Use a specific template from a collection
fledge init my-app --template CorvidLabs/templates/python-api

# Force re-download of a cached template
fledge init my-app --template user/my-template --refresh
```

Remote templates are cloned and cached locally. Post-create hooks from remote templates always require confirmation unless `--yes` is passed.

### Template Repositories

You can register template repos in your config so they appear in `fledge list`:

```toml
# ~/.config/fledge/config.toml
[templates]
repos = ["CorvidLabs/fledge-templates", "myorg/templates"]
```

## Configuration

fledge reads from `~/.config/fledge/config.toml`:

```toml
[defaults]
author = "Your Name"
github_org = "YourOrg"
license = "MIT"           # default license for new projects

[templates]
paths = ["~/my-templates"]                     # additional local template directories
repos = ["CorvidLabs/fledge-templates"]         # GitHub repos to include in template list

[github]
token = "ghp_..."         # for private template repos (also reads FLEDGE_GITHUB_TOKEN / GITHUB_TOKEN env vars)
```

If `author` is not set, fledge falls back to `git config user.name`. The GitHub token is checked in order: `FLEDGE_GITHUB_TOKEN` env var → `GITHUB_TOKEN` env var → config file.

## Creating Templates

A template is a directory with a `template.toml` manifest and any number of files. Files are rendered through [Tera](https://keats.github.io/tera/) (a Jinja2-like engine) before being written.

### Directory Structure

```
my-template/
├── template.toml          # manifest (required)
├── src/
│   └── main.rs            # template files — Tera syntax supported
├── README.md
├── Cargo.toml
└── .github/
    └── workflows/
        └── ci.yml
```

### template.toml Reference

```toml
[template]
name = "my-template"                           # template name (used in --template flag)
description = "A short description"            # shown in fledge list
min_fledge_version = "0.1.0"                   # optional minimum fledge version

[prompts]
# Each key becomes a template variable. Values have `message` and optional `default`.
description = { message = "Project description", default = "A new project" }
port = { message = "Default port", default = "3000" }

# Defaults can use Tera expressions referencing earlier variables:
repo_url = { message = "Repository URL", default = "https://github.com/{{ github_org }}/{{ project_name }}" }

[files]
render = ["**/*.rs", "**/*.toml", "**/*.md", "**/*.yml"]   # files to render through Tera
copy = ["**/*.png", "**/*.ico"]                             # files to copy as-is (binary files)
ignore = ["template.toml"]                                  # files to exclude from output

[hooks]
post_create = ["cargo fmt", "npm install"]     # commands to run after scaffolding
```

### Built-in Variables

These are always available in your templates — no need to define them in `[prompts]`:

| Variable | Description | Example |
|----------|-------------|---------|
| `project_name` | The project name as provided by the user | `my-cool-app` |
| `project_name_snake` | Snake case version | `my_cool_app` |
| `project_name_pascal` | PascalCase version | `MyCoolApp` |
| `author` | From config, git, or prompted | `Leif` |
| `github_org` | From config or prompted (default: `CorvidLabs`) | `CorvidLabs` |
| `license` | From config (default: `MIT`) | `MIT` |
| `year` | Current year | `2026` |
| `date` | Current date | `2026-04-18` |

### Tera Syntax

Templates use [Tera](https://keats.github.io/tera/docs/) syntax:

```
# {{ project_name }}

{{ description }}

## Author

Created by {{ author }} ({{ github_org }}) in {{ year }}.

{% if license == "MIT" %}
This project is MIT licensed.
{% endif %}
```

### File Rules

- **`render`** — glob patterns for files that should be processed through Tera. Template variables (`{{ project_name }}`, etc.) are replaced with actual values.
- **`copy`** — glob patterns for files that should be copied as-is. Use for binary files (images, fonts) that would break if parsed.
- **`ignore`** — glob patterns for files to exclude from the output entirely. `template.toml` should always be listed here.

Files not matching any rule are rendered by default.

### Post-Create Hooks

Commands listed in `hooks.post_create` run inside the newly created project directory after all files are written. Use them for dependency installation, formatting, or other setup:

```toml
[hooks]
post_create = ["bun install", "bun run format"]
```

For local (built-in) templates, hooks run automatically. For remote templates, fledge shows the commands and asks for confirmation before running — unless `--yes` is passed.

### Testing Templates Locally

Point fledge at your template directory during development:

```bash
# Add your template directory to config
# ~/.config/fledge/config.toml
[templates]
paths = ["~/dev/my-templates"]

# Or use it directly as a remote-style path
fledge init test-project --template ./my-template
```

Then iterate: edit template files, run `fledge init test-output --template my-template`, inspect the output, delete and repeat.

### Example: Creating a Template from Scratch

```bash
mkdir my-template && cd my-template
```

Create `template.toml`:

```toml
[template]
name = "python-api"
description = "Python FastAPI project with Docker"

[prompts]
description = { message = "Project description", default = "A FastAPI application" }
python_version = { message = "Python version", default = "3.12" }

[files]
render = ["**/*.py", "**/*.toml", "**/*.md", "**/*.yml", "Dockerfile"]
ignore = ["template.toml"]

[hooks]
post_create = ["python -m venv .venv"]
```

Create template files using Tera variables:

```python
# app/main.py
"""{{ description }}"""
from fastapi import FastAPI

app = FastAPI(title="{{ project_name_pascal }}")

@app.get("/")
def root():
    return {"name": "{{ project_name }}"}
```

```dockerfile
# Dockerfile
FROM python:{{ python_version }}-slim
WORKDIR /app
COPY . .
RUN pip install -e .
CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0"]
```

Test it:

```bash
fledge init test-api --template python-api --dry-run
fledge init test-api --template python-api
```

## License

MIT
