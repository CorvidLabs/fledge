# CLI Reference

Complete reference for all fledge commands and options.

## fledge init `<name>`

Create a new project from a template.

### Usage

```
fledge init <name> [OPTIONS]
```

### Arguments

- `<name>` — Project name

### Options

- `-t, --template <TEMPLATE>` — Template to use (skip interactive selection)
- `-o, --output <OUTPUT>` — Parent directory for the project [default: `.`]
- `--no-git` — Skip git init and initial commit
- `--no-install` — Skip dependency installation (post-create hooks)
- `--refresh` — Force re-clone of cached remote templates
- `--dry-run` — Show what would be created without writing anything
- `-y, --yes` — Skip all confirmation prompts (accept defaults)

### Examples

```bash
# Create with defaults
fledge init my-tool --template rust-cli

# Preview before creating
fledge init my-app --template react-app --dry-run

# Skip all prompts
fledge init my-lib --template rust-lib --yes

# Specify output directory
fledge init my-project --template ts-bun -o ~/projects
```

---

## fledge list

List all available templates (built-in + configured).

### Usage

```
fledge list
```

Shows template name, description, and source (built-in or configured repo).

---

## fledge tui

Interactive terminal UI for browsing templates and scaffolding projects.

*Requires `--features tui` at install time.*

### Usage

```
fledge tui [OPTIONS]
```

### Options

- `-o, --output <OUTPUT>` — Parent directory for the project [default: `.`]
- `--no-git` — Skip git init and initial commit

### Navigation

- **Arrow keys** — Browse templates
- **Tab** — Fill in project variables
- **Enter** — Confirm and create

The TUI provides an interactive experience for users who prefer guided project creation.

---

## fledge completions `<shell>`

Generate shell completions for your shell.

### Usage

```
fledge completions <shell>
```

### Supported Shells

- `bash`
- `zsh`
- `fish`
- `powershell`

### Examples

```bash
# Bash
fledge completions bash >> ~/.bashrc

# Zsh
fledge completions zsh > ~/.zfunc/_fledge

# Fish
fledge completions fish > ~/.config/fish/completions/fledge.fish

# PowerShell
fledge completions powershell | Out-String | Out-File -FilePath $PROFILE -Append
```

After adding completions, reload your shell or start a new terminal session to enable them.
