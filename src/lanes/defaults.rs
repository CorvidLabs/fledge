use anyhow::{bail, Context, Result};
use console::style;

use super::{FledgeFileWithLanes, LANES_INIT_SCHEMA};
use crate::run::detect_project_type;

pub(crate) fn lane_defaults(project_type: &str) -> &'static str {
    match project_type {
        "rust" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        "node" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["lint", "test"] },
]
"#
        }
        "go" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "test", "build"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        "python" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["fmt", "lint", "test"]

[lanes.check]
description = "Quick quality check"
steps = [
  { parallel = ["fmt", "lint"] },
  "test"
]
"#
        }
        "swift" => {
            r#"
[lanes.ci]
description = "Run full CI pipeline"
steps = ["build", "test"]
"#
        }
        _ => {
            r#"
# [lanes.ci]
# description = "Run full CI pipeline"
# steps = ["lint", "test", "build"]
"#
        }
    }
}

pub(crate) fn init_lanes(json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let path = cwd.join("fledge.toml");

    if !path.exists() {
        bail!(
            "No fledge.toml found. Run {} first, then add lanes.",
            style("fledge run --init").cyan()
        );
    }

    let content = std::fs::read_to_string(&path).context("reading fledge.toml")?;

    if content.contains("[lanes") {
        bail!("Lanes already defined in fledge.toml. Edit them manually.");
    }

    let project_type = detect_project_type(&cwd);
    let defaults = lane_defaults(project_type);

    let new_content = format!("{}{}", content.trim_end(), defaults);
    std::fs::write(&path, &new_content).context("writing fledge.toml")?;

    // Parse the just-written defaults so we can report which lane names landed.
    // This is a best-effort parse — if it fails, we still succeed but emit an
    // empty `lanes_added` list rather than crashing the json path.
    let lanes_added: Vec<String> = toml::from_str::<FledgeFileWithLanes>(&new_content)
        .map(|cfg| cfg.lanes.keys().cloned().collect())
        .unwrap_or_default();

    if json {
        let result = serde_json::json!({
            "schema_version": LANES_INIT_SCHEMA,
            "action": "init",
            "file": "fledge.toml",
            "project_type": project_type,
            "lanes_added": lanes_added,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "{} Added default lanes to {}",
            style("✅").green().bold(),
            style("fledge.toml").cyan()
        );
        println!("  Run {} to see them.", style("fledge lanes list").cyan());
    }
    Ok(())
}
