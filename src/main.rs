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
mod envelope;
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
    // Restore the default SIGPIPE disposition so fledge dies quietly when its
    // stdout is closed early (e.g. `fledge introspect --json | head`), like a
    // normal Unix tool. Rust ignores SIGPIPE by default, which otherwise turns a
    // broken pipe into a panic ("failed printing to stdout: Broken pipe") and
    // exit code 101.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

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
        Commands::Spec { action } => spec::run(spec_action_from(action))?,
        Commands::Work { action } => work::run(work_action_from(action))?,
        Commands::Run {
            task,
            init,
            list,
            lang,
            json,
            args,
        } => {
            run::run(run::RunOptions {
                task,
                init,
                list,
                lang,
                json,
                args,
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
        Commands::Lanes { action } => lanes::run(lane_action_from(action))?,
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
            plugin::run(plugin::PluginOptions {
                action: plugin_action_from(action),
                json,
            })?;
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
        Commands::Ai { action } => ai::run(ai_action_from(action))?,
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

// ── CLI-subcommand → handler-action translation ─────────────────────────────
// These map the clap-derived `*Subcommand` enums onto each handler's own action
// enum. Kept out of `run` so the top-level dispatch stays a flat one-line-per-
// command match, and so the non-trivial bits (plugin install's force||yes, the
// external-lane flag parsing) are isolated and unit-testable.

fn spec_action_from(action: SpecSubcommand) -> spec::SpecAction {
    match action {
        SpecSubcommand::Check { strict, json } => spec::SpecAction::Check { strict, json },
        SpecSubcommand::Init => spec::SpecAction::Init,
        SpecSubcommand::List { json } => spec::SpecAction::List { json },
        SpecSubcommand::New { name } => spec::SpecAction::New { name },
        SpecSubcommand::Show { name, json } => spec::SpecAction::Show { name, json },
    }
}

fn work_action_from(action: WorkSubcommand) -> work::WorkAction {
    match action {
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
    }
}

/// Parse the raw args of an external `fledge lanes <name> [flags]` invocation
/// (the clap `External` catch-all) into `(name, dry_run, json, from)`.
fn parse_external_lane_args(args: &[String]) -> (String, bool, bool, Option<String>) {
    let name = args.first().cloned().unwrap_or_default();
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let json = args.iter().any(|a| a == "--json");
    let from = args
        .iter()
        .position(|a| a == "--from")
        .and_then(|pos| args.get(pos + 1).cloned());
    (name, dry_run, json, from)
}

fn lane_action_from(action: LaneSubcommand) -> lanes::LaneAction {
    match action {
        LaneSubcommand::Run {
            name,
            dry_run,
            json,
            from,
        } => lanes::LaneAction::Run {
            name,
            dry_run,
            json,
            from,
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
            let (name, dry_run, json, from) = parse_external_lane_args(&args);
            lanes::LaneAction::Run {
                name,
                dry_run,
                json,
                from,
            }
        }
    }
}

fn plugin_action_from(action: PluginSubcommand) -> plugin::PluginAction {
    match action {
        PluginSubcommand::Install {
            source,
            force,
            yes,
            copy,
            defaults,
        } => plugin::PluginAction::Install {
            source,
            force: force || yes,
            copy,
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
            topic,
            trust_tier,
            limit,
            interactive,
        } => plugin::PluginAction::Search {
            query,
            author,
            topic,
            trust_tier,
            limit,
            interactive,
        },
        PluginSubcommand::Recommend => plugin::PluginAction::Recommend,
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
            wasm,
        } => plugin::PluginAction::Create {
            name,
            output,
            description,
            yes,
            wasm,
        },
        PluginSubcommand::Validate { path, strict, json } => {
            plugin::PluginAction::Validate { path, strict, json }
        }
    }
}

fn ai_action_from(action: AiSubcommand) -> ai::AiAction {
    match action {
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
    }
}

#[cfg(test)]
mod main_tests {
    use super::*;

    #[test]
    fn external_lane_args_bare_name() {
        let (name, dry_run, json, from) = parse_external_lane_args(&["ci".to_string()]);
        assert_eq!(name, "ci");
        assert!(!dry_run);
        assert!(!json);
        assert_eq!(from, None);
    }

    #[test]
    fn external_lane_args_flags_and_from() {
        let args = vec![
            "ci".to_string(),
            "--dry-run".to_string(),
            "--json".to_string(),
            "--from".to_string(),
            "build".to_string(),
        ];
        let (name, dry_run, json, from) = parse_external_lane_args(&args);
        assert_eq!(name, "ci");
        assert!(dry_run);
        assert!(json);
        assert_eq!(from.as_deref(), Some("build"));
    }

    #[test]
    fn external_lane_args_from_without_value_is_none() {
        let args = vec!["ci".to_string(), "--from".to_string()];
        let (_, _, _, from) = parse_external_lane_args(&args);
        assert_eq!(from, None);
    }

    #[test]
    fn external_lane_args_empty_name_defaults_blank() {
        let (name, _, _, _) = parse_external_lane_args(&[]);
        assert_eq!(name, "");
    }

    #[test]
    fn plugin_install_yes_implies_force() {
        // `--yes` is an alias for `--force` on install; the mapping must OR them.
        let action = plugin_action_from(PluginSubcommand::Install {
            source: Some("acme/plugin".to_string()),
            force: false,
            yes: true,
            copy: false,
            defaults: false,
        });
        match action {
            plugin::PluginAction::Install { force, .. } => assert!(force),
            _ => panic!("expected Install action"),
        }
    }

    #[test]
    fn plugin_install_neither_force_nor_yes_stays_false() {
        let action = plugin_action_from(PluginSubcommand::Install {
            source: Some("acme/plugin".to_string()),
            force: false,
            yes: false,
            copy: false,
            defaults: false,
        });
        match action {
            plugin::PluginAction::Install { force, .. } => assert!(!force),
            _ => panic!("expected Install action"),
        }
    }
}
