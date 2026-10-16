//! Finding the Git repositories under the user's project folders.

use std::path::{Path, PathBuf};

/// Folders never worth descending into: huge, and never hold a project.
const SKIP: [&str; 8] = [
    "node_modules",
    "target",
    ".build",
    "build",
    "dist",
    "Pods",
    "DerivedData",
    "vendor",
];

/// Main repositories (a `.git` directory) under `roots`, up to `max_depth`
/// levels down. Linked worktrees (a `.git` file) are skipped: they are found
/// through their main repository. Nested repositories are found too.
pub fn find_repos(roots: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in roots {
        walk(root, max_depth, &mut found);
    }
    found.sort();
    found.dedup();
    found
}

fn walk(dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if dir.join(".git").is_dir() {
        found.push(dir.to_path_buf());
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || SKIP.contains(&name.as_ref()) {
            continue;
        }
        // file_type() does not follow symlinks: no loops, no double counting.
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            walk(&entry.path(), depth - 1, found);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_main_repos_and_skips_noise() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        for dir in [
            "a/.git",
            "group/b/.git",
            "group/b/nested/.git",
            "c/node_modules/pkg/.git",
            ".hidden/d/.git",
            "too/deep/down/here/e/.git",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        // A linked worktree: `.git` is a file.
        std::fs::create_dir_all(root.join("wt")).unwrap();
        std::fs::write(root.join("wt/.git"), "gitdir: /elsewhere").unwrap();

        let repos = find_repos(&[root.to_path_buf()], 3);
        let rel: Vec<_> = repos
            .iter()
            .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(rel, ["a", "group/b", "group/b/nested"]);
    }
}
