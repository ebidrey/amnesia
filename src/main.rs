mod bm25;
mod commands;
mod config;
mod filter;
mod launcher;
mod model;
mod projects;
mod store;

use clap::{Parser, Subcommand};

use commands::get::GetArgs;
use commands::recent::RecentArgs;
use commands::save::SaveArgs;
use commands::search::SearchArgs;
use model::OpType;

#[derive(Parser)]
#[command(name = "amnesia", about = "Persistent memory CLI for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Save a new observation
    Save {
        #[arg(long)]
        agent: String,

        #[arg(long = "type")]
        op_type: OpType,

        #[arg(long)]
        title: String,

        #[arg(long)]
        content: String,

        /// Comma-separated file paths
        #[arg(long, default_value = "")]
        files: String,

        /// Comma-separated tags
        #[arg(long, default_value = "")]
        tags: String,
    },

    /// Full-text search with optional filters. Without a query, returns the most recent observations.
    Search {
        query: Option<String>,

        #[arg(long)]
        agent: Option<String>,

        #[arg(long = "type")]
        op_type: Option<OpType>,

        /// Include observations on or after this date (YYYY-MM-DD)
        #[arg(long)]
        after: Option<String>,

        /// Include observations on or before this date (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,

        /// Substring match against file paths
        #[arg(long)]
        files: Option<String>,

        /// Max results (defaults to config default_limit)
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Retrieve full content of a specific observation by id prefix
    Get { id: String },

    /// Show the N most recent observations, newest first
    Recent {
        #[arg(short = 'n', default_value_t = 10)]
        n: usize,

        #[arg(long)]
        agent: Option<String>,
    },

    /// Show store statistics
    Stats,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.command.is_none() {
        launcher::run()?;
        return Ok(());
    }

    let config = config::load();
    let store_path = resolve_store_path(&config);

    match cli.command.unwrap() {
        Command::Save { agent, op_type, title, content, files, tags } => {
            commands::save::run(
                SaveArgs {
                    agent,
                    op_type,
                    title,
                    content,
                    files: split_csv(&files),
                    tags: split_csv(&tags),
                },
                &store_path,
            )?;
        }

        Command::Search { query, agent, op_type, after, before, files, limit } => {
            commands::search::run(
                SearchArgs {
                    query,
                    agent,
                    op_type,
                    after,
                    before,
                    files,
                    limit: limit.unwrap_or(config.default_limit),
                },
                &store_path,
            )?;
        }

        Command::Get { id } => {
            commands::get::run(GetArgs { id_prefix: id }, &store_path)?;
        }

        Command::Recent { n, agent } => {
            commands::recent::run(RecentArgs { n, agent }, &store_path)?;
        }

        Command::Stats => {
            commands::stats::run(&store_path)?;
        }
    }

    Ok(())
}

fn resolve_store_path(config: &config::Config) -> std::path::PathBuf {
    if let Ok(project) = std::env::var("AMNESIA_PROJECT") {
        if !project.is_empty() {
            return config::project_store_path(&project);
        }
    }
    config.store_path_expanded()
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
