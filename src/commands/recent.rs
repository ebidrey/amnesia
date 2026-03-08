use std::path::Path;

use crate::model::Observation;
use crate::store;

pub struct RecentArgs {
    pub n: usize,
    pub agent: Option<String>,
    pub session_id: Option<String>,
}

pub fn run(args: RecentArgs, store_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut observations = store::load_from(store_path)?;

    if let Some(agent) = &args.agent {
        observations.retain(|o| &o.agent == agent);
    }

    if let Some(sid) = &args.session_id {
        observations.retain(|o| o.session_id.as_deref() == Some(sid));
    }

    // ULIDs are lexicographically sortable by creation time
    observations.sort_by(|a, b| b.id.cmp(&a.id));
    observations.truncate(args.n);

    for obs in &observations {
        print_compact(obs);
    }

    Ok(())
}

pub fn print_compact(obs: &Observation) {
    println!("id:        {}", obs.id);
    println!("agent:     {}", obs.agent);
    println!("type:      {}", obs.op_type);
    println!("timestamp: {}", obs.timestamp);
    println!("title:     {}", obs.title);
    if let Some(sid) = &obs.session_id {
        println!("session:   {}", sid);
    }
    if !obs.files.is_empty() {
        println!("files:     {}", obs.files.join(", "));
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::OpType;
    use crate::store;
    use tempfile::NamedTempFile;

    fn make_obs(id: &str, agent: &str, op_type: OpType, timestamp: &str) -> Observation {
        Observation {
            id: id.to_string(),
            timestamp: format!("{timestamp}T00:00:00Z"),
            agent: agent.to_string(),
            op_type,
            title: format!("Title for {id}"),
            content: "content".to_string(),
            files: vec!["src/lib.rs".to_string()],
            tags: vec![],
            session_id: None,
        }
    }

    fn write_observations(file: &NamedTempFile, observations: &[Observation]) {
        for obs in observations {
            store::append_to(file.path(), obs).unwrap();
        }
    }

    #[test]
    fn returns_newest_first() {
        let file = NamedTempFile::new().unwrap();
        write_observations(&file, &[
            make_obs("01JNAAAA", "agent", OpType::Bugfix,    "2026-01-01"),
            make_obs("01JNBBBB", "agent", OpType::Decision,  "2026-02-01"),
            make_obs("01JNCCCC", "agent", OpType::Discovery, "2026-03-01"),
        ]);

        let args = RecentArgs { n: 10, agent: None, session_id: None };
        let mut observations = store::load_from(file.path()).unwrap();
        observations.sort_by(|a, b| b.id.cmp(&a.id));

        assert_eq!(observations[0].id, "01JNCCCC");
        assert_eq!(observations[1].id, "01JNBBBB");
        assert_eq!(observations[2].id, "01JNAAAA");

        // run doesn't panic
        run(args, file.path()).unwrap();
    }

    #[test]
    fn respects_n_limit() {
        let file = NamedTempFile::new().unwrap();
        write_observations(&file, &[
            make_obs("01JNAAAA", "agent", OpType::Bugfix,   "2026-01-01"),
            make_obs("01JNBBBB", "agent", OpType::Decision, "2026-02-01"),
            make_obs("01JNCCCC", "agent", OpType::Summary,  "2026-03-01"),
        ]);

        let mut observations = store::load_from(file.path()).unwrap();
        observations.sort_by(|a, b| b.id.cmp(&a.id));
        observations.truncate(2);

        assert_eq!(observations.len(), 2);
        assert_eq!(observations[0].id, "01JNCCCC");
        assert_eq!(observations[1].id, "01JNBBBB");
    }

    #[test]
    fn filters_by_agent() {
        let file = NamedTempFile::new().unwrap();
        write_observations(&file, &[
            make_obs("01JNAAAA", "backend-developer", OpType::Bugfix,   "2026-01-01"),
            make_obs("01JNBBBB", "api-designer",      OpType::Decision, "2026-02-01"),
            make_obs("01JNCCCC", "backend-developer", OpType::Summary,  "2026-03-01"),
        ]);

        let args = RecentArgs { n: 10, agent: Some("backend-developer".to_string()), session_id: None };
        run(args, file.path()).unwrap();

        let mut observations = store::load_from(file.path()).unwrap();
        observations.retain(|o| o.agent == "backend-developer");
        assert_eq!(observations.len(), 2);
    }

    #[test]
    fn empty_store_returns_ok() {
        let file = NamedTempFile::new().unwrap();
        let args = RecentArgs { n: 10, agent: None, session_id: None };
        assert!(run(args, file.path()).is_ok());
    }

    #[test]
    fn n_larger_than_store_returns_all() {
        let file = NamedTempFile::new().unwrap();
        write_observations(&file, &[
            make_obs("01JNAAAA", "agent", OpType::Bugfix, "2026-01-01"),
            make_obs("01JNBBBB", "agent", OpType::Summary, "2026-02-01"),
        ]);

        let mut observations = store::load_from(file.path()).unwrap();
        observations.sort_by(|a, b| b.id.cmp(&a.id));
        observations.truncate(100);

        assert_eq!(observations.len(), 2);
    }

    #[test]
    fn filters_by_session_id() {
        let file = NamedTempFile::new().unwrap();
        let sid = "01JNSESSION0000000000000AA".to_string();

        let mut obs_with_session = make_obs("01JNCCCC", "agent", OpType::Summary, "2026-03-01");
        obs_with_session.session_id = Some(sid.clone());

        write_observations(&file, &[
            make_obs("01JNAAAA", "agent", OpType::Bugfix,   "2026-01-01"),
            make_obs("01JNBBBB", "agent", OpType::Decision, "2026-02-01"),
            obs_with_session,
        ]);

        let args = RecentArgs { n: 10, agent: None, session_id: Some(sid.clone()) };
        run(args, file.path()).unwrap();

        let mut observations = store::load_from(file.path()).unwrap();
        observations.retain(|o| o.session_id.as_deref() == Some(&sid));
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].id, "01JNCCCC");
    }
}
