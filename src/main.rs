use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use console::style;
use std::path::PathBuf;

mod ai;
mod ask;
mod changelog;
mod config;
mod create_template;
mod doctor;
mod github;
mod init;
mod introspect;
mod lanes;
mod llm;
mod meta;
mod plugin;
mod prompts;
mod protocol;
mod publish;
mod release;
mod remote;
mod review;
mod run;
mod search;
mod spec;
mod spinner;
mod templates;
mod trust;
mod utils;
mod validate;
mod versioning;
mod watch;
mod work;

#[derive(Parser)]
#[command(
    name = "fledge",
    version,
    about = "Dev-lifecycle CLI — get your projects ready to fly."
)]
struct Cli {
    /// Run without prompts: treat every interactive confirmation as --yes,
    /// and bail with a clear error on prompts that have no default. Also
    /// settable via the FLEDGE_NON_INTERACTIVE env var.
    #[arg(long, global = true, visible_alias = "ni")]
    non_interactive: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Manage AI provider and model selection
    Ai {
        #[command(subcommand)]
        action: AiSubcommand,
    },
    /// Ask a question about your codebase
    Ask {
        /// The question to ask
        question: Vec<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Include full spec + companions for these modules in the prompt
        /// (comma-separated, repeatable, use "all" for every spec)
        #[arg(long, value_name = "NAMES")]
        with_specs: Vec<String>,
        /// Omit the compact spec index from the prompt (saves tokens)
        #[arg(long)]
        no_spec_index: bool,
        /// LLM provider: claude (default) or ollama. Overrides
        /// FLEDGE_AI_PROVIDER and ai.provider in config.
        #[arg(long, value_name = "NAME", value_parser = ["claude", "ollama"])]
        provider: Option<String>,
        /// Model name. Overrides FLEDGE_AI_MODEL and
        /// ai.{claude,ollama}.model in config.
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
    },
    /// Generate a changelog from git tags and commits
    Changelog {
        /// Number of releases to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Show a specific tag only
        #[arg(short, long)]
        tag: Option<String>,
        /// Show unreleased changes since the latest tag
        #[arg(long)]
        unreleased: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for (auto-detects if omitted with --install)
        #[arg(value_enum)]
        shell: Option<Shell>,
        /// Install completions to the standard location for your shell
        #[arg(long)]
        install: bool,
    },
    /// Manage global configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Diagnose project environment health
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Dump the full command tree (for agents and tooling)
    Introspect {
        /// Output as JSON (default: pretty tree)
        #[arg(long)]
        json: bool,
    },
    /// Manage and run composable workflow pipelines
    #[command(alias = "lane")]
    Lanes {
        #[command(subcommand)]
        action: LaneSubcommand,
    },
    /// Manage plugins (install, remove, list, search)
    #[command(alias = "plugin")]
    Plugins {
        #[command(subcommand)]
        action: PluginSubcommand,
        /// Output as JSON
        #[arg(long, global = true)]
        json: bool,
    },
    /// Cut a release — bump version, changelog, tag, and optionally push
    Release {
        /// Version bump: major, minor, patch, or explicit version (e.g. "1.0.0")
        bump: String,
        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,
        /// Skip creating a git tag
        #[arg(long)]
        no_tag: bool,
        /// Skip changelog generation
        #[arg(long)]
        no_changelog: bool,
        /// Skip bumping any version files. Tag-only release — useful when the
        /// canonical version lives outside the tree (e.g. the GitHub Release
        /// tag itself is the source of truth).
        #[arg(long)]
        no_bump: bool,
        /// Push commit and tag to remote after release
        #[arg(long)]
        push: bool,
        /// Run a lane before releasing (e.g. "ci")
        #[arg(long)]
        pre_lane: Option<String>,
        /// Allow releasing with uncommitted changes
        #[arg(long)]
        allow_dirty: bool,
    },
    /// AI-powered code review of current changes
    Review {
        /// Base branch to diff against (default: auto-detect)
        #[arg(short, long)]
        base: Option<String>,
        /// Review only a specific file
        #[arg(short, long)]
        file: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Model name for the active provider (overrides FLEDGE_AI_MODEL
        /// and ai.{claude,ollama}.model in config)
        #[arg(short, long)]
        model: Option<String>,
        /// Custom review focus prompt (appended to default instructions)
        #[arg(short, long)]
        prompt: Option<String>,
        /// Output format: summary (default), checklist, inline
        #[arg(long, default_value = "summary")]
        format: String,
        /// Include full spec + companions for these modules in the review
        /// context (comma-separated, repeatable). Appended to any
        /// auto-detected specs.
        #[arg(long, value_name = "NAMES")]
        with_specs: Vec<String>,
        /// Disable auto-detection of specs based on files in the diff
        #[arg(long)]
        no_auto_specs: bool,
        /// LLM provider: claude (default) or ollama. Overrides
        /// FLEDGE_AI_PROVIDER and ai.provider in config.
        #[arg(long, value_name = "NAME", value_parser = ["claude", "ollama"])]
        provider: Option<String>,
        /// Add another model to the review panel — runs in parallel against
        /// the same diff + spec context. Format: provider[:model], e.g.
        /// `ollama:gpt-oss:120b-cloud` or just `claude` to use the active
        /// claude config. Repeatable and comma-separated.
        #[arg(long, value_name = "REF")]
        with_model: Vec<String>,
        /// Drop the active config (--provider/--model or
        /// `fledge ai use`) from the panel. Only the explicit --with-model
        /// entries will run. Useful for "compare exactly these N models".
        #[arg(long)]
        no_active: bool,
    },
    /// Run a project task defined in fledge.toml
    Run {
        /// Task name to run (lists tasks if omitted)
        task: Option<String>,
        /// Create a starter fledge.toml
        #[arg(long)]
        init: bool,
        /// List available tasks
        #[arg(short, long)]
        list: bool,
        /// Override detected project language (e.g. rust, node, go, python, swift, ruby, java-gradle, java-maven)
        #[arg(long)]
        lang: Option<String>,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage specs (check, init, new)
    Spec {
        #[command(subcommand)]
        action: SpecSubcommand,
    },
    /// Manage templates (init, create, validate, list, search, publish)
    #[command(alias = "template")]
    Templates {
        #[command(subcommand)]
        action: TemplatesSubcommand,
    },
    /// Watch for file changes and re-run a task or lane
    Watch {
        /// Task name to re-run on changes (use --lane for lanes)
        name: String,
        /// Watch and re-run a lane instead of a task
        #[arg(long)]
        lane: bool,
        /// Only watch a specific directory (default: current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Only trigger on specific file extensions (comma-separated, e.g. "rs,toml")
        #[arg(short, long)]
        ext: Option<String>,
        /// Debounce interval in milliseconds
        #[arg(short, long, default_value = "500")]
        debounce: u64,
        /// Clear terminal before each run
        #[arg(long)]
        clear: bool,
    },
    /// Feature branch and PR workflow
    Work {
        #[command(subcommand)]
        action: WorkSubcommand,
    },
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(clap::Subcommand)]
enum TemplatesSubcommand {
    /// Create a new project from a template
    Init {
        /// Project name
        name: String,
        /// Template to use (skip interactive selection)
        #[arg(short, long)]
        template: Option<String>,
        /// Parent directory for the project
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Author name (bypasses prompt; overrides config)
        #[arg(long)]
        author: Option<String>,
        /// GitHub organization (bypasses prompt; overrides config)
        #[arg(long)]
        org: Option<String>,
        /// Skip git init and initial commit
        #[arg(long)]
        no_git: bool,
        /// Skip dependency installation (post-create hooks)
        #[arg(long)]
        no_install: bool,
        /// Force re-clone of cached remote templates
        #[arg(long)]
        refresh: bool,
        /// Show what would be created without writing anything
        #[arg(long)]
        dry_run: bool,
        /// Skip all confirmation prompts (accept defaults)
        #[arg(short, long)]
        yes: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Scaffold a new fledge template
    Create {
        /// Template name
        name: String,
        /// Parent directory for the template
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Template description (bypasses prompt)
        #[arg(short, long)]
        description: Option<String>,
        /// Comma-separated file patterns to render through Tera (bypasses prompt)
        #[arg(long)]
        render_patterns: Option<String>,
        /// Include post-create hooks scaffold (bypasses prompt)
        #[arg(long, num_args = 0..=1, default_missing_value = "true")]
        hooks: Option<bool>,
        /// Include custom prompts scaffold (bypasses prompt)
        #[arg(long, num_args = 0..=1, default_missing_value = "true")]
        prompts: Option<bool>,
        /// Skip all interactive prompts (accept defaults)
        #[arg(short, long)]
        yes: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Validate a template or directory of templates
    Validate {
        /// Path to a template or directory of templates
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List available templates
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Search GitHub for community templates (fledge-template topic)
    Search {
        /// Keyword to filter results
        query: Option<String>,
        /// Filter by author/owner
        #[arg(short, long)]
        author: Option<String>,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Publish a template directory to GitHub
    Publish {
        /// Path to the template directory
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Publish under a GitHub organization
        #[arg(long)]
        org: Option<String>,
        /// Create as a private repository
        #[arg(long)]
        private: bool,
        /// Override the repository description
        #[arg(long)]
        description: Option<String>,
        /// Skip all confirmation prompts
        #[arg(short, long)]
        yes: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Subcommand)]
enum SpecSubcommand {
    /// Validate specs against source code
    Check {
        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Initialize spec-sync configuration
    Init,
    /// List all specs in the project
    #[command(alias = "ls")]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Scaffold a new spec module
    New {
        /// Module name
        name: String,
    },
    /// Show a single spec's frontmatter, sections, and companions
    Show {
        /// Module name
        name: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Subcommand)]
enum WorkSubcommand {
    /// Start a new work branch
    Start {
        /// Branch name (will be sanitized for git)
        name: String,
        /// Branch type: feat, fix, chore, docs, hotfix, refactor (default: feat)
        #[arg(short = 't', long = "branch-type", value_name = "TYPE")]
        branch_type: Option<String>,
        /// Link to GitHub issue (prefixes branch name with issue number)
        #[arg(short, long, value_name = "NUMBER")]
        issue: Option<u64>,
        /// Override branch prefix entirely (e.g. "user/leif")
        #[arg(long)]
        prefix: Option<String>,
        /// Base branch to branch from (default: main)
        #[arg(long)]
        base: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Create a pull request from the current branch
    Pr {
        /// PR title (auto-generated from branch name if omitted)
        #[arg(short, long)]
        title: Option<String>,
        /// PR body (auto-generated from commits if omitted)
        #[arg(short, long)]
        body: Option<String>,
        /// Create as a draft PR
        #[arg(long)]
        draft: bool,
        /// Target base branch for the PR
        #[arg(long)]
        base: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Skip the preview/confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
        /// Generate the PR body via the configured AI provider (uses commit log + diff as context)
        #[arg(long)]
        ai: bool,
        /// Override AI provider for --ai (claude or ollama)
        #[arg(long, value_parser = ["claude", "ollama"])]
        provider: Option<String>,
        /// Override AI model for --ai
        #[arg(long)]
        model: Option<String>,
    },
    /// Show current branch and PR status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Subcommand)]
enum AiSubcommand {
    /// Show the active AI provider, model, and host
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List available models for the active (or specified) provider
    Models {
        /// Provider: claude or ollama (default: active provider)
        #[arg(long, value_name = "NAME", value_parser = ["claude", "ollama"])]
        provider: Option<String>,
        /// Filter models by substring (case-insensitive)
        #[arg(long, value_name = "QUERY")]
        search: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Select the active provider (and optionally model); interactive if args
    /// are omitted
    #[command(name = "use")]
    Use {
        /// Provider: claude or ollama
        #[arg(value_parser = ["claude", "ollama"])]
        provider: Option<String>,
        /// Model name (e.g. qwen3-coder:480b-cloud)
        model: Option<String>,
    },
}

#[derive(clap::Subcommand)]
enum ConfigAction {
    /// Get a config value
    Get {
        /// Config key (e.g. defaults.github_org)
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key (e.g. defaults.github_org)
        key: String,
        /// Value to set
        value: String,
    },
    /// Remove a config value
    Unset {
        /// Config key (e.g. defaults.github_org)
        key: String,
    },
    /// Add a value to a list config key (templates.paths, templates.repos)
    Add {
        /// Config key (templates.paths or templates.repos)
        key: String,
        /// Value to add
        value: String,
    },
    /// Remove a value from a list config key (templates.paths, templates.repos)
    Remove {
        /// Config key (templates.paths or templates.repos)
        key: String,
        /// Value to remove
        value: String,
    },
    /// Show all config values
    List,
    /// Show config file path
    Path,
    /// Initialize config with a preset (e.g. corvidlabs)
    Init {
        /// Preset name (available: corvidlabs)
        #[arg(long)]
        preset: Option<String>,
    },
}

#[derive(clap::Subcommand)]
enum LaneSubcommand {
    /// Run a lane by name
    Run {
        /// Lane name
        name: String,
        /// Show execution plan without running
        #[arg(long)]
        dry_run: bool,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// List available lanes
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add default lanes to fledge.toml
    Init {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Search GitHub for community lanes
    Search {
        /// Keyword to filter results
        query: Option<String>,
        /// Filter by author/owner
        #[arg(short, long)]
        author: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Import lanes from a GitHub repo (owner/repo)
    Import {
        /// GitHub repo (owner/repo) or full URL, optionally with @ref
        source: String,
        /// Skip all confirmation prompts
        #[arg(short, long)]
        yes: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Publish lanes to GitHub
    Publish {
        /// Path to the directory containing fledge.toml with lanes
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Publish under a GitHub organization
        #[arg(long)]
        org: Option<String>,
        /// Create as a private repository
        #[arg(long)]
        private: bool,
        /// Override the repository description
        #[arg(long)]
        description: Option<String>,
        /// Skip all confirmation prompts
        #[arg(short, long)]
        yes: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Scaffold a new lane repo
    Create {
        /// Lane repo name
        name: String,
        /// Parent directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Description (bypasses prompt)
        #[arg(short, long)]
        description: Option<String>,
        /// Skip all interactive prompts
        #[arg(short, long)]
        yes: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Validate lane definitions in fledge.toml
    Validate {
        /// Path to a directory containing fledge.toml
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(clap::Subcommand)]
enum PluginSubcommand {
    /// Install a plugin from GitHub
    Install {
        /// GitHub repo (owner/repo[@ref]) or full URL — use @tag to pin a version. Omit when using --defaults.
        source: Option<String>,
        /// Reinstall if already present
        #[arg(long)]
        force: bool,
        /// Skip all confirmation prompts (accept defaults)
        #[arg(short, long)]
        yes: bool,
        /// Install fledge's curated set of default plugins (github, deps, metrics, templates-remote, doctor)
        #[arg(long, conflicts_with = "source")]
        defaults: bool,
    },
    /// Remove an installed plugin
    Remove {
        /// Plugin name
        name: String,
    },
    /// Update installed plugins (git pull + rebuild)
    Update {
        /// Plugin name (omit to update all)
        name: Option<String>,
        /// Update only fledge's curated default plugins (skip community plugins)
        #[arg(long, conflicts_with = "name")]
        defaults: bool,
    },
    /// List installed plugins
    List,
    /// Audit installed plugins — show trust tiers, capabilities, and hooks
    Audit,
    /// Search for plugins on GitHub
    Search {
        /// Search query
        query: Option<String>,
        /// Filter by author/owner
        #[arg(short, long)]
        author: Option<String>,
        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Run a plugin command
    Run {
        /// Plugin command name
        name: String,
        /// Arguments to pass to the plugin
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Publish a plugin to GitHub
    Publish {
        /// Path to the plugin directory
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Publish under a GitHub organization
        #[arg(long)]
        org: Option<String>,
        /// Create as a private repository
        #[arg(long)]
        private: bool,
        /// Override the repository description
        #[arg(long)]
        description: Option<String>,
        /// Skip all confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },
    /// Scaffold a new plugin
    Create {
        /// Plugin name
        name: String,
        /// Parent directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Description (bypasses prompt)
        #[arg(short, long)]
        description: Option<String>,
        /// Skip all interactive prompts
        #[arg(short, long)]
        yes: bool,
    },
    /// Validate a plugin manifest
    Validate {
        /// Path to a directory containing plugin.toml
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {:#}", style("error:").red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Env var is honored regardless of how the CLI is invoked, so agent shells
    // can set FLEDGE_NON_INTERACTIVE=1 once and forget about it.
    utils::init_non_interactive_from_env();
    let cli = Cli::parse();
    if cli.non_interactive {
        utils::set_non_interactive(true);
    }

    match cli.command {
        Commands::Templates { action } => {
            handle_templates(action)?;
        }
        Commands::Config { action } => {
            handle_config(action)?;
        }
        Commands::Spec { action } => {
            let action = match action {
                SpecSubcommand::Check { strict, json } => spec::SpecAction::Check { strict, json },
                SpecSubcommand::Init => spec::SpecAction::Init,
                SpecSubcommand::List { json } => spec::SpecAction::List { json },
                SpecSubcommand::New { name } => spec::SpecAction::New { name },
                SpecSubcommand::Show { name, json } => spec::SpecAction::Show { name, json },
            };
            spec::run(action)?;
        }
        Commands::Work { action } => {
            let action = match action {
                WorkSubcommand::Start {
                    name,
                    branch_type,
                    issue,
                    prefix,
                    base,
                    json,
                } => work::WorkAction::Start {
                    name,
                    branch_type,
                    issue,
                    prefix,
                    base,
                    json,
                },
                WorkSubcommand::Pr {
                    title,
                    body,
                    draft,
                    base,
                    json,
                    yes,
                    ai,
                    provider,
                    model,
                } => work::WorkAction::Pr {
                    title,
                    body,
                    draft,
                    base,
                    json,
                    yes,
                    ai,
                    provider,
                    model,
                },
                WorkSubcommand::Status { json } => work::WorkAction::Status { json },
            };
            work::run(action)?;
        }
        Commands::Run {
            task,
            init,
            list,
            lang,
            json,
        } => {
            run::run(run::RunOptions {
                task,
                init,
                list,
                lang,
                json,
            })?;
        }
        Commands::Watch {
            name,
            lane,
            path,
            ext,
            debounce,
            clear,
        } => {
            let extensions = ext.map(|e| watch::parse_extensions(&e)).unwrap_or_default();
            watch::run(watch::WatchOptions {
                name,
                lane,
                path,
                extensions,
                debounce_ms: debounce,
                clear,
            })?;
        }
        Commands::Review {
            base,
            file,
            json,
            model,
            prompt,
            format,
            with_specs,
            no_auto_specs,
            provider,
            with_model,
            no_active,
        } => {
            let format: review::ReviewFormat =
                format.parse().map_err(|e: String| anyhow::anyhow!(e))?;
            review::run(review::ReviewOptions {
                base,
                file,
                json,
                model,
                prompt,
                format,
                with_specs,
                no_auto_specs,
                provider,
                with_model,
                no_active,
            })?;
        }
        Commands::Lanes { action } => {
            let action = match action {
                LaneSubcommand::Run {
                    name,
                    dry_run,
                    json,
                } => lanes::LaneAction::Run {
                    name,
                    dry_run,
                    json,
                },
                LaneSubcommand::List { json } => lanes::LaneAction::List { json },
                LaneSubcommand::Init { json } => lanes::LaneAction::Init { json },
                LaneSubcommand::Search {
                    query,
                    author,
                    json,
                } => lanes::LaneAction::Search {
                    query,
                    author,
                    json,
                },
                LaneSubcommand::Import { source, yes, json } => {
                    lanes::LaneAction::Import { source, yes, json }
                }
                LaneSubcommand::Publish {
                    path,
                    org,
                    private,
                    description,
                    yes,
                    json,
                } => lanes::LaneAction::Publish {
                    path,
                    org,
                    private,
                    description,
                    yes,
                    json,
                },
                LaneSubcommand::Create {
                    name,
                    output,
                    description,
                    yes,
                    json,
                } => lanes::LaneAction::Create {
                    name,
                    output,
                    description,
                    yes,
                    json,
                },
                LaneSubcommand::Validate { path, strict, json } => {
                    lanes::LaneAction::Validate { path, strict, json }
                }
                LaneSubcommand::External(args) => {
                    let name = args.first().cloned().unwrap_or_default();
                    let dry_run = args.iter().any(|a| a == "--dry-run");
                    let json = args.iter().any(|a| a == "--json");
                    lanes::LaneAction::Run {
                        name,
                        dry_run,
                        json,
                    }
                }
            };
            lanes::run(action)?;
        }
        Commands::Changelog {
            limit,
            tag,
            unreleased,
            json,
        } => {
            changelog::run(changelog::ChangelogOptions {
                limit,
                tag,
                unreleased,
                json,
            })?;
        }
        Commands::Doctor { json } => {
            doctor::run(doctor::DoctorOptions { json })?;
        }
        Commands::Introspect { json } => {
            let cmd = <Cli as clap::CommandFactory>::command();
            introspect::run(introspect::IntrospectOptions { json }, cmd)?;
        }
        Commands::Plugins { action, json } => {
            let action = match action {
                PluginSubcommand::Install {
                    source,
                    force,
                    yes,
                    defaults,
                } => plugin::PluginAction::Install {
                    source,
                    force: force || yes,
                    defaults,
                },
                PluginSubcommand::Remove { name } => plugin::PluginAction::Remove { name },
                PluginSubcommand::Update { name, defaults } => {
                    plugin::PluginAction::Update { name, defaults }
                }
                PluginSubcommand::List => plugin::PluginAction::List,
                PluginSubcommand::Audit => plugin::PluginAction::Audit,
                PluginSubcommand::Search {
                    query,
                    author,
                    limit,
                } => plugin::PluginAction::Search {
                    query,
                    author,
                    limit,
                },
                PluginSubcommand::Run { name, args } => plugin::PluginAction::Run { name, args },
                PluginSubcommand::Publish {
                    path,
                    org,
                    private,
                    description,
                    yes,
                } => plugin::PluginAction::Publish {
                    path,
                    org,
                    private,
                    description,
                    yes,
                },
                PluginSubcommand::Create {
                    name,
                    output,
                    description,
                    yes,
                } => plugin::PluginAction::Create {
                    name,
                    output,
                    description,
                    yes,
                },
                PluginSubcommand::Validate { path, strict, json } => {
                    plugin::PluginAction::Validate { path, strict, json }
                }
            };
            plugin::run(plugin::PluginOptions { action, json })?;
        }
        Commands::Release {
            bump,
            dry_run,
            no_tag,
            no_changelog,
            no_bump,
            push,
            pre_lane,
            allow_dirty,
        } => {
            release::run(release::ReleaseOptions {
                bump,
                dry_run,
                no_tag,
                no_changelog,
                no_bump,
                push,
                pre_lane,
                allow_dirty,
            })?;
        }
        Commands::Ai { action } => {
            let action = match action {
                AiSubcommand::Status { json } => ai::AiAction::Status { json },
                AiSubcommand::Models {
                    provider,
                    search,
                    json,
                } => ai::AiAction::Models {
                    provider,
                    search,
                    json,
                },
                AiSubcommand::Use { provider, model } => ai::AiAction::Use { provider, model },
            };
            ai::run(action)?;
        }
        Commands::Ask {
            question,
            json,
            with_specs,
            no_spec_index,
            provider,
            model,
        } => {
            if question.is_empty() {
                anyhow::bail!("Please provide a question. Usage: fledge ask <question>");
            }
            ask::run(ask::AskOptions {
                question: question.join(" "),
                json,
                with_specs,
                no_spec_index,
                provider,
                model,
            })?;
        }
        Commands::Completions { shell, install } => {
            if install {
                install_completions(shell)?;
            } else {
                let shell = shell.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Shell argument is required when not using --install. Usage: fledge completions <bash|zsh|fish>"
                    )
                })?;
                clap_complete::generate(
                    shell,
                    &mut Cli::command(),
                    "fledge",
                    &mut std::io::stdout(),
                );
            }
        }
        Commands::External(args) => {
            let cmd_name = args.first().ok_or_else(|| {
                anyhow::anyhow!(
                    "no subcommand provided\n\n  tip: use {} for help",
                    style("fledge help").cyan()
                )
            })?;
            let cmd_args: Vec<String> = args[1..].to_vec();
            if plugin::resolve_plugin_command(cmd_name).is_some() {
                plugin::run(plugin::PluginOptions {
                    action: plugin::PluginAction::Run {
                        name: cmd_name.clone(),
                        args: cmd_args,
                    },
                    json: false,
                })?;
            } else {
                anyhow::bail!(
                    "unrecognized subcommand '{}'\n\n  tip: use {} for help",
                    cmd_name,
                    style("fledge help").cyan()
                );
            }
        }
    }

    Ok(())
}

fn handle_templates(action: TemplatesSubcommand) -> Result<()> {
    match action {
        TemplatesSubcommand::Init {
            name,
            template,
            output,
            author,
            org,
            no_git,
            no_install,
            refresh,
            dry_run,
            yes,
            json,
        } => {
            init::run(init::InitOptions {
                name,
                template,
                output,
                author,
                org,
                no_git,
                no_install,
                refresh,
                dry_run,
                yes,
                json,
            })?;
        }
        TemplatesSubcommand::Create {
            name,
            output,
            description,
            render_patterns,
            hooks,
            prompts,
            yes,
            json,
        } => {
            create_template::run(create_template::CreateTemplateOptions {
                name,
                output,
                description,
                render_patterns,
                hooks,
                prompts,
                yes,
                json,
            })?;
        }
        TemplatesSubcommand::Validate { path, strict, json } => {
            validate::run(validate::ValidateOptions { path, strict, json })?;
        }
        TemplatesSubcommand::List { json } => {
            list_templates(json)?;
        }
        TemplatesSubcommand::Search {
            query,
            author,
            limit,
            json,
        } => {
            search_templates(query.as_deref(), author.as_deref(), limit, json)?;
        }
        TemplatesSubcommand::Publish {
            path,
            org,
            private,
            description,
            yes,
            json,
        } => {
            publish_template(
                &path,
                org.as_deref(),
                private,
                description.as_deref(),
                yes,
                json,
            )?;
        }
    }
    Ok(())
}

fn handle_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Get { key } => {
            let config = config::Config::load()?;
            if !config::Config::is_valid_key(&key) {
                anyhow::bail!(
                    "Unknown config key '{}'. Valid keys: defaults.author, defaults.github_org, defaults.license, github.token, templates.paths, templates.repos",
                    key
                );
            }
            match config.get(&key) {
                Some(value) if !value.is_empty() => println!("{}", value),
                _ => println!("{} {} is not set", style("*").cyan().bold(), key),
            }
        }
        ConfigAction::Set { key, value } => {
            let mut config = config::Config::load()?;
            config.set(&key, &value)?;
            config.save()?;
            println!(
                "{} Set {} = {}",
                style("✅").green().bold(),
                style(&key).cyan(),
                style(&value).green()
            );
        }
        ConfigAction::Unset { key } => {
            let mut config = config::Config::load()?;
            config.unset(&key)?;
            config.save()?;
            println!(
                "{} Unset {}",
                style("✅").green().bold(),
                style(&key).cyan()
            );
        }
        ConfigAction::Add { key, value } => {
            let mut config = config::Config::load()?;
            config.add_to_list(&key, &value)?;
            config.save()?;
            println!(
                "{} Added {} to {}",
                style("✅").green().bold(),
                style(&value).green(),
                style(&key).cyan()
            );
        }
        ConfigAction::Remove { key, value } => {
            let mut config = config::Config::load()?;
            let removed = config.remove_from_list(&key, &value)?;
            if removed {
                config.save()?;
                println!(
                    "{} Removed {} from {}",
                    style("✅").green().bold(),
                    style(&value).green(),
                    style(&key).cyan()
                );
            } else {
                println!(
                    "{} {} not found in {}",
                    style("*").cyan().bold(),
                    style(&value).dim(),
                    style(&key).cyan()
                );
            }
        }
        ConfigAction::List => {
            let config = config::Config::load()?;
            let path = config::Config::config_path();
            println!(
                "{} Config: {}\n",
                style("*").cyan().bold(),
                style(path.display()).dim()
            );
            print_config_entry("defaults.author", &config.defaults.author);
            print_config_entry("defaults.github_org", &config.defaults.github_org);
            print_config_entry("defaults.license", &config.defaults.license);
            print_config_entry(
                "github.token",
                &config.github.token.as_ref().map(|_| "***".to_string()),
            );
            if config.templates.paths.is_empty() {
                println!(
                    "  {:<24} {}",
                    style("templates.paths").cyan(),
                    style("(none)").dim()
                );
            } else {
                for (i, p) in config.templates.paths.iter().enumerate() {
                    if i == 0 {
                        println!("  {:<24} {}", style("templates.paths").cyan(), p);
                    } else {
                        println!("  {:<24} {}", "", p);
                    }
                }
            }
            if config.templates.repos.is_empty() {
                println!(
                    "  {:<24} {}",
                    style("templates.repos").cyan(),
                    style("(none)").dim()
                );
            } else {
                for (i, r) in config.templates.repos.iter().enumerate() {
                    if i == 0 {
                        println!("  {:<24} {}", style("templates.repos").cyan(), r);
                    } else {
                        println!("  {:<24} {}", "", r);
                    }
                }
            }
        }
        ConfigAction::Path => {
            println!("{}", config::Config::config_path().display());
        }
        ConfigAction::Init { preset } => {
            config::init_config(preset.as_deref())?;
        }
    }
    Ok(())
}

fn print_config_entry(key: &str, value: &Option<impl std::fmt::Display>) {
    match value {
        Some(v) => println!("  {:<24} {}", style(key).cyan(), v),
        None => println!("  {:<24} {}", style(key).cyan(), style("(not set)").dim()),
    }
}

fn install_completions(shell: Option<Shell>) -> Result<()> {
    let shell = shell.unwrap_or_else(|| {
        let shell_env = std::env::var("SHELL").unwrap_or_default();
        if shell_env.ends_with("zsh") {
            Shell::Zsh
        } else if shell_env.ends_with("fish") {
            Shell::Fish
        } else {
            Shell::Bash
        }
    });

    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;

    let dest = match shell {
        Shell::Bash => {
            let dir = home.join(".local/share/bash-completion/completions");
            std::fs::create_dir_all(&dir)?;
            dir.join("fledge")
        }
        Shell::Zsh => {
            let dir = home.join(".zfunc");
            std::fs::create_dir_all(&dir)?;
            dir.join("_fledge")
        }
        Shell::Fish => {
            let dir = home.join(".config/fish/completions");
            std::fs::create_dir_all(&dir)?;
            dir.join("fledge.fish")
        }
        _ => anyhow::bail!(
            "auto-install not supported for {:?} — use `fledge completions <shell>` to generate manually",
            shell
        ),
    };

    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut Cli::command(), "fledge", &mut buf);
    std::fs::write(&dest, buf)?;

    println!(
        "{} Installed {} completions to {}",
        style("✅").green().bold(),
        style(format!("{shell:?}")).cyan(),
        style(dest.display()).dim()
    );

    if matches!(shell, Shell::Zsh) {
        println!(
            "\n  {}",
            style("Add to your .zshrc if not already present:").dim()
        );
        println!("    fpath=(~/.zfunc $fpath)");
        println!("    autoload -Uz compinit && compinit");
    }

    Ok(())
}

fn list_templates(json: bool) -> Result<()> {
    let config = config::Config::load()?;
    let extra_paths = config.extra_template_paths();
    let token = config.github_token();
    let available = templates::discover_templates_with_repos(
        &extra_paths,
        config.template_repos(),
        token.as_deref(),
    )?;

    if available.is_empty() {
        anyhow::bail!("No templates found. Configure template sources via `fledge config add templates.repos <owner/repo>`, add templates to the templates/ directory, or set templates.paths via `fledge config add templates.paths <path>`.");
    }

    if json {
        let entries: Vec<serde_json::Value> = available
            .iter()
            .map(|t| {
                let source_kind = match &t.source {
                    Some(s) if s.starts_with("http") || s.contains('/') => "remote",
                    Some(_) => "local",
                    None => "builtin",
                };
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "source": source_kind,
                    "source_ref": t.source,
                    "path": t.path.display().to_string(),
                })
            })
            .collect();
        let result = serde_json::json!({
            "schema_version": 1,
            "templates": entries,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", style("Available templates:").bold());
        for t in &available {
            println!(
                "  {:<14} {}",
                style(&t.name).green(),
                style(&t.description).dim()
            );
        }
    }

    Ok(())
}

fn search_templates(
    query: Option<&str>,
    author: Option<&str>,
    limit: usize,
    json: bool,
) -> Result<()> {
    use anyhow::Context as _;
    let config = config::Config::load()?;
    let token = config.github_token();
    let q = search::build_search_query_ex(query, author, "fledge-template");
    let per_page = limit.clamp(1, 100).to_string();

    let sp = spinner::Spinner::start("Searching GitHub for community templates:");
    let body = github::github_api_get(
        "/search/repositories",
        token.as_deref(),
        &[("q", &q), ("sort", "stars"), ("per_page", &per_page)],
    )
    .context("searching GitHub for template repos")?;
    sp.finish();

    let mut results = search::parse_search_response(&body)?;
    results.truncate(limit);

    if results.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "{} No community templates found{}.",
                style("*").cyan().bold(),
                query
                    .map(|q| format!(" matching '{q}'"))
                    .unwrap_or_default()
            );
        }
        return Ok(());
    }

    if json {
        let entries: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let tier = trust::determine_trust_tier_from_owner(&r.owner);
                serde_json::json!({
                    "owner": r.owner,
                    "name": r.name,
                    "description": r.description,
                    "stars": r.stars,
                    "url": r.url,
                    "topics": r.topics,
                    "trust_tier": tier.label(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    println!("{}\n", style("Community templates on GitHub:").bold());
    let max_name = results
        .iter()
        .map(|r| r.full_name().len())
        .max()
        .unwrap_or(0);
    for r in &results {
        let tier = trust::determine_trust_tier_from_owner(&r.owner);
        let stars = search::format_stars(r.stars);
        let desc = if r.description.chars().count() > 60 {
            let truncated: String = r.description.chars().take(57).collect();
            format!("{truncated}...")
        } else {
            r.description.clone()
        };
        let topic_str = if r.topics.is_empty() {
            String::new()
        } else {
            format!(" [{}]", r.topics.join(", "))
        };
        println!(
            "  {:<width$}  [{}]  {}  {}{}",
            style(&r.full_name()).green(),
            tier.styled_label(),
            style(format!("(⭐ {})", stars)).dim(),
            style(&desc).dim(),
            style(&topic_str).cyan(),
            width = max_name,
        );
    }
    println!(
        "\n{}",
        style("Use with: fledge templates init --template <owner/repo>").dim()
    );
    Ok(())
}

fn publish_template(
    path: &std::path::Path,
    org: Option<&str>,
    private: bool,
    description: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    use anyhow::Context as _;
    let yes = yes || utils::is_non_interactive() || json;
    let config = config::Config::load()?;
    let token = config.github_token().ok_or_else(|| {
        anyhow::anyhow!(
            "No GitHub token configured. Run: fledge config set github.token <your-token>"
        )
    })?;

    let path = path
        .canonicalize()
        .with_context(|| format!("Directory not found: {}", path.display()))?;

    // Validate the template before publishing — same gate `fledge templates validate` uses.
    validate::run(validate::ValidateOptions {
        path: path.clone(),
        strict: false,
        json: false,
    })?;

    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("fledge-template");
    let repo_name = dir_name.to_string();
    let desc = description.unwrap_or("A fledge template");

    let owner = match org {
        Some(o) => o.to_string(),
        None => publish::get_authenticated_user(&token)?,
    };

    if !json {
        println!(
            "{} Publishing template as {}/{}",
            style("➡️").cyan().bold(),
            style(&owner).green(),
            style(&repo_name).green()
        );
    }

    let sp = if json {
        None
    } else {
        Some(spinner::Spinner::start("Checking repository:"))
    };
    let repo_exists = publish::check_repo_exists(&owner, &repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }

    let mut created_repo = false;
    if repo_exists {
        if !yes {
            utils::require_interactive("yes")?;
            let confirm =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt(format!(
                        "Repository {}/{} already exists. Push update?",
                        owner, repo_name
                    ))
                    .default(false)
                    .interact()?;
            if !confirm {
                if json {
                    let result = serde_json::json!({
                        "schema_version": 1,
                        "action": "publish",
                        "cancelled": true,
                        "repo": {
                            "owner": owner,
                            "name": repo_name,
                            "url": format!("https://github.com/{owner}/{repo_name}"),
                            "exists": true,
                        },
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{} Cancelled.", style("*").cyan().bold());
                }
                return Ok(());
            }
        }
    } else {
        let sp = if json {
            None
        } else {
            Some(spinner::Spinner::start("Creating repository:"))
        };
        publish::create_github_repo(&repo_name, desc, private, org, &token)?;
        if let Some(s) = sp {
            s.finish();
        }
        created_repo = true;
        if !json {
            println!(
                "  {} Created repository {}/{}",
                style("✅").green().bold(),
                owner,
                repo_name
            );
        }
    }

    let sp = if json {
        None
    } else {
        Some(spinner::Spinner::start("Setting repository topics:"))
    };
    publish::set_repo_topic(&owner, &repo_name, "fledge-template", &token)?;
    if let Some(s) = sp {
        s.finish();
    }
    if !json {
        println!(
            "  {} Set {} topic",
            style("✅").green().bold(),
            style("fledge-template").cyan()
        );
    }

    let sp = if json {
        None
    } else {
        Some(spinner::Spinner::start("Pushing template files:"))
    };
    publish::push_directory(&path, &owner, &repo_name, &token)?;
    if let Some(s) = sp {
        s.finish();
    }

    if json {
        let result = serde_json::json!({
            "schema_version": 1,
            "action": "publish",
            "repo": {
                "owner": owner,
                "name": repo_name,
                "url": format!("https://github.com/{owner}/{repo_name}"),
                "created": created_repo,
                "private": private,
            },
            "template": {
                "description": desc,
            },
            "topic": "fledge-template",
            "use_hint": format!("fledge templates init <name> --template {owner}/{repo_name}"),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("  {} Pushed template files", style("✅").green().bold());
        println!(
            "\n{} Published! Use with:\n\n  {}",
            style("✅").green().bold(),
            style(format!(
                "fledge templates init --template {}/{}",
                owner, repo_name
            ))
            .cyan()
        );
    }

    Ok(())
}
