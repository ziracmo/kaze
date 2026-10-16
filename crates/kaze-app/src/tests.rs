//! UI tests: the real view in a headless window.

use std::path::PathBuf;

use gpui_kit::{AppContext as _, TestAppContext};

use crate::home::Home;
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
        })
        .unwrap();
    cx.run_until_parked();
}
