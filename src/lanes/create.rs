use anyhow::{bail, Context, Result};
use console::style;
use std::path::Path;

use super::LANES_CREATE_SCHEMA;

pub(crate) fn create_lane_repo(
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
        description.unwrap_or("Shared fledge lanes").to_string()
    } else {
        let theme = dialoguer::theme::ColorfulTheme::default();
        dialoguer::Input::with_theme(&theme)
            .with_prompt("Description")
            .default(description.unwrap_or("Shared fledge lanes").to_string())
            .interact_text()?
    };

    std::fs::create_dir_all(&target).with_context(|| format!("creating {}", target.display()))?;

    let fledge_toml = format!(
        r#"[tasks]
lint = "echo 'lint placeholder'"
test = "echo 'test placeholder'"
build = "echo 'build placeholder'"
fmt = "echo 'fmt placeholder'"

[lanes.ci]
description = {desc:?}
steps = ["lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  {{ parallel = ["lint", "fmt"] }},
  "test"
]
"#,
        desc = format!("{name} CI pipeline")
    );
    std::fs::write(target.join("fledge.toml"), fledge_toml).context("writing fledge.toml")?;

    std::fs::write(
        target.join("README.md"),
        format!(
            r#"# {name} — fledge lanes

{desc}

## Usage

Import these lanes into any fledge project:

```bash
fledge lanes import ./{name}
```

Or after publishing:

```bash
fledge lanes import owner/{name}
```

## Lanes

| Lane | Description |
|------|-------------|
| `ci` | {name} CI pipeline |
| `check` | Quick quality check |

## Customization

Edit `fledge.toml` to add, modify, or remove lanes and tasks.
See [fledge docs](https://github.com/CorvidLabs/fledge) for lane syntax.
"#
        ),
    )
    .context("writing README.md")?;

    std::fs::write(target.join(".gitignore"), "# OS\n.DS_Store\nThumbs.db\n")
        .context("writing .gitignore")?;

    let files_created = vec![
        "fledge.toml".to_string(),
        "README.md".to_string(),
        ".gitignore".to_string(),
    ];

    if json {
        let result = crate::envelope::action(
            LANES_CREATE_SCHEMA,
            "create",
            serde_json::json!({
                "path": target.display().to_string(),
                "name": name,
                "description": desc,
                "files_created": files_created,
            }),
        );
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "\n{} Created lane repo at {}",
            style("✅").green().bold(),
            style(target.display()).cyan()
        );
        println!(
            "\n  {} Edit lanes in {}",
            style("1.").dim(),
            style("fledge.toml").green()
        );
        println!(
            "  {} Validate with: {}",
            style("2.").dim(),
            style(format!("fledge lanes validate ./{name}")).cyan()
        );
        println!(
            "  {} Publish with: {}",
            style("3.").dim(),
            style(format!("fledge lanes publish ./{name}")).cyan()
        );
    }

    Ok(())
}
