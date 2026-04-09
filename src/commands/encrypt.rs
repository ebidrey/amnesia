use std::fs::{self, OpenOptions};
use std::io::{Read as _, Write};
use std::path::Path;

use age::secrecy::ExposeSecret;
use age::x25519;
use base64::{Engine as _, engine::general_purpose::STANDARD};

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

/// Parse the x25519 identity (secret key) from an age key file.
pub fn load_identity(identity_path: &Path) -> Result<x25519::Identity, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(identity_path)?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("AGE-SECRET-KEY-") {
            return trimmed
                .parse::<x25519::Identity>()
                .map_err(|e| e.into());
        }
    }
    Err("no AGE-SECRET-KEY found in identity file".into())
}

/// Encrypt a plaintext string using the given identity's public key. Returns base64.
pub fn encrypt_with(
    plaintext: &str,
    identity: &x25519::Identity,
) -> Result<String, Box<dyn std::error::Error>> {
    let recipient = identity.to_public();

    let encryptor = age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))?;

    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(plaintext.as_bytes())?;
    writer.finish()?;

    Ok(STANDARD.encode(&encrypted))
}

/// Decrypt a base64-encoded age ciphertext using the given identity. Returns the plaintext string.
pub fn decrypt_with(
    ciphertext_b64: &str,
    identity: &x25519::Identity,
) -> Result<String, Box<dyn std::error::Error>> {
    let ciphertext = STANDARD.decode(ciphertext_b64)?;

    let decryptor = age::Decryptor::new_buffered(&ciphertext[..])?;
    let mut reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;
    let mut plaintext = String::new();
    reader.read_to_string(&mut plaintext)?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encrypt_line(
        plaintext: &str,
        identity_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let identity = load_identity(identity_path)?;
        encrypt_with(plaintext, &identity)
    }

    fn decrypt_line(
        ciphertext_b64: &str,
        identity_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let identity = load_identity(identity_path)?;
        decrypt_with(ciphertext_b64, &identity)
    }
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
    fn load_identity_reads_generated_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.key");
        ensure_identity(&path).unwrap();

        let identity = load_identity(&path).unwrap();
        let _recipient = identity.to_public();
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.key");
        ensure_identity(&path).unwrap();

        let plaintext = r#"{"id":"01ABC","title":"test observation"}"#;
        let encrypted = encrypt_line(plaintext, &path).unwrap();

        assert!(!encrypted.contains("test observation"));
        assert!(!encrypted.contains("01ABC"));

        let decrypted = decrypt_line(&encrypted, &path).unwrap();
        assert_eq!(decrypted, plaintext);
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
