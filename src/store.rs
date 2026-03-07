use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::model::Observation;

type StoreResult<T> = Result<T, Box<dyn std::error::Error>>;

// --- path -------------------------------------------------------------------

pub fn store_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".context-memory")
        .join("store.ndjson")
}

fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

// --- core (path-parametric, used by public API and tests) -------------------

pub fn load_from(path: &Path) -> StoreResult<Vec<Observation>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut observations = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let obs: Observation = serde_json::from_str(&line)?;
        observations.push(obs);
    }

    Ok(observations)
}

pub fn append_to(path: &Path, obs: &Observation) -> StoreResult<()> {
    ensure_parent(path)?;

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(obs)?;
    writeln!(file, "{}", line)?;

    Ok(())
}

// --- public API (uses default store path) -----------------------------------

pub fn load_all() -> StoreResult<Vec<Observation>> {
    load_from(&store_path())
}

pub fn append(obs: &Observation) -> StoreResult<()> {
    append_to(&store_path(), obs)
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::OpType;
    use tempfile::NamedTempFile;

    fn sample(id: &str, op_type: OpType) -> Observation {
        Observation {
            id: id.to_string(),
            timestamp: "2026-03-07T14:23:01Z".to_string(),
            agent: "backend-developer".to_string(),
            op_type,
            title: "Test observation".to_string(),
            content: "Some content".to_string(),
            files: vec!["src/main.rs".to_string()],
            tags: vec!["rust".to_string()],
        }
    }

    #[test]
    fn load_from_nonexistent_file_returns_empty() {
        let path = Path::new("/tmp/amnesia_nonexistent_store_xyz.ndjson");
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
    fn append_then_load_single_observation() {
        let file = NamedTempFile::new().unwrap();
        let obs = sample("01A", OpType::Bugfix);

        append_to(file.path(), &obs).unwrap();

        let loaded = load_from(file.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], obs);
    }

    #[test]
    fn append_preserves_insertion_order() {
        let file = NamedTempFile::new().unwrap();
        let first = sample("01A", OpType::Decision);
        let second = sample("01B", OpType::Bugfix);
        let third = sample("01C", OpType::Summary);

        append_to(file.path(), &first).unwrap();
        append_to(file.path(), &second).unwrap();
        append_to(file.path(), &third).unwrap();

        let loaded = load_from(file.path()).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].id, "01A");
        assert_eq!(loaded[1].id, "01B");
        assert_eq!(loaded[2].id, "01C");
    }

    #[test]
    fn load_skips_blank_lines() {
        use std::io::Write;

        let mut file = NamedTempFile::new().unwrap();
        let obs = sample("01A", OpType::Discovery);
        let line = serde_json::to_string(&obs).unwrap();

        writeln!(file, "{}", line).unwrap();
        writeln!(file).unwrap(); // blank line
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
        let obs = Observation {
            id: "01HX4K2M3N5P6Q7R8S9T0U1V2W".to_string(),
            timestamp: "2026-03-07T14:23:01Z".to_string(),
            agent: "api-designer".to_string(),
            op_type: OpType::Pattern,
            title: "Repository pattern for all data access".to_string(),
            content: "All DB access goes through repository classes.".to_string(),
            files: vec!["src/repos/user.rs".to_string(), "src/repos/mod.rs".to_string()],
            tags: vec!["architecture".to_string(), "pattern".to_string()],
        };

        append_to(file.path(), &obs).unwrap();
        let loaded = load_from(file.path()).unwrap();

        assert_eq!(loaded[0], obs);
    }
}
