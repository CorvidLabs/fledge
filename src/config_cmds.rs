use anyhow::Result;
use console::style;

use crate::config;
use crate::utils;
use crate::ConfigAction;

pub fn handle_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Get { key } => {
            let config = config::Config::load()?;
            if !config::Config::is_valid_key(&key) {
                anyhow::bail!(
                    "Unknown config key '{}'. {}",
                    key,
                    config::Config::valid_keys_hint()
                );
            }
            if config::Config::is_secret_key(&key) {
                match config.get(&key) {
                    Some(v) if !v.is_empty() => println!("***"),
                    _ => println!("{} {} is not set", style("*").cyan().bold(), key),
                }
            } else {
                match config.get(&key) {
                    Some(value) if !value.is_empty() => println!("{}", value),
                    _ => println!("{} {} is not set", style("*").cyan().bold(), key),
                }
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
        ConfigAction::Edit => {
            utils::require_interactive("fledge config edit")?;
            interactive_config_edit()?;
        }
        ConfigAction::List => {
            let config = config::Config::load()?;
            let path = config::Config::config_path();
            println!(
                "{} Config: {}\n",
                style("*").cyan().bold(),
                style(path.display()).dim()
            );

            println!("  {}", style("Defaults").bold().underlined());
            print_config_described(
                "defaults.author",
                &config.defaults.author,
                "Author name for new projects",
            );
            print_config_described(
                "defaults.github_org",
                &config.defaults.github_org,
                "GitHub org for new projects",
            );
            print_config_described(
                "defaults.license",
                &config.defaults.license,
                "Default license (e.g. MIT, Apache-2.0)",
            );
            println!();

            println!("  {}", style("GitHub").bold().underlined());
            print_config_described(
                "github.token",
                &config.github.token.as_ref().map(|_| "***".to_string()),
                "API token for GitHub operations",
            );
            println!();

            println!("  {}", style("Templates").bold().underlined());
            print_config_list_described(
                "templates.paths",
                &config.templates.paths,
                "Local dirs with project templates",
            );
            print_config_list_described(
                "templates.repos",
                &config.templates.repos,
                "GitHub repos with templates (owner/repo)",
            );
            println!();

            println!("  {}", style("Trust").bold().underlined());
            print_config_list_described(
                "trust.orgs",
                &config.trust.orgs,
                "Extra trusted orgs (team tier)",
            );
            print_config_list_described(
                "trust.users",
                &config.trust.users,
                "Extra trusted users (team tier)",
            );
            println!();

            println!("  {}", style("AI").bold().underlined());
            print_config_described(
                "ai.provider",
                &config.ai.provider,
                "LLM backend: anthropic, openai, or ollama",
            );
            print_config_described(
                "ai.anthropic.model",
                &config.ai.anthropic.model,
                "Anthropic model id",
            );

            let anthropic_key_env = std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|k| !k.is_empty());
            if anthropic_key_env.is_some() {
                print_config_value_described(
                    "ai.anthropic.api_key",
                    &format!("*** {}", style("(from ANTHROPIC_API_KEY env)").dim()),
                    "Anthropic API key",
                );
            } else {
                print_config_described(
                    "ai.anthropic.api_key",
                    &config
                        .ai
                        .anthropic
                        .api_key
                        .as_ref()
                        .map(|_| "***".to_string()),
                    "Anthropic API key (or export ANTHROPIC_API_KEY)",
                );
            }
            if let Some(b) = &config.ai.anthropic.base_url {
                print_config_value_described("ai.anthropic.base_url", b, "Anthropic base URL");
            }

            if let Some(b) = &config.ai.openai.base_url {
                print_config_value_described(
                    "ai.openai.base_url",
                    b,
                    "OpenAI-compatible base URL (gateway)",
                );
            }
            let openai_key_env = std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|k| !k.is_empty());
            if openai_key_env.is_some() {
                print_config_value_described(
                    "ai.openai.api_key",
                    &format!("*** {}", style("(from OPENAI_API_KEY env)").dim()),
                    "OpenAI-compatible API key",
                );
            } else {
                print_config_described(
                    "ai.openai.api_key",
                    &config.ai.openai.api_key.as_ref().map(|_| "***".to_string()),
                    "OpenAI-compatible API key (or export OPENAI_API_KEY)",
                );
            }
            print_config_described(
                "ai.openai.model",
                &config.ai.openai.model,
                "OpenAI-compatible model id",
            );

            let host_override = std::env::var("OLLAMA_HOST").ok();
            if host_override.is_some() {
                print_config_value_described(
                    "ai.ollama.host",
                    &format!(
                        "{} {}",
                        config.ai.ollama.host,
                        style("(⚠ overridden by OLLAMA_HOST env)").yellow()
                    ),
                    "Ollama API endpoint URL",
                );
            } else {
                print_config_value_described(
                    "ai.ollama.host",
                    &config.ai.ollama.host,
                    "Ollama API endpoint URL",
                );
            }

            let key_override = std::env::var("OLLAMA_API_KEY")
                .ok()
                .filter(|k| !k.is_empty());
            if key_override.is_some() {
                print_config_value_described(
                    "ai.ollama.api_key",
                    &format!("*** {}", style("(from OLLAMA_API_KEY env)").dim()),
                    "Ollama Cloud API key",
                );
            } else {
                print_config_described(
                    "ai.ollama.api_key",
                    &config.ai.ollama.api_key.as_ref().map(|_| "***".to_string()),
                    "Ollama Cloud API key",
                );
            }

            print_config_value_described(
                "ai.ollama.model",
                &config.ai.ollama.model,
                "Ollama model name",
            );
            print_config_value_described(
                "ai.ollama.timeout_seconds",
                &config.ai.ollama.timeout_seconds.to_string(),
                "Request timeout in seconds",
            );
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

pub fn print_config_described(key: &str, value: &Option<impl std::fmt::Display>, desc: &str) {
    match value {
        Some(v) => println!(
            "  {:<28} {:<24} {}",
            style(key).cyan(),
            v,
            style(desc).dim()
        ),
        None => println!(
            "  {:<28} {:<24} {}",
            style(key).cyan(),
            style("(not set)"),
            style(desc).dim()
        ),
    }
}

pub fn print_config_value_described(key: &str, value: &impl std::fmt::Display, desc: &str) {
    println!(
        "  {:<28} {:<24} {}",
        style(key).cyan(),
        value,
        style(desc).dim()
    );
}

pub fn print_config_list_described(key: &str, values: &[String], desc: &str) {
    if values.is_empty() {
        println!(
            "  {:<28} {:<24} {}",
            style(key).cyan(),
            style("(none)"),
            style(desc).dim()
        );
    } else {
        for (i, v) in values.iter().enumerate() {
            if i == 0 {
                println!(
                    "  {:<28} {:<24} {}",
                    style(key).cyan(),
                    v,
                    style(desc).dim()
                );
            } else {
                println!("  {:<28} {}", "", v);
            }
        }
    }
}

pub fn interactive_config_edit() -> Result<()> {
    use dialoguer::{Input, Select};
    let theme = dialoguer::theme::ColorfulTheme::default();

    struct ConfigKey {
        key: &'static str,
        desc: &'static str,
        kind: KeyKind,
    }

    enum KeyKind {
        Text,
        Secret,
        Enum(&'static [&'static str]),
        Number,
        List,
    }

    let keys = vec![
        ConfigKey {
            key: "defaults.author",
            desc: "Author name for new projects",
            kind: KeyKind::Text,
        },
        ConfigKey {
            key: "defaults.github_org",
            desc: "GitHub org for new projects",
            kind: KeyKind::Text,
        },
        ConfigKey {
            key: "defaults.license",
            desc: "Default license",
            kind: KeyKind::Enum(&[
                "MIT",
                "Apache-2.0",
                "GPL-3.0",
                "BSD-3-Clause",
                "ISC",
                "UNLICENSED",
            ]),
        },
        ConfigKey {
            key: "github.token",
            desc: "API token for GitHub operations",
            kind: KeyKind::Secret,
        },
        ConfigKey {
            key: "templates.paths",
            desc: "Local dirs with project templates",
            kind: KeyKind::List,
        },
        ConfigKey {
            key: "templates.repos",
            desc: "GitHub repos with templates (owner/repo)",
            kind: KeyKind::List,
        },
        ConfigKey {
            key: "trust.orgs",
            desc: "Extra trusted orgs (team tier for plugins/lanes)",
            kind: KeyKind::List,
        },
        ConfigKey {
            key: "trust.users",
            desc: "Extra trusted users (team tier for plugins/lanes)",
            kind: KeyKind::List,
        },
        ConfigKey {
            key: "ai.provider",
            desc: "LLM backend",
            kind: KeyKind::Enum(&["anthropic", "openai", "ollama"]),
        },
        ConfigKey {
            key: "ai.anthropic.model",
            desc: "Anthropic model id",
            kind: KeyKind::Text,
        },
        ConfigKey {
            key: "ai.anthropic.api_key",
            desc: "Anthropic API key (or export ANTHROPIC_API_KEY)",
            kind: KeyKind::Secret,
        },
        ConfigKey {
            key: "ai.anthropic.base_url",
            desc: "Anthropic base URL override",
            kind: KeyKind::Text,
        },
        ConfigKey {
            key: "ai.openai.base_url",
            desc: "OpenAI-compatible base URL (OpenAI, OpenRouter, Groq, ...)",
            kind: KeyKind::Text,
        },
        ConfigKey {
            key: "ai.openai.api_key",
            desc: "OpenAI-compatible API key (or export OPENAI_API_KEY)",
            kind: KeyKind::Secret,
        },
        ConfigKey {
            key: "ai.openai.model",
            desc: "OpenAI-compatible model id",
            kind: KeyKind::Text,
        },
        ConfigKey {
            key: "ai.ollama.host",
            desc: "Ollama API endpoint URL",
            kind: KeyKind::Text,
        },
        ConfigKey {
            key: "ai.ollama.api_key",
            desc: "Ollama Cloud API key",
            kind: KeyKind::Secret,
        },
        ConfigKey {
            key: "ai.ollama.model",
            desc: "Ollama model name",
            kind: KeyKind::Text,
        },
        ConfigKey {
            key: "ai.ollama.timeout_seconds",
            desc: "Request timeout in seconds",
            kind: KeyKind::Number,
        },
    ];

    loop {
        let config = config::Config::load()?;

        let items: Vec<String> = keys
            .iter()
            .map(|k| {
                let current = match k.kind {
                    KeyKind::Secret => config.get(k.key).map(|_| "***".to_string()),
                    KeyKind::List => {
                        let val = config.get(k.key).unwrap_or_default();
                        if val.is_empty() {
                            Some("(none)".to_string())
                        } else {
                            Some(val.replace('\n', ", "))
                        }
                    }
                    _ => config.get(k.key),
                };
                let val_str = current
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "(not set)".to_string());
                format!("{:<28} {:<20} {}", k.key, val_str, k.desc)
            })
            .collect();

        let mut menu_items = items.clone();
        menu_items.push("Done — save and exit".to_string());

        let selection = Select::with_theme(&theme)
            .with_prompt("Select a config key to edit")
            .items(&menu_items)
            .default(0)
            .interact()?;

        if selection >= keys.len() {
            println!("{} Config saved.", style("✅").green().bold());
            break;
        }

        let entry = &keys[selection];
        let mut config = config::Config::load()?;

        match entry.kind {
            KeyKind::Enum(options) => {
                let current = config.get(entry.key).unwrap_or_default();
                let default_idx = options.iter().position(|o| *o == current).unwrap_or(0);

                let choice = Select::with_theme(&theme)
                    .with_prompt(format!("{} — {}", entry.key, entry.desc))
                    .items(options)
                    .default(default_idx)
                    .interact()?;

                config.set(entry.key, options[choice])?;
                config.save()?;
                println!(
                    "{} Set {} = {}",
                    style("✅").green().bold(),
                    style(entry.key).cyan(),
                    style(options[choice]).green()
                );
            }
            KeyKind::Secret => {
                let value: String = dialoguer::Password::with_theme(&theme)
                    .with_prompt(format!("{} — {}", entry.key, entry.desc))
                    .allow_empty_password(true)
                    .interact()?;

                if value.is_empty() {
                    config.unset(entry.key)?;
                    config.save()?;
                    println!(
                        "{} Cleared {}",
                        style("✅").green().bold(),
                        style(entry.key).cyan()
                    );
                } else {
                    config.set(entry.key, &value)?;
                    config.save()?;
                    println!(
                        "{} Set {} = ***",
                        style("✅").green().bold(),
                        style(entry.key).cyan()
                    );
                }
            }
            KeyKind::Number => {
                let current = config.get(entry.key).unwrap_or_default();
                let value: String = Input::with_theme(&theme)
                    .with_prompt(format!("{} — {}", entry.key, entry.desc))
                    .default(current)
                    .validate_with(|input: &String| -> std::result::Result<(), String> {
                        input
                            .trim()
                            .parse::<u64>()
                            .map(|_| ())
                            .map_err(|_| "Must be a non-negative integer".to_string())
                    })
                    .interact_text()?;

                config.set(entry.key, value.trim())?;
                config.save()?;
                println!(
                    "{} Set {} = {}",
                    style("✅").green().bold(),
                    style(entry.key).cyan(),
                    style(value.trim()).green()
                );
            }
            KeyKind::List => {
                let current_values: Vec<String> = config
                    .get(entry.key)
                    .unwrap_or_default()
                    .split('\n')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();

                let mut list_items: Vec<String> = current_values
                    .iter()
                    .map(|v| format!("Remove: {}", v))
                    .collect();
                list_items.push("Add new value".to_string());
                list_items.push("Back".to_string());

                let choice = Select::with_theme(&theme)
                    .with_prompt(format!("{} — {}", entry.key, entry.desc))
                    .items(&list_items)
                    .default(list_items.len() - 1)
                    .interact()?;

                if choice < current_values.len() {
                    let removed = &current_values[choice];
                    config.remove_from_list(entry.key, removed)?;
                    config.save()?;
                    println!(
                        "{} Removed {} from {}",
                        style("✅").green().bold(),
                        style(removed).red(),
                        style(entry.key).cyan()
                    );
                } else if choice == current_values.len() {
                    let value: String = Input::with_theme(&theme)
                        .with_prompt("Value to add")
                        .interact_text()?;
                    if !value.trim().is_empty() {
                        config.add_to_list(entry.key, value.trim())?;
                        config.save()?;
                        println!(
                            "{} Added {} to {}",
                            style("✅").green().bold(),
                            style(value.trim()).green(),
                            style(entry.key).cyan()
                        );
                    }
                }
            }
            KeyKind::Text => {
                let current = config.get(entry.key).unwrap_or_default();
                let mut input = Input::<String>::with_theme(&theme)
                    .with_prompt(format!("{} — {} (empty to clear)", entry.key, entry.desc))
                    .allow_empty(true);

                if !current.is_empty() {
                    input = input.default(current);
                }

                let value: String = input.interact_text()?;

                if value.trim().is_empty() {
                    config.unset(entry.key)?;
                    config.save()?;
                    println!(
                        "{} Cleared {}",
                        style("✅").green().bold(),
                        style(entry.key).cyan()
                    );
                } else {
                    config.set(entry.key, value.trim())?;
                    config.save()?;
                    println!(
                        "{} Set {} = {}",
                        style("✅").green().bold(),
                        style(entry.key).cyan(),
                        style(value.trim()).green()
                    );
                }
            }
        }

        println!();
    }

    Ok(())
}
