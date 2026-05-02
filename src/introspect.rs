use anyhow::Result;
use clap::{Arg, Command};
use serde::Serialize;

/// Current schema version of `fledge introspect --json` output. Bumped only on
/// breaking changes to the JSON shape; additive fields do not require a bump.
pub const INTROSPECT_SCHEMA_VERSION: u32 = 1;

pub struct IntrospectOptions {
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct IntrospectOutput {
    schema_version: u32,
    #[serde(flatten)]
    root: CommandNode,
}

pub fn run(opts: IntrospectOptions, cmd: Command) -> Result<()> {
    let tree = build_tree(&cmd);
    if opts.json {
        let output = IntrospectOutput {
            schema_version: INTROSPECT_SCHEMA_VERSION,
            root: tree,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        render_pretty(&tree, 0);
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CommandNode {
    pub name: String,
    pub about: Option<String>,
    pub aliases: Vec<String>,
    pub args: Vec<ArgNode>,
    pub subcommands: Vec<CommandNode>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArgNode {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<char>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    pub required: bool,
    pub takes_value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_name: Option<String>,
    pub global: bool,
}

fn build_tree(cmd: &Command) -> CommandNode {
    build_tree_with_inherited(cmd, &[])
}

/// Walk the clap tree, threading inherited `global = true` args down through
/// each subcommand. Each node's `args` is the union of the args declared on
/// that command and any inherited globals from ancestors (deduplicated by
/// name — a child re-declaring an arg keeps its own copy). Inherited args
/// retain `global: true` so agents can distinguish them from locally-declared
/// args if they want to.
fn build_tree_with_inherited(cmd: &Command, inherited: &[ArgNode]) -> CommandNode {
    let own_args: Vec<ArgNode> = cmd
        .get_arguments()
        .filter(|a| {
            // Skip the implicit `--help` / `--version` globals — they're
            // on every command and add noise.
            let id = a.get_id().as_str();
            id != "help" && id != "version"
        })
        .map(build_arg)
        .collect();

    let mut all_args = own_args;
    for inherited_arg in inherited {
        if !all_args.iter().any(|a| a.name == inherited_arg.name) {
            all_args.push(inherited_arg.clone());
        }
    }

    // Globals to pass to children: every arg currently visible at this level
    // that is itself global. Locally-declared globals start propagating here;
    // ancestor globals continue propagating.
    let next_inherited: Vec<ArgNode> = all_args.iter().filter(|a| a.global).cloned().collect();

    CommandNode {
        name: cmd.get_name().to_string(),
        about: cmd.get_about().map(|s| s.to_string()),
        aliases: cmd.get_visible_aliases().map(|s| s.to_string()).collect(),
        args: all_args,
        subcommands: cmd
            .get_subcommands()
            .filter(|s| s.get_name() != "help")
            .map(|s| build_tree_with_inherited(s, &next_inherited))
            .collect(),
    }
}

fn build_arg(arg: &Arg) -> ArgNode {
    let takes_value = arg
        .get_num_args()
        .map(|n| n.takes_values())
        .unwrap_or(false);
    let mut aliases: Vec<String> = arg
        .get_visible_aliases()
        .map(|v| v.into_iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    if let Some(short_aliases) = arg.get_visible_short_aliases() {
        for c in short_aliases {
            aliases.push(c.to_string());
        }
    }
    ArgNode {
        name: arg.get_id().as_str().to_string(),
        long: arg.get_long().map(|s| s.to_string()),
        short: arg.get_short(),
        aliases,
        help: arg.get_help().map(|s| s.to_string()),
        required: arg.is_required_set(),
        takes_value,
        // Only expose value_name when the arg actually takes a value —
        // clap synthesizes uppercase names for bool flags, which is noise
        // for agents trying to generate invocations.
        value_name: if takes_value {
            arg.get_value_names()
                .and_then(|v| v.first().map(|s| s.to_string()))
        } else {
            None
        },
        global: arg.is_global_set(),
    }
}

fn render_pretty(node: &CommandNode, indent: usize) {
    let pad = "  ".repeat(indent);
    let alias_suffix = if node.aliases.is_empty() {
        String::new()
    } else {
        format!(" (aliases: {})", node.aliases.join(", "))
    };
    println!("{pad}{}{}", node.name, alias_suffix);
    if let Some(about) = &node.about {
        println!("{pad}  {about}");
    }
    for arg in &node.args {
        let flags = match (arg.long.as_deref(), arg.short) {
            (Some(long), Some(short)) => format!("-{short}, --{long}"),
            (Some(long), None) => format!("--{long}"),
            (None, Some(short)) => format!("-{short}"),
            (None, None) => format!("<{}>", arg.name),
        };
        let value = arg
            .value_name
            .as_deref()
            .map(|v| format!(" <{v}>"))
            .unwrap_or_default();
        let required_marker = if arg.required { "*" } else { "" };
        println!("{pad}  {required_marker}{flags}{value}");
    }
    for sub in &node.subcommands {
        render_pretty(sub, indent + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // Minimal test CLI so we don't depend on the real Cli struct.
    #[derive(clap::Parser)]
    #[command(name = "testcli", about = "Test CLI")]
    struct TestCli {
        #[arg(long, global = true)]
        verbose: bool,

        #[command(subcommand)]
        command: TestCommands,
    }

    #[derive(clap::Subcommand)]
    enum TestCommands {
        /// Say hi
        Hello {
            /// Name to greet
            name: String,
            /// Output JSON
            #[arg(long)]
            json: bool,
        },
    }

    #[test]
    fn build_tree_captures_top_level() {
        let cmd = TestCli::command();
        let tree = build_tree(&cmd);
        assert_eq!(tree.name, "testcli");
        assert_eq!(tree.about.as_deref(), Some("Test CLI"));
    }

    #[test]
    fn build_tree_captures_global_args() {
        let cmd = TestCli::command();
        let tree = build_tree(&cmd);
        let verbose = tree.args.iter().find(|a| a.name == "verbose").unwrap();
        assert!(verbose.global);
        assert_eq!(verbose.long.as_deref(), Some("verbose"));
    }

    #[test]
    fn global_args_propagate_to_subcommands() {
        // Locks the introspect contract: a `global = true` arg declared on a
        // parent command appears on every descendant subcommand's `args`,
        // marked `global: true`. Agents reading any node's args see the full
        // set of flags accepted at that level.
        let cmd = TestCli::command();
        let tree = build_tree(&cmd);
        let hello = tree.subcommands.iter().find(|s| s.name == "hello").unwrap();
        let verbose = hello
            .args
            .iter()
            .find(|a| a.name == "verbose")
            .expect("inherited global should appear on subcommand");
        assert!(verbose.global);
    }

    #[derive(clap::Parser)]
    #[command(name = "deepcli")]
    struct DeepCli {
        #[arg(long, global = true)]
        root_global: bool,

        #[command(subcommand)]
        command: DeepMid,
    }

    #[derive(clap::Subcommand)]
    enum DeepMid {
        Mid {
            #[arg(long, global = true)]
            mid_global: bool,

            #[command(subcommand)]
            action: DeepLeaf,
        },
    }

    #[derive(clap::Subcommand)]
    enum DeepLeaf {
        Leaf {
            #[arg(long)]
            leaf_local: bool,
        },
    }

    #[test]
    fn globals_propagate_through_multiple_levels() {
        let cmd = DeepCli::command();
        let tree = build_tree(&cmd);
        let mid = tree.subcommands.iter().find(|s| s.name == "mid").unwrap();
        let leaf = mid.subcommands.iter().find(|s| s.name == "leaf").unwrap();

        // Leaf sees: its own arg, mid's global, root's global
        assert!(leaf.args.iter().any(|a| a.name == "leaf_local"));
        assert!(leaf.args.iter().any(|a| a.name == "mid_global" && a.global));
        assert!(leaf
            .args
            .iter()
            .any(|a| a.name == "root_global" && a.global));
    }

    #[derive(clap::Parser)]
    #[command(name = "shadowcli")]
    struct ShadowCli {
        #[arg(long, global = true)]
        json: bool,

        #[command(subcommand)]
        command: ShadowSub,
    }

    #[derive(clap::Subcommand)]
    enum ShadowSub {
        /// Subcommand that redeclares `--json` with its own settings
        Sub {
            #[arg(long, help = "child's own json flag")]
            json: bool,
        },
    }

    #[test]
    fn child_redeclaration_does_not_duplicate_inherited_arg() {
        let cmd = ShadowCli::command();
        let tree = build_tree(&cmd);
        let sub = tree.subcommands.iter().find(|s| s.name == "sub").unwrap();
        let json_args: Vec<&ArgNode> = sub.args.iter().filter(|a| a.name == "json").collect();
        assert_eq!(
            json_args.len(),
            1,
            "child redeclaring an inherited arg should not produce a duplicate, got: {:?}",
            json_args
        );
        // The child's local declaration wins; help text comes from the child.
        assert_eq!(json_args[0].help.as_deref(), Some("child's own json flag"));
    }

    #[test]
    fn build_tree_captures_subcommand_with_required_arg() {
        let cmd = TestCli::command();
        let tree = build_tree(&cmd);
        let hello = tree.subcommands.iter().find(|s| s.name == "hello").unwrap();
        assert_eq!(hello.about.as_deref(), Some("Say hi"));
        let name_arg = hello.args.iter().find(|a| a.name == "name").unwrap();
        assert!(name_arg.required);
        let json_arg = hello.args.iter().find(|a| a.name == "json").unwrap();
        assert!(!json_arg.required);
        assert_eq!(json_arg.long.as_deref(), Some("json"));
    }

    #[test]
    fn build_tree_skips_help_and_version_args() {
        let cmd = TestCli::command();
        let tree = build_tree(&cmd);
        assert!(!tree.args.iter().any(|a| a.name == "help"));
        assert!(!tree.args.iter().any(|a| a.name == "version"));
    }

    #[test]
    fn build_tree_skips_help_subcommand() {
        let cmd = TestCli::command();
        let tree = build_tree(&cmd);
        assert!(!tree.subcommands.iter().any(|s| s.name == "help"));
    }

    #[test]
    fn tree_serializes_to_valid_json() {
        let cmd = TestCli::command();
        let tree = build_tree(&cmd);
        let json = serde_json::to_string(&tree).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed["name"].as_str(), Some("testcli"));
        assert!(parsed["subcommands"].is_array());
    }

    #[derive(clap::Parser)]
    #[command(name = "aliascli")]
    struct AliasCli {
        /// Global flag with alias
        #[arg(long, global = true, visible_alias = "ni", visible_short_alias = 'n')]
        non_interactive: bool,

        #[command(subcommand)]
        command: AliasCommands,
    }

    #[derive(clap::Subcommand)]
    enum AliasCommands {
        Dummy,
    }

    #[test]
    fn introspect_json_schema_snapshot() {
        use crate::cli::Cli;
        let cmd = Cli::command();
        let tree = build_tree(&cmd);
        let output = IntrospectOutput {
            schema_version: INTROSPECT_SCHEMA_VERSION,
            root: tree,
        };
        insta::assert_json_snapshot!("introspect_schema", output);
    }

    #[test]
    fn build_arg_surfaces_long_and_short_aliases() {
        let cmd = AliasCli::command();
        let tree = build_tree(&cmd);
        let ni = tree
            .args
            .iter()
            .find(|a| a.name == "non_interactive")
            .expect("non_interactive arg should be present");
        assert!(
            ni.aliases.contains(&"ni".to_string()),
            "expected 'ni' in aliases, got: {:?}",
            ni.aliases
        );
        assert!(
            ni.aliases.contains(&"n".to_string()),
            "expected short alias 'n' in aliases, got: {:?}",
            ni.aliases
        );
    }
}
