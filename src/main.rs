use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use console::style;
use std::path::PathBuf;

mod config;
mod create_template;
mod init;
mod prompts;
mod publish;
mod remote;
mod search;
mod templates;
#[cfg(feature = "tui")]
mod tui;

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
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
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
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "fledge", &mut std::io::stdout());
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
