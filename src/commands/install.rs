use std::fs;
use std::path::{Path, PathBuf};

const CLAUDE_SKILL_TEMPLATE: &str = include_str!("../templates/claude_skill.md");
const OPENCODE_SKILL_TEMPLATE: &str = include_str!("../templates/opencode_skill.md");
const CLAUDE_MD_SNIPPET: &str = include_str!("../templates/claude_md_snippet.md");
const AGENTS_MD_SNIPPET: &str = include_str!("../templates/agents_md_snippet.md");

const BEGIN_MARKER: &str = "[//]: # (BEGIN:amnesia)";
const END_MARKER: &str = "[//]: # (END:amnesia)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// Install if missing, skip if exists
    Install,
    /// Report whether files are up to date
    Check,
    /// Overwrite existing installations
    Update,
}

struct Target {
    label: &'static str,
    path: PathBuf,
    kind: TargetKind,
}

enum TargetKind {
    /// Write the full file (e.g. SKILL.md)
    FullFile { content: &'static str },
    /// Inject a section between markers inside an existing file
    Inject { snippet: &'static str },
}

pub fn run(mode: Mode) -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
    let home = Path::new(&home);

    let targets = vec![
        Target {
            label: "Claude Code skill",
            path: home.join(".claude/skills/amnesia/SKILL.md"),
            kind: TargetKind::FullFile { content: CLAUDE_SKILL_TEMPLATE },
        },
        Target {
            label: "OpenCode skill",
            path: home.join(".config/opencode/skills/amnesia/SKILL.md"),
            kind: TargetKind::FullFile { content: OPENCODE_SKILL_TEMPLATE },
        },
        Target {
            label: "Claude Code CLAUDE.md",
            path: home.join(".claude/CLAUDE.md"),
            kind: TargetKind::Inject { snippet: CLAUDE_MD_SNIPPET },
        },
        Target {
            label: "OpenCode AGENTS.md",
            path: home.join(".config/opencode/AGENTS.md"),
            kind: TargetKind::Inject { snippet: AGENTS_MD_SNIPPET },
        },
    ];

    let mut all_ok = true;

    for target in &targets {
        match &target.kind {
            TargetKind::FullFile { content } => {
                handle_full_file(target.label, &target.path, content, mode, &mut all_ok)?;
            }
            TargetKind::Inject { snippet } => {
                handle_inject(target.label, &target.path, snippet, mode, &mut all_ok)?;
            }
        }
    }

    if mode == Mode::Check && !all_ok {
        std::process::exit(1);
    }

    Ok(())
}

fn handle_full_file(
    label: &str,
    path: &Path,
    content: &str,
    mode: Mode,
    all_ok: &mut bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        Mode::Install => {
            if path.exists() {
                let existing = fs::read_to_string(path)?;
                if existing == content {
                    println!("  {label}: up to date");
                } else {
                    println!("  {label}: exists (use --update to overwrite)");
                }
            } else {
                write_file(path, content)?;
                println!("  {label}: installed");
            }
        }
        Mode::Check => {
            if !path.exists() {
                println!("  {label}: missing");
                *all_ok = false;
            } else {
                let existing = fs::read_to_string(path)?;
                if existing == content {
                    println!("  {label}: up to date");
                } else {
                    println!("  {label}: outdated");
                    *all_ok = false;
                }
            }
        }
        Mode::Update => {
            if path.exists() {
                let existing = fs::read_to_string(path)?;
                if existing == content {
                    println!("  {label}: up to date");
                } else {
                    write_file(path, content)?;
                    println!("  {label}: updated");
                }
            } else {
                write_file(path, content)?;
                println!("  {label}: installed");
            }
        }
    }
    Ok(())
}

fn handle_inject(
    label: &str,
    path: &Path,
    snippet: &str,
    mode: Mode,
    all_ok: &mut bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let wrapped = wrap_with_markers(snippet);
    let existing = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    let current_section = extract_section(&existing);

    match mode {
        Mode::Install => {
            if let Some(section) = &current_section {
                if *section == wrapped {
                    println!("  {label}: up to date");
                } else {
                    println!("  {label}: exists (use --update to overwrite)");
                }
            } else {
                let new_content = append_section(&existing, &wrapped);
                write_file(path, &new_content)?;
                println!("  {label}: installed");
            }
        }
        Mode::Check => {
            if current_section.is_none() {
                println!("  {label}: missing");
                *all_ok = false;
            } else if current_section.as_deref() != Some(&*wrapped) {
                println!("  {label}: outdated");
                *all_ok = false;
            } else {
                println!("  {label}: up to date");
            }
        }
        Mode::Update => {
            if let Some(section) = &current_section {
                if *section == wrapped {
                    println!("  {label}: up to date");
                } else {
                    let new_content = replace_section(&existing, &wrapped);
                    write_file(path, &new_content)?;
                    println!("  {label}: updated");
                }
            } else {
                let new_content = append_section(&existing, &wrapped);
                write_file(path, &new_content)?;
                println!("  {label}: installed");
            }
        }
    }
    Ok(())
}

fn wrap_with_markers(snippet: &str) -> String {
    format!("{BEGIN_MARKER}\n{snippet}{END_MARKER}\n")
}

/// Extract the full marked section (including markers) from file content.
fn extract_section(content: &str) -> Option<String> {
    let start = content.find(BEGIN_MARKER)?;
    let end_marker_pos = content[start..].find(END_MARKER)?;
    let end = start + end_marker_pos + END_MARKER.len();
    // Include trailing newline if present
    let end = if content[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    Some(content[start..end].to_string())
}

/// Replace the existing marked section with new content.
fn replace_section(content: &str, new_section: &str) -> String {
    let start = content.find(BEGIN_MARKER).expect("BEGIN marker not found");
    let end_marker_pos = content[start..].find(END_MARKER).expect("END marker not found");
    let end = start + end_marker_pos + END_MARKER.len();
    let end = if content[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };

    let mut result = String::with_capacity(content.len());
    result.push_str(&content[..start]);
    result.push_str(new_section);
    result.push_str(&content[end..]);
    result
}

/// Append a new marked section to the end of the file content.
fn append_section(content: &str, section: &str) -> String {
    let mut result = content.to_string();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    if !result.is_empty() && !result.ends_with("\n\n") {
        result.push('\n');
    }
    result.push_str(section);
    result
}

fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn wrap_with_markers_produces_correct_output() {
        let snippet = "hello\nworld\n";
        let wrapped = wrap_with_markers(snippet);
        assert!(wrapped.starts_with(BEGIN_MARKER));
        assert!(wrapped.ends_with(&format!("{END_MARKER}\n")));
        assert!(wrapped.contains("hello\nworld\n"));
    }

    #[test]
    fn extract_section_finds_marked_block() {
        let content = format!(
            "# Header\n\n{BEGIN_MARKER}\nsome content\n{END_MARKER}\n\n# Footer\n"
        );
        let section = extract_section(&content).unwrap();
        assert!(section.starts_with(BEGIN_MARKER));
        assert!(section.contains("some content"));
        assert!(section.contains(END_MARKER));
    }

    #[test]
    fn extract_section_returns_none_when_missing() {
        let content = "# Header\nno markers here\n";
        assert!(extract_section(content).is_none());
    }

    #[test]
    fn replace_section_swaps_content() {
        let old = format!(
            "# Header\n\n{BEGIN_MARKER}\nold content\n{END_MARKER}\n\n# Footer\n"
        );
        let new_section = wrap_with_markers("new content\n");
        let result = replace_section(&old, &new_section);
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        assert!(result.contains("# Header"));
        assert!(result.contains("# Footer"));
    }

    #[test]
    fn append_section_adds_to_empty_file() {
        let section = wrap_with_markers("content\n");
        let result = append_section("", &section);
        assert_eq!(result, section);
    }

    #[test]
    fn append_section_adds_with_spacing() {
        let existing = "# Existing content\n";
        let section = wrap_with_markers("new\n");
        let result = append_section(existing, &section);
        assert!(result.starts_with("# Existing content\n\n"));
        assert!(result.contains(BEGIN_MARKER));
    }

    #[test]
    fn full_file_install_creates_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("skills/amnesia/SKILL.md");
        let mut ok = true;
        handle_full_file("test", &path, "content here", Mode::Install, &mut ok).unwrap();
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "content here");
    }

    #[test]
    fn full_file_install_skips_when_exists_different() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, "old content").unwrap();
        let mut ok = true;
        handle_full_file("test", &path, "new content", Mode::Install, &mut ok).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "old content");
    }

    #[test]
    fn full_file_update_overwrites() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, "old content").unwrap();
        let mut ok = true;
        handle_full_file("test", &path, "new content", Mode::Update, &mut ok).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
    }

    #[test]
    fn full_file_update_skips_when_identical() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, "same content").unwrap();
        let mut ok = true;
        handle_full_file("test", &path, "same content", Mode::Update, &mut ok).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "same content");
    }

    #[test]
    fn full_file_check_reports_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.md");
        let mut ok = true;
        handle_full_file("test", &path, "content", Mode::Check, &mut ok).unwrap();
        assert!(!ok);
    }

    #[test]
    fn full_file_check_reports_outdated() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, "old").unwrap();
        let mut ok = true;
        handle_full_file("test", &path, "new", Mode::Check, &mut ok).unwrap();
        assert!(!ok);
    }

    #[test]
    fn full_file_check_reports_up_to_date() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("SKILL.md");
        fs::write(&path, "same").unwrap();
        let mut ok = true;
        handle_full_file("test", &path, "same", Mode::Check, &mut ok).unwrap();
        assert!(ok);
    }

    #[test]
    fn inject_install_into_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let mut ok = true;
        handle_inject("test", &path, "snippet\n", Mode::Install, &mut ok).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains(BEGIN_MARKER));
        assert!(content.contains("snippet"));
        assert!(content.contains(END_MARKER));
    }

    #[test]
    fn inject_install_appends_to_existing_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "# My Rules\n\nDo stuff.\n").unwrap();
        let mut ok = true;
        handle_inject("test", &path, "snippet\n", Mode::Install, &mut ok).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# My Rules"));
        assert!(content.contains(BEGIN_MARKER));
        assert!(content.contains("snippet"));
    }

    #[test]
    fn inject_install_skips_when_markers_exist_different() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let existing = format!("{BEGIN_MARKER}\nold snippet\n{END_MARKER}\n");
        fs::write(&path, &existing).unwrap();
        let mut ok = true;
        handle_inject("test", &path, "new snippet\n", Mode::Install, &mut ok).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("old snippet"));
        assert!(!content.contains("new snippet"));
    }

    #[test]
    fn inject_update_replaces_existing_section() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let existing = format!(
            "# Header\n\n{BEGIN_MARKER}\nold snippet\n{END_MARKER}\n\n# Footer\n"
        );
        fs::write(&path, &existing).unwrap();
        let mut ok = true;
        handle_inject("test", &path, "new snippet\n", Mode::Update, &mut ok).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("new snippet"));
        assert!(!content.contains("old snippet"));
        assert!(content.contains("# Header"));
        assert!(content.contains("# Footer"));
    }

    #[test]
    fn inject_update_installs_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "# Existing\n").unwrap();
        let mut ok = true;
        handle_inject("test", &path, "snippet\n", Mode::Update, &mut ok).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Existing"));
        assert!(content.contains(BEGIN_MARKER));
        assert!(content.contains("snippet"));
    }

    #[test]
    fn inject_check_reports_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "# No markers\n").unwrap();
        let mut ok = true;
        handle_inject("test", &path, "snippet\n", Mode::Check, &mut ok).unwrap();
        assert!(!ok);
    }

    #[test]
    fn inject_check_reports_outdated() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let existing = format!("{BEGIN_MARKER}\nold\n{END_MARKER}\n");
        fs::write(&path, &existing).unwrap();
        let mut ok = true;
        handle_inject("test", &path, "new\n", Mode::Check, &mut ok).unwrap();
        assert!(!ok);
    }

    #[test]
    fn inject_check_reports_up_to_date() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let snippet = "snippet\n";
        let wrapped = wrap_with_markers(snippet);
        fs::write(&path, &wrapped).unwrap();
        let mut ok = true;
        handle_inject("test", &path, snippet, Mode::Check, &mut ok).unwrap();
        assert!(ok);
    }

    #[test]
    fn inject_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let mut ok = true;

        // Install twice
        handle_inject("test", &path, "snippet\n", Mode::Install, &mut ok).unwrap();
        let first = fs::read_to_string(&path).unwrap();
        handle_inject("test", &path, "snippet\n", Mode::Update, &mut ok).unwrap();
        let second = fs::read_to_string(&path).unwrap();

        assert_eq!(first, second);
    }
}
