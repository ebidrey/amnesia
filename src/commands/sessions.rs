use std::path::Path;

use crate::model::Session;
use crate::sessions;

pub struct SessionsArgs {
    pub n: usize,
}

pub fn run(args: SessionsArgs, sessions_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut all = sessions::load_from(sessions_path)?;
    all.sort_by(|a, b| b.id.cmp(&a.id));
    all.truncate(args.n);

    for s in &all {
        print_session(s);
    }

    Ok(())
}

fn print_session(s: &Session) {
    println!("id:           {}", s.id);
    println!("project:      {}", s.project);
    println!("orchestrator: {}", s.orchestrator);
    println!("started_at:   {}", s.started_at);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions;
    use tempfile::NamedTempFile;

    fn sample(id: &str, orchestrator: &str) -> Session {
        Session {
            id: id.to_string(),
            project: "myproject".to_string(),
            orchestrator: orchestrator.to_string(),
            started_at: "2026-03-08T22:05:00Z".to_string(),
        }
    }

    #[test]
    fn empty_store_runs_ok() {
        let file = NamedTempFile::new().unwrap();
        assert!(run(SessionsArgs { n: 10 }, file.path()).is_ok());
    }

    #[test]
    fn nonexistent_store_runs_ok() {
        let path = Path::new("/tmp/amnesia_sessions_nonexistent_xyz.ndjson");
        assert!(run(SessionsArgs { n: 10 }, path).is_ok());
    }

    #[test]
    fn returns_newest_first() {
        let file = NamedTempFile::new().unwrap();
        sessions::append_to(file.path(), &sample("01JNAAAA0000000000000000AA", "claude")).unwrap();
        sessions::append_to(file.path(), &sample("01JNCCCC0000000000000000CC", "opencode")).unwrap();
        sessions::append_to(file.path(), &sample("01JNBBBB0000000000000000BB", "cursor")).unwrap();

        let mut all = sessions::load_from(file.path()).unwrap();
        all.sort_by(|a, b| b.id.cmp(&a.id));

        assert_eq!(all[0].id, "01JNCCCC0000000000000000CC");
        assert_eq!(all[1].id, "01JNBBBB0000000000000000BB");
        assert_eq!(all[2].id, "01JNAAAA0000000000000000AA");

        assert!(run(SessionsArgs { n: 10 }, file.path()).is_ok());
    }

    #[test]
    fn respects_n_limit() {
        let file = NamedTempFile::new().unwrap();
        sessions::append_to(file.path(), &sample("01JNAAAA0000000000000000AA", "claude")).unwrap();
        sessions::append_to(file.path(), &sample("01JNBBBB0000000000000000BB", "claude")).unwrap();
        sessions::append_to(file.path(), &sample("01JNCCCC0000000000000000CC", "claude")).unwrap();

        let mut all = sessions::load_from(file.path()).unwrap();
        all.sort_by(|a, b| b.id.cmp(&a.id));
        all.truncate(2);

        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "01JNCCCC0000000000000000CC");
    }

    #[test]
    fn n_larger_than_store_returns_all() {
        let file = NamedTempFile::new().unwrap();
        sessions::append_to(file.path(), &sample("01JNAAAA0000000000000000AA", "claude")).unwrap();

        assert!(run(SessionsArgs { n: 100 }, file.path()).is_ok());

        let all = sessions::load_from(file.path()).unwrap();
        assert_eq!(all.len(), 1);
    }
}
