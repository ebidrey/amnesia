use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::model::Session;

type SessionsResult<T> = Result<T, Box<dyn std::error::Error>>;

fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

pub fn load_from(path: &Path) -> SessionsResult<Vec<Session>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut sessions = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let session: Session = serde_json::from_str(&line)?;
        sessions.push(session);
    }

    Ok(sessions)
}

pub fn append_to(path: &Path, session: &Session) -> SessionsResult<()> {
    ensure_parent(path)?;

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(session)?;
    writeln!(file, "{}", line)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn sample(id: &str) -> Session {
        Session {
            id: id.to_string(),
            project: "myproject".to_string(),
            orchestrator: "claude".to_string(),
            started_at: "2026-03-08T22:05:00Z".to_string(),
        }
    }

    #[test]
    fn load_from_nonexistent_file_returns_empty() {
        let path = Path::new("/tmp/amnesia_nonexistent_sessions_xyz.ndjson");
        let result = load_from(path).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn load_from_empty_file_returns_empty() {
        let file = NamedTempFile::new().unwrap();
        let result = load_from(file.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn append_then_load_single_session() {
        let file = NamedTempFile::new().unwrap();
        let session = sample("01JNSESSION0000000000000AA");

        append_to(file.path(), &session).unwrap();

        let loaded = load_from(file.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], session);
    }

    #[test]
    fn append_preserves_insertion_order() {
        let file = NamedTempFile::new().unwrap();
        let first = sample("01JNSESSION0000000000000AA");
        let second = sample("01JNSESSION0000000000000BB");
        let third = sample("01JNSESSION0000000000000CC");

        append_to(file.path(), &first).unwrap();
        append_to(file.path(), &second).unwrap();
        append_to(file.path(), &third).unwrap();

        let loaded = load_from(file.path()).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].id, "01JNSESSION0000000000000AA");
        assert_eq!(loaded[1].id, "01JNSESSION0000000000000BB");
        assert_eq!(loaded[2].id, "01JNSESSION0000000000000CC");
    }

    #[test]
    fn load_skips_blank_lines() {
        use std::io::Write;

        let mut file = NamedTempFile::new().unwrap();
        let session = sample("01JNSESSION0000000000000AA");
        let line = serde_json::to_string(&session).unwrap();

        writeln!(file, "{}", line).unwrap();
        writeln!(file).unwrap();
        writeln!(file, "{}", line).unwrap();

        let loaded = load_from(file.path()).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn load_returns_error_on_malformed_json() {
        use std::io::Write;

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{{not valid json}}").unwrap();

        let result = load_from(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn round_trip_all_fields_preserved() {
        let file = NamedTempFile::new().unwrap();
        let session = Session {
            id: "01JNSESSION0000000000000AA".to_string(),
            project: "amnesia".to_string(),
            orchestrator: "claude".to_string(),
            started_at: "2026-03-08T22:05:00Z".to_string(),
        };

        append_to(file.path(), &session).unwrap();
        let loaded = load_from(file.path()).unwrap();

        assert_eq!(loaded[0], session);
    }

    #[test]
    fn append_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("projects").join("myproject").join("sessions.ndjson");
        let session = sample("01JNSESSION0000000000000AA");

        append_to(&path, &session).unwrap();

        assert!(path.exists());
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.len(), 1);
    }
}
