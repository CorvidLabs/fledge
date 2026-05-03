use anyhow::{bail, Context, Result};
use console::style;
use std::fs;
use std::path::Path;

use super::PluginManifest;

#[derive(Default, serde::Serialize)]
pub(crate) struct PluginValidationReport {
    pub(crate) path: String,
    pub(crate) plugin_name: String,
    pub(crate) errors: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

pub(crate) fn validate_plugin(path: &Path, strict: bool, json: bool) -> Result<()> {
    let path = path.canonicalize().unwrap_or(path.to_path_buf());

    let manifest_path = path.join("plugin.toml");
    if !manifest_path.exists() {
        bail!(
            "No plugin.toml found in {}. Point to a directory containing plugin.toml.",
            path.display()
        );
    }

    let content = fs::read_to_string(&manifest_path).context("reading plugin.toml")?;
    let mut report = PluginValidationReport {
        path: path.display().to_string(),
        ..Default::default()
    };

    let manifest: PluginManifest = match toml::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            report.errors.push(format!("Invalid plugin.toml: {e}"));
            return print_plugin_report(&report, strict, json);
        }
    };

    report.plugin_name = manifest.plugin.name.clone();

    if manifest.plugin.name.is_empty() {
        report.errors.push("plugin.name is empty".to_string());
    }

    if manifest.plugin.version.is_empty() {
        report.errors.push("plugin.version is empty".to_string());
    } else if crate::versioning::parse_version(&manifest.plugin.version).is_err() {
        report.errors.push(format!(
            "plugin.version is not valid semver: '{}' (expected major.minor.patch)",
            manifest.plugin.version
        ));
    }

    if manifest.plugin.description.is_none() {
        report
            .warnings
            .push("plugin.description is not set".to_string());
    }

    if manifest.plugin.author.is_none() {
        report.warnings.push("plugin.author is not set".to_string());
    }

    if manifest.commands.is_empty() {
        report
            .warnings
            .push("No [[commands]] defined — plugin won't register any subcommands".to_string());
    }

    for cmd in &manifest.commands {
        if cmd.name.is_empty() {
            report.errors.push("Command has empty name".to_string());
        }

        if cmd.binary.is_empty() {
            report
                .errors
                .push(format!("Command '{}' has empty binary path", cmd.name));
        } else {
            let bin_path = path.join(&cmd.binary);
            if !bin_path.exists() {
                let has_build = manifest.hooks.build.is_some();
                if has_build {
                    report.warnings.push(format!(
                        "Command '{}' binary '{}' not found (may be created by build hook)",
                        cmd.name, cmd.binary
                    ));
                } else {
                    report.errors.push(format!(
                        "Command '{}' binary '{}' not found and no build hook defined",
                        cmd.name, cmd.binary
                    ));
                }
            }
        }
    }

    if let Some(ref build) = manifest.hooks.build {
        if build.trim().is_empty() {
            report
                .warnings
                .push("hooks.build is set but empty".to_string());
        }
    }

    if let Some(ref rt) = manifest.plugin.runtime {
        if rt != "wasm" && rt != "native" {
            report.errors.push(format!(
                "plugin.runtime must be \"wasm\" or \"native\", got {:?}",
                rt
            ));
        }
    }

    if let Some(ref fs_cap) = manifest.capabilities.filesystem {
        match fs_cap.as_str() {
            "none" | "project" | "plugin" => {}
            other => {
                report.errors.push(format!(
                    "capabilities.filesystem must be \"none\", \"project\", or \"plugin\", got {:?}",
                    other
                ));
            }
        }
    }

    let is_wasm = manifest.plugin.runtime.as_deref() == Some("wasm");
    if is_wasm {
        if manifest.plugin.protocol.as_deref() != Some("fledge-v1") {
            report.errors.push(
                "WASM plugins must set plugin.protocol = \"fledge-v1\" — \
                 without this, capabilities will not be granted at install time"
                    .to_string(),
            );
        }
        for cmd in &manifest.commands {
            if !cmd.binary.is_empty() && !cmd.binary.ends_with(".wasm") {
                report.warnings.push(format!(
                    "WASM command '{}' binary '{}' does not end in .wasm — \
                     WASM plugins should point to a .wasm file (e.g. target/wasm32-wasip1/release/{}.wasm)",
                    cmd.name, cmd.binary, cmd.name
                ));
            }
        }
        if manifest.hooks.build.is_none() {
            report.warnings.push(
                "WASM plugin has no build hook — add [hooks] build = \"cargo build --target wasm32-wasip1 --release\" \
                 so the .wasm binary is compiled during install"
                    .to_string(),
            );
        }
    }

    print_plugin_report(&report, strict, json)
}

pub(crate) fn print_plugin_report(
    report: &PluginValidationReport,
    strict: bool,
    json: bool,
) -> Result<()> {
    if json {
        // Wrap the report in an envelope so the top level carries
        // schema_version (matches plugins list/audit/search shape).
        // The full report is flattened so existing fields (path,
        // plugin_name, errors, warnings) sit at the same level.
        let mut value = serde_json::to_value(report)?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "schema_version".to_string(),
                serde_json::Value::Number(serde_json::Number::from(1)),
            );
        }
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if report.errors.is_empty() && report.warnings.is_empty() {
        let name = if report.plugin_name.is_empty() {
            &report.path
        } else {
            &report.plugin_name
        };
        println!(
            "{} {} — valid",
            style("✅").green().bold(),
            style(name).green()
        );
    } else {
        let name = if report.plugin_name.is_empty() {
            &report.path
        } else {
            &report.plugin_name
        };
        println!("{}", style(name).bold());
        for e in &report.errors {
            println!("  {} {}", style("error:").red().bold(), e);
        }
        for w in &report.warnings {
            println!("  {} {}", style("warn:").yellow().bold(), w);
        }
    }

    let has_errors = !report.errors.is_empty();
    let has_warnings = !report.warnings.is_empty();
    if has_errors || (strict && has_warnings) {
        bail!("Validation failed");
    }

    Ok(())
}
