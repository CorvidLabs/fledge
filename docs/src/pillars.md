# The Six Pillars

fledge organizes your dev workflow into six stages. Each one maps to a set of commands, and they flow naturally from project creation to release.

```
Start --> Build --> Develop --> Review --> Ship
                                              \
                       Extend (runs alongside all stages)
```

## Start: Scaffold and discover

Get a project off the ground. Pick a template (built-in, remote, or your own), scaffold it, and you're writing code in under a minute.

**Commands:** `init`, `list`, `search`, `create-template`, `publish`, `validate-template`, `update`

## Build: Configure and run

Define your tasks, wire them into pipelines, set your defaults, and make sure your environment is ready. This is where `fledge.toml` lives.

**Commands:** `run`, `flow`, `config`, `doctor`

## Develop: Branch and spec

Work on features with proper branch isolation and keep your specs in sync with the code.

**Commands:** `work`, `spec`

## Review: Quality and insight

Check your code before it ships. AI review, codebase Q&A, code metrics, and dependency health. All from the terminal.

**Commands:** `review`, `ask`, `metrics`, `deps`

## Ship: Track and release

Manage issues, review PRs, check CI status, and generate changelogs. Everything you need to get code out the door.

**Commands:** `issues`, `prs`, `checks`, `changelog`

## Extend: Grow the tool

Install community plugins, write your own, set up shell completions, and browse templates interactively.

**Commands:** `plugin`, `completions`, `tui`
