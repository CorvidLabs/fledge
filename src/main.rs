use clap::Parser;

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
        /// Template to use
        #[arg(short, long)]
        template: Option<String>,
    },
    /// List available templates
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name, template } => {
            println!("Scaffolding project: {name}");
            if let Some(t) = template {
                println!("Using template: {t}");
            }
        }
        Commands::List => {
            println!("Available templates:");
            println!("  (none yet — add templates to get started)");
        }
    }
}
