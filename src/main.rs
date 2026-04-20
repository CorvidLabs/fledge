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
mod github;
mod init;
mod issues;
mod prompts;
mod prs;
mod publish;
mod remote;
mod review;
mod run;
mod search;
mod spec;
mod templates;
#[cfg(feature = "tui")]
mod tui;
mod update;
mod versioning;
mod work;

#[derive(Parser)]
#[command(name = "fledge", version, about = "Get your projects ready to fly.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
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
    /// List available templates
    List,
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
    /// Scaffold a new fledge template
    CreateTemplate {
        /// Template name
        name: String,
        /// Parent directory for the template
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    /// Search for templates on GitHub
    Search {
        /// Keyword to filter results
        query: Option<String>,
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
        /// Skip all confirmation prompts
        #[arg(short, long)]
        yes: bool,
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
    /// Ask a question about your codebase
    Ask {
        /// The question to ask
        question: Vec<String>,
    },
    /// Interactive TUI for browsing and scaffolding templates (requires --features tui)
    #[cfg(feature = "tui")]
    Tui {
        /// Parent directory for the project
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// Skip git init and initial commit
        #[arg(long)]
        no_git: bool,
    },
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
    /// Start a new feature branch
    Start {
        /// Feature name (will be sanitized and prefixed with feat/)
        name: String,
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
        Commands::Init {
            name,
            template,
            output,
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
                no_git,
                no_install,
                refresh,
                dry_run,
                yes,
            })?;
        }
        Commands::List => {
            list_templates()?;
        }
        Commands::Config { action } => {
            handle_config(action)?;
        }
        Commands::CreateTemplate { name, output } => {
            create_template::run(create_template::CreateTemplateOptions { name, output })?;
        }
        Commands::Update {
            dry_run,
            refresh,
            yes,
        } => {
            update::run(update::UpdateOptions {
                dry_run,
                refresh,
                yes,
            })?;
        }
        Commands::Search { query, limit, json } => {
            search::run(search::SearchOptions { query, limit, json })?;
        }
        Commands::Publish {
            path,
            org,
            private,
            description,
        } => {
            publish::run(publish::PublishOptions {
                path,
                org,
                private,
                description,
            })?;
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
                WorkSubcommand::Start { name, base } => work::WorkAction::Start { name, base },
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
        Commands::Run { task, init, list } => {
            run::run(run::RunOptions { task, init, list })?;
        }
        Commands::Review { base, file } => {
            review::run(review::ReviewOptions { base, file })?;
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
        Commands::Ask { question } => {
            if question.is_empty() {
                anyhow::bail!("Please provide a question. Usage: fledge ask <question>");
            }
            ask::run(ask::AskOptions {
                question: question.join(" "),
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
        #[cfg(feature = "tui")]
        Commands::Tui { output, no_git } => {
            tui::run(output, no_git)?;
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
                style("✓").green().bold(),
                style(&key).cyan(),
                style(&value).green()
            );
        }
        ConfigAction::Unset { key } => {
            let mut config = config::Config::load()?;
            config.unset(&key)?;
            config.save()?;
            println!("{} Unset {}", style("✓").green().bold(), style(&key).cyan());
        }
        ConfigAction::Add { key, value } => {
            let mut config = config::Config::load()?;
            config.add_to_list(&key, &value)?;
            config.save()?;
            println!(
                "{} Added {} to {}",
                style("✓").green().bold(),
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
                    style("✓").green().bold(),
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
        style("✓").green().bold(),
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
        println!("No templates found.");
        return Ok(());
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
