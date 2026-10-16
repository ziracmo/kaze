//! The scan: every linked worktree under the project folders, its size, and
//! why it looks dead.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use crate::{discover, size, worktree};

#[derive(Debug, Clone)]
pub struct Options {
    /// How deep to look for repositories under each project folder.
    pub max_depth: usize,
    /// Untouched for this long counts as dead.
    pub inactive_after: Duration,
    pub now: SystemTime,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_depth: 4,
            inactive_after: Duration::from_secs(30 * 24 * 3600),
            now: SystemTime::now(),
        }
    }
}

/// Why a worktree looks dead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Its folder no longer exists; git still lists it.
    Missing,
    /// Its branch is merged into the base branch.
    Merged,
    /// Its branch was deleted on the remote (usually a merged pull request).
    UpstreamGone,
    /// Nobody touched it for this many days.
    Inactive(u64),
}

#[derive(Debug, Clone)]
pub struct Report {
    /// The main repository.
    pub repo: PathBuf,
    pub path: PathBuf,
    pub branch: Option<String>,
    /// Bytes on disk (0 when the folder is missing).
    pub size: u64,
    pub reasons: Vec<Reason>,
    pub last_activity: Option<SystemTime>,
    /// Uncommitted changes: removing it loses work.
    pub dirty: bool,
    /// Commits no remote has: removing the branch loses them.
    pub unpushed: u32,
    /// Locked with `git worktree lock`: the user asked to keep it.
    pub locked: bool,
}

impl Report {
    pub fn is_dead(&self) -> bool {
        !self.reasons.is_empty()
    }

    /// Dead, and removing it loses nothing.
    pub fn is_safe_to_remove(&self) -> bool {
        self.is_dead() && !self.dirty && !self.locked && self.unpushed == 0
    }
}

/// Scans `roots` for linked worktrees, largest dead ones first. Repositories
/// are processed in parallel.
pub fn scan(roots: &[PathBuf], options: &Options) -> Vec<Report> {
    let repos = discover::find_repos(roots, options.max_depth);
    let queue = Mutex::new(repos.into_iter());
    let reports = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let Some(repo) = queue.lock().unwrap().next() else {
                        break;
                    };
                    let found = scan_repo(&repo, options);
                    reports.lock().unwrap().extend(found);
                }
            });
        }
    });
    let mut reports = reports.into_inner().unwrap();
    reports.sort_by(|a, b| {
        b.is_dead()
            .cmp(&a.is_dead())
            .then(b.size.cmp(&a.size))
            .then(a.path.cmp(&b.path))
    });
    reports
}

/// The linked worktrees of one repository (the main worktree is never listed).
pub fn scan_repo(repo: &std::path::Path, options: &Options) -> Vec<Report> {
    let Ok(list) = worktree::list(repo) else {
        return Vec::new();
    };
    let bases = worktree::base_branches(repo);
    let gone = worktree::gone_branches(repo);
    list.into_iter()
        .skip(1)
        .filter(|wt| !wt.bare)
        .map(|wt| {
            let mut reasons = Vec::new();
            let exists = !wt.prunable && wt.path.is_dir();
            if !exists {
                reasons.push(Reason::Missing);
            }
            let rev = wt.branch.clone().unwrap_or_else(|| wt.head.clone());
            if bases
                .iter()
                .any(|base| worktree::is_merged(repo, &rev, base, wt.branch.as_deref()))
            {
                reasons.push(Reason::Merged);
            }
            if wt.branch.as_ref().is_some_and(|b| gone.contains(b)) {
                reasons.push(Reason::UpstreamGone);
            }
            let last_activity = exists.then(|| worktree::last_activity(&wt.path)).flatten();
            if let Some(idle) = last_activity.and_then(|t| options.now.duration_since(t).ok())
                && idle >= options.inactive_after
            {
                reasons.push(Reason::Inactive(idle.as_secs() / 86_400));
            }
            Report {
                repo: repo.to_path_buf(),
                size: if exists {
                    size::disk_usage(&wt.path)
                } else {
                    0
                },
                dirty: exists && worktree::is_dirty(&wt.path),
                unpushed: worktree::unpushed(repo, &rev, &bases),
                path: wt.path,
                branch: wt.branch,
                reasons,
                last_activity,
                locked: wt.locked,
            }
        })
        .collect()
}
