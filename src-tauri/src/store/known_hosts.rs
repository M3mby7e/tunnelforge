use std::fs;
use std::path::Path;

use crate::error::Result;

/// Remove any pinned host key for `host:port` from the known_hosts file, so the
/// next connection re-learns it (trust-on-first-use). Used to recover from a
/// legitimate server key change. No-op if the file or entry is absent.
pub fn forget_host(path: &Path, host: &str, port: u16) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let host_port = if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    };

    let content = fs::read_to_string(path)?;
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| {
            let first = line.split_whitespace().next().unwrap_or("");
            // A line may list several comma-separated hostnames.
            !first.split(',').any(|entry| entry == host_port)
        })
        .collect();

    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn removes_matching_host_keeps_others() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("known_hosts");
        fs::write(
            &path,
            "example.com ssh-ed25519 AAAAkey1\n[db.internal]:2222 ssh-ed25519 AAAAkey2\nother.host ssh-ed25519 AAAAkey3\n",
        )
        .unwrap();

        forget_host(&path, "example.com", 22).unwrap();
        forget_host(&path, "db.internal", 2222).unwrap();

        let remaining = fs::read_to_string(&path).unwrap();
        assert!(!remaining.contains("example.com"));
        assert!(!remaining.contains("db.internal"));
        assert!(remaining.contains("other.host"));
    }

    #[test]
    fn missing_file_is_ok() {
        let dir = tempdir().unwrap();
        forget_host(&dir.path().join("nope"), "example.com", 22).unwrap();
    }
}
