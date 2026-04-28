use clap::{Parser, Subcommand};

use crate::error::PowError;

#[derive(Parser, Debug)]
#[command(
    name = "pow",
    version,
    about = "Project-oriented workspace CLI — manage multi-repo workspaces via git worktrees.",
    long_about = None,
    arg_required_else_help = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new (empty) workspace at ~/workspaces/<name>/.
    New {
        name: String,
        /// If the directory already exists, remove and recreate it.
        #[arg(long)]
        force: bool,
        /// Template to apply (adds the template's repos after creating).
        #[arg(short = 't', long)]
        template: Option<String>,
        /// Base branch/ref to create branches from when applying a template.
        #[arg(short = 'f', long)]
        from: Option<String>,
        /// Skip per-repo setup hooks when applying a template.
        #[arg(long)]
        no_setup: bool,
    },
    /// Add a repo as a worktree in a workspace.
    Add {
        /// Repo name (bare or source/name).
        repo: String,
        /// Workspace to add to. Defaults to $POW_ACTIVE.
        #[arg(short = 'w', long)]
        workspace: Option<String>,
        /// Branch to check out (defaults to workspace name).
        #[arg(short = 'b', long)]
        branch: Option<String>,
        /// Base branch/ref to create the branch from.
        #[arg(short = 'f', long)]
        from: Option<String>,
        /// Skip per-repo setup hooks defined in .pow.toml.
        #[arg(long)]
        no_setup: bool,
    },
    /// Remove a worktree from a workspace.
    Forget {
        repo: String,
        #[arg(short = 'w', long)]
        workspace: Option<String>,
        /// Also delete the branch if it's safe to do so.
        #[arg(long)]
        prune_branch: bool,
    },
    /// Tear down an entire workspace.
    Rm {
        name: String,
        /// Also delete each entry's branch.
        #[arg(long)]
        prune_branches: bool,
        /// Skip confirmation prompts.
        #[arg(long)]
        force: bool,
    },
    /// List all workspaces.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Show the contents of a workspace.
    Show {
        /// Workspace name. Defaults to $POW_ACTIVE.
        name: Option<String>,
        #[arg(long)]
        json: bool,
        /// Skip git status calls.
        #[arg(long)]
        no_status: bool,
    },
    /// Set the active workspace (requires shell integration).
    Use { name: String },
    /// cd into a workspace or entry (requires shell integration).
    Cd {
        /// Workspace name. Optional; defaults to active.
        name: Option<String>,
        /// Entry name within the workspace.
        entry: Option<String>,
    },
    /// Print the active workspace.
    Current {
        #[arg(long)]
        json: bool,
    },
    /// Switch an entry to a different branch or commit.
    Switch {
        repo: String,
        /// Branch or commit to switch to.
        target: String,
        /// Create a new branch from current HEAD.
        #[arg(long)]
        new: bool,
        #[arg(short = 'w', long)]
        workspace: Option<String>,
    },
    /// Fetch in the underlying source clones.
    Sync {
        /// Optional repo name; defaults to all entries in active workspace.
        repo: Option<String>,
        /// Fetch for every source in config.
        #[arg(long)]
        all: bool,
        #[arg(short = 'w', long)]
        workspace: Option<String>,
    },
    /// Git status across entries in a workspace.
    Status {
        name: Option<String>,
        #[arg(long)]
        dirty_only: bool,
        #[arg(long)]
        short: bool,
    },
    /// Run a command in every entry directory of a workspace.
    Exec {
        /// Command to run (passed to the shell).
        command: String,
        #[arg(short = 'w', long)]
        workspace: Option<String>,
        /// Run this many in parallel.
        #[arg(long)]
        parallel: Option<usize>,
        /// Print commands without executing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Manage sources.
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    /// Manage workspace templates.
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
    },
    /// Print, get, or set configuration.
    Config {
        /// Print the config as JSON.
        #[arg(long)]
        json: bool,
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },
    /// Print zsh shell integration script.
    Init,
    /// Print zsh completion script.
    Completions {
        /// Shell to generate completions for.
        #[arg(default_value = "zsh")]
        shell: clap_complete::Shell,
    },

    /// Internal: resolve workspace name for shell integration.
    #[command(name = "__resolve-use", hide = true)]
    ResolveUse { name: String },
    /// Internal: resolve cd target for shell integration.
    #[command(name = "__resolve-cd", hide = true)]
    ResolveCd { args: Vec<String> },
    /// Internal: print completion candidates for shell integration.
    #[command(name = "__complete", hide = true)]
    Complete {
        #[command(subcommand)]
        kind: CompleteKind,
    },
}

#[derive(Subcommand, Debug)]
pub enum CompleteKind {
    /// Workspace names.
    Workspaces,
    /// Entries (repos) inside a workspace. Defaults to $POW_ACTIVE.
    Entries {
        #[arg(short = 'w', long)]
        workspace: Option<String>,
    },
    /// Repo names from registered sources (for `pow add`).
    Repos {
        /// Limit to a single source by name.
        #[arg(long)]
        source: Option<String>,
    },
    /// Registered source names.
    Sources,
    /// Configured template names.
    Templates,
    /// Known config keys.
    ConfigKeys,
}

#[derive(Subcommand, Debug)]
pub enum SourceCommand {
    /// Register a source directory (optionally clone from a GitHub org).
    Add {
        name: String,
        path: String,
        #[arg(long)]
        github_org: Option<String>,
        #[arg(long, default_value = "main")]
        base_branch: String,
        /// Include pattern (glob). Repeatable.
        #[arg(long)]
        include: Vec<String>,
        /// Exclude pattern (glob). Repeatable.
        #[arg(long)]
        exclude: Vec<String>,
        /// Skip interactive picker, clone everything matching filters.
        #[arg(long)]
        all: bool,
        /// Skip archived repos (default true).
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        skip_archived: bool,
    },
    /// List registered sources.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Clone new repos from a source's GitHub org.
    Sync {
        name: String,
        #[arg(long)]
        dry_run: bool,
        /// Remove local repos no longer present in the filtered set.
        #[arg(long)]
        prune: bool,
        #[arg(long)]
        parallel: Option<usize>,
    },
    /// Unregister a source (does not touch its files).
    Remove {
        name: String,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum TemplateCommand {
    /// List configured templates.
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Get a single config value by dotted key.
    Get { key: String },
    /// Set a single config value by dotted key.
    Set { key: String, value: String },
}

pub async fn dispatch(cli: Cli) -> Result<(), PowError> {
    match cli.command {
        Commands::New {
            name,
            force,
            template,
            from,
            no_setup,
        } => crate::workspace::lifecycle::new(
            &name,
            force,
            template.as_deref(),
            from.as_deref(),
            no_setup,
        ),
        Commands::Add {
            repo,
            workspace,
            branch,
            from,
            no_setup,
        } => crate::workspace::lifecycle::add(
            &repo,
            workspace.as_deref(),
            branch.as_deref(),
            from.as_deref(),
            no_setup,
        ),
        Commands::Forget {
            repo,
            workspace,
            prune_branch,
        } => crate::workspace::lifecycle::forget(&repo, workspace.as_deref(), prune_branch),
        Commands::Rm {
            name,
            prune_branches,
            force,
        } => crate::workspace::lifecycle::rm(&name, prune_branches, force),
        Commands::List { json } => crate::workspace::show::list(json),
        Commands::Show {
            name,
            json,
            no_status,
        } => crate::workspace::show::show(name.as_deref(), json, no_status),
        Commands::Use { name } => crate::workspace::nav::use_workspace(&name),
        Commands::Cd { name, entry } => {
            crate::workspace::nav::cd(name.as_deref(), entry.as_deref())
        }
        Commands::Current { json } => crate::workspace::nav::current(json),
        Commands::Switch {
            repo,
            target,
            new,
            workspace,
        } => crate::workspace::work::switch(&repo, &target, new, workspace.as_deref()),
        Commands::Sync {
            repo,
            all,
            workspace,
        } => crate::workspace::work::sync(repo.as_deref(), all, workspace.as_deref()).await,
        Commands::Status {
            name,
            dirty_only,
            short,
        } => crate::workspace::work::status(name.as_deref(), dirty_only, short),
        Commands::Exec {
            command,
            workspace,
            parallel,
            dry_run,
        } => crate::workspace::work::exec(&command, workspace.as_deref(), parallel, dry_run).await,
        Commands::Source { command } => dispatch_source(command).await,
        Commands::Template { command } => dispatch_template(command),
        Commands::Config { json, command } => dispatch_config(json, command),
        Commands::Init => crate::shell::print_shell_init(),
        Commands::Completions { shell } => {
            if shell == clap_complete::Shell::Zsh {
                print!("{}", include_str!("../assets/_pow.zsh"));
            } else {
                use clap::CommandFactory;
                let mut cmd = Cli::command();
                clap_complete::generate(shell, &mut cmd, "pow", &mut std::io::stdout());
            }
            Ok(())
        }
        Commands::ResolveUse { name } => crate::workspace::nav::resolve_use(&name),
        Commands::ResolveCd { args } => crate::workspace::nav::resolve_cd(&args),
        Commands::Complete { kind } => {
            crate::complete::run(kind);
            Ok(())
        }
    }
}

async fn dispatch_source(cmd: SourceCommand) -> Result<(), PowError> {
    match cmd {
        SourceCommand::Add {
            name,
            path,
            github_org,
            base_branch,
            include,
            exclude,
            all,
            skip_archived,
        } => {
            crate::source::add::run(
                &name,
                &path,
                github_org.as_deref(),
                &base_branch,
                &include,
                &exclude,
                all,
                skip_archived,
            )
            .await
        }
        SourceCommand::List { json } => crate::source::list(json),
        SourceCommand::Sync {
            name,
            dry_run,
            prune,
            parallel,
        } => crate::source::sync::run(&name, dry_run, prune, parallel).await,
        SourceCommand::Remove { name, force } => crate::source::remove(&name, force),
    }
}

fn dispatch_template(cmd: TemplateCommand) -> Result<(), PowError> {
    match cmd {
        TemplateCommand::List { json } => crate::template::list(json),
    }
}

fn dispatch_config(json: bool, command: Option<ConfigCommand>) -> Result<(), PowError> {
    match command {
        None => crate::config::cmd_print(json),
        Some(ConfigCommand::Get { key }) => crate::config::cmd_get(&key),
        Some(ConfigCommand::Set { key, value }) => crate::config::cmd_set(&key, &value),
    }
}
