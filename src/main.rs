#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod models;
mod utils;
mod providers;
mod modules;
mod work_queue;
mod ui;
mod themes;
#[cfg(test)]
mod tests;

use gpui::prelude::*;
use gpui::{Bounds, Pixels, Point, Size, WindowBounds, px};
use gpui_component::Root;

use models::AppConfig;
use ui::{AppState, MainWindow};

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        themes::load_all(cx);

        cx.spawn(async move |cx| {
            let state = match AppState::new() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to initialize app state: {}", e);
                    std::process::exit(1);
                }
            };

            // Apply saved theme before opening the window
            let _ = cx.update(|cx| {
                let theme_name = state.config().theme_name.clone();
                themes::apply(&theme_name, cx);
            });

            // Build the restore bounds (used by Windowed, Maximized, Fullscreen alike)
            let cfg = state.config();
            let restore_bounds: Bounds<Pixels> = if let (Some(x), Some(y), Some(w), Some(h)) =
                (cfg.window_x, cfg.window_y, cfg.window_width, cfg.window_height)
            {
                Bounds {
                    origin: Point { x: px(x), y: px(y) },
                    size: Size {
                        width:  px(w.max(400.0)),
                        height: px(h.max(300.0)),
                    },
                }
            } else {
                // First launch: center 1200×800 on primary display
                let win_size = Size { width: px(1200.0), height: px(800.0) };
                cx.update(|cx| {
                    let display_id = cx.displays().first().map(|d| d.id());
                    Bounds::<Pixels>::centered(display_id, win_size, cx)
                })
            };

            // Reconstruct the full WindowBounds (fullscreen → Fullscreen, maximized → Maximized,
            // otherwise → Windowed).  All three variants carry the restore bounds.
            let win_bounds: WindowBounds = if cfg.window_fullscreen {
                WindowBounds::Fullscreen(restore_bounds)
            } else if cfg.window_maximized {
                WindowBounds::Maximized(restore_bounds)
            } else {
                WindowBounds::Windowed(restore_bounds)
            };

            cx.open_window(
                gpui::WindowOptions {
                    window_bounds: Some(win_bounds),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("E2MD — Everything to Markdown".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                {
                    let state = state.clone();
                    move |window, cx| {
                        // Save window state on close
                        window.on_window_should_close(cx, move |window, _cx| {
                            if let Ok(mut cfg) = AppConfig::load() {
                                match window.window_bounds() {
                                    WindowBounds::Fullscreen(b) => {
                                        save_bounds(&mut cfg, &b);
                                        cfg.window_fullscreen = true;
                                        cfg.window_maximized  = false;
                                    }
                                    WindowBounds::Maximized(b) => {
                                        save_bounds(&mut cfg, &b);
                                        cfg.window_fullscreen = false;
                                        cfg.window_maximized  = true;
                                    }
                                    WindowBounds::Windowed(b) => {
                                        save_bounds(&mut cfg, &b);
                                        cfg.window_fullscreen = false;
                                        cfg.window_maximized  = false;
                                    }
                                }
                                let _ = cfg.save();
                            }
                            true // allow close
                        });

                        let view = cx.new(|cx| MainWindow::new(state.clone(), window, cx));
                        cx.new(|cx| Root::new(view, window, cx))
                    }
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}

fn save_bounds(cfg: &mut AppConfig, b: &Bounds<Pixels>) {
    cfg.window_x      = Some(b.origin.x.as_f32());
    cfg.window_y      = Some(b.origin.y.as_f32());
    cfg.window_width  = Some(b.size.width.as_f32());
    cfg.window_height = Some(b.size.height.as_f32());
}
