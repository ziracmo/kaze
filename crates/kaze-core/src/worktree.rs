//! What git knows about a repository's worktrees.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, bail};

/// Runs git in `dir` and returns its stdout. Never takes optional locks, so a
/// scan doesn't rewrite indexes or race with the user's own git commands.
pub fn git(dir: &Path, args: &[&str]) -> anyhow::Result<String> {
    let out = crate::process::command("git")
        .current_dir(dir)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .args(args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// One entry of `git worktree list`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Worktree {
    pub path: PathBuf,
    pub head: String,
    /// Short branch name; `None` when detached.
    pub branch: Option<String>,
    pub bare: bool,
    pub locked: bool,
    /// Its folder is gone (`git worktree prune` would drop it).
    pub prunable: bool,
}

/// Parses `git worktree list --porcelain -z`. The first entry is the main worktree.
pub fn parse_list(raw: &str) -> Vec<Worktree> {
    let mut list = Vec::new();
    let mut current: Option<Worktree> = None;
    for field in raw.split('\0') {
        if field.is_empty() {
            list.extend(current.take());
            continue;
        }
        let (key, value) = field.split_once(' ').unwrap_or((field, ""));
        if key == "worktree" {
            list.extend(current.take());
            current = Some(Worktree {
                path: PathBuf::from(value),
                ..Default::default()
            });
            continue;
        }
        let Some(wt) = current.as_mut() else { continue };
        match key {
            "HEAD" => wt.head = value.to_string(),
            "branch" => {
                wt.branch = Some(
                    value
                        .strip_prefix("refs/heads/")
                        .unwrap_or(value)
                        .to_string(),
                )
            }
            "bare" => wt.bare = true,
            "locked" => wt.locked = true,
            "prunable" => wt.prunable = true,
            _ => {}
        }
    }
    list.extend(current);
    list
}

pub fn list(repo: &Path) -> anyhow::Result<Vec<Worktree>> {
    Ok(parse_list(&git(
        repo,
        &["worktree", "list", "--porcelain", "-z"],
    )?))
}

/// The branches work gets merged into: `origin/HEAD` when known, plus the local
/// `main`, `master`, `trunk` or `develop` (merges can land on either side first).
pub fn base_branches(repo: &Path) -> Vec<String> {
    let mut bases = Vec::new();
    if let Ok(out) = git(
        repo,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    ) {
        let name = out.trim();
        if !name.is_empty() {
            bases.push(name.to_string());
        }
    }
    bases.extend(
        ["main", "master", "trunk", "develop"]
            .into_iter()
            .filter(|b| {
                git(
                    repo,
                    &[
                        "rev-parse",
                        "--verify",
                        "--quiet",
                        &format!("refs/heads/{b}"),
                    ],
                )
                .is_ok()
            })
            .map(str::to_string),
    );
    bases
}

/// Local branches whose upstream was deleted on the remote (`[gone]`), the
/// usual sign of a merged-and-deleted pull request (squash merges included).
pub fn gone_branches(repo: &Path) -> Vec<String> {
    let Ok(out) = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)%00%(upstream:track)",
            "refs/heads",
        ],
    ) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|l| l.split_once('\0'))
        .filter(|(_, track)| *track == "[gone]")
        .map(|(name, _)| name.to_string())
        .collect()
}

/// `rev` is fully contained in `base` and was actually worked on: a branch just
/// created from an older `base` (one reflog entry, "Created from") doesn't count.
pub fn is_merged(repo: &Path, rev: &str, base: &str, branch: Option<&str>) -> bool {
    let (Ok(tip), Ok(base_tip)) = (rev_parse(repo, rev), rev_parse(repo, base)) else {
        return false;
    };
    if tip == base_tip || git(repo, &["merge-base", "--is-ancestor", &tip, &base_tip]).is_err() {
        return false;
    }
    match branch {
        Some(b) => {
            let log = git(
                repo,
                &["reflog", "show", "--format=%gs", &format!("refs/heads/{b}")],
            )
            .unwrap_or_default();
            let entries: Vec<_> = log.lines().collect();
            !(entries.len() == 1 && entries[0].starts_with("branch: Created from"))
        }
        None => true,
    }
}

fn rev_parse(repo: &Path, rev: &str) -> anyhow::Result<String> {
    Ok(git(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ],
    )?
    .trim()
    .to_string())
}

/// Commits on `rev` that neither a remote branch nor a base branch has (lost if the
/// branch is deleted).
pub fn unpushed(repo: &Path, rev: &str, bases: &[String]) -> u32 {
    let mut args = vec!["rev-list", "--count", rev, "--not", "--remotes"];
    args.extend(bases.iter().map(String::as_str));
    git(repo, &args)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Uncommitted changes or untracked (not ignored) files in the worktree.
pub fn is_dirty(worktree: &Path) -> bool {
    git(
        worktree,
        &["status", "--porcelain", "--untracked-files=normal"],
    )
    .map(|s| !s.trim().is_empty())
    .unwrap_or(false)
}

/// Last time someone worked there: the newest of the HEAD commit and the
/// index (touched by checkouts, adds, commits).
pub fn last_activity(worktree: &Path) -> Option<SystemTime> {
    let commit = git(worktree, &["log", "-1", "--format=%ct", "HEAD"])
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs));
    let index = git(worktree, &["rev-parse", "--absolute-git-dir"])
        .ok()
        .and_then(|dir| std::fs::metadata(Path::new(dir.trim()).join("index")).ok())
        .and_then(|m| m.modified().ok());
    commit.max(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_list() {
        let raw = "worktree /repo\0HEAD aaa\0branch refs/heads/main\0\0\
                   worktree /repo-wt\0HEAD bbb\0detached\0locked reason\0\0\
                   worktree /gone\0HEAD ccc\0branch refs/heads/feat/x\0prunable gitdir file points to non-existent location\0\0";
        let list = parse_list(raw);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].branch.as_deref(), Some("main"));
        assert_eq!(list[1].branch, None);
        assert!(list[1].locked && !list[1].prunable);
        assert_eq!(list[2].path, PathBuf::from("/gone"));
        assert_eq!(list[2].branch.as_deref(), Some("feat/x"));
        assert!(list[2].prunable);
    }
}
