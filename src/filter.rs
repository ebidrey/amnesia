use crate::model::{Observation, OpType};

pub struct FilterOptions {
    pub agent: Option<String>,
    pub op_type: Option<OpType>,
    pub after: Option<String>,       // YYYY-MM-DD
    pub before: Option<String>,      // YYYY-MM-DD
    pub files: Option<String>,       // substring match against any path in files
    pub session_id: Option<String>,
}

pub fn apply(observations: Vec<Observation>, opts: &FilterOptions) -> Vec<Observation> {
    observations
        .into_iter()
        .filter(|obs| matches_all(obs, opts))
        .collect()
}

fn matches_all(obs: &Observation, opts: &FilterOptions) -> bool {
    if let Some(agent) = &opts.agent {
        if &obs.agent != agent {
            return false;
        }
    }

    if let Some(op_type) = &opts.op_type {
        if &obs.op_type != op_type {
            return false;
        }
    }

    if let Some(after) = &opts.after {
        // timestamps are ISO 8601 — first 10 chars are YYYY-MM-DD, lexicographic
        // comparison works because the format is fixed-width and zero-padded
        if obs.timestamp.len() < 10 || &obs.timestamp[..10] < after.as_str() {
            return false;
        }
    }

    if let Some(before) = &opts.before {
        if obs.timestamp.len() < 10 || &obs.timestamp[..10] > before.as_str() {
            return false;
        }
    }

    if let Some(file_substr) = &opts.files {
        if !obs.files.iter().any(|f| f.contains(file_substr.as_str())) {
            return false;
        }
    }

    if let Some(sid) = &opts.session_id {
        if obs.session_id.as_deref() != Some(sid.as_str()) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(id: &str, agent: &str, op_type: OpType, date: &str, files: &[&str]) -> Observation {
        Observation {
            id: id.to_string(),
            timestamp: format!("{}T00:00:00Z", date),
            agent: agent.to_string(),
            op_type,
            title: format!("Title {}", id),
            content: "Some content".to_string(),
            files: files.iter().map(|s| s.to_string()).collect(),
            tags: vec![],
            session_id: None,
        }
    }

    fn all_opts() -> FilterOptions {
        FilterOptions {
            agent: None,
            op_type: None,
            after: None,
            before: None,
            files: None,
            session_id: None,
        }
    }

    fn sample_set() -> Vec<Observation> {
        vec![
            obs("01", "backend-developer", OpType::Bugfix,    "2026-01-10", &["api/views.py"]),
            obs("02", "api-designer",      OpType::Decision,  "2026-02-15", &["api/routes.py"]),
            obs("03", "backend-developer", OpType::Discovery, "2026-03-01", &["src/main.rs"]),
            obs("04", "orchestrator",      OpType::Summary,   "2026-03-07", &["docs/plan.md"]),
        ]
    }

    #[test]
    fn no_filters_returns_all() {
        let result = apply(sample_set(), &all_opts());
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn filter_by_agent() {
        let opts = FilterOptions {
            agent: Some("backend-developer".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|o| o.agent == "backend-developer"));
    }

    #[test]
    fn filter_by_op_type() {
        let opts = FilterOptions {
            op_type: Some(OpType::Decision),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "02");
    }

    #[test]
    fn filter_after_date_inclusive() {
        let opts = FilterOptions {
            after: Some("2026-03-01".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "03");
        assert_eq!(result[1].id, "04");
    }

    #[test]
    fn filter_before_date_inclusive() {
        let opts = FilterOptions {
            before: Some("2026-02-15".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "01");
        assert_eq!(result[1].id, "02");
    }

    #[test]
    fn filter_after_and_before_is_range() {
        let opts = FilterOptions {
            after: Some("2026-02-01".to_string()),
            before: Some("2026-02-28".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "02");
    }

    #[test]
    fn filter_by_files_substring() {
        let opts = FilterOptions {
            files: Some("api/".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "01");
        assert_eq!(result[1].id, "02");
    }

    #[test]
    fn multiple_filters_are_and_conditions() {
        let opts = FilterOptions {
            agent: Some("backend-developer".to_string()),
            after: Some("2026-02-01".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "03");
    }

    #[test]
    fn filter_that_matches_nothing_returns_empty() {
        let opts = FilterOptions {
            agent: Some("nonexistent-agent".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_by_files_exact_filename() {
        let opts = FilterOptions {
            files: Some("main.rs".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "03");
    }

    #[test]
    fn filter_by_session_id() {
        let sid = "01JNSESSION0000000000000AA".to_string();
        let mut with_session = obs("05", "agent", OpType::Summary, "2026-03-08", &[]);
        with_session.session_id = Some(sid.clone());

        let mut data = sample_set();
        data.push(with_session);

        let opts = FilterOptions {
            session_id: Some(sid.clone()),
            ..all_opts()
        };
        let result = apply(data, &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "05");
        assert_eq!(result[0].session_id, Some(sid));
    }

    #[test]
    fn filter_by_session_id_no_match_returns_empty() {
        let opts = FilterOptions {
            session_id: Some("01JNSESSION0000000000000XX".to_string()),
            ..all_opts()
        };
        let result = apply(sample_set(), &opts);
        assert!(result.is_empty());
    }
}
