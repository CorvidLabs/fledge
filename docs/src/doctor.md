# Doctor: Environment Diagnostics

`fledge doctor` checks your environment for issues that might cause problems. Run it when something isn't working right, or proactively before starting a new project.

## Usage

```bash
fledge doctor
fledge doctor --json
```

## What It Checks

Doctor auto-detects your project type and checks relevant tools:

**Toolchain** (project-type-specific):

| Project type | What it checks |
|--------------|----------------|
| Rust | `rustc`, `cargo`, `cargo-clippy`, `rustfmt` |
| Node.js | `node`, `npm` or `yarn` |
| Go | `go` |
| Python | `python3` or `python`, `pip` |
| Ruby | `ruby`, `gem`, `bundler` |
| Java (Gradle) | `java`, `gradle` |
| Java (Maven) | `java`, `mvn` |
| Swift | `swift`, `swiftlint` (optional) |

**Dependencies** — checks for lockfiles and build artifacts (e.g. `Cargo.lock`, `node_modules/`, `go.sum`).

**Git** — `git` installed, repository initialized, remote configured, working tree status.

**AI** — `claude` CLI installed (enables `fledge review` and `fledge ask`).

## Output

Doctor reports each check as passing or failing:

- **Pass** (✅): tool is installed and working, with version info
- **Fail** (❌): tool is missing or errored, with a suggested fix command

## When to Run It

- Before your first `fledge templates init` to make sure your environment is ready
- When `fledge run` can't find the right command for your project type
- When GitHub commands (`issues`, `prs`, `checks`, `work pr`) fail with auth errors
- After upgrading your toolchain or switching machines

## JSON Output

```bash
fledge doctor --json
```

Returns a structured report for scripting or CI checks. Each entry includes the check name, status, version (if applicable), and any diagnostic message.
