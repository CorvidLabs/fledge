use anyhow::{bail, Context, Result};
use console::style;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use crate::trust::determine_trust_tier;

mod community;
mod create;
mod defaults;
mod execute;
mod publish;
mod validate;

#[cfg(test)]
mod tests;

#[cfg(test)]
use community::{base64_decode, parse_import_source};
#[cfg(test)]
use create::create_lane_repo;
#[cfg(test)]
use defaults::lane_defaults;
#[cfg(test)]
use execute::execute_lane;
#[cfg(test)]
use validate::validate_lanes;

/// Per-command JSON schema versions. Each constant tracks the wire shape of one
/// `lanes` subcommand's `--json` envelope independently so that future shape
/// changes can bump exactly the affected envelope without semantically
/// corrupting the meaning of `schema_version` for unrelated commands. Additive
/// changes (new optional fields) do not bump.
pub(super) const LANES_LIST_SCHEMA: u32 = 1;
pub(super) const LANES_DRY_RUN_SCHEMA: u32 = 1;
pub(super) const LANES_RUN_SCHEMA: u32 = 1;
pub(super) const LANES_INIT_SCHEMA: u32 = 1;
pub(super) const LANES_SEARCH_SCHEMA: u32 = 1;
pub(super) const LANES_IMPORT_SCHEMA: u32 = 1;
pub(super) const LANES_CREATE_SCHEMA: u32 = 1;
pub(super) const LANES_PUBLISH_SCHEMA: u32 = 1;

#[derive(Debug, Deserialize)]
pub(super) struct FledgeFileWithLanes {
    #[serde(default)]
    pub(super) tasks: BTreeMap<String, TaskDef>,
    #[serde(default)]
    pub(super) lanes: BTreeMap<String, LaneDef>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum TaskDef {
    Short(String),
    Full(TaskConfig),
}

#[derive(Debug, Deserialize)]
pub(super) struct TaskConfig {
    pub(super) cmd: String,
    #[serde(default)]
    pub(super) deps: Vec<String>,
    #[serde(default)]
    pub(super) env: BTreeMap<String, String>,
    #[serde(default)]
    pub(super) dir: Option<String>,
}

impl TaskDef {
    pub(super) fn cmd(&self) -> &str {
        match self {
            TaskDef::Short(s) => s,
            TaskDef::Full(c) => &c.cmd,
        }
    }

    pub(super) fn deps(&self) -> &[String] {
        match self {
            TaskDef::Short(_) => &[],
            TaskDef::Full(c) => &c.deps,
        }
    }

    pub(super) fn env(&self) -> &BTreeMap<String, String> {
        static EMPTY: BTreeMap<String, String> = BTreeMap::new();
        match self {
            TaskDef::Short(_) => &EMPTY,
            TaskDef::Full(c) => &c.env,
        }
    }

    pub(super) fn dir(&self) -> Option<&str> {
        match self {
            TaskDef::Short(_) => None,
            TaskDef::Full(c) => c.dir.as_deref(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LaneDef {
    #[serde(default)]
    pub(super) description: Option<String>,
    pub(super) steps: Vec<Step>,
    #[serde(default = "default_true")]
    pub(super) fail_fast: bool,
    #[serde(skip, default)]
    pub(super) source: Option<String>,
}

pub(super) fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum Step {
    TaskRef(String),
    Inline { run: String },
    Parallel { parallel: Vec<ParallelItem> },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum ParallelItem {
    TaskRef(String),
    Inline { run: String },
}

pub enum LaneAction {
    Run {
        name: String,
        dry_run: bool,
        json: bool,
    },
    List {
        json: bool,
    },
    Init {
        json: bool,
    },
    Search {
        query: Option<String>,
        author: Option<String>,
        json: bool,
    },
    Import {
        source: String,
        yes: bool,
        json: bool,
    },
    Publish {
        path: PathBuf,
        org: Option<String>,
        private: bool,
        description: Option<String>,
        yes: bool,
        json: bool,
    },
    Create {
        name: String,
        output: PathBuf,
        description: Option<String>,
        yes: bool,
        json: bool,
    },
    Validate {
        path: PathBuf,
        strict: bool,
        json: bool,
    },
}

pub fn run(action: LaneAction) -> Result<()> {
    match action {
        LaneAction::Search {
            query,
            author,
            json,
        } => community::search_lanes(query.as_deref(), author.as_deref(), json),
        LaneAction::Import { source, yes, json } => community::import_lanes(&source, yes, json),
        LaneAction::Init { json } => defaults::init_lanes(json),
        LaneAction::Publish {
            path,
            org,
            private,
            description,
            yes,
            json,
        } => publish::publish_lanes(
            &path,
            org.as_deref(),
            private,
            description.as_deref(),
            yes,
            json,
        ),
        LaneAction::Create {
            name,
            output,
            description,
            yes,
            json,
        } => create::create_lane_repo(&name, &output, description.as_deref(), yes, json),
        LaneAction::Validate { path, strict, json } => {
            validate::validate_lanes(&path, strict, json)
        }
        LaneAction::List { json } => {
            let config = load_lane_config()?;
            list_lanes(&config.lanes, json)
        }
        LaneAction::Run {
            name,
            dry_run,
            json,
        } => {
            let config = load_lane_config()?;
            let lane = config.lanes.get(&name).ok_or_else(|| {
                let available: Vec<&str> = config.lanes.keys().map(|s| s.as_str()).collect();
                anyhow::anyhow!(
                    "Unknown lane '{}'. Available lanes: {}",
                    name,
                    available.join(", ")
                )
            })?;

            if lane.steps.is_empty() {
                bail!("Lane '{}' has no steps defined", name);
            }

            validate_lane(&name, lane, &config.tasks)?;

            if dry_run {
                dry_run_lane(&name, lane, json)
            } else {
                let project_dir = std::env::current_dir().context("getting current directory")?;
                execute::execute_lane(&name, lane, &config.tasks, &project_dir, json)
            }
        }
    }
}

/// Run a lane as a pre-release gate without emitting anything to stdout.
/// The release command's own JSON envelope is the only thing the agent
/// consumer parses; the lane runs silently and bails on failure.
pub fn run_for_pre_release(name: &str, dry_run: bool) -> Result<()> {
    let config = load_lane_config()?;
    let lane = config.lanes.get(name).ok_or_else(|| {
        let available: Vec<&str> = config.lanes.keys().map(|s| s.as_str()).collect();
        anyhow::anyhow!(
            "Unknown lane '{}'. Available lanes: {}",
            name,
            available.join(", ")
        )
    })?;

    if lane.steps.is_empty() {
        bail!("Lane '{}' has no steps defined", name);
    }

    validate_lane(name, lane, &config.tasks)?;

    if dry_run {
        return Ok(());
    }

    let project_dir = std::env::current_dir().context("getting current directory")?;
    execute::execute_lane_silent(name, lane, &config.tasks, &project_dir)
}

pub(super) fn load_lane_config() -> Result<FledgeFileWithLanes> {
    let project_dir = std::env::current_dir().context("getting current directory")?;
    let config_path = project_dir.join("fledge.toml");

    if !config_path.exists() {
        bail!(
            "No fledge.toml found in current directory.\n  Run {} to create one.",
            style("fledge run --init").cyan()
        );
    }

    let content = std::fs::read_to_string(&config_path).context("reading fledge.toml")?;
    let mut config: FledgeFileWithLanes = toml::from_str(&content).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("lanes") || msg.contains("steps") {
            anyhow::anyhow!(
                "Error parsing lanes in fledge.toml: {e}\n\n  \
                 Lanes must use table syntax:\n    \
                 [lanes.ci]\n    \
                 steps = [\"lint\", \"test\", \"build\"]\n\n  \
                 Not shorthand like: ci = [\"lint\", \"test\"]"
            )
        } else {
            anyhow::anyhow!("parsing fledge.toml: {e}")
        }
    })?;

    let lanes_dir = project_dir.join(".fledge").join("lanes");
    if lanes_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&lanes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml") {
                    let imported_content = std::fs::read_to_string(&path)
                        .with_context(|| format!("reading {}", path.display()))?;
                    let imported: FledgeFileWithLanes = toml::from_str(&imported_content)
                        .with_context(|| format!("parsing {}", path.display()))?;
                    let import_source = imported_content
                        .lines()
                        .find(|l| l.starts_with("# Imported from "))
                        .map(|l| l.trim_start_matches("# Imported from ").trim().to_string());
                    for (name, task) in imported.tasks {
                        config.tasks.entry(name).or_insert(task);
                    }
                    for (name, mut lane) in imported.lanes {
                        lane.source = import_source.clone();
                        config.lanes.entry(name).or_insert(lane);
                    }
                }
            }
        }
    }

    if config.lanes.is_empty() {
        bail!(
            "No lanes defined.\n  Add lanes to fledge.toml, import with {}, or run {} to add defaults.",
            style("fledge lanes import <source>").cyan(),
            style("fledge lanes init").cyan()
        );
    }

    Ok(config)
}

pub(super) fn list_lanes(lanes: &BTreeMap<String, LaneDef>, json: bool) -> Result<()> {
    if json {
        let entries: Vec<serde_json::Value> = lanes
            .iter()
            .map(|(name, lane)| {
                let mut entry = serde_json::json!({
                    "name": name,
                    "description": lane.description,
                    "step_count": lane.steps.len(),
                    "fail_fast": lane.fail_fast,
                });
                if let Some(ref src) = lane.source {
                    let tier = determine_trust_tier(src);
                    entry["source"] = serde_json::json!(src);
                    entry["trust_tier"] = serde_json::json!(tier.label());
                } else {
                    entry["trust_tier"] = serde_json::json!("local");
                }
                entry
            })
            .collect();
        let result = serde_json::json!({
            "schema_version": LANES_LIST_SCHEMA,
            "lanes": entries,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("{}", style("Available lanes:").bold());
    let max_name_len = lanes.keys().map(|k| k.len()).max().unwrap_or(0);
    for (name, lane) in lanes {
        let desc = lane.description.as_deref().unwrap_or("(no description)");
        let tier_label = match &lane.source {
            Some(src) => {
                let tier = determine_trust_tier(src);
                format!(" [{}]", tier.styled_label())
            }
            None => String::new(),
        };
        println!(
            "  {:<width$}  {}{}",
            style(name).green(),
            style(desc).dim(),
            tier_label,
            width = max_name_len
        );
    }
    Ok(())
}

pub(super) fn validate_lane(
    lane_name: &str,
    lane: &LaneDef,
    tasks: &BTreeMap<String, TaskDef>,
) -> Result<()> {
    for (i, step) in lane.steps.iter().enumerate() {
        match step {
            Step::TaskRef(name) => {
                if !tasks.contains_key(name) {
                    bail!(
                        "Lane '{}' step {} references unknown task '{}'.\n  Define it in [tasks] first.",
                        lane_name,
                        i + 1,
                        name
                    );
                }
                check_dep_cycle(name, tasks, &mut HashSet::new())
                    .map_err(|e| anyhow::anyhow!("Lane '{}' step {}: {}", lane_name, i + 1, e))?;
            }
            Step::Inline { .. } => {}
            Step::Parallel { parallel } => {
                for item in parallel {
                    if let ParallelItem::TaskRef(name) = item {
                        if !tasks.contains_key(name) {
                            bail!(
                                "Lane '{}' step {} parallel group references unknown task '{}'.\n  Define it in [tasks] first.",
                                lane_name,
                                i + 1,
                                name
                            );
                        }
                        check_dep_cycle(name, tasks, &mut HashSet::new()).map_err(|e| {
                            anyhow::anyhow!("Lane '{}' step {}: {}", lane_name, i + 1, e)
                        })?;
                    }
                }
            }
        }
    }
    Ok(())
}

pub(super) fn check_dep_cycle(
    name: &str,
    tasks: &BTreeMap<String, TaskDef>,
    visiting: &mut HashSet<String>,
) -> Result<()> {
    if !visiting.insert(name.to_string()) {
        bail!("circular dependency detected involving task '{}'", name);
    }
    if let Some(task) = tasks.get(name) {
        for dep in task.deps() {
            check_dep_cycle(dep, tasks, visiting)?;
        }
    }
    visiting.remove(name);
    Ok(())
}

pub(super) fn dry_run_lane(lane_name: &str, lane: &LaneDef, json: bool) -> Result<()> {
    let desc = lane.description.as_deref().unwrap_or("(no description)");

    if json {
        let steps: Vec<serde_json::Value> = lane
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| match step {
                Step::TaskRef(name) => serde_json::json!({
                    "step": i + 1,
                    "kind": "task",
                    "name": name,
                }),
                Step::Inline { run: cmd } => serde_json::json!({
                    "step": i + 1,
                    "kind": "inline",
                    "name": cmd,
                }),
                Step::Parallel { parallel } => {
                    let items: Vec<serde_json::Value> = parallel
                        .iter()
                        .map(|item| match item {
                            ParallelItem::TaskRef(name) => serde_json::json!({
                                "kind": "task",
                                "name": name,
                            }),
                            ParallelItem::Inline { run: cmd } => serde_json::json!({
                                "kind": "inline",
                                "name": cmd,
                            }),
                        })
                        .collect();
                    serde_json::json!({
                        "step": i + 1,
                        "kind": "parallel",
                        "items": items,
                    })
                }
            })
            .collect();

        let output = serde_json::json!({
            "schema_version": LANES_DRY_RUN_SCHEMA,
            "lane": lane_name,
            "description": lane.description.as_deref().unwrap_or(""),
            "total_steps": lane.steps.len(),
            "fail_fast": lane.fail_fast,
            "dry_run": true,
            "steps": steps,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!(
        "{} {} — {}",
        style("Lane:").bold(),
        style(lane_name).green(),
        style(desc).dim()
    );
    if !lane.fail_fast {
        println!("  {} fail_fast = false", style("⚙").dim());
    }
    for (i, step) in lane.steps.iter().enumerate() {
        match step {
            Step::TaskRef(name) => {
                println!(
                    "  {}. {} {}",
                    i + 1,
                    style(name).cyan(),
                    style("(task)").dim()
                );
            }
            Step::Inline { run: cmd } => {
                println!(
                    "  {}. {} {}",
                    i + 1,
                    style(cmd).cyan(),
                    style("(inline)").dim()
                );
            }
            Step::Parallel { parallel } => {
                let names: Vec<String> = parallel
                    .iter()
                    .map(|item| match item {
                        ParallelItem::TaskRef(name) => name.clone(),
                        ParallelItem::Inline { run: cmd } => format!("run: {cmd}"),
                    })
                    .collect();
                println!(
                    "  {}. {} {}",
                    i + 1,
                    style(names.join(", ")).cyan(),
                    style("(parallel)").dim()
                );
            }
        }
    }
    Ok(())
}

pub(super) fn step_description(step: &Step) -> String {
    match step {
        Step::TaskRef(name) => name.clone(),
        Step::Inline { run: cmd } => cmd.clone(),
        Step::Parallel { parallel } => {
            let names: Vec<String> = parallel
                .iter()
                .map(|item| match item {
                    ParallelItem::TaskRef(name) => name.clone(),
                    ParallelItem::Inline { run: cmd } => cmd.clone(),
                })
                .collect();
            format!("parallel({})", names.join(", "))
        }
    }
}

pub(super) fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    if secs >= 60 {
        let mins = secs / 60;
        let remaining = secs % 60;
        format!("{mins}m {remaining}.{millis:03}s")
    } else if secs > 0 {
        format!("{secs}.{millis:03}s")
    } else {
        format!("{millis}ms")
    }
}

pub(super) fn escape_toml_value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn format_lane_toml(name: &str, lane: &LaneDef) -> String {
    let mut out = format!("\n[lanes.{}]\n", name);
    if let Some(ref desc) = lane.description {
        out.push_str(&format!("description = \"{}\"\n", escape_toml_value(desc)));
    }
    if !lane.fail_fast {
        out.push_str("fail_fast = false\n");
    }
    out.push_str("steps = [");
    for (i, step) in lane.steps.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        match step {
            Step::TaskRef(name) => {
                out.push_str(&format!("\"{}\"", escape_toml_value(name)));
            }
            Step::Inline { run: cmd } => {
                out.push_str(&format!("{{ run = \"{}\" }}", escape_toml_value(cmd)));
            }
            Step::Parallel { parallel } => {
                let items: Vec<String> = parallel
                    .iter()
                    .map(|item| match item {
                        ParallelItem::TaskRef(name) => {
                            format!("\"{}\"", escape_toml_value(name))
                        }
                        ParallelItem::Inline { run: cmd } => {
                            format!("{{ run = \"{}\" }}", escape_toml_value(cmd))
                        }
                    })
                    .collect();
                out.push_str(&format!("{{ parallel = [{}] }}", items.join(", ")));
            }
        }
    }
    out.push_str("]\n");
    out
}
