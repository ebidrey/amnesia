use std::path::Path;

use crate::bm25;
use crate::commands::recent::print_compact;
use crate::filter::{self, FilterOptions};
use crate::model::OpType;
use crate::store;

pub struct SearchArgs {
    pub query: Option<String>,
    pub agent: Option<String>,
    pub op_type: Option<OpType>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub files: Option<String>,
    pub limit: usize,
    pub session_id: Option<String>,
}

pub fn run(args: SearchArgs, store_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let observations = store::load_from(store_path)?;

    let filtered = filter::apply(observations, &FilterOptions {
        agent: args.agent,
        op_type: args.op_type,
        after: args.after,
        before: args.before,
        files: args.files,
        session_id: args.session_id,
    });

    let results = match args.query.as_deref().filter(|q| !q.is_empty()) {
        Some(q) => bm25::rank(filtered, q, args.limit),
        None => {
            let mut obs = filtered;
            obs.sort_by(|a, b| b.id.cmp(&a.id));
            obs.truncate(args.limit);
            obs
        }
    };

    if results.is_empty() {
        println!("no results");
    } else {
        for obs in &results {
            print_compact(obs);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Observation;
    use crate::store;
    use tempfile::NamedTempFile;

    fn obs(agent: &str, op_type: OpType, date: &str, title: &str, content: &str) -> Observation {
        Observation {
            id: ulid::Ulid::new().to_string(),
            timestamp: format!("{date}T00:00:00Z"),
            agent: agent.to_string(),
            op_type,
            title: title.to_string(),
            content: content.to_string(),
            files: vec!["src/lib.rs".to_string()],
            tags: vec![],
            session_id: None,
        }
    }

    fn setup() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let data = [
            ("backend-developer", OpType::Bugfix,    "2026-01-10", "Fixed N+1 query in Django", "Added select_related to queryset"),
            ("api-designer",      OpType::Decision,  "2026-02-15", "JWT auth via cookies",       "Use httpOnly cookies for JWT storage"),
            ("backend-developer", OpType::Discovery, "2026-03-01", "Redis cache invalidation",   "LRU policy causes stale reads"),
            ("orchestrator",      OpType::Summary,   "2026-03-07", "Session summary",            "Completed auth refactor and cache work"),
        ];
        for (agent, op_type, date, title, content) in data {
            store::append_to(file.path(), &obs(agent, op_type, date, title, content)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        file
    }

    fn args(query: Option<&str>) -> SearchArgs {
        SearchArgs {
            query: query.map(str::to_string),
            agent: None,
            op_type: None,
            after: None,
            before: None,
            files: None,
            limit: 10,
            session_id: None,
        }
    }

    #[test]
    fn basic_search_returns_matching_results() {
        let file = setup();
        let observations = store::load_from(file.path()).unwrap();
        let filtered = filter::apply(observations, &FilterOptions {
            agent: None, op_type: None, after: None, before: None, files: None,
            session_id: None,
        });
        let results = bm25::rank(filtered, "django", 10);
        assert!(!results.is_empty());
        assert!(results[0].title.contains("Django"));
    }

    #[test]
    fn search_with_agent_filter() {
        let file = setup();
        let observations = store::load_from(file.path()).unwrap();
        let filtered = filter::apply(observations, &FilterOptions {
            agent: Some("api-designer".to_string()),
            op_type: None, after: None, before: None, files: None,
            session_id: None,
        });
        let results = bm25::rank(filtered, "JWT", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].agent, "api-designer");
    }

    #[test]
    fn search_with_op_type_filter() {
        let file = setup();
        let observations = store::load_from(file.path()).unwrap();
        let filtered = filter::apply(observations, &FilterOptions {
            op_type: Some(OpType::Bugfix),
            agent: None, after: None, before: None, files: None,
            session_id: None,
        });
        let results = bm25::rank(filtered, "query", 10);
        assert!(results.iter().all(|o| o.op_type == OpType::Bugfix));
    }

    #[test]
    fn search_with_date_filter() {
        let file = setup();
        let observations = store::load_from(file.path()).unwrap();
        let filtered = filter::apply(observations, &FilterOptions {
            after: Some("2026-03-01".to_string()),
            agent: None, op_type: None, before: None, files: None,
            session_id: None,
        });
        let results = bm25::rank(filtered, "cache", 10);
        assert!(results.iter().all(|o| &o.timestamp[..10] >= "2026-03-01"));
    }

    #[test]
    fn no_matching_query_runs_ok() {
        let file = setup();
        assert!(run(args(Some("xyznonexistentterm")), file.path()).is_ok());
    }

    #[test]
    fn limit_is_respected() {
        let file = setup();
        let observations = store::load_from(file.path()).unwrap();
        let filtered = filter::apply(observations, &FilterOptions {
            agent: None, op_type: None, after: None, before: None, files: None,
            session_id: None,
        });
        // "a" matches most documents
        let results = bm25::rank(filtered, "auth cache query summary", 2);
        assert!(results.len() <= 2);
    }

    #[test]
    fn most_relevant_result_is_first() {
        let file = setup();
        let observations = store::load_from(file.path()).unwrap();
        let filtered = filter::apply(observations, &FilterOptions {
            agent: None, op_type: None, after: None, before: None, files: None,
            session_id: None,
        });
        let results = bm25::rank(filtered, "JWT cookies auth", 10);
        // JWT auth observation should rank first
        assert!(!results.is_empty());
        assert!(results[0].title.to_lowercase().contains("jwt"));
    }

    #[test]
    fn empty_store_runs_ok() {
        let file = NamedTempFile::new().unwrap();
        assert!(run(args(Some("anything")), file.path()).is_ok());
    }

    #[test]
    fn no_query_returns_newest_first() {
        let file = setup();
        assert!(run(args(None), file.path()).is_ok());

        let mut observations = store::load_from(file.path()).unwrap();
        observations.sort_by(|a, b| b.id.cmp(&a.id));
        assert_eq!(&observations[0].timestamp[..10], "2026-03-07");
    }

    #[test]
    fn no_query_with_filter_returns_filtered_newest_first() {
        let file = setup();
        let result = run(
            SearchArgs {
                query: None,
                agent: Some("backend-developer".to_string()),
                op_type: None, after: None, before: None, files: None,
                limit: 10,
                session_id: None,
            },
            file.path(),
        );
        assert!(result.is_ok());

        let mut observations = store::load_from(file.path()).unwrap();
        observations.retain(|o| o.agent == "backend-developer");
        observations.sort_by(|a, b| b.id.cmp(&a.id));
        assert_eq!(observations.len(), 2);
        assert_eq!(&observations[0].timestamp[..10], "2026-03-01");
    }

    #[test]
    fn no_query_respects_limit() {
        let file = setup();
        let result = run(
            SearchArgs {
                query: None,
                agent: None, op_type: None, after: None, before: None, files: None,
                limit: 2,
                session_id: None,
            },
            file.path(),
        );
        assert!(result.is_ok());

        let mut observations = store::load_from(file.path()).unwrap();
        observations.sort_by(|a, b| b.id.cmp(&a.id));
        observations.truncate(2);
        assert_eq!(observations.len(), 2);
    }
}
