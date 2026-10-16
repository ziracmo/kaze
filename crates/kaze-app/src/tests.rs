//! UI tests: the real view in a headless window.
#![allow(clippy::disallowed_methods)] // test fixtures

use std::path::{Path, PathBuf};
use std::process::Command;

use gpui_kit::{AppContext as _, TestAppContext};

use crate::home::{Home, Scan};
use crate::settings::Settings;

fn open(cx: &mut TestAppContext) -> gpui_kit::WindowHandle<Home> {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::apply(cx);
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|_| Home::new(Settings::default(), false))
        })
        .unwrap()
    })
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
}

#[gpui_kit::test]
fn adds_and_removes_project_folders(cx: &mut TestAppContext) {
    let window = open(cx);
    window
        .update(cx, |home, _, cx| {
            home.add_folders(vec![PathBuf::from("/work"), PathBuf::from("/oss")], cx);
            home.add_folders(vec![PathBuf::from("/work")], cx);
            assert_eq!(home.folders().len(), 2);
            home.remove_folder(0, cx);
            home.remove_folder(9, cx);
            assert_eq!(home.folders(), [PathBuf::from("/oss")]);
            home.remove_folder(0, cx);
            assert!(matches!(home.scan, Scan::Idle), "nothing to scan");
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn adding_a_folder_scans_it(cx: &mut TestAppContext) {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path().join("projects/app");
    std::fs::create_dir_all(&app).unwrap();
    git(&app, &["init", "-q", "-b", "main"]);
    git(&app, &["commit", "-q", "--allow-empty", "-m", "c1"]);
    let wt = tmp.path().join("wt");
    git(
        &app,
        &["worktree", "add", "-q", "-b", "done", wt.to_str().unwrap()],
    );
    git(&wt, &["commit", "-q", "--allow-empty", "-m", "work"]);
    git(&app, &["merge", "-q", "--no-ff", "done", "-m", "merge"]);

    let window = open(cx);
    window
        .update(cx, |home, _, cx| {
            home.add_folders(vec![tmp.path().join("projects")], cx);
            assert!(matches!(home.scan, Scan::Running));
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(cx, |home, _, _| {
            let Scan::Done(reports) = &home.scan else {
                panic!("scan not finished");
            };
            assert_eq!(reports.len(), 1);
            assert_eq!(reports[0].branch.as_deref(), Some("done"));
            assert!(reports[0].is_dead());
        })
        .unwrap();
}
