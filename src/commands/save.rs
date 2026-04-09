use std::path::Path;

use chrono::Utc;
use ulid::Ulid;

use crate::model::{Observation, OpType};
use crate::store;

pub struct SaveArgs {
    pub agent: String,
    pub op_type: OpType,
    pub title: String,
    pub content: String,
    pub files: Vec<String>,
    pub tags: Vec<String>,
    pub session_id: Option<String>,
}

pub fn run(
    args: SaveArgs,
    store_path: &Path,
    identity_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let obs = Observation {
        id: Ulid::new().to_string(),
        timestamp: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        agent: args.agent,
        op_type: args.op_type,
        title: args.title,
        content: args.content,
        files: args.files,
        tags: args.tags,
        session_id: args.session_id,
    };

    match identity_path {
        Some(id) => store::append_encrypted(store_path, &obs, id)?,
        None => store::append_to(store_path, &obs)?,
    }
    println!("saved {}", obs.id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store;
    use tempfile::NamedTempFile;

    fn run_save(file: &NamedTempFile, op_type: OpType) {
        run(
            SaveArgs {
                agent: "test-agent".to_string(),
                op_type,
                title: "Test title".to_string(),
                content: "Test content".to_string(),
                files: vec!["src/main.rs".to_string()],
                tags: vec!["rust".to_string()],
                session_id: None,
            },
            file.path(),
            None,
        )
        .unwrap();
    }

    #[test]
    fn saved_observation_can_be_loaded_back() {
        let file = NamedTempFile::new().unwrap();
        run_save(&file, OpType::Bugfix);

        let observations = store::load_from(file.path()).unwrap();
        assert_eq!(observations.len(), 1);

        let obs = &observations[0];
        assert_eq!(obs.agent, "test-agent");
        assert_eq!(obs.op_type, OpType::Bugfix);
        assert_eq!(obs.title, "Test title");
        assert_eq!(obs.files, vec!["src/main.rs"]);
        assert_eq!(obs.tags, vec!["rust"]);
    }

    #[test]
    fn id_is_valid_ulid() {
        let file = NamedTempFile::new().unwrap();
        run_save(&file, OpType::Decision);

        let observations = store::load_from(file.path()).unwrap();
        let id = &observations[0].id;

        // ULID: 26 chars, Crockford base32 alphabet
        assert_eq!(id.len(), 26);
        assert!(id.chars().all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)));
    }

    #[test]
    fn timestamp_is_iso8601_utc() {
        let file = NamedTempFile::new().unwrap();
        run_save(&file, OpType::Summary);

        let observations = store::load_from(file.path()).unwrap();
        let ts = &observations[0].timestamp;

        // format: "2026-03-07T14:23:01Z"
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[19..20], "Z");
    }

    #[test]
    fn multiple_saves_append_in_order() {
        let file = NamedTempFile::new().unwrap();
        run_save(&file, OpType::Decision);
        run_save(&file, OpType::Bugfix);
        run_save(&file, OpType::Summary);

        let observations = store::load_from(file.path()).unwrap();
        assert_eq!(observations.len(), 3);
        assert_eq!(observations[0].op_type, OpType::Decision);
        assert_eq!(observations[1].op_type, OpType::Bugfix);
        assert_eq!(observations[2].op_type, OpType::Summary);
    }

    #[test]
    fn op_type_from_str_all_variants() {
        use std::str::FromStr;
        assert_eq!(OpType::from_str("decision").unwrap(),  OpType::Decision);
        assert_eq!(OpType::from_str("bugfix").unwrap(),    OpType::Bugfix);
        assert_eq!(OpType::from_str("discovery").unwrap(), OpType::Discovery);
        assert_eq!(OpType::from_str("pattern").unwrap(),   OpType::Pattern);
        assert_eq!(OpType::from_str("warning").unwrap(),   OpType::Warning);
        assert_eq!(OpType::from_str("summary").unwrap(),   OpType::Summary);
    }

    #[test]
    fn op_type_from_str_case_insensitive() {
        use std::str::FromStr;
        assert_eq!(OpType::from_str("BUGFIX").unwrap(),  OpType::Bugfix);
        assert_eq!(OpType::from_str("Decision").unwrap(), OpType::Decision);
    }

    #[test]
    fn op_type_from_str_invalid_returns_err() {
        use std::str::FromStr;
        assert!(OpType::from_str("unknown").is_err());
        assert!(OpType::from_str("").is_err());
    }

    #[test]
    fn session_id_stored_when_provided() {
        let file = NamedTempFile::new().unwrap();
        let sid = "01JNSESSION0000000000000AA".to_string();
        run(
            SaveArgs {
                agent: "test-agent".to_string(),
                op_type: OpType::Summary,
                title: "Session test".to_string(),
                content: "content".to_string(),
                files: vec![],
                tags: vec![],
                session_id: Some(sid.clone()),
            },
            file.path(),
            None,
        )
        .unwrap();

        let observations = store::load_from(file.path()).unwrap();
        assert_eq!(observations[0].session_id, Some(sid));
    }

    #[test]
    fn session_id_none_when_not_provided() {
        let file = NamedTempFile::new().unwrap();
        run_save(&file, OpType::Discovery);

        let observations = store::load_from(file.path()).unwrap();
        assert_eq!(observations[0].session_id, None);
    }
}
