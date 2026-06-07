use clap::Parser;
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "fledge",
    version,
    about = "Dev-lifecycle CLI — get your projects ready to fly."
)]
pub struct Cli {
    /// Run without prompts: treat every interactive confirmation as --yes,
    /// and bail with a clear error on prompts that have no default. Also
    /// settable via the FLEDGE_NON_INTERACTIVE env var.
    #[arg(long, global = true, visible_alias = "ni")]
    pub non_interactive: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    /// Manage AI provider and model selection
    Ai {
        #[command(subcommand)]
        action: AiSubcommand,
    },
    /// Ask a question about your codebase
    Ask {
        /// The question to ask
        question: Vec<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Include full spec + companions for these modules in the prompt
        /// (comma-separated, repeatable, use "all" for every spec)
        #[arg(long, value_name = "NAMES")]
        with_specs: Vec<String>,
        /// Omit the compact spec index from the prompt (saves tokens)
        #[arg(long)]
        no_spec_index: bool,
        /// LLM provider: claude (default) or ollama. Overrides
        /// FLEDGE_AI_PROVIDER and ai.provider in config.
        #[arg(long, value_name = "NAME", value_parser = ["anthropic", "openai", "ollama", "claude"])]
        provider: Option<String>,
        /// Model name. Overrides FLEDGE_AI_MODEL and
        /// ai.{claude,ollama}.model in config.
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
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
    /// Diagnose project environment health
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Dump the full command tree (for agents and tooling)
    Introspect {
        /// Output as JSON (default: pretty tree)
        #[arg(long)]
        json: bool,
    },
    /// Manage and run composable workflow pipelines
    #[command(alias = "lane")]
    Lanes {
        #[command(subcommand)]
        action: LaneSubcommand,
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
    /// Cut a release: bump version, changelog, tag, and optionally push.
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
        /// Skip bumping any version files. Tag-only release, useful when the
        /// canonical version lives outside the tree (e.g. the GitHub Release
        /// tag itself is the source of truth).
        #[arg(long)]
        no_bump: bool,
        /// Push commit and tag to remote after release
        #[arg(long)]
        push: bool,
        /// Run a lane before releasing (e.g. "ci")
        #[arg(long)]
        pre_lane: Option<String>,
        /// Allow releasing with uncommitted changes
        #[arg(long)]
        allow_dirty: bool,
        /// Output as JSON
        #[arg(long)]
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
        /// Model name for the active provider (overrides FLEDGE_AI_MODEL
        /// and ai.{claude,ollama}.model in config)
        #[arg(short, long)]
        model: Option<String>,
        /// Custom review focus prompt (appended to default instructions)
        #[arg(short, long)]
        prompt: Option<String>,
        /// Output format: summary (default), checklist, inline
        #[arg(long, default_value = "summary")]
        format: String,
        /// Include full spec + companions for these modules in the review
        /// context (comma-separated, repeatable). Appended to any
        /// auto-detected specs.
        #[arg(long, value_name = "NAMES")]
        with_specs: Vec<String>,
        /// Disable auto-detection of specs based on files in the diff
        #[arg(long)]
        no_auto_specs: bool,
        /// LLM provider: claude (default) or ollama. Overrides
        /// FLEDGE_AI_PROVIDER and ai.provider in config.
        #[arg(long, value_name = "NAME", value_parser = ["anthropic", "openai", "ollama", "claude"])]
        provider: Option<String>,
        /// Add another model to the review panel — runs in parallel against
        /// the same diff + spec context. Format: `provider[:model]`, e.g.
        /// `ollama:gpt-oss:120b-cloud` or just `claude` to use the active
        /// claude config. Repeatable and comma-separated.
        #[arg(long, value_name = "REF")]
        with_model: Vec<String>,
        /// Drop the active config (--provider/--model or
        /// `fledge ai use`) from the panel. Only the explicit --with-model
        /// entries will run. Useful for "compare exactly these N models".
        #[arg(long)]
        no_active: bool,
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
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage specs (check, init, new)
    Spec {
        #[command(subcommand)]
        action: SpecSubcommand,
    },
    /// Manage templates (init, create, validate, list, search, publish)
    #[command(alias = "template")]
    Templates {
        #[command(subcommand)]
        action: TemplatesSubcommand,
    },
    /// Watch for file changes and re-run a task or lane
    Watch {
        /// Task name to re-run on changes (use --lane for lanes)
        name: String,
        /// Watch and re-run a lane instead of a task
        #[arg(long)]
        lane: bool,
        /// Only watch a specific directory (default: current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Only trigger on specific file extensions (comma-separated, e.g. "rs,toml")
        #[arg(short, long)]
        ext: Option<String>,
        /// Debounce interval in milliseconds
        #[arg(short, long, default_value = "500")]
        debounce: u64,
        /// Clear terminal before each run
        #[arg(long)]
        clear: bool,
    },
    /// Feature branch and PR workflow
    Work {
        #[command(subcommand)]
        action: WorkSubcommand,
    },
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(clap::Subcommand)]
pub enum TemplatesSubcommand {
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
        /// Skip all confirmation prompts (accept defaults). For **local**
        /// templates this also auto-confirms post-create hooks. For
        /// **remote** templates it does NOT — use `--trust-hooks` to
        /// authorize hook execution from a remote source.
        #[arg(short, long)]
        yes: bool,
        /// Authorize post-create hook execution for remote templates without
        /// an interactive prompt. Hooks run arbitrary shell commands — only
        /// pass this for remote templates from sources you trust. For local
        /// templates, `--yes` already authorizes hooks (they're authored by
        /// you). Also settable via `FLEDGE_TRUST_HOOKS=1`.
        #[arg(long)]
        trust_hooks: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Search GitHub for community templates (fledge-template topic)
    Search {
        /// Keyword to filter results
        query: Option<String>,
        /// Filter by author/owner
        #[arg(short, long)]
        author: Option<String>,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Publish a template directory to GitHub
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Subcommand)]
pub enum SpecSubcommand {
    /// Validate specs against source code
    Check {
        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Initialize spec-sync configuration
    Init,
    /// List all specs in the project
    #[command(alias = "ls")]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Scaffold a new spec module
    New {
        /// Module name
        name: String,
    },
    /// Show a single spec's frontmatter, sections, and companions
    Show {
        /// Module name
        name: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Subcommand)]
pub enum WorkSubcommand {
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Stage changes and create a conventional commit
    Commit {
        /// Commit message (prompted interactively if omitted)
        #[arg(short, long)]
        message: Option<String>,
        /// Commit type: feat, fix, chore, docs, refactor, etc. (default: from branch or "feat")
        #[arg(short = 't', long = "type", value_name = "TYPE")]
        commit_type: Option<String>,
        /// Scope for conventional commit (e.g. "work", "cli")
        #[arg(short, long)]
        scope: Option<String>,
        /// Stage all changes (including untracked) before committing
        #[arg(short, long)]
        all: bool,
        /// Generate commit message via AI from the staged diff
        #[arg(long)]
        ai: bool,
        /// Override AI provider for --ai (claude or ollama)
        #[arg(long, value_parser = ["anthropic", "openai", "ollama", "claude"])]
        provider: Option<String>,
        /// Override AI model for --ai
        #[arg(long)]
        model: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Push the current branch to origin
    Push {
        /// Force push (--force-with-lease for safety)
        #[arg(short, long)]
        force: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show current branch status (pure git, no GitHub dependency)
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// `[Deprecated]` Use `fledge github prs create` (fledge-plugin-github) to open pull requests
    #[command(hide = true)]
    Pr {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },
}

#[derive(clap::Subcommand)]
pub enum AiSubcommand {
    /// Show the active AI provider, model, and host
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List available models for the active (or specified) provider
    Models {
        /// Provider: claude or ollama (default: active provider)
        #[arg(long, value_name = "NAME", value_parser = ["anthropic", "openai", "ollama", "claude"])]
        provider: Option<String>,
        /// Filter models by substring (case-insensitive)
        #[arg(long, value_name = "QUERY")]
        search: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Select the active provider (and optionally model); interactive if args
    /// are omitted
    #[command(name = "use")]
    Use {
        /// Provider: claude or ollama
        #[arg(value_parser = ["anthropic", "openai", "ollama", "claude"])]
        provider: Option<String>,
        /// Model name (e.g. qwen3-coder:480b-cloud)
        model: Option<String>,
    },
}

#[derive(clap::Subcommand)]
pub enum ConfigAction {
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
    /// Interactively edit config values
    Edit,
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
pub enum LaneSubcommand {
    /// Run a lane by name
    Run {
        /// Lane name
        name: String,
        /// Show execution plan without running
        #[arg(long)]
        dry_run: bool,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
        /// Start execution from this step (name or 1-based index)
        #[arg(long)]
        from: Option<String>,
    },
    /// List available lanes
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add default lanes to fledge.toml
    Init {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
pub enum PluginSubcommand {
    /// Install a plugin from a local path or git source
    Install {
        /// Local path, git URL, or GitHub repo (`owner/repo[@ref]`). Omit when using `--defaults`.
        source: Option<String>,
        /// Reinstall if already present
        #[arg(long)]
        force: bool,
        /// Skip all confirmation prompts (accept defaults)
        #[arg(short, long)]
        yes: bool,
        /// Copy a local plugin into fledge's config dir instead of live-linking it
        #[arg(long)]
        copy: bool,
        /// Install fledge's curated set of default plugins (github, deps, metrics)
        #[arg(long, conflicts_with = "source")]
        defaults: bool,
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
        /// Update only fledge's curated default plugins (skip community plugins)
        #[arg(long, conflicts_with = "name")]
        defaults: bool,
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
        /// Filter by GitHub topic (e.g. `ci`, `rust`, `testing`)
        #[arg(short, long)]
        topic: Option<String>,
        /// Filter by trust tier (`official`, `team`, or `unverified`) — applied client-side after fetching
        #[arg(long = "trust-tier", value_name = "TIER")]
        trust_tier: Option<crate::trust::TrustTier>,
        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Interactive fuzzy-search — pick a plugin to install
        #[arg(short, long)]
        interactive: bool,
    },
    /// Recommend plugins based on your project's language and tooling
    Recommend,
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
        /// Create a WASM plugin (Rust + wasm32-wasip1)
        #[arg(long)]
        wasm: bool,
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
