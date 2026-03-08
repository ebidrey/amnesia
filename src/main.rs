mod bm25;
mod commands;
mod config;
mod filter;
mod launcher;
mod model;
mod projects;
mod sessions;
mod store;

use clap::{Parser, Subcommand};

use commands::get::GetArgs;
use commands::recent::RecentArgs;
use commands::save::SaveArgs;
use commands::search::SearchArgs;
use commands::sessions::SessionsArgs;
use model::OpType;

#[derive(Parser)]
#[command(name = "amnesia", about = "Persistent memory CLI for AI agents")]
struct Cli {
    /// Project name (overrides AMNESIA_PROJECT env var)
    #[arg(long, global = true)]
    project: Option<String>,

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

        #[arg(long)]
        session: Option<String>,
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

        #[arg(long)]
        session: Option<String>,
    },

    /// Retrieve full content of a specific observation by id prefix
    Get { id: String },

    /// Show the N most recent observations, newest first
    Recent {
        #[arg(short = 'n', default_value_t = 10)]
        n: usize,

        #[arg(long)]
        agent: Option<String>,

        #[arg(long)]
        session: Option<String>,
    },

    /// Show store statistics
    Stats,

    /// List sessions for the current project
    Sessions {
        #[arg(short = 'n', default_value_t = 10)]
        n: usize,
    },
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
    let store_path = resolve_store_path(&config, cli.project.as_deref());

    match cli.command.unwrap() {
        Command::Save { agent, op_type, title, content, files, tags, session } => {
            let session_id = session.or_else(|| std::env::var("AMNESIA_SESSION").ok().filter(|s| !s.is_empty()));
            commands::save::run(
                SaveArgs {
                    agent,
                    op_type,
                    title,
                    content,
                    files: split_csv(&files),
                    tags: split_csv(&tags),
                    session_id,
                },
                &store_path,
            )?;
        }

        Command::Search { query, agent, op_type, after, before, files, limit, session } => {
            commands::search::run(
                SearchArgs {
                    query,
                    agent,
                    op_type,
                    after,
                    before,
                    files,
                    limit: limit.unwrap_or(config.default_limit),
                    session_id: session,
                },
                &store_path,
            )?;
        }

        Command::Get { id } => {
            commands::get::run(GetArgs { id_prefix: id }, &store_path)?;
        }

        Command::Recent { n, agent, session } => {
            commands::recent::run(RecentArgs { n, agent, session_id: session }, &store_path)?;
        }

        Command::Stats => {
            commands::stats::run(&store_path)?;
        }

        Command::Sessions { n } => {
            let sessions_path = resolve_sessions_path(cli.project.as_deref())?;
            commands::sessions::run(SessionsArgs { n }, &sessions_path)?;
        }
    }

    Ok(())
}

fn resolve_store_path(config: &config::Config, project: Option<&str>) -> std::path::PathBuf {
    let project = project
        .map(str::to_string)
        .or_else(|| std::env::var("AMNESIA_PROJECT").ok().filter(|s| !s.is_empty()));
    match project {
        Some(p) => config::project_store_path(&p),
        None => config.store_path_expanded(),
    }
}

fn resolve_sessions_path(project: Option<&str>) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let project = project
        .map(str::to_string)
        .or_else(|| std::env::var("AMNESIA_PROJECT").ok().filter(|s| !s.is_empty()));
    match project {
        Some(p) => Ok(config::project_sessions_path(&p)),
        None => Err("sessions require AMNESIA_PROJECT to be set (or pass --project)".into()),
    }
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn default_config() -> config::Config {
        config::Config::default()
    }

    // Serialize all tests that mutate AMNESIA_PROJECT to avoid races in parallel test runs.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Save and restore AMNESIA_PROJECT around a closure, holding ENV_LOCK for the duration.
    fn with_env_project<F: FnOnce()>(value: Option<&str>, f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let saved = std::env::var("AMNESIA_PROJECT").ok();
        unsafe {
            match value {
                Some(v) => std::env::set_var("AMNESIA_PROJECT", v),
                None => std::env::remove_var("AMNESIA_PROJECT"),
            }
        }
        f();
        unsafe {
            match saved {
                Some(v) => std::env::set_var("AMNESIA_PROJECT", v),
                None => std::env::remove_var("AMNESIA_PROJECT"),
            }
        }
    }

    #[test]
    fn store_path_uses_flag_project() {
        let path = resolve_store_path(&default_config(), Some("myproject"));
        assert!(path.ends_with(".context-memory/projects/myproject/store.ndjson"));
    }

    #[test]
    fn store_path_falls_back_to_global_when_no_project() {
        with_env_project(None, || {
            let path = resolve_store_path(&default_config(), None);
            assert!(path.ends_with("store.ndjson"));
            assert!(!path.to_string_lossy().contains("/projects/"));
        });
    }

    #[test]
    fn store_path_flag_takes_precedence_over_env_var() {
        with_env_project(Some("env-project"), || {
            let path = resolve_store_path(&default_config(), Some("flag-project"));
            assert!(path.ends_with(".context-memory/projects/flag-project/store.ndjson"));
        });
    }

    #[test]
    fn store_path_uses_env_var_when_no_flag() {
        with_env_project(Some("env-project"), || {
            let path = resolve_store_path(&default_config(), None);
            assert!(path.ends_with(".context-memory/projects/env-project/store.ndjson"));
        });
    }

    #[test]
    fn sessions_path_uses_flag_project() {
        let path = resolve_sessions_path(Some("myproject")).unwrap();
        assert!(path.ends_with(".context-memory/projects/myproject/sessions.ndjson"));
    }

    #[test]
    fn sessions_path_errors_without_project() {
        with_env_project(None, || {
            assert!(resolve_sessions_path(None).is_err());
        });
    }

    #[test]
    fn sessions_path_uses_env_var_when_no_flag() {
        with_env_project(Some("env-project"), || {
            let path = resolve_sessions_path(None).unwrap();
            assert!(path.ends_with(".context-memory/projects/env-project/sessions.ndjson"));
        });
    }

    #[test]
    fn sessions_path_flag_takes_precedence_over_env_var() {
        with_env_project(Some("env-project"), || {
            let path = resolve_sessions_path(Some("flag-project")).unwrap();
            assert!(path.ends_with(".context-memory/projects/flag-project/sessions.ndjson"));
        });
    }
}
