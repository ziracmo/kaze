//! The main window: the project folders Kaze watches and the worktrees it found
//! in them, dead ones first.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use gpui_kit::assets::IconName as Lucide;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, TitleBar, h_flex, v_flex};
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, AppContext as _, Context, Hsla,
    InteractiveElement as _, IntoElement, ParentElement as _, PathPromptOptions, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use kaze_core::{Options, Reason, Report, fmt};

use crate::i18n::tr;
use crate::settings::Settings;
use crate::theme;

/// Height of the title bar; the traffic lights are centred in it.
pub const TITLE_HEIGHT: f32 = 44.;

pub enum Scan {
    Idle,
    Running,
    Done(Vec<Report>),
}

pub struct Home {
    settings: Settings,
    /// False in tests, so they never touch the real config file.
    persist: bool,
    pub scan: Scan,
    /// Bumped on every scan; a slower, older scan never overwrites a newer one.
    generation: u64,
}

impl Home {
    pub fn new(settings: Settings, persist: bool) -> Self {
        Self {
            settings,
            persist,
            scan: Scan::Idle,
            generation: 0,
        }
    }

    pub fn folders(&self) -> &[PathBuf] {
        &self.settings.project_folders
    }

    pub fn add_folders(&mut self, folders: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.settings.add_folders(folders);
        self.save();
        self.rescan(cx);
    }

    pub fn remove_folder(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.settings.project_folders.len() {
            self.settings.project_folders.remove(index);
            self.save();
            self.rescan(cx);
        }
    }

    fn save(&self) {
        if self.persist {
            self.settings.save();
        }
    }

    /// Scans the project folders in the background (it walks the disk).
    pub fn rescan(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        let generation = self.generation;
        if self.folders().is_empty() {
            self.scan = Scan::Idle;
            cx.notify();
            return;
        }
        self.scan = Scan::Running;
        cx.notify();
        let roots = self.folders().to_vec();
        let task = cx.background_spawn(async move { kaze_core::scan(&roots, &Options::default()) });
        cx.spawn(async move |this, cx| {
            let reports = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation == generation {
                    this.scan = Scan::Done(reports);
                    cx.notify();
                }
            });
        })
        .detach();
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

    fn render_welcome(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .id("welcome")
            .test_support()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_6()
            .child(
                v_flex()
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
                    ))),
            )
            .child(
                primary_button(
                    "add-folders",
                    Lucide::FolderPlus,
                    tr(
                        "Choisir mes dossiers projets…",
                        "Choose my project folders…",
                    ),
                )
                .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
            )
            .into_any_element()
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let summary: SharedString = match &self.scan {
            Scan::Idle => "".into(),
            Scan::Running => tr("Analyse en cours…", "Scanning…").into(),
            Scan::Done(reports) => {
                let dead: Vec<_> = reports.iter().filter(|r| r.is_dead()).collect();
                let bytes: u64 = dead.iter().map(|r| r.size).sum();
                trf!(
                    "{} worktrees morts sur {} · {} récupérables",
                    "{} dead worktrees out of {} · {} to reclaim",
                    dead.len(),
                    reports.len(),
                    fmt::bytes(bytes)
                )
                .into()
            }
        };
        let running = matches!(self.scan, Scan::Running);
        h_flex()
            .px_6()
            .pt_4()
            .pb_3()
            .gap_3()
            .items_center()
            .child(
                v_flex()
                    .flex_1()
                    .gap_0p5()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .text_color(gpui_kit::white())
                            .child("Worktrees"),
                    )
                    .child(
                        div()
                            .id("summary")
                            .test_support()
                            .text_sm()
                            .text_color(theme::muted())
                            .child(summary),
                    ),
            )
            .child(
                ghost_button(
                    "add-folder",
                    Lucide::FolderPlus,
                    tr("Ajouter un dossier", "Add folder"),
                )
                .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
            )
            .child(
                primary_button("rescan", Lucide::RefreshCw, tr("Analyser", "Scan"))
                    .when(running, |b| b.opacity(0.5))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !matches!(this.scan, Scan::Running) {
                            this.rescan(cx)
                        }
                    })),
            )
    }

    fn render_folders(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let home = dirs::home_dir();
        let chips = self.folders().iter().enumerate().map(|(i, path)| {
            h_flex()
                .id(("folder", i))
                .h(px(26.))
                .pl_2()
                .pr_1()
                .gap_1p5()
                .rounded_md()
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::panel())
                .text_xs()
                .child(
                    Icon::new(Lucide::Folder)
                        .xsmall()
                        .text_color(theme::accent()),
                )
                .child(fmt::display_path(path, home.as_deref()))
                .child(
                    div()
                        .id(("remove-folder", i))
                        .test_support()
                        .p_0p5()
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme::raised()))
                        .child(Icon::new(Lucide::X).xsmall().text_color(theme::muted()))
                        .on_click(cx.listener(move |this, _, _, cx| this.remove_folder(i, cx))),
                )
        });
        h_flex().px_6().pb_3().gap_2().flex_wrap().children(chips)
    }

    fn render_results(&self) -> AnyElement {
        let body: AnyElement = match &self.scan {
            Scan::Idle => div().into_any_element(),
            Scan::Running => centered(
                Icon::new(Lucide::LoaderCircle)
                    .text_color(theme::accent())
                    .with_animation(
                        "spin",
                        Animation::new(Duration::from_millis(900)).repeat(),
                        |icon, t| {
                            icon.transform(gpui_kit::Transformation::rotate(gpui_kit::percentage(
                                t,
                            )))
                        },
                    )
                    .into_any_element(),
                tr("Recherche des worktrees…", "Looking for worktrees…"),
            ),
            Scan::Done(reports) if reports.is_empty() => centered(
                Icon::new(Lucide::CircleCheck)
                    .text_color(theme::accent())
                    .into_any_element(),
                tr(
                    "Aucun worktree dans ces dossiers.",
                    "No worktrees in these folders.",
                ),
            ),
            Scan::Done(reports) => {
                let now = SystemTime::now();
                let home = dirs::home_dir();
                v_flex()
                    .gap_1()
                    .children(
                        reports
                            .iter()
                            .enumerate()
                            .map(|(i, r)| report_row(i, r, now, home.as_deref())),
                    )
                    .into_any_element()
            }
        };
        div()
            .id("results")
            .test_support()
            .flex_1()
            .min_h_0()
            .mx_4()
            .mb_4()
            .p_2()
            .rounded_xl()
            .border_1()
            .border_color(theme::hairline())
            .bg(theme::alpha(gpui_kit::black(), 0.25))
            .overflow_y_scroll()
            .child(body)
            .into_any_element()
    }
}

impl Render for Home {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.folders().is_empty() {
            self.render_welcome(cx)
        } else {
            v_flex()
                .flex_1()
                .min_h_0()
                .child(self.render_header(cx))
                .child(self.render_folders(cx))
                .child(self.render_results())
                .into_any_element()
        };
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

fn primary_button(
    id: &'static str,
    icon: Lucide,
    label: &'static str,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    h_flex()
        .id(id)
        .flex_none()
        .gap_2()
        .px_4()
        .h(px(32.))
        .rounded_lg()
        .cursor_pointer()
        .bg(theme::accent())
        .text_color(gpui_kit::black())
        .text_sm()
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .hover(|s| s.bg(theme::alpha(theme::accent(), 0.85)))
        .child(Icon::new(icon).small())
        .child(label)
}

fn ghost_button(
    id: &'static str,
    icon: Lucide,
    label: &'static str,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    h_flex()
        .id(id)
        .flex_none()
        .gap_2()
        .px_3()
        .h(px(32.))
        .rounded_lg()
        .cursor_pointer()
        .border_1()
        .border_color(theme::hairline())
        .text_sm()
        .hover(|s| s.bg(theme::raised()))
        .child(Icon::new(icon).small().text_color(theme::muted()))
        .child(label)
}

fn centered(icon: AnyElement, text: &'static str) -> AnyElement {
    v_flex()
        .size_full()
        .py_16()
        .items_center()
        .justify_center()
        .gap_3()
        .text_sm()
        .text_color(theme::muted())
        .child(icon)
        .child(text)
        .into_any_element()
}

fn badge(text: impl Into<SharedString>, color: Hsla) -> impl IntoElement {
    div()
        .px_1p5()
        .py_0p5()
        .rounded_md()
        .text_xs()
        .bg(theme::alpha(color, 0.14))
        .text_color(color)
        .child(text.into())
}

pub fn reason_label(reason: Reason) -> String {
    match reason {
        Reason::Missing => tr("Dossier supprimé", "Folder gone").into(),
        Reason::Merged => tr("Mergé", "Merged").into(),
        Reason::UpstreamGone => tr("Branche distante supprimée", "Remote branch deleted").into(),
        Reason::Inactive(days) => trf!("Inactif depuis {days} j", "Idle for {days} d"),
    }
}

fn report_row(
    i: usize,
    r: &Report,
    now: SystemTime,
    home: Option<&std::path::Path>,
) -> impl IntoElement {
    let name = r
        .branch
        .clone()
        .unwrap_or_else(|| tr("(détaché)", "(detached)").into());
    let repo = r
        .repo
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let activity = r.last_activity.map(|t| {
        let days = fmt::days_since(t, now);
        if days == 0 {
            tr("aujourd'hui", "today").to_string()
        } else {
            trf!("il y a {days} j", "{days} d ago")
        }
    });
    let dead = r.is_dead();
    h_flex()
        .id(("report", i))
        .test_support()
        .w_full()
        .px_3()
        .py_2()
        .gap_3()
        .rounded_lg()
        .hover(|s| s.bg(theme::raised()))
        .when(!dead, |s| s.opacity(0.55))
        .child(div().size(px(8.)).flex_none().rounded_full().bg(if dead {
            theme::accent()
        } else {
            theme::muted()
        }))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui_kit::FontWeight::MEDIUM)
                                .truncate()
                                .child(name),
                        )
                        .child(div().text_xs().text_color(theme::muted()).child(repo))
                        .children(
                            r.reasons
                                .iter()
                                .map(|&reason| badge(reason_label(reason), theme::accent())),
                        )
                        .when(r.dirty, |s| {
                            s.child(badge(
                                tr("Modifs non commitées", "Uncommitted changes"),
                                theme::amber(),
                            ))
                        })
                        .when(r.unpushed > 0, |s| {
                            s.child(badge(
                                trf!(
                                    "{} commit(s) non poussé(s)",
                                    "{} unpushed commit(s)",
                                    r.unpushed
                                ),
                                theme::amber(),
                            ))
                        })
                        .when(r.locked, |s| {
                            s.child(badge(tr("Verrouillé", "Locked"), theme::muted()))
                        }),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .truncate()
                        .child(fmt::display_path(&r.path, home)),
                ),
        )
        .child(
            v_flex()
                .flex_none()
                .items_end()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .child(fmt::bytes(r.size)),
                )
                .children(activity.map(|a| div().text_xs().text_color(theme::muted()).child(a))),
        )
}
