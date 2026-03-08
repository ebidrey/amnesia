use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectsConfig {
    #[serde(default)]
    pub projects: Vec<Project>,
}

pub fn projects_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".context-memory").join("projects.toml")
}

// --- core (path-parametric, used by public API and tests) -------------------

pub fn load_from(path: &Path) -> ProjectsConfig {
    if !path.exists() {
        return ProjectsConfig::default();
    }

    let contents = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return ProjectsConfig::default(),
    };

    toml::from_str(&contents).unwrap_or_default()
}

pub fn save_to(path: &Path, config: &ProjectsConfig) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let contents = toml::to_string(config)?;
    fs::write(path, contents)?;
    Ok(())
}

// ----------------------------------------------------------------------------

pub fn load() -> ProjectsConfig {
    load_from(&projects_path())
}

pub fn save(config: &ProjectsConfig) -> Result<(), Box<dyn std::error::Error>> {
    save_to(&projects_path(), config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn two_projects() -> ProjectsConfig {
        ProjectsConfig {
            projects: vec![
                Project { name: "amnesia".to_string() },
                Project { name: "my-app".to_string() },
            ],
        }
    }

    // --- projects_path -------------------------------------------------------

    #[test]
    fn projects_path_ends_with_expected_suffix() {
        let path = projects_path();
        assert!(path.ends_with(".context-memory/projects.toml"));
    }

    #[test]
    fn projects_path_is_under_home() {
        let home = std::env::var("HOME").unwrap();
        let path = projects_path();
        assert!(path.starts_with(home));
    }

    // --- load_from -----------------------------------------------------------

    #[test]
    fn load_from_missing_file_returns_empty_config() {
        let path = std::path::Path::new("/tmp/amnesia_projects_nonexistent_xyz.toml");
        let config = load_from(path);
        assert!(config.projects.is_empty());
    }

    #[test]
    fn load_from_empty_file_returns_empty_config() {
        let file = NamedTempFile::new().unwrap();
        let config = load_from(file.path());
        assert!(config.projects.is_empty());
    }

    #[test]
    fn load_from_invalid_toml_returns_empty_config() {
        use std::io::Write;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "this is not valid toml !!!").unwrap();
        let config = load_from(file.path());
        assert!(config.projects.is_empty());
    }

    #[test]
    fn load_from_parses_project_names() {
        use std::io::Write;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "[[projects]]\nname = \"amnesia\"\n\n[[projects]]\nname = \"my-app\""
        )
        .unwrap();
        let config = load_from(file.path());
        assert_eq!(config.projects.len(), 2);
        assert_eq!(config.projects[0].name, "amnesia");
        assert_eq!(config.projects[1].name, "my-app");
    }

    // --- save_to / round-trip ------------------------------------------------

    #[test]
    fn save_to_then_load_from_round_trips() {
        let file = NamedTempFile::new().unwrap();
        let original = two_projects();

        save_to(file.path(), &original).unwrap();
        let loaded = load_from(file.path());

        assert_eq!(loaded, original);
    }

    #[test]
    fn save_to_overwrites_previous_content() {
        let file = NamedTempFile::new().unwrap();

        save_to(file.path(), &two_projects()).unwrap();

        let single = ProjectsConfig {
            projects: vec![Project { name: "only-one".to_string() }],
        };
        save_to(file.path(), &single).unwrap();

        let loaded = load_from(file.path());
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].name, "only-one");
    }

    #[test]
    fn save_to_creates_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("projects.toml");

        let config = ProjectsConfig {
            projects: vec![Project { name: "test".to_string() }],
        };
        save_to(&path, &config).unwrap();

        assert!(path.exists());
        let loaded = load_from(&path);
        assert_eq!(loaded.projects[0].name, "test");
    }

    #[test]
    fn empty_config_round_trips() {
        let file = NamedTempFile::new().unwrap();
        let empty = ProjectsConfig::default();

        save_to(file.path(), &empty).unwrap();
        let loaded = load_from(file.path());

        assert_eq!(loaded, empty);
    }

    // --- ProjectsConfig / Project --------------------------------------------

    #[test]
    fn project_name_preserved_through_serde() {
        let p = Project { name: "my-project".to_string() };
        let toml_str = toml::to_string(&ProjectsConfig { projects: vec![p] }).unwrap();
        let restored: ProjectsConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(restored.projects[0].name, "my-project");
    }

    #[test]
    fn projects_config_default_has_no_projects() {
        let config = ProjectsConfig::default();
        assert!(config.projects.is_empty());
    }
}
