# fledge

[![CI](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml/badge.svg)](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/fledge)](https://crates.io/crates/fledge)
[![Downloads](https://img.shields.io/crates/d/fledge)](https://crates.io/crates/fledge)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-brightgreen)](https://corvidlabs.github.io/fledge/)

One Rust binary that runs the dev loop. Six pillars: scaffold, run, spec, AI, ship, extend. Plugins handle anything ecosystem-specific. Every command emits `--json` so an LLM can drive the same CLI you do.

```bash
fledge templates init my-tool --template rust-cli
cd my-tool
fledge lanes run ci
```

Working with AI agents? See [AGENTS.md](./AGENTS.md). Every command emits `{schema_version: 1, ...}`, `FLEDGE_NON_INTERACTIVE=1` silences prompts, `fledge ask` and `fledge review` are spec-aware, and `fledge introspect --json` dumps the full command tree. Works with Claude CLI or any Ollama endpoint (local, cloud, or self-hosted).

## Install

```bash
cargo install fledge                       # crates.io
brew install CorvidLabs/tap/fledge         # homebrew

fledge plugins install --defaults          # curated plugin set
```

<details>
<summary>More install options</summary>

```bash
curl -fsSL https://raw.githubusercontent.com/CorvidLabs/fledge/main/install.sh | sh
nix run github:CorvidLabs/fledge
git clone https://github.com/CorvidLabs/fledge.git && cd fledge && cargo install --path .
```

</details>

## Quick start

Already have a project? `cd` into it, fledge auto-detects the stack:

```bash
fledge run test       # runs your language's test command
fledge run build      # same for build
fledge review         # AI code review against the default branch
fledge review --with-model ollama:gpt-oss:120b-cloud,ollama:qwen3-coder:480b-cloud
              # multi-model panel, parallel critiques on the same diff
```

Starting fresh? Scaffold from a template:

```bash
fledge templates init my-app --template rust-cli     # built-in
fledge templates init my-app --template user/repo    # any GitHub repo
fledge templates init my-app                         # interactive picker
```

Switch AI providers without editing config:

```bash
fledge ai use                                  # interactive picker (live Ollama model list)
fledge ai use ollama qwen3-coder:480b-cloud    # scriptable
fledge ai status                               # show active provider/model and where each value came from
```

## The six pillars

| Pillar | Commands | What it does |
|--------|----------|-------------|
| Scaffold | `templates` (`init`, `create`, `list`, `search`, `validate`, `publish`) | Start a project from a template, local or remote |
| Run | `run`, `lanes`, `watch` | Task runner, composable pipelines, file-watch reruns |
| Spec | `spec` | spec-sync. Modules declare their contract, AI uses it as context |
| AI | `ai`, `ask`, `review` | Provider/model selection, spec-aware Q&A, single and multi-model code review |
| Ship | `work`, `release`, `changelog` | Branch and PR flow with AI-drafted bodies, version bump, tag, push |
| Extend | `plugins`, `config`, `introspect`, `completions`, `doctor` | Plugin protocol, global config, command-tree introspection, env health |

That is the whole core. Anything else is a plugin.

## Default plugins

Three plugins extend fledge with commands that don't belong in every install. One line installs them all:

```bash
fledge plugins install --defaults
```

| Plugin | Adds | Replaces (pre-v0.15) |
|--------|------|----------------------|
| [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github) | `checks`, `issues`, `prs` | the GitHub-specific browsing trio |
| [`fledge-plugin-deps`](https://github.com/CorvidLabs/fledge-plugin-deps) | `deps` | polyglot lockfile audits |
| [`fledge-plugin-metrics`](https://github.com/CorvidLabs/fledge-plugin-metrics) | `metrics` | LOC, churn, test/source ratio (via `tokei` + `git`) |

Not every fledge user is on GitHub or runs a polyglot project. The core stays tight, you opt in to what you need.

(`fledge-plugin-templates-remote` and `fledge-plugin-doctor` were dropped from the default set in v0.15.2 and re-absorbed into core. They're now `fledge templates search`/`publish` and the `Toolchains` section of `fledge doctor`. The standalone plugin repos still exist but are no longer part of `--defaults`.)

## Built-in templates

`rust-cli`, `ts-bun`, `python-cli`, `go-cli`, `ts-node`, `static-site`

Browse community templates: `fledge templates search <keyword>`

## Examples

- [Community templates](https://github.com/CorvidLabs/fledge-templates). 18 ready-to-use templates (angular-app, bun-api, deno-cli, mcp-server, rust-workspace, swift-pkg, and more)
- [Example lanes](https://github.com/CorvidLabs/fledge-lanes). Language-specific CI/CD pipelines
- [Example plugin](https://github.com/CorvidLabs/fledge-plugin-deploy). Deploy/rollback plugin reference

## Learn more

- [Full documentation](https://corvidlabs.github.io/fledge/). Commands, configuration, guides
- [Template authoring](https://corvidlabs.github.io/fledge/template-authoring.html). How to create and publish your own templates
- [Lanes guide](https://corvidlabs.github.io/fledge/lanes.html). Task pipelines and workflow automation
- [Plugins guide](https://corvidlabs.github.io/fledge/plugins.html). Extend fledge with community tools

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, guidelines, and how to submit changes.

## Security

See [SECURITY.md](SECURITY.md) for the security policy and how to report vulnerabilities.

## License

MIT
