//! The main window: the folders Kaze watches, and (soon) what it found in them.

use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::assets::IconName as Lucide;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, TitleBar, h_flex, v_flex};
use gpui_kit::{
    Animation, AnimationExt as _, Context, InteractiveElement as _, IntoElement,
    ParentElement as _, PathPromptOptions, Render, StatefulInteractiveElement as _, Styled as _,
    Window, div, prelude::FluentBuilder as _, px,
};

use crate::i18n::tr;
use crate::settings::Settings;
use crate::theme;

/// Height of the title bar; the traffic lights are centred in it.
pub const TITLE_HEIGHT: f32 = 44.;

pub struct Home {
    settings: Settings,
    /// False in tests, so they never touch the real config file.
    persist: bool,
}

impl Home {
    pub fn new(settings: Settings, persist: bool) -> Self {
        Self { settings, persist }
    }

    pub fn folders(&self) -> &[PathBuf] {
        &self.settings.project_folders
    }

    pub fn add_folders(&mut self, folders: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.settings.add_folders(folders);
        self.save();
        cx.notify();
    }

    pub fn remove_folder(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.settings.project_folders.len() {
            self.settings.project_folders.remove(index);
            self.save();
            cx.notify();
        }
    }

    fn save(&self) {
        if self.persist {
            self.settings.save();
        }
    }

    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some(tr("Ajouter", "Add").into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await {
                let _ = this.update(cx, |this, cx| this.add_folders(paths, cx));
            }
        })
        .detach();
    }

    fn render_folders(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.folders().iter().enumerate().map(|(i, path)| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            h_flex()
                .id(("folder", i))
                .w_full()
                .h(px(40.))
                .px_3()
                .gap_3()
                .rounded_lg()
                .hover(|s| s.bg(theme::raised()))
                .child(
                    div()
                        .size(px(26.))
                        .flex_none()
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(theme::alpha(gpui_kit::white(), 0.05))
                        .child(
                            Icon::new(Lucide::Folder)
                                .small()
                                .text_color(theme::accent()),
                        ),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(div().text_sm().truncate().child(name))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::muted())
                                .truncate()
                                .child(path.display().to_string()),
                        ),
                )
                .child(
                    div()
                        .id(("remove-folder", i))
                        .test_support()
                        .p_1()
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme::raised()))
                        .child(Icon::new(Lucide::X).xsmall().text_color(theme::muted()))
                        .on_click(cx.listener(move |this, _, _, cx| this.remove_folder(i, cx))),
                )
        });
        v_flex()
            .id("folders")
            .test_support()
            .w(px(480.))
            .p_2()
            .rounded_xl()
            .border_1()
            .border_color(theme::hairline())
            .bg(theme::panel())
            .child(
                div()
                    .px_3()
                    .py_1()
                    .text_xs()
                    .text_color(theme::muted())
                    .child(tr("Dossiers projets", "Project folders")),
            )
            .children(rows)
    }
}

impl Render for Home {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_folders = !self.folders().is_empty();
        let hero = v_flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_3xl()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .text_color(gpui_kit::white())
                    .child("風 Kaze"),
            )
            .child(div().text_sm().text_color(theme::muted()).child(tr(
                "Libère l'espace pris par tes worktrees oubliés.",
                "Free the space taken by forgotten worktrees.",
            )));
        let add = h_flex()
            .id("add-folders")
            .test_support()
            .gap_2()
            .px_4()
            .h(px(36.))
            .rounded_lg()
            .cursor_pointer()
            .bg(theme::accent())
            .text_color(gpui_kit::black())
            .text_sm()
            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
            .hover(|s| s.bg(theme::alpha(theme::accent(), 0.85)))
            .child(Icon::new(Lucide::FolderPlus).small())
            .child(if has_folders {
                tr("Ajouter un dossier…", "Add a folder…")
            } else {
                tr(
                    "Choisir mes dossiers projets…",
                    "Choose my project folders…",
                )
            })
            .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx)));
        let body = v_flex()
            .id("home")
            .test_support()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_6()
            .child(hero)
            .child(add)
            .when(has_folders, |s| s.child(self.render_folders(cx)))
            .with_animation(
                "home-in",
                Animation::new(Duration::from_millis(450)),
                |el, t| {
                    let k = 1. - (1. - t).powi(3);
                    el.opacity(k)
                },
            );
        v_flex()
            .size_full()
            .text_color(cx.theme().foreground)
            .child(
                TitleBar::new().h(px(TITLE_HEIGHT)).child(
                    h_flex()
                        .size_full()
                        .px_3()
                        .items_center()
                        .text_sm()
                        .text_color(theme::muted())
                        .child("Kaze"),
                ),
            )
            .child(body)
    }
}
