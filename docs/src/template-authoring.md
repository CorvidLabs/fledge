# Template Authoring Guide

Create your own templates to share with your team or the community.

## Overview

A template is a directory with a `template.toml` manifest and any number of template files. Files are rendered through [Tera](https://keats.github.io/tera/) (a Jinja2-like engine) before being written.

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

## template.toml Reference

The `template.toml` manifest defines template metadata, prompts, file rules, and hooks.

### Basic Structure

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

### Section: template

| Key | Type | Required | Notes |
|-----|------|----------|-------|
| `name` | string | Yes | Used with `--template` flag |
| `description` | string | No | Shown in `fledge list` |
| `min_fledge_version` | string | No | Minimum fledge version required |

### Section: prompts

Define custom variables that will be prompted to the user. Each prompt becomes a template variable.

```toml
[prompts]
# Simple prompt with default
description = { message = "Project description", default = "A new project" }

# Prompt without default (required)
main_author = { message = "Primary author" }

# Default can reference earlier variables
repo_url = { message = "Repository URL", default = "https://github.com/{{ github_org }}/{{ project_name }}" }
```

### Section: files

Define which files to render, copy, or ignore.

**Rules are applied in order and first match wins.**

- **`render`** — glob patterns for files that should be processed through Tera
- **`copy`** — glob patterns for files that should be copied as-is (binary files, images)
- **`ignore`** — glob patterns for files to exclude entirely

Files not matching any rule are rendered by default.

```toml
[files]
render = ["**/*.rs", "**/*.toml", "**/*.md", "**/*.yml"]
copy = ["**/*.png", "**/*.ico", "assets/**"]
ignore = ["template.toml", "node_modules/**"]
```

### Section: hooks

Commands to run after scaffolding is complete, inside the newly created project directory.

```toml
[hooks]
post_create = ["cargo fmt", "cargo test", "git add -A && git commit -m 'Initial commit'"]
```

For local (built-in) templates, hooks run automatically. For remote templates, fledge shows the commands and asks for confirmation unless `--yes` is passed.

## Built-in Variables

These variables are always available in your templates — no need to define them in `[prompts]`:

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

## Tera Syntax

Templates use [Tera](https://keats.github.io/tera/docs/) syntax. Common patterns:

### Variable Substitution

```
# {{ project_name }}

{{ description }}
```

### Conditionals

```
{% if license == "MIT" %}
This project is MIT licensed.
{% endif %}
```

### Loops

```
{% for dep in dependencies %}
- {{ dep }}
{% endfor %}
```

### Filters

```
Project slug: {{ project_name | slugify }}
Uppercase: {{ author | upper }}
```

### Complete Example

```
# {{ project_name }}

{{ description }}

## Author

Created by {{ author }} ({{ github_org }}) in {{ year }}.

{% if license == "MIT" %}
This project is MIT licensed.
{% endif %}

## Quick Start

```
cd {{ project_name_snake }}
cargo build
```
```

## Creating a Template from Scratch

### 1. Create the template directory

```bash
mkdir python-api && cd python-api
```

### 2. Create template.toml

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

### 3. Create template files

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

### 4. Test locally

```bash
fledge init test-api --template ./python-api --dry-run
fledge init test-api --template ./python-api
```

## Testing Templates

### Point to Local Template Directory

Add your template directory to config:

```toml
# ~/.config/fledge/config.toml
[templates]
paths = ["~/dev/my-templates"]
```

Or use it directly as a path:

```bash
fledge init test-project --template ./my-template
```

### Iterate and Test

1. Edit template files
2. Run `fledge init test-output --template my-template`
3. Inspect the output
4. Delete test output and repeat

## Sharing Templates

### Use a GitHub Repository

Any GitHub repository can be a template source. Use the `owner/repo` syntax:

```bash
fledge init my-app --template user/my-template
```

### Register Template Repositories

Users can register your template repo in their config to have it appear in `fledge list`:

```toml
# ~/.config/fledge/config.toml
[templates]
repos = ["CorvidLabs/fledge-templates", "myorg/templates"]
```

### Best Practices

- **Clear names** — use descriptive template names
- **Good docs** — include a README explaining what the template does
- **Sensible defaults** — prompt for what's necessary, provide defaults for everything else
- **Test hooks** — ensure post-create hooks are idempotent and handle missing tools gracefully
- **Version requirements** — use `min_fledge_version` to signal breaking changes
