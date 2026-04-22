# Template Authoring Guide

How to build your own fledge templates.

## Overview

A template is a directory with a `template.toml` manifest and whatever files you want. Files get rendered through [Tera](https://keats.github.io/tera/) (Jinja2-style) before being written to the output.

### Directory Structure

```
my-template/
├── template.toml          # manifest (required)
├── src/
│   └── main.rs            # template files, Tera syntax works here
├── README.md
├── Cargo.toml
└── .github/
    └── workflows/
        └── ci.yml
```

## template.toml Reference

This is where you define metadata, prompts, file rules, and hooks.

### Basic Structure

```toml
[template]
name = "my-template"
description = "A short description"
min_fledge_version = "0.1.0"          # optional

[prompts]
description = { message = "Project description", default = "A new project" }
port = { message = "Default port", default = "3000" }

# Defaults can reference earlier variables:
repo_url = { message = "Repository URL", default = "https://github.com/{{ github_org }}/{{ project_name }}" }

[files]
render = ["**/*.rs", "**/*.toml", "**/*.md", "**/*.yml"]
copy = ["**/*.png", "**/*.ico"]
ignore = ["template.toml"]

[hooks]
post_create = ["cargo fmt", "npm install"]
```

### [template] section

| Key | Type | Required | Notes |
|-----|------|----------|-------|
| `name` | string | Yes | What you pass to `--template` |
| `description` | string | No | Shows up in `fledge templates list` |
| `min_fledge_version` | string | No | Minimum fledge version needed |

### [prompts] section

Each key becomes a template variable that gets prompted to the user.

```toml
[prompts]
# With a default
description = { message = "Project description", default = "A new project" }

# No default, user has to answer
main_author = { message = "Primary author" }

# Default can reference other variables
repo_url = { message = "Repository URL", default = "https://github.com/{{ github_org }}/{{ project_name }}" }
```

### [files] section

Controls which files get rendered, copied, or skipped. Rules apply in order, first match wins.

- **`render`** - process through Tera
- **`copy`** - copy as-is (for binary files, images, etc.)
- **`ignore`** - skip entirely

Anything not matching a rule gets rendered by default.

```toml
[files]
render = ["**/*.rs", "**/*.toml", "**/*.md", "**/*.yml"]
copy = ["**/*.png", "**/*.ico", "assets/**"]
ignore = ["template.toml", "node_modules/**"]
```

### [hooks] section

Commands that run after scaffolding, inside the new project directory.

```toml
[hooks]
post_create = ["cargo fmt", "cargo test", "git add -A && git commit -m 'Initial commit'"]
```

Built-in templates run hooks automatically. Remote templates show the commands and ask for confirmation (unless you pass `--yes`).

## Built-in Variables

These are always available. You don't need to define them in `[prompts]`:

| Variable | What it is | Example |
|----------|-----------|---------|
| `project_name` | Name as the user typed it | `my-cool-app` |
| `project_name_snake` | Snake case | `my_cool_app` |
| `project_name_pascal` | PascalCase | `MyCoolApp` |
| `author` | From config, git, or prompted | `Leif` |
| `github_org` | From config or prompted | `CorvidLabs` |
| `license` | From config, defaults to `MIT` | `MIT` |
| `year` | Current year | `2026` |
| `date` | Current date | `2026-04-18` |

## Tera Syntax

Templates use [Tera](https://keats.github.io/tera/docs/) syntax. Here's the stuff you'll actually use:

### Variables

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

### Putting it together

````
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
````

## Building a Template from Scratch

### 1. Make the directory

```bash
mkdir python-api && cd python-api
```

### 2. Write template.toml

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

### 3. Add your files

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

### 4. Test it

```bash
fledge templates init test-api --template ./python-api --dry-run
fledge templates init test-api --template ./python-api
```

## Testing

Add your template directory to config:

```toml
# ~/.config/fledge/config.toml
[templates]
paths = ["~/dev/my-templates"]
```

Or point at it directly:

```bash
fledge templates init test-project --template ./my-template
```

The loop is: edit files → `fledge templates init test-output --template my-template` → check the output → delete test output → repeat.

## Sharing

### GitHub

Push your template to a GitHub repo. Anyone can use it with:

```bash
fledge templates init my-app --template user/my-template
```

### Template Repos

Users can register your repo so it shows up in `fledge templates list`:

```toml
[templates]
repos = ["CorvidLabs/fledge-templates", "myorg/templates"]
```

### Tips

- **Name it clearly.** `python-api` beats `template-1`.
- **Write a README.** Explain what the template does and what variables it uses.
- **Default everything you can.** Only prompt for things that actually vary.
- **Test your hooks.** Make sure `post_create` commands handle missing tools gracefully.
- **Use `min_fledge_version`** if you depend on newer features.
