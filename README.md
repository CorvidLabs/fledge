# fledge

[![CI](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml/badge.svg)](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/fledge)](https://crates.io/crates/fledge)
[![Downloads](https://img.shields.io/crates/d/fledge)](https://crates.io/crates/fledge)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-brightgreen)](https://corvidlabs.github.io/fledge/)

> **fledge: one Rust binary, six pillars, spec-driven by default. Templates scaffold, lanes run, plugins extend, spec-sync keeps the docs honest about the code — and any LLM drives the same CLI you do.**

```bash
fledge templates init my-tool --template rust-cli
cd my-tool
fledge lanes run ci  # lint + test + build, works out of the box
```

> **Working with AI agents?** fledge has a first-class agent surface: every read command exposes `--json`, `FLEDGE_NON_INTERACTIVE=1` silences every prompt, `fledge ask` and `fledge review` are automatically spec-aware, and `fledge introspect --json` dumps the full command tree. Works with Claude CLI or any Ollama-speaking endpoint (local, cloud, or self-hosted). See [AGENTS.md](./AGENTS.md) for the one-page guide.

## Install

```bash
cargo install fledge                       # from crates.io
brew install CorvidLabs/tap/fledge         # homebrew

fledge plugins install --defaults          # one line to install the curated plugin set
```

<details>
<summary>More install options</summary>

```bash
curl -fsSL https://raw.githubusercontent.com/CorvidLabs/fledge/main/install.sh | sh
nix run github:CorvidLabs/fledge
git clone https://github.com/CorvidLabs/fledge.git && cd fledge && cargo install --path .
```

</details>

## Quick Start

**Already have a project?** Just use it — fledge auto-detects your stack:

```bash
fledge run test       # runs your language's test command
fledge run build      # same for build
fledge review         # single-model AI code review
fledge review --with-model ollama:gpt-oss:120b-cloud,ollama:qwen3-coder:480b-cloud
              # multi-model panel — same diff, parallel critiques, one merge decision
```

**Starting fresh?** Scaffold from a template:

```bash
fledge templates init my-app --template rust-cli     # built-in template
fledge templates init my-app --template user/repo    # any GitHub repo
fledge templates init my-app                         # interactive picker
```

**Switch AI providers in one line:**

```bash
fledge ai use                                  # interactive picker (live model list for Ollama)
fledge ai use ollama qwen3-coder:480b-cloud    # scriptable
fledge ai status                               # shows provider, model, and where each value came from
```

## The Six Pillars

| Pillar | Commands | What it does |
|--------|----------|-------------|
| **Scaffold** | `templates` (`init`, `create`, `validate`, `list`) | Local templates pillar — start any project |
| **Run** | `run`, `lanes`, `watch` | Task runner, composable pipelines, file-watch reruns |
| **Spec** | `spec` | spec-sync — modules declare their contract; AI uses it as context |
| **AI** | `ai`, `ask`, `review` | Provider+model selection, spec-aware Q&A, single- and multi-model code review |
| **Ship** | `work`, `release`, `changelog` | Branch + PR flow with AI-drafted bodies, version bump, tag, push |
| **Extend** | `plugins`, `config`, `introspect`, `completions`, `doctor` | Plugin protocol, global config, command-tree introspection, env health |

That's the whole core. **Anything else is a plugin.** See `fledge plugins install --defaults` below.

## Default Plugins

The plugins that took over commands removed from core in v0.15. Install all five with one command:

```bash
fledge plugins install --defaults
```

| Plugin | Adds | Replaces (pre-v0.15) |
|--------|------|----------------------|
| [`fledge-plugin-github`](https://github.com/CorvidLabs/fledge-plugin-github) | `checks`, `issues`, `prs` | the GitHub-specific browsing trio |
| [`fledge-plugin-deps`](https://github.com/CorvidLabs/fledge-plugin-deps) | `deps` | polyglot lockfile audits |
| [`fledge-plugin-metrics`](https://github.com/CorvidLabs/fledge-plugin-metrics) | `metrics` | LOC/churn/test-ratio (now via `tokei` + `git`) |
| [`fledge-plugin-templates-remote`](https://github.com/CorvidLabs/fledge-plugin-templates-remote) | `templates-search`, `templates-publish` | GitHub template registry |
| [`fledge-plugin-doctor`](https://github.com/CorvidLabs/fledge-plugin-doctor) | `doctor-tools` | toolchain probes (rust/node/python/swift/...) |

Why split them out? Because not every fledge user is on GitHub, runs a polyglot project, or cares about LOC counts. The core stays tight; you opt in to what you need.

## Built-in Templates

`rust-cli` · `ts-bun` · `python-cli` · `go-cli` · `ts-node` · `static-site`

Browse community templates with the remote-templates plugin: `fledge templates-search <keyword>`

## Examples

- **[Community Templates](https://github.com/CorvidLabs/fledge-templates)** — 18 ready-to-use templates (angular-app, bun-api, deno-cli, mcp-server, rust-workspace, swift-pkg, and more)
- **[Example Lanes](https://github.com/CorvidLabs/fledge-lanes)** — language-specific CI/CD pipelines
- **[Example Plugin](https://github.com/CorvidLabs/fledge-plugin-deploy)** — deploy/rollback plugin reference

## Learn More

- **[Full Documentation](https://corvidlabs.github.io/fledge/)** — commands, configuration, guides
- **[Template Authoring](https://corvidlabs.github.io/fledge/template-authoring.html)** — create and publish your own templates
- **[Lanes Guide](https://corvidlabs.github.io/fledge/lanes.html)** — task pipelines and workflow automation
- **[Plugins Guide](https://corvidlabs.github.io/fledge/plugins.html)** — extend fledge with community tools

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, guidelines, and how to submit changes.

## Security

See [SECURITY.md](SECURITY.md) for the security policy and how to report vulnerabilities.

## License

MIT
