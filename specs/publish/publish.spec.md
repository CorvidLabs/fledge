---
module: publish
version: 2
status: active
files:
  - src/publish.rs

db_tables: []
depends_on:
  - config
  - templates
---

# Publish

## Purpose

Publishes a local fledge template directory as a GitHub repository with the `fledge-template` discovery topic, making it discoverable via `fledge templates search`. Validates the content, creates or updates the GitHub repo, and pushes the files. Lanes and plugins have their own publish subcommands that reuse shared helpers from this module.

## Public API

### Exported Functions

| Export | Description |
|--------|-------------|
| `PublishOptions` | Options struct for the publish command |
| `run` | Entry point that validates, creates repo, and pushes template |
| `validate_template` | Checks that directory contains a valid template.toml |
| `get_authenticated_user` | Fetches the GitHub username for the configured token |
| `check_repo_exists` | Checks whether a repo already exists on GitHub |
| `create_github_repo` | Creates a new GitHub repository via the API |
| `set_repo_topics` | Sets repository topics including `fledge-template` (delegates to `set_repo_topic`) |
| `set_repo_topic` | Sets a single topic on a GitHub repository |
| `push_directory` | Initializes git (if needed) and pushes directory contents to GitHub |
| `run_git` | Runs a git command in a given directory |

### Structs & Enums

| Type | Description |
|------|-------------|
| `PublishOptions` | Command options: path to template, optional org, private flag, description override |

## Invariants

1. A valid `template.toml` must exist at the root of the template directory
2. A GitHub token with `repo` scope must be configured via `fledge config set github.token`
3. The `fledge-template` topic is always added to published repos
4. If the repo already exists on GitHub, the user is prompted to confirm update
5. Template name from `template.toml` is used as the repo name unless overridden
6. Files matching template.toml `ignore` patterns are still pushed (they're part of the template)

## Behavioral Examples

### Scenario: Publish a new template
```
Given a directory with a valid template.toml
And a GitHub token is configured
When the user runs `fledge publish ./my-template`
Then a new GitHub repo is created with the template name
And the `fledge-template` topic is set
And the repo description matches template.toml
And all files are pushed
And the user sees the install command
```

### Scenario: Publish under an organization
```
Given a valid template directory
And a GitHub token is configured
When the user runs `fledge publish ./my-template --org CorvidLabs`
Then the repo is created under the CorvidLabs organization
```

### Scenario: Publish with private visibility
```
Given a valid template directory
When the user runs `fledge publish ./my-template --private`
Then the repo is created as private
```

### Scenario: Template already published (repo exists)
```
Given a valid template directory
And a GitHub repo with the same name already exists
When the user runs `fledge publish ./my-template`
Then the user is prompted to confirm the update
And if confirmed, files are pushed to the existing repo
```

### Scenario: No GitHub token configured
```
Given a valid template directory
And no GitHub token is configured
When the user runs `fledge publish`
Then an error is shown guiding the user to `fledge config set github.token <token>`
```

### Scenario: Invalid template
```
Given a directory without template.toml
When the user runs `fledge publish ./bad-dir`
Then an error is shown: "No template.toml found"
```

## Error Cases

| Error | Condition |
|-------|-----------|
| No template.toml | Target directory has no template.toml |
| Invalid template.toml | template.toml cannot be parsed |
| No GitHub token | `github.token` not set in config |
| Repo creation failed | GitHub API returns error (permission, name conflict, etc.) |
| Push failed | Git push fails (auth, network, etc.) |
| Directory not found | Specified path does not exist |

## Dependencies

### Consumes

| Crate/Module | What is used |
|-------------|-------------|
| `ureq` | HTTP client for GitHub API |
| `serde_json` | JSON construction and parsing for API calls |
| `console` | `style` for colored output |
| `anyhow` | Error handling |
| `dialoguer` | `Confirm` for update prompts |
| `config` | `Config::load()`, `github_token()` for authentication |
| `templates` | `TemplateManifest` for template validation |

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 2 | 2026-04-22 | Updated exports for plugin/lane publish support; document newly-public helpers |
| 1 | 2026-04-19 | Initial spec |
