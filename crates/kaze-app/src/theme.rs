//! Mugetsu's "cyber dark" glass look: a translucent blurred base, soft accents
//! and a dense layout. Same palette, so the two apps feel like siblings.

use std::rc::Rc;

use gpui_kit::component::{Theme, ThemeConfig, ThemeMode, ThemeRegistry};
use gpui_kit::{App, Hsla, rgb};

const THEME_JSON: &str = include_str!("../assets/theme.json");
const ACCENT: u32 = 0x6cc7dc;

pub fn accent() -> Hsla {
    rgb(ACCENT).into()
}
pub fn amber() -> Hsla {
    rgb(0xd9b26f).into()
}
/// Light frosting over the window glass (headers, sidebar).
pub fn panel() -> Hsla {
    alpha(rgb(0xffffff).into(), 0.025)
}
/// Hover / raised overlay, works on any surface.
pub fn raised() -> Hsla {
    alpha(rgb(0xffffff).into(), 0.06)
}
/// Hairline used for glass edges.
pub fn hairline() -> Hsla {
    alpha(rgb(0xffffff).into(), 0.08)
}
pub fn muted() -> Hsla {
    rgb(0x8a8a8a).into()
}

/// Same color with a different alpha.
pub fn alpha(color: Hsla, a: f32) -> Hsla {
    Hsla { a, ..color }
}

/// Installs the dark glass theme with the accent colour.
pub fn apply(cx: &mut App) {
    let hex = |a: u8| format!("#{ACCENT:06x}{a:02x}");
    let mut set: serde_json::Value =
        serde_json::from_str(THEME_JSON).expect("theme.json is valid JSON");
    let colors = &mut set["themes"][0]["colors"];
    for (key, a) in [
        ("primary.background", 0xd9),
        ("primary.hover.background", 0xff),
        ("primary.active.background", 0xb3),
        ("caret", 0xff),
        ("ring", 0x80),
        ("selection.background", 0x40),
        ("list.active.background", 0x24),
        ("list.active.border", 0x80),
        ("link.foreground", 0xff),
        ("progress_bar.background", 0xff),
        ("scrollbar.thumb.hover.background", 0x80),
    ] {
        colors[key] = serde_json::Value::String(hex(a));
    }
    colors["primary.foreground"] = "#ffffff".into();
    let config: ThemeConfig = serde_json::from_value(set["themes"][0].clone())
        .expect("theme.json matches the theme schema");
    let config = Rc::new(config);
    let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(&set.to_string());
    Theme::update(cx, |theme| {
        theme.dark_theme = config.clone();
        theme.mode = ThemeMode::Dark;
    });
    Theme::change(ThemeMode::Dark, None, cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_only_changes_opacity() {
        let faded = alpha(accent(), 0.25);
        assert_eq!(faded.a, 0.25);
        let c = accent();
        assert_eq!((faded.h, faded.s, faded.l), (c.h, c.s, c.l));
    }

    #[test]
    fn bundled_theme_parses() {
        let value: serde_json::Value = serde_json::from_str(THEME_JSON).unwrap();
        assert_eq!(value["themes"][0]["mode"], "dark");
    }
}
