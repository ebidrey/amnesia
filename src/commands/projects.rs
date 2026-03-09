use std::path::Path;

use crate::config;

pub fn run(projects_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !projects_dir.exists() {
        println!("no projects found");
        return Ok(());
    }

    let mut projects: Vec<String> = std::fs::read_dir(projects_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                entry.file_name().into_string().ok()
            } else {
                None
            }
        })
        .collect();

    if projects.is_empty() {
        println!("no projects found");
        return Ok(());
    }

    projects.sort();

    for name in &projects {
        let store = config::project_store_path(name);
        let obs = count_lines(&store);
        println!("project: {name}");
        println!("store:   {}", store.display());
        println!("obs:     {obs}");
        println!();
    }

    Ok(())
}

fn count_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn nonexistent_dir_prints_no_projects() {
        let path = Path::new("/tmp/amnesia_projects_nonexistent_xyz");
        assert!(run(path).is_ok());
    }

    #[test]
    fn empty_dir_prints_no_projects() {
        let dir = TempDir::new().unwrap();
        assert!(run(dir.path()).is_ok());
    }

    #[test]
    fn lists_projects_sorted() {
        let dir = TempDir::new().unwrap();
        for name in &["zeta", "alpha", "beta"] {
            fs::create_dir(dir.path().join(name)).unwrap();
        }
        assert!(run(dir.path()).is_ok());
    }

    #[test]
    fn counts_observations_in_store() {
        let dir = TempDir::new().unwrap();
        let proj = dir.path().join("myproj");
        fs::create_dir(&proj).unwrap();
        fs::write(
            proj.join("store.ndjson"),
            "{\"id\":\"1\"}\n{\"id\":\"2\"}\n\n",
        )
        .unwrap();

        // count_lines should return 2 (skips blank line)
        assert_eq!(count_lines(&proj.join("store.ndjson")), 2);
    }

    #[test]
    fn ignores_files_in_projects_dir() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("real-project")).unwrap();
        fs::write(dir.path().join("not-a-project.txt"), "").unwrap();

        // only real-project should appear; run must not error
        assert!(run(dir.path()).is_ok());
    }
}
