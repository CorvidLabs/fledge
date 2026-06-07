use anyhow::{Context, Result};
use console::style;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::lanes;
use crate::run as task_runner;

/// Directories to always ignore when watching.
const IGNORE_DIRS: &[&str] = &[".git", "target", "node_modules", ".fledge", "__pycache__"];

pub struct WatchOptions {
    pub name: String,
    pub lane: bool,
    pub path: Option<PathBuf>,
    pub extensions: Vec<String>,
    pub debounce_ms: u64,
    pub clear: bool,
}

pub fn run(opts: WatchOptions) -> Result<()> {
    let watch_path = match &opts.path {
        Some(p) => p.clone(),
        None => std::env::current_dir().context("getting current directory")?,
    };

    if !watch_path.exists() {
        anyhow::bail!("Watch path does not exist: {}", watch_path.display());
    }

    let mode_label = if opts.lane { "lane" } else { "task" };

    println!(
        "{} Watching for changes to re-run {} {}",
        style("*").cyan().bold(),
        mode_label,
        style(&opts.name).green()
    );
    println!(
        "  {} {}",
        style("Path:").dim(),
        style(watch_path.display()).dim()
    );
    if !opts.extensions.is_empty() {
        println!(
            "  {} {}",
            style("Extensions:").dim(),
            style(opts.extensions.join(", ")).dim()
        );
    }
    println!(
        "  {} {}ms",
        style("Debounce:").dim(),
        style(opts.debounce_ms).dim()
    );
    println!("  {}", style("Press Ctrl+C to stop.").dim());
    println!();

    // Initial run
    run_target(&opts);

    // Set up file watcher
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx).context("failed to create file watcher")?;

    watcher
        .watch(&watch_path, RecursiveMode::Recursive)
        .with_context(|| format!("failed to watch path: {}", watch_path.display()))?;

    let debounce = Duration::from_millis(opts.debounce_ms);

    println!("{}", style("Watching for changes...").cyan());

    loop {
        // Wait for first event
        let event = match rx.recv() {
            Ok(Ok(event)) => event,
            Ok(Err(e)) => {
                eprintln!("{} Watcher error: {}", style("warning:").yellow().bold(), e);
                continue;
            }
            Err(_) => {
                // Channel disconnected, watcher dropped
                break;
            }
        };

        // Filter: only care about create/modify/remove events
        if !is_relevant_event(&event) {
            continue;
        }

        // Filter by path ignore patterns and extensions
        let changed_paths: Vec<&Path> = event
            .paths
            .iter()
            .filter(|p| !should_ignore_path(p))
            .filter(|p| matches_extensions(p, &opts.extensions))
            .map(|p| p.as_path())
            .collect();

        if changed_paths.is_empty() {
            continue;
        }

        // Debounce: drain any further events within the debounce window
        drain_events(&rx, debounce, &opts.extensions);

        // Show what changed
        for p in &changed_paths {
            let display = p.strip_prefix(&watch_path).unwrap_or(p).display();
            println!(
                "{} {}",
                style("Change detected:").yellow().bold(),
                style(display).dim()
            );
        }

        // Run the target
        run_target(&opts);

        println!("\n{}", style("Watching for changes...").cyan());
    }

    Ok(())
}

fn run_target(opts: &WatchOptions) {
    if opts.clear {
        // ANSI escape to clear terminal
        print!("\x1B[2J\x1B[1;1H");
    }

    let start = Instant::now();
    let mode_label = if opts.lane { "lane" } else { "task" };

    println!(
        "{} Re-running {}: {}",
        style(">>>").cyan().bold(),
        mode_label,
        style(&opts.name).green()
    );
    println!();

    let result = if opts.lane {
        lanes::run(lanes::LaneAction::Run {
            name: opts.name.clone(),
            dry_run: false,
            json: false,
            from: None,
        })
    } else {
        task_runner::run(task_runner::RunOptions {
            task: Some(opts.name.clone()),
            init: false,
            list: false,
            lang: None,
            json: false,
            args: Vec::new(),
        })
    };

    let elapsed = start.elapsed();
    let elapsed_str = format_duration(elapsed);

    match result {
        Ok(()) => {
            println!(
                "\n{} Completed in {}",
                style("OK").green().bold(),
                style(&elapsed_str).dim()
            );
        }
        Err(e) => {
            eprintln!(
                "\n{} Failed in {}: {:#}",
                style("FAIL").red().bold(),
                style(&elapsed_str).dim(),
                e
            );
        }
    }
}

fn is_relevant_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

/// Check if a path should be ignored based on directory name patterns.
pub fn should_ignore_path(path: &Path) -> bool {
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            let name_str = name.to_string_lossy();
            for ignored in IGNORE_DIRS {
                if name_str == *ignored {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a path matches the requested extensions filter.
/// If no extensions are specified, all files match.
pub fn matches_extensions(path: &Path, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    match path.extension() {
        Some(ext) => {
            let ext_str = ext.to_string_lossy();
            extensions.iter().any(|e| e == ext_str.as_ref())
        }
        None => false,
    }
}

/// Parse a comma-separated extension string into a list of extensions.
pub fn parse_extensions(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().trim_start_matches('.').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Drain events from the channel within a debounce window.
/// Each relevant event resets the deadline so rapid-fire changes collapse into one run.
fn drain_events(
    rx: &mpsc::Receiver<notify::Result<Event>>,
    debounce: Duration,
    extensions: &[String],
) {
    let mut deadline = Instant::now() + debounce;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(Ok(event)) => {
                let dominated = event
                    .paths
                    .iter()
                    .all(|p| should_ignore_path(p) || !matches_extensions(p, extensions));
                if dominated {
                    continue;
                }
                deadline = Instant::now() + debounce;
            }
            Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn format_duration(d: Duration) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_git_directory() {
        assert!(should_ignore_path(Path::new("/project/.git/objects/abc")));
        assert!(should_ignore_path(Path::new("some/path/.git/HEAD")));
    }

    #[test]
    fn ignore_target_directory() {
        assert!(should_ignore_path(Path::new("/project/target/debug/build")));
    }

    #[test]
    fn ignore_node_modules() {
        assert!(should_ignore_path(Path::new(
            "/project/node_modules/foo/index.js"
        )));
    }

    #[test]
    fn ignore_fledge_directory() {
        assert!(should_ignore_path(Path::new(
            "/project/.fledge/lanes/foo.toml"
        )));
    }

    #[test]
    fn ignore_pycache() {
        assert!(should_ignore_path(Path::new(
            "/project/__pycache__/mod.pyc"
        )));
    }

    #[test]
    fn do_not_ignore_regular_path() {
        assert!(!should_ignore_path(Path::new("/project/src/main.rs")));
        assert!(!should_ignore_path(Path::new("src/lib.rs")));
    }

    #[test]
    fn do_not_ignore_similar_names() {
        // "target_dir" is not "target"
        assert!(!should_ignore_path(Path::new("/project/target_dir/foo")));
        // ".github" is not ".git"
        assert!(!should_ignore_path(Path::new(
            "/project/.github/workflows/ci.yml"
        )));
    }

    #[test]
    fn extension_filter_matches_rs() {
        let exts = vec!["rs".to_string(), "toml".to_string()];
        assert!(matches_extensions(Path::new("src/main.rs"), &exts));
        assert!(matches_extensions(Path::new("Cargo.toml"), &exts));
        assert!(!matches_extensions(Path::new("README.md"), &exts));
    }

    #[test]
    fn extension_filter_empty_matches_all() {
        let exts: Vec<String> = vec![];
        assert!(matches_extensions(Path::new("anything.xyz"), &exts));
        assert!(matches_extensions(Path::new("no_ext"), &exts));
    }

    #[test]
    fn extension_filter_no_ext_file() {
        let exts = vec!["rs".to_string()];
        assert!(!matches_extensions(Path::new("Makefile"), &exts));
    }

    #[test]
    fn parse_extensions_basic() {
        let result = parse_extensions("rs,toml");
        assert_eq!(result, vec!["rs", "toml"]);
    }

    #[test]
    fn parse_extensions_with_dots() {
        let result = parse_extensions(".rs,.toml");
        assert_eq!(result, vec!["rs", "toml"]);
    }

    #[test]
    fn parse_extensions_with_spaces() {
        let result = parse_extensions("rs , toml , py");
        assert_eq!(result, vec!["rs", "toml", "py"]);
    }

    #[test]
    fn parse_extensions_empty() {
        let result = parse_extensions("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_extensions_single() {
        let result = parse_extensions("rs");
        assert_eq!(result, vec!["rs"]);
    }

    #[test]
    fn format_duration_millis() {
        let d = Duration::from_millis(42);
        assert_eq!(format_duration(d), "42ms");
    }

    #[test]
    fn format_duration_seconds() {
        let d = Duration::from_millis(3456);
        assert_eq!(format_duration(d), "3.456s");
    }

    #[test]
    fn format_duration_minutes() {
        let d = Duration::from_secs(125) + Duration::from_millis(100);
        assert_eq!(format_duration(d), "2m 5.100s");
    }

    #[test]
    fn format_duration_zero() {
        let d = Duration::from_millis(0);
        assert_eq!(format_duration(d), "0ms");
    }

    #[test]
    fn ignore_dirs_list_is_complete() {
        // Verify the expected ignore dirs are present
        assert!(IGNORE_DIRS.contains(&".git"));
        assert!(IGNORE_DIRS.contains(&"target"));
        assert!(IGNORE_DIRS.contains(&"node_modules"));
        assert!(IGNORE_DIRS.contains(&".fledge"));
        assert!(IGNORE_DIRS.contains(&"__pycache__"));
    }

    #[test]
    fn watch_options_defaults() {
        let opts = WatchOptions {
            name: "build".to_string(),
            lane: false,
            path: None,
            extensions: vec![],
            debounce_ms: 500,
            clear: false,
        };
        assert_eq!(opts.name, "build");
        assert!(!opts.lane);
        assert!(opts.path.is_none());
        assert!(opts.extensions.is_empty());
        assert_eq!(opts.debounce_ms, 500);
        assert!(!opts.clear);
    }

    #[test]
    fn combined_ignore_and_extension_filter() {
        let exts = vec!["rs".to_string()];
        // Ignored path even with matching extension
        let ignored = Path::new("/project/target/debug/foo.rs");
        assert!(should_ignore_path(ignored));

        // Non-ignored path with matching extension
        let ok = Path::new("/project/src/main.rs");
        assert!(!should_ignore_path(ok));
        assert!(matches_extensions(ok, &exts));

        // Non-ignored path without matching extension
        let no_match = Path::new("/project/src/README.md");
        assert!(!should_ignore_path(no_match));
        assert!(!matches_extensions(no_match, &exts));
    }
}
