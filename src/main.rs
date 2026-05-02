use anyhow::Result;
use clap::{CommandFactory, Parser};
use console::style;

mod ai;
mod ask;
mod changelog;
mod cli;
mod config;
mod config_cmds;
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
mod template_cmds;
mod templates;
mod trust;
mod utils;
mod validate;
mod versioning;
mod watch;
mod work;

#[cfg(test)]
mod test_support;

use cli::*;

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
            template_cmds::handle_templates(action)?;
        }
        Commands::Config { action } => {
            config_cmds::handle_config(action)?;
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
                WorkSubcommand::Commit {
                    message,
                    commit_type,
                    scope,
                    all,
                    ai,
                    provider,
                    model,
                    json,
                } => work::WorkAction::Commit {
                    message,
                    commit_type,
                    scope,
                    all,
                    ai,
                    provider,
                    model,
                    json,
                },
                WorkSubcommand::Push { force, json } => work::WorkAction::Push { force, json },
                WorkSubcommand::Status { json } => work::WorkAction::Status { json },
                WorkSubcommand::Pr { _args: _ } => work::WorkAction::DeprecatedPr,
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
            json,
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
                json,
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
                template_cmds::install_completions(shell)?;
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
