---
spec: deps.spec.md
---

## User Stories

- As a developer, I want to list all my project dependencies so I can audit what's in my dependency tree
- As a developer, I want to check for outdated dependencies so I can keep my project up to date
- As a security-conscious developer, I want to run a security audit so I can identify vulnerable dependencies
- As a compliance engineer, I want to scan dependency licenses so I can verify they meet my organization's policy

## Acceptance Criteria

- `fledge deps` lists all dependencies from the project's lock file with name and version
- `fledge deps --outdated` shells out to the ecosystem's outdated checker
- `fledge deps --audit` shells out to the ecosystem's security audit tool
- `fledge deps --licenses` shells out to the ecosystem's license scanner
- `fledge deps --json` outputs a structured JSON report with ecosystem, source file, and dependency array
- Dependencies are sorted alphabetically by name
- Supported ecosystems: Rust (Cargo.lock), Node (package-lock.json, yarn.lock), Go (go.sum), Python (requirements.txt, Pipfile.lock, poetry.lock), Ruby (Gemfile.lock)
- Java (Gradle/Maven) gracefully reports that lock file parsing is unsupported and suggests --outdated or --audit
- pnpm-lock.yaml is detected but not parsed, with guidance to use pnpm commands directly
- Missing lock files produce an error with the install command to generate one
- Missing ecosystem tools produce a clear error with install guidance

## Constraints

- Lock file parsing is offline -- no network access required for basic dependency listing
- Ecosystem tool invocations (outdated, audit, licenses) require those tools to be installed separately
- Project type detection is delegated to `run::detect_project_type`

## Out of Scope

- Transitive dependency tree visualization
- Automatic dependency updates or PRs
- pnpm YAML lock file parsing
- Java lock file parsing (Gradle/Maven)
