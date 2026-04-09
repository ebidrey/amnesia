use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use age::secrecy::ExposeSecret;
use age::x25519;

/// Ensure the age identity exists at the given path, creating it silently if missing.
pub fn ensure_identity(identity_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if identity_path.exists() {
        return Ok(());
    }

    if let Some(parent) = identity_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let identity = x25519::Identity::generate();
    let recipient = identity.to_public().to_string();
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(identity_path)?;
    writeln!(file, "# public key: {recipient}")?;
    writeln!(file, "{}", identity.to_string().expose_secret())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(identity_path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn ensure_identity_creates_key_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("keys").join("amnesia.key");

        ensure_identity(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# public key:"));
        assert!(content.contains("AGE-SECRET-KEY-"));
    }

    #[test]
    fn ensure_identity_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("keys").join("amnesia.key");

        ensure_identity(&path).unwrap();
        let first = fs::read_to_string(&path).unwrap();

        ensure_identity(&path).unwrap();
        let second = fs::read_to_string(&path).unwrap();

        assert_eq!(first, second);
    }
}
