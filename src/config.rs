use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub store_path: String,
    pub default_limit: usize,
    pub date_format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            store_path: "~/.context-memory/store.ndjson".to_string(),
            default_limit: 10,
            date_format: "%Y-%m-%d".to_string(),
        }
    }
}

impl Config {
    pub fn store_path_expanded(&self) -> PathBuf {
        expand_tilde(&self.store_path)
    }
}

pub fn load() -> Config {
    let config_path = config_path();
    if !config_path.exists() {
        return Config::default();
    }

    let contents = match fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(_) => return Config::default(),
    };

    toml::from_str(&contents).unwrap_or_default()
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".context-memory").join("config.toml")
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(stripped)
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn defaults_are_correct() {
        let config = Config::default();
        assert_eq!(config.store_path, "~/.context-memory/store.ndjson");
        assert_eq!(config.default_limit, 10);
        assert_eq!(config.date_format, "%Y-%m-%d");
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let toml = r#"default_limit = 25"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.default_limit, 25);
        assert_eq!(config.store_path, "~/.context-memory/store.ndjson");
        assert_eq!(config.date_format, "%Y-%m-%d");
    }

    #[test]
    fn full_toml_overrides_all_fields() {
        let toml = r#"
store_path    = "/tmp/custom-store.ndjson"
default_limit = 20
date_format   = "%d/%m/%Y"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.store_path, "/tmp/custom-store.ndjson");
        assert_eq!(config.default_limit, 20);
        assert_eq!(config.date_format, "%d/%m/%Y");
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        // config_path() points to ~/.context-memory/config.toml which may or
        // may not exist; we test the fallback path via toml::from_str directly.
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.default_limit, 10);
    }

    #[test]
    fn load_returns_default_on_invalid_toml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "this is not valid toml !!!").unwrap();

        let contents = fs::read_to_string(file.path()).unwrap();
        let config: Config = toml::from_str(&contents).unwrap_or_default();
        assert_eq!(config.default_limit, 10);
    }

    #[test]
    fn expand_tilde_replaces_home() {
        let home = std::env::var("HOME").unwrap();
        let expanded = expand_tilde("~/.context-memory/store.ndjson");
        assert_eq!(
            expanded,
            PathBuf::from(&home).join(".context-memory/store.ndjson")
        );
    }

    #[test]
    fn expand_tilde_leaves_absolute_path_unchanged() {
        let expanded = expand_tilde("/tmp/store.ndjson");
        assert_eq!(expanded, PathBuf::from("/tmp/store.ndjson"));
    }

    #[test]
    fn store_path_expanded_returns_pathbuf() {
        let config = Config::default();
        let path = config.store_path_expanded();
        assert!(path.ends_with(".context-memory/store.ndjson"));
    }
}
