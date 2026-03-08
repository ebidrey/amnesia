use std::path::Path;

use crate::model::Observation;
use crate::store;

const COL: usize = 11; // label column width — "timestamp: " is the longest at 11 chars

pub struct GetArgs {
    pub id_prefix: String,
}

pub fn run(args: GetArgs, store_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let observations = store::load_from(store_path)?;

    let obs = observations
        .iter()
        .find(|o| o.id.starts_with(&args.id_prefix))
        .ok_or_else(|| format!("no observation found with id prefix '{}'", args.id_prefix))?;

    print_full(obs);
    Ok(())
}

fn print_full(obs: &Observation) {
    let indent = " ".repeat(COL);

    println!("{:<COL$}{}", "id:", obs.id);
    println!("{:<COL$}{}", "timestamp:", obs.timestamp);
    println!("{:<COL$}{}", "agent:", obs.agent);
    println!("{:<COL$}{}", "type:", obs.op_type);
    if let Some(sid) = &obs.session_id {
        println!("{:<COL$}{}", "session:", sid);
    }
    println!("{:<COL$}{}", "title:", obs.title);

    // multi-line content: first line with label, rest indented
    let content_lines: Vec<&str> = obs.content.lines().collect();
    match content_lines.as_slice() {
        [] => println!("{:<COL$}", "content:"),
        [first, rest @ ..] => {
            println!("{:<COL$}{}", "content:", first);
            for line in rest {
                println!("{indent}{line}");
            }
        }
    }

    // files: one per line, all aligned
    match obs.files.as_slice() {
        [] => println!("{:<COL$}", "files:"),
        [first, rest @ ..] => {
            println!("{:<COL$}{}", "files:", first);
            for file in rest {
                println!("{indent}{file}");
            }
        }
    }

    println!("{:<COL$}{}", "tags:", obs.tags.join(", "));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::OpType;
    use crate::store;
    use tempfile::NamedTempFile;

    fn make_obs(id: &str) -> Observation {
        Observation {
            id: id.to_string(),
            timestamp: "2026-03-07T14:23:01Z".to_string(),
            agent: "backend-developer".to_string(),
            op_type: OpType::Bugfix,
            title: "Fixed N+1 in product list".to_string(),
            content: "First line.\nSecond line.\nThird line.".to_string(),
            files: vec!["api/views.py".to_string(), "api/serializers.py".to_string()],
            tags: vec!["django".to_string(), "performance".to_string()],
            session_id: None,
        }
    }

    #[test]
    fn finds_by_exact_id() {
        let file = NamedTempFile::new().unwrap();
        let obs = make_obs("01JNABCDEFGHIJKLMNOPQRSTUV");
        store::append_to(file.path(), &obs).unwrap();

        let args = GetArgs { id_prefix: "01JNABCDEFGHIJKLMNOPQRSTUV".to_string() };
        assert!(run(args, file.path()).is_ok());
    }

    #[test]
    fn finds_by_short_prefix() {
        let file = NamedTempFile::new().unwrap();
        let obs = make_obs("01JNABCDEFGHIJKLMNOPQRSTUV");
        store::append_to(file.path(), &obs).unwrap();

        let args = GetArgs { id_prefix: "01JNAB".to_string() };
        assert!(run(args, file.path()).is_ok());
    }

    #[test]
    fn returns_error_when_not_found() {
        let file = NamedTempFile::new().unwrap();
        let obs = make_obs("01JNABCDEFGHIJKLMNOPQRSTUV");
        store::append_to(file.path(), &obs).unwrap();

        let args = GetArgs { id_prefix: "01ZZZZZZ".to_string() };
        let result = run(args, file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no observation found"));
    }

    #[test]
    fn returns_error_on_empty_store() {
        let file = NamedTempFile::new().unwrap();
        let args = GetArgs { id_prefix: "01JNAAAA".to_string() };
        assert!(run(args, file.path()).is_err());
    }

    #[test]
    fn finds_correct_observation_among_multiple() {
        let file = NamedTempFile::new().unwrap();
        store::append_to(file.path(), &make_obs("01JNAAAA0000000000000000AA")).unwrap();
        store::append_to(file.path(), &make_obs("01JNBBBB0000000000000000BB")).unwrap();
        store::append_to(file.path(), &make_obs("01JNCCCC0000000000000000CC")).unwrap();

        let observations = store::load_from(file.path()).unwrap();
        let found = observations.iter().find(|o| o.id.starts_with("01JNBBBB"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "01JNBBBB0000000000000000BB");
    }

    #[test]
    fn handles_empty_files_and_tags() {
        let file = NamedTempFile::new().unwrap();
        let obs = Observation {
            id: "01JNABCDEFGHIJKLMNOPQRSTUV".to_string(),
            timestamp: "2026-03-07T00:00:00Z".to_string(),
            agent: "agent".to_string(),
            op_type: OpType::Discovery,
            title: "A discovery".to_string(),
            content: "Found something.".to_string(),
            files: vec![],
            tags: vec![],
            session_id: None,
        };
        store::append_to(file.path(), &obs).unwrap();

        let args = GetArgs { id_prefix: "01JNAB".to_string() };
        assert!(run(args, file.path()).is_ok());
    }
}
