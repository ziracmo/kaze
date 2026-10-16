//! Disk usage of a folder, as `du` counts it.

use std::collections::HashSet;
use std::path::Path;

/// Bytes actually allocated on disk under `path` (sparse files and clones count
/// for what they use, hard links once). Symlinks are not followed; unreadable
/// entries are skipped.
pub fn disk_usage(path: &Path) -> u64 {
    let mut seen = HashSet::new();
    usage(path, &mut seen)
}

fn usage(path: &Path, seen: &mut HashSet<(u64, u64)>) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    let mut total = allocated(&meta, seen);
    if meta.is_dir()
        && let Ok(entries) = std::fs::read_dir(path)
    {
        for entry in entries.flatten() {
            total += usage(&entry.path(), seen);
        }
    }
    total
}

#[cfg(unix)]
fn allocated(meta: &std::fs::Metadata, seen: &mut HashSet<(u64, u64)>) -> u64 {
    use std::os::unix::fs::MetadataExt as _;
    if meta.nlink() > 1 && !meta.is_dir() && !seen.insert((meta.dev(), meta.ino())) {
        return 0;
    }
    meta.blocks() * 512
}

#[cfg(not(unix))]
fn allocated(meta: &std::fs::Metadata, _: &mut HashSet<(u64, u64)>) -> u64 {
    meta.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_files_once_and_ignores_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("d");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("big"), vec![1u8; 200_000]).unwrap();
        std::fs::write(dir.join("sub/small"), vec![1u8; 10_000]).unwrap();

        let total = disk_usage(&dir);
        assert!(total >= 210_000, "{total}");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&dir, dir.join("loop")).unwrap();
            std::fs::hard_link(dir.join("big"), dir.join("big2")).unwrap();
            let after = disk_usage(&dir);
            assert!(
                after - total < 200_000,
                "hard link counted once: {total} → {after}"
            );
        }
        assert_eq!(disk_usage(&tmp.path().join("missing")), 0);
    }
}
