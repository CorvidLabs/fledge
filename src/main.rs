use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use console::style;
use std::path::PathBuf;

mod ask;
mod changelog;
mod checks;
mod config;
mod create_template;
mod deps;
mod doctor;
mod github;
mod init;
mod issues;
mod lanes;
mod metrics;
mod plugin;
mod prompts;
mod protocol;
mod prs;
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
mod update;
mod utils;
mod validate;
mod versioning;
mod work;

#[derive(Parser)]
#[command(
    name = "fledge",
    version,
    about = "Dev-lifecycle CLI — get your projects ready to fly."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Manage templates (init, create, search, update, publish, validate, list)
    #[command(alias = "template")]
    Templates {
        #[command(subcommand)]
        action: TemplatesSubcommand,
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
    /// Manage specs (check, init, new)
    Spec {
        #[command(subcommand)]
        action: SpecSubcommand,
    },
    /// Feature branch and PR workflow
    Work {
        #[command(subcommand)]
        action: WorkSubcommand,
    },
    /// List and view GitHub issues
    Issues {
        #[command(subcommand)]
        action: Option<IssuesSubcommand>,
        /// Filter by state (open, closed, all)
        #[arg(short, long, default_value = "open", global = true)]
        state: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "20", global = true)]
        limit: usize,
        /// Output results as JSON
        #[arg(long, global = true)]
        json: bool,
        /// Filter by label
        #[arg(long, global = true)]
        label: Option<String>,
    },
    /// List and view GitHub pull requests
    Prs {
        #[command(subcommand)]
        action: Option<PrsSubcommand>,
        /// Filter by state (open, closed, all)
        #[arg(short, long, default_value = "open", global = true)]
        state: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "20", global = true)]
        limit: usize,
        /// Output results as JSON
        #[arg(long, global = true)]
        json: bool,
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
    },
    /// View CI/CD check status for a branch
    Checks {
        /// Branch to check (default: current branch)
        #[arg(short, long)]
        branch: Option<String>,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
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
    },
    /// Manage and run composable workflow pipelines
    #[command(alias = "lane")]
    Lanes {
        #[command(subcommand)]
        action: LaneSubcommand,
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
    /// Diagnose project environment health
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Project code metrics (LOC, churn, test ratio)
    Metrics {
        /// Show file churn from git history
        #[arg(long)]
        churn: bool,
        /// Show test file detection and ratio
        #[arg(long)]
        tests: bool,
        /// Maximum entries for churn output
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Check dependency health (outdated, audit, licenses)
    Deps {
        /// Check for outdated dependencies
        #[arg(long)]
        outdated: bool,
        /// Run security audit via ecosystem tools
        #[arg(long)]
        audit: bool,
        /// Show dependency licenses
        #[arg(long)]
        licenses: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    /// Ask a question about your codebase
    Ask {
        /// The question to ask
        question: Vec<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    },
    /// Search for templates on GitHub
    Search {
        /// Keyword to filter results
        query: Option<String>,
        /// Filter by author/owner
        #[arg(short, long)]
        author: Option<String>,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Update project from its source template
    Update {
        /// Show what would change without writing anything
        #[arg(long)]
        dry_run: bool,
        /// Force re-clone of cached remote templates
        #[arg(long)]
        refresh: bool,
    },
    /// Publish a template to GitHub
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
    List,
}

#[derive(clap::Subcommand)]
enum SpecSubcommand {
    /// Validate specs against source code
    Check {
        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
    },
    /// Initialize spec-sync configuration
    Init,
    /// Scaffold a new spec module
    New {
        /// Module name
        name: String,
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
    },
    /// Create a pull request from the current branch
    Pr {
        /// PR title (auto-generated from branch name if omitted)
        #[arg(short, long)]
        title: Option<String>,
        /// PR body/description
        #[arg(short, long)]
        body: Option<String>,
        /// Create as a draft PR
        #[arg(long)]
        draft: bool,
        /// Target base branch for the PR
        #[arg(long)]
        base: Option<String>,
    },
    /// Show current branch and PR status
    Status,
}

#[derive(clap::Subcommand)]
enum IssuesSubcommand {
    /// View a specific issue
    View {
        /// Issue number
        number: u64,
    },
}

#[derive(clap::Subcommand)]
enum PrsSubcommand {
    /// View a specific pull request
    View {
        /// PR number
        number: u64,
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
    },
    /// List available lanes
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add default lanes to fledge.toml
    Init,
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
        /// GitHub repo (owner/repo[@ref]) or full URL — use @tag to pin a version
        source: String,
        /// Reinstall if already present
        #[arg(long)]
        force: bool,
        /// Skip all confirmation prompts (accept defaults)
        #[arg(short, long)]
        yes: bool,
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
    let cli = Cli::parse();

    match cli.command {
        Commands::Templates { action } => {
            handle_templates(action)?;
        }
        Commands::Config { action } => {
            handle_config(action)?;
        }
        Commands::Spec { action } => {
            let action = match action {
                SpecSubcommand::Check { strict } => spec::SpecAction::Check { strict },
                SpecSubcommand::Init => spec::SpecAction::Init,
                SpecSubcommand::New { name } => spec::SpecAction::New { name },
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
                } => work::WorkAction::Start {
                    name,
                    branch_type,
                    issue,
                    prefix,
                    base,
                },
                WorkSubcommand::Pr {
                    title,
                    body,
                    draft,
                    base,
                } => work::WorkAction::Pr {
                    title,
                    body,
                    draft,
                    base,
                },
                WorkSubcommand::Status => work::WorkAction::Status,
            };
            work::run(action)?;
        }
        Commands::Issues {
            action,
            state,
            limit,
            json,
            label,
        } => {
            let action = match action {
                Some(IssuesSubcommand::View { number }) => {
                    issues::IssuesAction::View { number, json }
                }
                None => issues::IssuesAction::List {
                    state,
                    limit,
                    json,
                    label,
                },
            };
            issues::run(action)?;
        }
        Commands::Prs {
            action,
            state,
            limit,
            json,
        } => {
            let action = match action {
                Some(PrsSubcommand::View { number }) => prs::PrsAction::View { number, json },
                None => prs::PrsAction::List { state, limit, json },
            };
            prs::run(action)?;
        }
        Commands::Checks { branch, json } => {
            checks::run(checks::ChecksOptions { branch, json })?;
        }
        Commands::Run {
            task,
            init,
            list,
            lang,
        } => {
            run::run(run::RunOptions {
                task,
                init,
                list,
                lang,
            })?;
        }
        Commands::Review { base, file, json } => {
            review::run(review::ReviewOptions { base, file, json })?;
        }
        Commands::Lanes { action } => {
            let action = match action {
                LaneSubcommand::Run { name, dry_run } => lanes::LaneAction::Run { name, dry_run },
                LaneSubcommand::List { json } => lanes::LaneAction::List { json },
                LaneSubcommand::Init => lanes::LaneAction::Init,
                LaneSubcommand::Search {
                    query,
                    author,
                    json,
                } => lanes::LaneAction::Search {
                    query,
                    author,
                    json,
                },
                LaneSubcommand::Import { source, yes } => lanes::LaneAction::Import { source, yes },
                LaneSubcommand::Publish {
                    path,
                    org,
                    private,
                    description,
                    yes,
                } => lanes::LaneAction::Publish {
                    path,
                    org,
                    private,
                    description,
                    yes,
                },
                LaneSubcommand::Create {
                    name,
                    output,
                    description,
                    yes,
                } => lanes::LaneAction::Create {
                    name,
                    output,
                    description,
                    yes,
                },
                LaneSubcommand::Validate { path, strict, json } => {
                    lanes::LaneAction::Validate { path, strict, json }
                }
                LaneSubcommand::External(args) => {
                    let name = args.first().cloned().unwrap_or_default();
                    let dry_run = args.iter().any(|a| a == "--dry-run");
                    lanes::LaneAction::Run { name, dry_run }
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
        Commands::Metrics {
            churn,
            tests,
            limit,
            json,
        } => {
            metrics::run(metrics::MetricsOptions {
                churn,
                tests,
                json,
                limit,
            })?;
        }
        Commands::Deps {
            outdated,
            audit,
            licenses,
            json,
        } => {
            deps::run(deps::DepsOptions {
                outdated,
                audit,
                licenses,
                json,
            })?;
        }
        Commands::Plugins { action, json } => {
            let action = match action {
                PluginSubcommand::Install { source, force, yes } => plugin::PluginAction::Install {
                    source,
                    force: force || yes,
                },
                PluginSubcommand::Remove { name } => plugin::PluginAction::Remove { name },
                PluginSubcommand::Update { name } => plugin::PluginAction::Update { name },
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
            push,
            pre_lane,
            allow_dirty,
        } => {
            release::run(release::ReleaseOptions {
                bump,
                dry_run,
                no_tag,
                no_changelog,
                push,
                pre_lane,
                allow_dirty,
            })?;
        }
        Commands::Ask { question, json } => {
            if question.is_empty() {
                anyhow::bail!("Please provide a question. Usage: fledge ask <question>");
            }
            ask::run(ask::AskOptions {
                question: question.join(" "),
                json,
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
        } => {
            create_template::run(create_template::CreateTemplateOptions {
                name,
                output,
                description,
                render_patterns,
                hooks,
                prompts,
                yes,
            })?;
        }
        TemplatesSubcommand::Search {
            query,
            author,
            limit,
            json,
        } => {
            search::run(search::SearchOptions {
                query,
                author,
                limit,
                json,
            })?;
        }
        TemplatesSubcommand::Update { dry_run, refresh } => {
            update::run(update::UpdateOptions { dry_run, refresh })?;
        }
        TemplatesSubcommand::Publish {
            path,
            org,
            private,
            description,
            yes,
        } => {
            publish::run(publish::PublishOptions {
                path,
                org,
                private,
                description,
                yes,
            })?;
        }
        TemplatesSubcommand::Validate { path, strict, json } => {
            validate::run(validate::ValidateOptions { path, strict, json })?;
        }
        TemplatesSubcommand::List => {
            list_templates()?;
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

fn list_templates() -> Result<()> {
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

    println!("{}", style("Available templates:").bold());
    for t in &available {
        println!(
            "  {:<14} {}",
            style(&t.name).green(),
            style(&t.description).dim()
        );
    }

    Ok(())
}
