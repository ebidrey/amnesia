use std::collections::HashMap;
use std::path::Path;

use crate::store;

const COL: usize = 10;

pub fn run(store_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let observations = store::load_from(store_path)?;

    // total
    println!("{:<COL$}{} observations", "total:", observations.len());

    // agents — sorted by count desc, then name asc for ties
    let mut agent_counts: HashMap<&str, usize> = HashMap::new();
    for obs in &observations {
        *agent_counts.entry(obs.agent.as_str()).or_insert(0) += 1;
    }
    let mut agents: Vec<(&str, usize)> = agent_counts.into_iter().collect();
    agents.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let agents_str = if agents.is_empty() {
        "-".to_string()
    } else {
        agents.iter().map(|(a, c)| format!("{a} ({c})")).collect::<Vec<_>>().join(", ")
    };
    println!("{:<COL$}{}", "agents:", agents_str);

    // types — same ordering
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for obs in &observations {
        *type_counts.entry(obs.op_type.to_string()).or_insert(0) += 1;
    }
    let mut types: Vec<(String, usize)> = type_counts.into_iter().collect();
    types.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let types_str = if types.is_empty() {
        "-".to_string()
    } else {
        types.iter().map(|(t, c)| format!("{t} ({c})")).collect::<Vec<_>>().join(", ")
    };
    println!("{:<COL$}{}", "types:", types_str);

    // oldest / newest — lexicographic min/max on the date prefix of the timestamp
    let oldest = observations.iter().map(|o| &o.timestamp[..o.timestamp.len().min(10)]).min();
    let newest = observations.iter().map(|o| &o.timestamp[..o.timestamp.len().min(10)]).max();
    println!("{:<COL$}{}", "oldest:", oldest.unwrap_or("-"));
    println!("{:<COL$}{}", "newest:", newest.unwrap_or("-"));

    // file path + size
    let display = store_path.to_string_lossy();
    if store_path.exists() {
        let bytes = std::fs::metadata(store_path)?.len();
        println!("{:<COL$}{} ({})", "file:", display, format_size(bytes));
    } else {
        println!("{:<COL$}{} (not created yet)", "file:", display);
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{}MB", bytes / (1024 * 1024))
    } else if bytes >= 1024 {
        format!("{}KB", bytes / 1024)
    } else {
        format!("{bytes}B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Observation, OpType};
    use crate::store;
    use tempfile::NamedTempFile;

    fn obs(agent: &str, op_type: OpType, date: &str) -> Observation {
        Observation {
            id: ulid::Ulid::new().to_string(),
            timestamp: format!("{date}T00:00:00Z"),
            agent: agent.to_string(),
            op_type,
            title: "title".to_string(),
            content: "content".to_string(),
            files: vec![],
            tags: vec![],
        }
    }

    fn write(file: &NamedTempFile, observations: Vec<Observation>) {
        for o in observations {
            store::append_to(file.path(), &o).unwrap();
        }
    }

    #[test]
    fn empty_store_runs_without_error() {
        let file = NamedTempFile::new().unwrap();
        assert!(run(file.path()).is_ok());
    }

    #[test]
    fn nonexistent_store_runs_without_error() {
        let path = Path::new("/tmp/amnesia_stats_nonexistent_xyz.ndjson");
        assert!(run(path).is_ok());
    }

    #[test]
    fn counts_are_correct() {
        let file = NamedTempFile::new().unwrap();
        write(&file, vec![
            obs("backend-developer", OpType::Bugfix,    "2026-01-01"),
            obs("backend-developer", OpType::Bugfix,    "2026-01-02"),
            obs("api-designer",      OpType::Decision,  "2026-01-03"),
        ]);
        let observations = store::load_from(file.path()).unwrap();
        assert_eq!(observations.len(), 3);

        let mut agent_counts: HashMap<&str, usize> = HashMap::new();
        for o in &observations {
            *agent_counts.entry(o.agent.as_str()).or_insert(0) += 1;
        }
        assert_eq!(agent_counts["backend-developer"], 2);
        assert_eq!(agent_counts["api-designer"], 1);
    }

    #[test]
    fn agents_sorted_by_count_descending() {
        let file = NamedTempFile::new().unwrap();
        write(&file, vec![
            obs("api-designer",      OpType::Decision, "2026-01-01"),
            obs("backend-developer", OpType::Bugfix,   "2026-01-02"),
            obs("backend-developer", OpType::Summary,  "2026-01-03"),
            obs("backend-developer", OpType::Bugfix,   "2026-01-04"),
        ]);
        let observations = store::load_from(file.path()).unwrap();

        let mut agent_counts: HashMap<&str, usize> = HashMap::new();
        for o in &observations {
            *agent_counts.entry(o.agent.as_str()).or_insert(0) += 1;
        }
        let mut agents: Vec<(&str, usize)> = agent_counts.into_iter().collect();
        agents.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

        assert_eq!(agents[0].0, "backend-developer");
        assert_eq!(agents[0].1, 3);
        assert_eq!(agents[1].0, "api-designer");
        assert_eq!(agents[1].1, 1);
    }

    #[test]
    fn oldest_and_newest_dates() {
        let file = NamedTempFile::new().unwrap();
        write(&file, vec![
            obs("agent", OpType::Bugfix,   "2026-03-07"),
            obs("agent", OpType::Decision, "2026-01-01"),
            obs("agent", OpType::Summary,  "2026-06-15"),
        ]);
        let observations = store::load_from(file.path()).unwrap();

        let oldest = observations.iter().map(|o| &o.timestamp[..10]).min().unwrap();
        let newest = observations.iter().map(|o| &o.timestamp[..10]).max().unwrap();

        assert_eq!(oldest, "2026-01-01");
        assert_eq!(newest, "2026-06-15");
    }

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1023), "1023B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1KB");
        assert_eq!(format_size(84 * 1024), "84KB");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1MB");
        assert_eq!(format_size(5 * 1024 * 1024), "5MB");
    }
}
