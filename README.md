# fledge

[![CI](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml/badge.svg)](https://github.com/CorvidLabs/fledge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/fledge)](https://crates.io/crates/fledge)
[![Downloads](https://img.shields.io/crates/d/fledge)](https://crates.io/crates/fledge)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-brightgreen)](https://corvidlabs.github.io/fledge/)

**One CLI, your whole dev lifecycle.** Scaffold, build, review, ship — zero config for the common case.

```bash
fledge init my-tool --template rust-cli
cd my-tool
fledge lane ci     # lint + test + build, works out of the box
```

## Install

```bash
cargo install fledge              # from crates.io
brew install CorvidLabs/tap/fledge # homebrew
```

<details>
<summary>More install options</summary>

```bash
cargo install fledge --features tui   # with TUI browser
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
fledge init my-app --template rust-cli     # built-in template
fledge init my-app --template user/repo    # any GitHub repo
fledge init my-app                         # interactive picker
```

## What's Inside

| Stage | Commands | What it does |
|-------|----------|-------------|
| **Start** | `init`, `list`, `search`, `create-template` | Scaffold projects from local or remote templates |
| **Build** | `run`, `lane`, `config`, `doctor` | Task runner, workflow pipelines, environment checks |
| **Develop** | `work`, `spec` | Feature branches, PRs, spec-sync |
| **Review** | `review`, `ask`, `metrics`, `deps` | AI code review, codebase Q&A, health checks |
| **Ship** | `issues`, `prs`, `checks`, `changelog` | GitHub integration, CI status, release notes |
| **Extend** | `plugin`, `completions`, `tui` | Community plugins, shell completions, TUI browser |

## Built-in Templates

`rust-cli` · `ts-bun` · `python-cli` · `go-cli` · `ts-node` · `static-site`

Browse community templates: `fledge search <keyword>`

## Examples

- **[Official Templates](https://github.com/CorvidLabs/fledge-templates)** — hello-world, bun-api, ts-lib, and more
- **[Example Lanes](https://github.com/CorvidLabs/fledge-lanes)** — language-specific CI/CD pipelines
- **[Example Plugin](https://github.com/CorvidLabs/fledge-deploy)** — deploy/rollback plugin reference

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
