use anyhow::{bail, Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;

pub(crate) fn handle_prompt(
    message: &str,
    default: Option<&str>,
    validate: Option<&str>,
) -> Result<String> {
    if !crate::utils::is_interactive() {
        if let Some(d) = default {
            return Ok(d.to_string());
        }
        bail!(
            "Plugin requested input '{}' but stdin is not a TTY and no default was provided.",
            message
        );
    }

    let theme = dialoguer::theme::ColorfulTheme::default();
    let mut prompt = dialoguer::Input::<String>::with_theme(&theme).with_prompt(message);

    if let Some(d) = default {
        prompt = prompt.default(d.to_string());
    }

    if let Some(v) = validate {
        match v {
            "non_empty" => {
                prompt = prompt.validate_with(|input: &String| -> Result<(), String> {
                    if input.trim().is_empty() {
                        Err("Input cannot be empty".to_string())
                    } else {
                        Ok(())
                    }
                });
            }
            "integer" => {
                prompt = prompt.validate_with(|input: &String| -> Result<(), String> {
                    input
                        .parse::<i64>()
                        .map(|_| ())
                        .map_err(|_| "Must be an integer".to_string())
                });
            }
            "path_exists" => {
                prompt = prompt.validate_with(|input: &String| -> Result<(), String> {
                    if Path::new(input).exists() {
                        Ok(())
                    } else {
                        Err("Path does not exist".to_string())
                    }
                });
            }
            "url" => {
                prompt = prompt.validate_with(|input: &String| -> Result<(), String> {
                    if input.starts_with("http://") || input.starts_with("https://") {
                        Ok(())
                    } else {
                        Err("Must be a valid URL (http:// or https://)".to_string())
                    }
                });
            }
            _ => {}
        }
    }

    prompt.interact_text().context("reading user input")
}

pub(crate) fn handle_confirm(message: &str, default: bool) -> Result<bool> {
    if !crate::utils::is_interactive() {
        return Ok(default);
    }
    let theme = dialoguer::theme::ColorfulTheme::default();
    dialoguer::Confirm::with_theme(&theme)
        .with_prompt(message)
        .default(default)
        .interact()
        .context("reading confirmation")
}

pub(crate) fn handle_select(
    message: &str,
    options: &[String],
    default: Option<usize>,
) -> Result<String> {
    if !crate::utils::is_interactive() {
        let idx = default.unwrap_or(0);
        if idx < options.len() {
            return Ok(options[idx].clone());
        }
        bail!(
            "Plugin requested selection '{}' but stdin is not a TTY.",
            message
        );
    }
    let theme = dialoguer::theme::ColorfulTheme::default();
    let mut select = dialoguer::Select::with_theme(&theme)
        .with_prompt(message)
        .items(options);

    if let Some(d) = default {
        select = select.default(d);
    }

    let idx = select.interact().context("reading selection")?;
    Ok(options[idx].clone())
}

pub(crate) fn handle_multi_select(
    message: &str,
    options: &[String],
    defaults: Option<&[usize]>,
) -> Result<Vec<String>> {
    if !crate::utils::is_interactive() {
        let indices = defaults.unwrap_or(&[]);
        return Ok(indices
            .iter()
            .filter(|&&i| i < options.len())
            .map(|&i| options[i].clone())
            .collect());
    }
    let theme = dialoguer::theme::ColorfulTheme::default();
    let mut select = dialoguer::MultiSelect::with_theme(&theme)
        .with_prompt(message)
        .items(options);

    if let Some(d) = defaults {
        let bools: Vec<bool> = (0..options.len()).map(|i| d.contains(&i)).collect();
        select = select.defaults(&bools);
    }

    let indices = select.interact().context("reading multi-selection")?;
    Ok(indices.into_iter().map(|i| options[i].clone()).collect())
}

pub(crate) fn handle_progress(
    bar: &mut Option<ProgressBar>,
    plugin_name: &str,
    message: Option<&str>,
    current: Option<u64>,
    total: Option<u64>,
    done: bool,
) {
    if done {
        clear_progress(bar);
        return;
    }

    let msg = message.unwrap_or("Working");

    match (current, total) {
        (Some(cur), Some(tot)) => {
            let pb = bar.get_or_insert_with(|| {
                let pb = ProgressBar::new(tot);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template(&format!(
                            "  {} {{msg}} [{{bar:30}}] {{pos}}/{{len}}",
                            style("▶").cyan().bold()
                        ))
                        .expect("valid bar template")
                        .progress_chars("==>  "),
                );
                pb
            });
            pb.set_length(tot);
            pb.set_position(cur);
            pb.set_message(format!("{} ({})", msg, plugin_name));
        }
        _ => {
            if bar.is_none() {
                let sp = ProgressBar::new_spinner();
                sp.set_style(
                    ProgressStyle::default_spinner()
                        .template(&format!(
                            "  {} {{msg}} {{spinner}}",
                            style("▶").cyan().bold()
                        ))
                        .expect("valid spinner template"),
                );
                sp.enable_steady_tick(Duration::from_millis(100));
                *bar = Some(sp);
            }
            if let Some(pb) = bar.as_ref() {
                pb.set_message(format!("{} ({})", msg, plugin_name));
            }
        }
    }
}

pub(crate) fn clear_progress(bar: &mut Option<ProgressBar>) {
    if let Some(pb) = bar.take() {
        pb.finish_and_clear();
    }
}

pub(crate) fn handle_log(plugin_name: &str, level: &str, message: &str) {
    let prefix = match level {
        "debug" => style("DEBUG").dim(),
        "info" => style("INFO").cyan(),
        "warn" => style("WARN").yellow(),
        "error" => style("ERROR").red().bold(),
        _ => style(level).dim(),
    };
    eprintln!("  {} [{}] {}", prefix, style(plugin_name).dim(), message);
}
