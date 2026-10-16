#[macro_use]
mod i18n;
mod home;
mod settings;
mod theme;

use gpui_kit::component::TitleBar;
use gpui_kit::{
    AppContext as _, WindowBackgroundAppearance, WindowBounds, WindowOptions, px, size,
};

use home::Home;
use settings::Settings;

fn window_options(cx: &mut gpui_kit::App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(1100.), px(720.)), cx)),
        window_min_size: Some(size(px(760.), px(520.))),
        app_id: Some("dev.kaze".into()),
        // Glass look: the translucent theme lets the blurred desktop through.
        window_background: WindowBackgroundAppearance::Blurred,
        titlebar: Some(gpui_kit::TitlebarOptions {
            traffic_light_position: Some(gpui_kit::point(px(14.), px(15.))),
            ..TitleBar::title_bar_options()
        }),
        ..TitleBar::window_options()
    }
}

fn main() {
    kaze_core::process::inherit_shell_path();
    let settings = Settings::load();
    let lang = sys_locale::get_locale().unwrap_or_default();
    i18n::set_english(!lang.to_lowercase().starts_with("fr"));
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            theme::apply(cx);
            let options = window_options(cx);
            gpui_kit::open_window(options, cx, |window, cx| {
                window.set_window_title("Kaze");
                cx.new(|cx| {
                    let mut home = Home::new(settings, true);
                    home.rescan(cx);
                    home
                })
            })
            .expect("failed to open the main window");
            cx.on_window_closed(|cx, _| cx.quit()).detach();
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests;
