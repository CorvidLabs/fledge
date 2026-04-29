use anyhow::{bail, Context, Result};
use console::style;
use std::fs;
use std::path::Path;

use super::PLUGINS_CREATE_SCHEMA;

pub(crate) fn create_plugin(
    name: &str,
    output: &Path,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let yes = yes || crate::utils::is_non_interactive() || json;
    let target = output.join(name);

    if target.exists() {
        bail!("Directory '{}' already exists", target.display());
    }

    let desc = if yes || !crate::utils::is_interactive() {
        description.unwrap_or("A fledge plugin").to_string()
    } else {
        let theme = dialoguer::theme::ColorfulTheme::default();
        dialoguer::Input::with_theme(&theme)
            .with_prompt("Description")
            .default(description.unwrap_or("A fledge plugin").to_string())
            .interact_text()?
    };

    std::fs::create_dir_all(target.join("bin"))
        .with_context(|| format!("creating {}/bin", target.display()))?;

    let plugin_toml = format!(
        r#"[plugin]
name = {name:?}
version = "0.1.0"
description = {desc:?}
# author = "your-name"

[[commands]]
name = {name:?}
description = {desc:?}
binary = "bin/{name}"

[hooks]
# build = "cargo build --release"
# post_install = "hooks/post-install.sh"

[capabilities]
exec = false
store = false
metadata = false
"#,
    );
    fs::write(target.join("plugin.toml"), plugin_toml).context("writing plugin.toml")?;

    let script = format!(
        r#"#!/usr/bin/env bash
# fledge plugin entry point.
#
# fledge sets FLEDGE_PLUGIN_DIR to this plugin's source directory before
# invoking your binary. Use it to reach sibling files in `bin/`, hooks,
# fixtures, etc. Don't use `dirname "$0"` — the binary fledge invokes is
# a symlink in a shared bin/, so $0 won't point to your repo.
set -euo pipefail
PLUGIN_DIR="${{FLEDGE_PLUGIN_DIR:?FLEDGE_PLUGIN_DIR not set — fledge >= 0.15.3 sets it automatically}}"

echo "{name} plugin running with args: $@"
echo "(plugin dir: $PLUGIN_DIR)"

# To dispatch to sibling helpers in the same `bin/` (a common multi-subcommand
# pattern), use:
#
#   exec "$PLUGIN_DIR/bin/{name}-${{1?missing subcommand}}" "${{@:2}}"
"#
    );
    let script_path = target.join("bin").join(name);
    fs::write(&script_path, script).context("writing bin script")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
            .context("setting executable permission")?;
    }

    fs::write(
        target.join("README.md"),
        format!(
            r#"# {name} — fledge plugin

{desc}

## Install

```bash
fledge plugins install ./{name}
```

Or after publishing:

```bash
fledge plugins install owner/{name}
```

## Commands

| Command | Description |
|---------|-------------|
| `fledge {name}` | {desc} |

## Development

Edit `plugin.toml` to configure commands, hooks, and capabilities.
See [fledge plugin docs](https://github.com/CorvidLabs/fledge) for the full plugin format.
"#
        ),
    )
    .context("writing README.md")?;

    fs::write(
        target.join(".gitignore"),
        "# Build artifacts\n/target/\n/dist/\n\n# OS\n.DS_Store\nThumbs.db\n",
    )
    .context("writing .gitignore")?;

    let files_created = vec![
        "plugin.toml".to_string(),
        format!("bin/{name}"),
        "README.md".to_string(),
        ".gitignore".to_string(),
    ];

    if json {
        let result = serde_json::json!({
            "schema_version": PLUGINS_CREATE_SCHEMA,
            "action": "create",
            "path": target.display().to_string(),
            "name": name,
            "description": desc,
            "files_created": files_created,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "\n{} Created plugin at {}",
            style("✅").green().bold(),
            style(target.display()).cyan()
        );
        println!(
            "\n  {} Edit manifest in {}",
            style("1.").dim(),
            style("plugin.toml").green()
        );
        println!(
            "  {} Validate with: {}",
            style("2.").dim(),
            style(format!("fledge plugins validate ./{name}")).cyan()
        );
        println!(
            "  {} Publish with: {}",
            style("3.").dim(),
            style(format!("fledge plugins publish ./{name}")).cyan()
        );
    }

    Ok(())
}
