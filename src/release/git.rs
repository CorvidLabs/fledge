use anyhow::{bail, Context, Result};
use console::style;
use std::path::Path;
use std::process::Command;

use crate::versioning::Version;

pub(super) fn create_release_commit(
    dir: &Path,
    version: &Version,
    bumped_files: &[String],
    has_changelog: bool,
    quiet: bool,
) -> Result<()> {
    let mut files_to_add: Vec<String> = bumped_files.to_vec();
    if has_changelog && dir.join("CHANGELOG.md").exists() {
        files_to_add.push("CHANGELOG.md".to_string());
    }

    if !files_to_add.is_empty() {
        let mut cmd = Command::new("git");
        cmd.arg("add").current_dir(dir);
        for f in &files_to_add {
            cmd.arg(f);
        }
        let output = cmd.output().context("staging release files")?;
        if !output.status.success() {
            bail!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    let msg = format!("chore: release v{version}");
    let mut commit_args = vec!["commit", "-m", &msg];
    if files_to_add.is_empty() {
        commit_args.push("--allow-empty");
    }
    let output = Command::new("git")
        .args(&commit_args)
        .current_dir(dir)
        .output()
        .context("creating release commit")?;

    if !output.status.success() {
        bail!(
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if !quiet {
        println!("  Created commit: {}", style(&msg).dim());
    }
    Ok(())
}

pub(super) fn create_tag(dir: &Path, version: &Version, quiet: bool) -> Result<()> {
    let tag = format!("v{version}");

    let check = Command::new("git")
        .args(["tag", "-l", &tag])
        .current_dir(dir)
        .output()
        .context("checking existing tags")?;
    if !String::from_utf8_lossy(&check.stdout).trim().is_empty() {
        bail!(
            "Tag '{}' already exists. Delete it first with: git tag -d {}",
            tag,
            tag
        );
    }

    let output = Command::new("git")
        .args(["tag", "-a", &tag, "-m", &format!("Release {tag}")])
        .current_dir(dir)
        .output()
        .context("creating git tag")?;

    if !output.status.success() {
        bail!(
            "git tag failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if !quiet {
        println!("  Created tag: {}", style(&tag).cyan());
    }
    Ok(())
}

pub(super) fn push_release(
    dir: &Path,
    version: &Version,
    has_tag: bool,
    quiet: bool,
) -> Result<()> {
    let output = Command::new("git")
        .args(["push"])
        .current_dir(dir)
        .output()
        .context("pushing release commit")?;

    if !output.status.success() {
        bail!(
            "git push failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if has_tag {
        let tag = format!("v{version}");
        let output = Command::new("git")
            .args(["push", "origin", &tag])
            .current_dir(dir)
            .output()
            .context("pushing release tag")?;

        if !output.status.success() {
            bail!(
                "git push tag failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    if !quiet {
        println!("  Pushed to remote");
    }
    Ok(())
}
