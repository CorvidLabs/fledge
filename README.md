# fledge

[![CI](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml/badge.svg)](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/fledge)](https://crates.io/crates/fledge)
[![Downloads](https://img.shields.io/crates/d/fledge)](https://crates.io/crates/fledge)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-brightgreen)](https://corvidlabs.github.io/fledge/)

**One CLI, your whole dev lifecycle.** Scaffold, build, review, ship — zero config for the common case.

```bash
fledge templates init my-tool --template rust-cli
cd my-tool
fledge lanes run ci  # lint + test + build, works out of the box
```

## Install

```bash
cargo install fledge              # from crates.io
brew install CorvidLabs/tap/fledge # homebrew
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
fledge review         # AI code review via Claude
```

**Starting fresh?** Scaffold from a template:

```bash
fledge templates init my-app --template rust-cli     # built-in template
fledge templates init my-app --template user/repo    # any GitHub repo
fledge templates init my-app                         # interactive picker
```

## What's Inside

| Stage | Commands | What it does |
|-------|----------|-------------|
| **Start** | `templates` (`init`, `create`, `search`, `publish`, `validate`, `update`, `list`) | Scaffold projects from local or remote templates |
| **Build** | `run`, `lanes`, `config`, `doctor` | Task runner, workflow pipelines, environment checks |
| **Develop** | `work`, `spec` | Feature branches, PRs, spec-sync |
| **Review** | `review`, `ask`, `metrics`, `deps` | AI code review, codebase Q&A, health checks |
| **Ship** | `issues`, `prs`, `checks`, `changelog`, `release` | GitHub integration, CI status, releases |
| **Extend** | `plugins`, `completions` | Community plugins, shell completions |

## Built-in Templates

`rust-cli` · `ts-bun` · `python-cli` · `go-cli` · `ts-node` · `static-site`

Browse community templates: `fledge templates search <keyword>`

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
