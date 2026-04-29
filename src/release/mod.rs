use anyhow::{bail, Context, Result};
use console::style;
use std::path::Path;
use std::process::Command;

use crate::versioning::Version;

mod bump;
mod changelog;
mod git;
mod toml_utils;
mod version;

#[cfg(test)]
mod tests;

/// JSON schema version for the `release` envelope (covers both dry-run and real
/// runs since they share the same shape, distinguished by the `dry_run` bool).
/// See lanes.rs for the per-command rationale.
const RELEASE_SCHEMA: u32 = 1;

pub struct ReleaseOptions {
    pub bump: String,
    pub dry_run: bool,
    pub no_tag: bool,
    pub no_changelog: bool,
    /// Skip bumping any version files. Tag-only release, useful for repos
    /// whose version source-of-truth lives outside the working tree (e.g. a
    /// GitHub Release whose tag is the canonical version).
    pub no_bump: bool,
    pub push: bool,
    pub pre_lane: Option<String>,
    pub allow_dirty: bool,
    pub json: bool,
}

struct BumpResult {
    old: Version,
    new: Version,
    files_bumped: Vec<String>,
}

pub fn run(opts: ReleaseOptions) -> Result<()> {
    let dir = std::env::current_dir()?;

    preflight_checks(&dir, opts.allow_dirty)?;

    if let Some(ref lane) = opts.pre_lane {
        run_pre_lane(lane, opts.dry_run, opts.json)?;
    }

    let new_version = version::resolve_target_version(&dir, &opts.bump)?;

    if opts.dry_run {
        let files_to_bump = if opts.no_bump {
            Vec::new()
        } else {
            bump::detect_version_files(&dir)
        };

        if opts.json {
            let envelope = serde_json::json!({
                "schema_version": RELEASE_SCHEMA,
                "action": "release",
                "dry_run": true,
                "version": new_version.to_string(),
                "no_bump": opts.no_bump,
                "files_to_bump": files_to_bump,
                "will_changelog": !opts.no_changelog,
                "will_tag": !opts.no_tag,
                "will_push": opts.push,
                "tag": format!("v{}", new_version),
            });
            println!("{}", serde_json::to_string_pretty(&envelope)?);
            return Ok(());
        }

        println!(
            "{} Would release v{} (dry run)",
            style("*").cyan().bold(),
            new_version
        );
        if opts.no_bump {
            println!("  Tag-only release (--no-bump)");
        } else if files_to_bump.is_empty() {
            println!("  Tag-only release (no version files detected)");
        } else {
            for f in &files_to_bump {
                println!("  Would bump: {}", style(f).cyan());
            }
        }
        if !opts.no_changelog {
            println!("  Would update CHANGELOG.md");
        }
        if !opts.no_tag {
            println!("  Would create tag v{}", new_version);
        }
        if opts.push {
            println!("  Would push commit and tag");
        }
        return Ok(());
    }

    let result = if opts.no_bump {
        BumpResult {
            old: version::detect_current_version(&dir).unwrap_or(Version {
                major: 0,
                minor: 0,
                patch: 0,
            }),
            new: new_version.clone(),
            files_bumped: Vec::new(),
        }
    } else {
        bump::bump_version_files(&dir, &new_version)?
    };

    if !opts.json {
        println!(
            "{} {} → {}",
            style("📦").bold(),
            style(&result.old).dim(),
            style(&result.new).green().bold()
        );

        for f in &result.files_bumped {
            println!("  Bumped {}", style(f).cyan());
        }

        if result.files_bumped.is_empty() {
            println!("  Tag-only release (no version files found)");
        }
    }

    let mut changelog_updated = false;
    if !opts.no_changelog {
        changelog_updated = changelog::generate_changelog_entry(&dir, &new_version, opts.json)?;
    }

    git::create_release_commit(
        &dir,
        &new_version,
        &result.files_bumped,
        !opts.no_changelog,
        opts.json,
    )?;

    if !opts.no_tag {
        git::create_tag(&dir, &new_version, opts.json)?;
    }

    if opts.push {
        git::push_release(&dir, &new_version, !opts.no_tag, opts.json)?;
    }

    if opts.json {
        let envelope = serde_json::json!({
            "schema_version": RELEASE_SCHEMA,
            "action": "release",
            "dry_run": false,
            "version": new_version.to_string(),
            "old_version": result.old.to_string(),
            "files_bumped": result.files_bumped,
            "changelog_updated": changelog_updated,
            "commit_created": true,
            "tag_created": !opts.no_tag,
            "tag": format!("v{}", new_version),
            "pushed": opts.push,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        return Ok(());
    }

    println!(
        "\n{} Released v{}",
        style("✅").green().bold(),
        style(&new_version).green().bold()
    );

    if !opts.push {
        println!(
            "  Push with: {} && {}",
            style("git push").cyan(),
            style(format!("git push origin v{new_version}")).cyan()
        );
    }

    Ok(())
}

fn preflight_checks(dir: &Path, allow_dirty: bool) -> Result<()> {
    if !dir.join(".git").exists() {
        bail!("Not a git repository. Run `git init` first.");
    }

    if !allow_dirty {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(dir)
            .output()
            .context("running git status")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            bail!(
                "Working tree is not clean. Commit or stash changes first, or use --allow-dirty.\n\n{}",
                stdout.trim()
            );
        }
    }

    Ok(())
}

fn run_pre_lane(lane: &str, dry_run: bool, json: bool) -> Result<()> {
    if json {
        // Stdout is reserved for release's own JSON envelope. Run the lane
        // silently; failure bails with a plain stderr error per envelope rules.
        return crate::lanes::run_for_pre_release(lane, dry_run);
    }

    println!(
        "{} Running pre-release lane: {}",
        style("🔄").bold(),
        style(lane).cyan()
    );

    let action = crate::lanes::LaneAction::Run {
        name: lane.to_string(),
        dry_run,
        json: false,
    };
    crate::lanes::run(action)?;

    println!("{} Pre-release lane passed", style("✅").green().bold());
    Ok(())
}
