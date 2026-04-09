use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::commands::encrypt;

/// Re-encrypt a store file in place: plaintext lines become encrypted, already-encrypted lines
/// are left as-is. Returns the number of lines that were migrated.
pub fn run(store_path: &Path, identity_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !store_path.exists() {
        println!("nothing to migrate (store does not exist)");
        return Ok(());
    }

    let identity = encrypt::load_identity(identity_path)?;

    let file = File::open(store_path)?;
    let reader = BufReader::new(file);
    let mut output_lines = Vec::new();
    let mut migrated = 0usize;

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        if line.starts_with('{') {
            // plaintext JSON — encrypt it
            let encrypted = encrypt::encrypt_with(&line, &identity)?;
            output_lines.push(encrypted);
            migrated += 1;
        } else {
            // already encrypted — keep as-is
            output_lines.push(line);
        }
    }

    if migrated == 0 {
        println!("nothing to migrate (all lines already encrypted)");
        return Ok(());
    }

    // Write atomically via temp file
    let tmp_path = store_path.with_extension("ndjson.tmp");
    {
        let mut tmp = File::create(&tmp_path)?;
        for line in &output_lines {
            writeln!(tmp, "{}", line)?;
        }
        tmp.flush()?;
    }
    fs::rename(&tmp_path, store_path)?;

    println!("migrated {} plaintext lines", migrated);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::encrypt::ensure_identity;
    use crate::model::{Observation, OpType};
    use crate::store;
    use tempfile::tempdir;

    fn sample(id: &str) -> Observation {
        Observation {
            id: id.to_string(),
            timestamp: "2026-03-07T14:23:01Z".to_string(),
            agent: "main".to_string(),
            op_type: OpType::Decision,
            title: "Test".to_string(),
            content: "Content".to_string(),
            files: vec![],
            tags: vec![],
            session_id: None,
        }
    }

    #[test]
    fn migrates_plaintext_lines() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("test.key");
        ensure_identity(&key_path).unwrap();

        let store_path = dir.path().join("store.ndjson");
        store::append_to(&store_path, &sample("01A")).unwrap();
        store::append_to(&store_path, &sample("01B")).unwrap();

        run(&store_path, &key_path).unwrap();

        // Verify no plaintext remains
        let raw = std::fs::read_to_string(&store_path).unwrap();
        for line in raw.lines() {
            assert!(!line.starts_with('{'), "found plaintext after migration");
        }

        // Verify data is still readable
        let loaded = store::load_encrypted(&store_path, &key_path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "01A");
        assert_eq!(loaded[1].id, "01B");
    }

    #[test]
    fn skips_already_encrypted_lines() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("test.key");
        ensure_identity(&key_path).unwrap();

        let store_path = dir.path().join("store.ndjson");
        store::append_encrypted(&store_path, &sample("01A"), &key_path).unwrap();

        run(&store_path, &key_path).unwrap();

        let loaded = store::load_encrypted(&store_path, &key_path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "01A");
    }

    #[test]
    fn handles_mixed_plaintext_and_encrypted() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("test.key");
        ensure_identity(&key_path).unwrap();

        let store_path = dir.path().join("store.ndjson");

        // Write one plaintext, one encrypted
        store::append_to(&store_path, &sample("01A")).unwrap();
        store::append_encrypted(&store_path, &sample("01B"), &key_path).unwrap();

        run(&store_path, &key_path).unwrap();

        let raw = std::fs::read_to_string(&store_path).unwrap();
        for line in raw.lines() {
            assert!(!line.starts_with('{'));
        }

        let loaded = store::load_encrypted(&store_path, &key_path).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn nonexistent_store_is_noop() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("test.key");
        ensure_identity(&key_path).unwrap();

        let store_path = dir.path().join("nonexistent.ndjson");
        assert!(run(&store_path, &key_path).is_ok());
    }

    #[test]
    fn all_encrypted_is_noop() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("test.key");
        ensure_identity(&key_path).unwrap();

        let store_path = dir.path().join("store.ndjson");
        store::append_encrypted(&store_path, &sample("01A"), &key_path).unwrap();

        run(&store_path, &key_path).unwrap();

        // File should still exist and be readable
        let loaded = store::load_encrypted(&store_path, &key_path).unwrap();
        assert_eq!(loaded.len(), 1);
    }
}
