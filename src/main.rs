use anyhow::Result;
use clap::Parser;
use console::style;
use std::path::PathBuf;

mod config;
mod init;
mod prompts;
mod remote;
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
    },
    /// List available templates
    List,
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            name,
            template,
            output,
            no_git,
            no_install,
            refresh,
        } => {
            init::run(init::InitOptions {
                name,
                template,
                output,
                no_git,
                no_install,
                refresh,
            })?;
        }
        Commands::List => {
            list_templates()?;
        }
        #[cfg(feature = "tui")]
        Commands::Tui { output, no_git } => {
            tui::run(output, no_git)?;
        }
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
