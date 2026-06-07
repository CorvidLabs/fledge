# fledge

[![CI](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml/badge.svg)](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/fledge)](https://crates.io/crates/fledge)
[![Downloads](https://img.shields.io/crates/d/fledge)](https://crates.io/crates/fledge)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-brightgreen)](https://corvidlabs.github.io/fledge/)

One CLI for the dev loop. Any language. JSON by default. Read the docs and go.

```bash
fledge templates init my-tool --template rust-cli
cd my-tool
fledge lanes init
fledge lanes run ci
```

Working with AI agents? See [AGENTS.md](./AGENTS.md). Every command emits `{schema_version: 1, ...}`, `FLEDGE_NON_INTERACTIVE=1` silences prompts, `fledge ask` and `fledge review` are spec-aware, and `fledge introspect --json` dumps the full command tree. Talks to the Anthropic API, any OpenAI-compatible endpoint (OpenAI, OpenRouter, Groq, ...), or any Ollama endpoint (local, cloud, or self-hosted) over plain HTTP. No CLI required.

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
| Spec | `spec` | [spec-sync](https://github.com/CorvidLabs/spec-sync). Modules declare their contract, AI uses it as context |
| AI | `ai`, `ask`, `review` | Provider/model selection, spec-aware Q&A, single and multi-model code review |
| Ship | `work`, `release`, `changelog` | Branch and PR flow with AI-drafted bodies, version bump, tag, push |
| Extend | `plugins`, `config`, `introspect`, `completions`, `doctor` | Plugin protocol, global config, command-tree introspection, env health |

That is the whole core. Anything else is a plugin.

## Plugins

Plugins extend fledge with community-built commands. Native plugins run as regular executables. **WASM plugins** run in a sandboxed Wasmtime runtime with no host access by default.

```bash
fledge plugins install --defaults          # curated native plugin set
fledge plugins create my-lint --wasm       # scaffold a sandboxed WASM plugin
```

### Default plugins

Three native plugins ship as the default set:

| Plugin | Adds |
|--------|------|
| [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github) | `checks`, `issues`, `prs`. GitHub PR/issue/CI flow |
| [`fledge-plugin-deps`](https://github.com/CorvidLabs/fledge-plugin-deps) | `deps`. Polyglot lockfile audits |
| [`fledge-plugin-metrics`](https://github.com/CorvidLabs/fledge-plugin-metrics) | `metrics`. LOC, churn, test/source ratio (via `tokei` + `git`) |

### WASM plugins

WASM plugins are ideal for pure-computation tasks (linting, formatting, analysis) where you want strong isolation without trusting arbitrary binaries:

- Sandboxed by default. No filesystem, no network
- Opt-in capabilities prompted at install time
- Fuel-bounded execution (no infinite loops)
- 256 MB memory cap
- Cross-platform single `.wasm` binary

See the [WASM plugin guide](https://corvidlabs.github.io/fledge/docs/reference/wasm-plugins) for authoring details.

## Built-in templates

`rust-cli`, `ts-bun`, `python-cli`, `go-cli`, `ts-node`, `static-site`, `kotlin-kmp`, `kotlin-ktor-api`

Browse community templates: `fledge templates search <keyword>`

## Examples

- [Community templates](https://github.com/CorvidLabs/fledge-templates). A growing collection covering Angular, Bun APIs, Deno CLIs, MCP servers, Rust workspaces, Swift packages, and more
- [Example lanes](https://github.com/CorvidLabs/fledge-lanes). Language-specific CI/CD pipelines
- [Example plugin](https://github.com/CorvidLabs/fledge-plugin-deploy). Deploy/rollback plugin reference

## Learn more

- [Full documentation](https://corvidlabs.github.io/fledge/). Commands, configuration, guides
- [Template authoring](https://corvidlabs.github.io/fledge/docs/resources/template-authoring). How to create and publish your own templates
- [Lanes guide](https://corvidlabs.github.io/fledge/docs/lanes). Task pipelines and workflow automation
- [Plugins guide](https://corvidlabs.github.io/fledge/docs/plugins). Extend fledge with community tools
- [WASM plugins](https://corvidlabs.github.io/fledge/docs/reference/wasm-plugins). Build sandboxed plugins with Wasmtime

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, guidelines, and how to submit changes.

## Security

See [SECURITY.md](SECURITY.md) for the security policy and how to report vulnerabilities.

## License

MIT
