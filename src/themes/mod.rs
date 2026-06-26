use gpui::App;
use gpui_component::theme::{Theme, ThemeMode, ThemeRegistry};

const DRACULA: &str = include_str!("dracula.json");
const CATPPUCCIN_MOCHA: &str = include_str!("catppuccin_mocha.json");
const NORD: &str = include_str!("nord.json");
const SOLARIZED_LIGHT: &str = include_str!("solarized_light.json");

/// Load all bundled extra themes into the global `ThemeRegistry`.
/// Must be called after `gpui_component::init(cx)`.
pub fn load_all(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);
    for (name, json) in [
        ("dracula", DRACULA),
        ("catppuccin_mocha", CATPPUCCIN_MOCHA),
        ("nord", NORD),
        ("solarized_light", SOLARIZED_LIGHT),
    ] {
        if let Err(e) = registry.load_themes_from_str(json) {
            eprintln!("Failed to load bundled theme '{}': {}", name, e);
        }
    }
}

/// Apply a theme by name.  Dark themes get `ThemeMode::Dark`, light themes get `ThemeMode::Light`.
pub fn apply(theme_name: &str, cx: &mut App) {
    let mode = mode_for(theme_name);

    // Set the named theme as the active light or dark theme.
    let themes = ThemeRegistry::global(cx).themes().clone();
    let key: gpui::SharedString = theme_name.into();
    if let Some(config) = themes.get(&key).cloned() {
        if mode.is_dark() {
            Theme::global_mut(cx).dark_theme = config;
        } else {
            Theme::global_mut(cx).light_theme = config;
        }
    }

    Theme::change(mode, None, cx);
    cx.refresh_windows();
}

fn mode_for(name: &str) -> ThemeMode {
    match name {
        "Default Light" | "Solarized Light" => ThemeMode::Light,
        _ => ThemeMode::Dark,
    }
}
