//! The scan against real repositories built with the git CLI.
#![allow(clippy::disallowed_methods)] // test fixtures

use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use kaze_core::{Options, Reason, scan};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn commit(dir: &Path, file: &str, msg: &str) {
    std::fs::write(dir.join(file), msg).unwrap();
    git(dir, &["add", file]);
    git(dir, &["commit", "-q", "-m", msg]);
}

/// projects/app (with an `origin` remote) and its worktrees:
/// - `merged`: branch merged into main
/// - `gone`: pushed, then deleted on the remote (squash-merged PR)
/// - `wip`: unmerged commit, never pushed, plus uncommitted changes
/// - `fresh`: just created from an older main, never worked on
/// - `deleted`: folder removed by hand
fn fixture(root: &Path) -> std::path::PathBuf {
    let remote = root.join("remote.git");
    std::fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "-q", "--bare", "-b", "main"]);

    let projects = root.join("projects");
    let app = projects.join("app");
    std::fs::create_dir_all(&app).unwrap();
    git(&app, &["init", "-q", "-b", "main"]);
    commit(&app, "a.txt", "c1");
    std::fs::write(app.join(".gitignore"), "node_modules/\n").unwrap();
    git(&app, &["add", ".gitignore"]);
    git(&app, &["commit", "-q", "-m", "ignore deps"]);
    git(&app, &["remote", "add", "origin", remote.to_str().unwrap()]);
    git(&app, &["push", "-q", "-u", "origin", "main"]);
    git(&app, &["remote", "set-head", "origin", "main"]);

    let wt = |name: &str| root.join("wt").join(name);
    for name in ["merged", "gone", "wip", "deleted"] {
        git(
            &app,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                name,
                wt(name).to_str().unwrap(),
            ],
        );
    }
    commit(&wt("merged"), "m.txt", "merged work");
    git(&app, &["merge", "-q", "--no-ff", "merged", "-m", "merge"]);

    commit(&wt("gone"), "g.txt", "gone work");
    git(&wt("gone"), &["push", "-q", "-u", "origin", "gone"]);
    git(&app, &["push", "-q", "origin", "--delete", "gone"]);
    git(&app, &["fetch", "-q", "--prune"]);

    commit(&wt("wip"), "w.txt", "wip work");
    std::fs::write(wt("wip").join("draft.txt"), "draft").unwrap();

    commit(&app, "a2.txt", "main moves on");
    git(
        &app,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "fresh",
            wt("fresh").to_str().unwrap(),
            "HEAD~1",
        ],
    );

    // Some weight, as node_modules would bring.
    std::fs::create_dir_all(wt("merged").join("node_modules")).unwrap();
    std::fs::write(wt("merged").join("node_modules/blob"), vec![7u8; 300_000]).unwrap();

    std::fs::remove_dir_all(wt("deleted")).unwrap();
    projects
}

#[test]
fn flags_dead_worktrees_and_keeps_live_ones() {
    let tmp = tempfile::tempdir().unwrap();
    let projects = fixture(tmp.path());
    let reports = scan(&[projects], &Options::default());
    let by_branch = |b: &str| {
        reports
            .iter()
            .find(|r| r.branch.as_deref() == Some(b))
            .unwrap_or_else(|| panic!("no report for {b}: {reports:#?}"))
    };
    assert_eq!(reports.len(), 5, "main worktree not listed");

    let merged = by_branch("merged");
    assert_eq!(merged.reasons, [Reason::Merged]);
    assert!(merged.size >= 300_000);
    assert!(merged.is_safe_to_remove());
    assert_eq!(
        reports[0].branch.as_deref(),
        Some("merged"),
        "largest dead first"
    );

    let gone = by_branch("gone");
    assert_eq!(gone.reasons, [Reason::UpstreamGone]);
    assert_eq!(
        gone.unpushed, 1,
        "its commit only lived on the deleted remote branch"
    );
    assert!(!gone.is_safe_to_remove());

    let wip = by_branch("wip");
    assert!(wip.reasons.is_empty(), "{:?}", wip.reasons);
    assert!(wip.dirty);
    assert_eq!(wip.unpushed, 1);

    let fresh = by_branch("fresh");
    assert!(
        fresh.reasons.is_empty(),
        "never worked on ≠ merged: {:?}",
        fresh.reasons
    );

    let deleted = by_branch("deleted");
    assert_eq!(deleted.reasons, [Reason::Missing]);
    assert_eq!(deleted.size, 0);
    assert!(deleted.is_safe_to_remove());
}

#[test]
fn flags_worktrees_nobody_touched_for_a_while() {
    let tmp = tempfile::tempdir().unwrap();
    let projects = fixture(tmp.path());
    let options = Options {
        now: SystemTime::now() + Duration::from_secs(45 * 86_400),
        ..Options::default()
    };
    let reports = scan(&[projects], &options);
    let wip = reports
        .iter()
        .find(|r| r.branch.as_deref() == Some("wip"))
        .unwrap();
    assert!(matches!(wip.reasons[..], [Reason::Inactive(d)] if (44..=46).contains(&d)));
    assert!(!wip.is_safe_to_remove(), "still has uncommitted work");
}
