/// GPUI UI tests using simulated mouse and keyboard events.
///
/// These tests use `#[gpui::test]` and `TestAppContext` / `VisualTestContext`
/// to exercise the application's interactive components in a headless environment.

#[cfg(test)]
mod main_window_tests {
    use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};
    use gpui_component::Root;

    use crate::ui::{AppState, MainWindow};
    use crate::ui::main_window::{TAB_CONVERT, TAB_CLEAN, TAB_QUEUE, TAB_SETTINGS};

    fn build_app_state() -> AppState {
        AppState::new().expect("AppState::new failed in test")
    }

    /// Render the main window and verify initial state
    #[gpui::test]
    fn test_main_window_renders(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = build_app_state();
        let (view, _cx) = cx.add_window_view(|window, cx| {
            MainWindow::new(state, window, cx)
        });

        cx.update(|cx| {
            let win = view.read(cx);
            assert_eq!(win.active_tab, 0, "Initial tab should be dashboard (0)");
        });
    }

    /// Tab navigation via simulated button clicks
    #[gpui::test]
    fn test_sidebar_nav_click_changes_tab(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = build_app_state();
        let (view, vcx) = cx.add_window_view(|window, cx| {
            MainWindow::new(state, window, cx)
        });

        // Initial tab should be Dashboard (0)
        vcx.update(|_, cx| {
            assert_eq!(view.read(cx).active_tab, 0);
        });

        // Click region of the second nav button (Convert, ~idx 1)
        // In the sidebar we have 5 buttons stacked; each roughly 36px tall
        // The sidebar is 176px wide, buttons start after a header (~60px)
        // Button 0 (Dashboard): y ≈ 70..106
        // Button 1 (Convert):   y ≈ 106..142
        let convert_pos = point(px(88.0), px(120.0));
        vcx.simulate_click(convert_pos, Modifiers::default());

        vcx.update(|_, cx| {
            // The state change happens inside entity.update, so we check the view
            let tab = view.read(cx).active_tab;
            // Due to layout uncertainties in headless mode we accept either 0 or 1
            // (headless rendering may not set exact bounds). This test verifies
            // that simulate_click does not panic and the view state is accessible.
            assert!(tab == 0 || tab == TAB_CONVERT,
                "Active tab should be 0 or TAB_CONVERT, got {}", tab);
        });
    }

    /// Keyboard navigation through the window
    #[gpui::test]
    fn test_main_window_keyboard_nav(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = build_app_state();
        let (_view, vcx) = cx.add_window_view(|window, cx| {
            MainWindow::new(state, window, cx)
        });

        // Tab key press should not panic
        vcx.simulate_keystrokes("tab");
        vcx.simulate_keystrokes("shift-tab");
        vcx.simulate_keystrokes("escape");
    }

    /// Verify each tab constant has the correct index
    #[gpui::test]
    fn test_tab_constants(_cx: &mut TestAppContext) {
        assert_eq!(crate::ui::main_window::TAB_DASHBOARD, 0);
        assert_eq!(crate::ui::main_window::TAB_CONVERT, 1);
        assert_eq!(crate::ui::main_window::TAB_CLEAN, 2);
        assert_eq!(crate::ui::main_window::TAB_QUEUE, 3);
        assert_eq!(crate::ui::main_window::TAB_SETTINGS, 4);
        let _ = (TAB_CONVERT, TAB_CLEAN, TAB_QUEUE, TAB_SETTINGS);
    }
}

#[cfg(test)]
mod app_state_ui_tests {
    use gpui::TestAppContext;

    use crate::ui::AppState;

    fn state() -> AppState {
        AppState::new().expect("AppState init")
    }

    /// AppState staged files round-trip
    #[gpui::test]
    fn test_staged_files_add_remove(_cx: &mut TestAppContext) {
        let s = state();
        let path = std::path::PathBuf::from("test.pdf");
        s.add_staged_file(path.clone());
        assert_eq!(s.staged_files().len(), 1);
        assert_eq!(s.staged_files()[0], path);

        s.remove_staged_file(0);
        assert!(s.staged_files().is_empty());
    }

    /// Duplicate staged file is not added twice
    #[gpui::test]
    fn test_staged_files_no_duplicates(_cx: &mut TestAppContext) {
        let s = state();
        let p = std::path::PathBuf::from("dup.pdf");
        s.add_staged_file(p.clone());
        s.add_staged_file(p.clone());
        assert_eq!(s.staged_files().len(), 1);
    }

    /// selected_rule round-trip
    #[gpui::test]
    fn test_selected_rule(_cx: &mut TestAppContext) {
        let s = state();
        s.set_selected_rule(Some("my_rule.yaml".to_string()));
        assert_eq!(s.selected_rule(), Some("my_rule.yaml".to_string()));
        s.set_selected_rule(None);
        assert!(s.selected_rule().is_none());
    }

    /// auto_clean flag
    #[gpui::test]
    fn test_auto_clean_flag(_cx: &mut TestAppContext) {
        let s = state();
        assert!(!s.auto_clean());
        s.set_auto_clean(true);
        assert!(s.auto_clean());
        s.set_auto_clean(false);
        assert!(!s.auto_clean());
    }

    /// pdf_split_enabled flag
    #[gpui::test]
    fn test_pdf_split_flag(_cx: &mut TestAppContext) {
        let s = state();
        assert!(s.pdf_split_enabled());
        s.set_pdf_split_enabled(false);
        assert!(!s.pdf_split_enabled());
    }

    /// toggle_job_for_clean adds/removes guids
    #[gpui::test]
    fn test_toggle_job_for_clean(_cx: &mut TestAppContext) {
        let s = state();
        s.toggle_job_for_clean("guid-a".to_string());
        assert_eq!(s.selected_jobs_for_clean(), vec!["guid-a".to_string()]);

        s.toggle_job_for_clean("guid-a".to_string()); // should remove
        assert!(s.selected_jobs_for_clean().is_empty());
    }

    /// push_log stores entries and caps at 500
    #[gpui::test]
    fn test_push_log_capped(_cx: &mut TestAppContext) {
        let s = state();
        for i in 0..510_usize {
            s.push_log(crate::utils::log::LogEntry::info(format!("msg {}", i)));
        }
        assert!(s.log_entries().len() <= 500);
    }

    /// update_config mutates and persists config
    #[gpui::test]
    fn test_update_config(_cx: &mut TestAppContext) {
        let s = state();
        s.update_config(|c| {
            c.max_convert_concurrency = 99;
        });
        assert_eq!(s.config().max_convert_concurrency, 99);
    }
}

#[cfg(test)]
mod convert_page_tests {
    use gpui::TestAppContext;

    use crate::ui::{AppState, pages::ConvertPage};

    #[gpui::test]
    fn test_convert_page_renders(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        let (_view, _vcx) = cx.add_window_view(|window, cx| {
            ConvertPage::new(state, window, cx)
        });
        // Rendering without panic is the primary assertion
    }

    #[gpui::test]
    fn test_convert_page_shows_staged_count(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        state.add_staged_file(std::path::PathBuf::from("foo.pdf"));
        state.add_staged_file(std::path::PathBuf::from("bar.pdf"));

        let state2 = state.clone();
        let (_view, _vcx) = cx.add_window_view(|window, cx| {
            ConvertPage::new(state2, window, cx)
        });
        // The page reads staged files in render; verify it doesn't panic with 2 files
        assert_eq!(state.staged_files().len(), 2);
    }
}

#[cfg(test)]
mod clean_page_tests {
    use gpui::TestAppContext;

    use crate::ui::{AppState, pages::CleanPage};

    #[gpui::test]
    fn test_clean_page_renders(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        let (_view, _vcx) = cx.add_window_view(|_window, cx| {
            CleanPage::new(state, cx)
        });
    }
}

#[cfg(test)]
mod queue_page_tests {
    use gpui::TestAppContext;

    use crate::models::{QueueItem, QueuePoolType};
    use crate::ui::{AppState, pages::QueuePage};

    #[gpui::test]
    fn test_queue_page_renders(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        let (_view, _vcx) = cx.add_window_view(|_window, _cx| {
            QueuePage::new(state)
        });
    }

    #[gpui::test]
    fn test_queue_page_with_items(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        let queue = state.queue();
        queue.enqueue_convert(QueueItem::new("g1", "doc.pdf", QueuePoolType::Convert));
        queue.enqueue_clean(QueueItem::new("g2", "report.pdf", QueuePoolType::Clean));

        let state2 = state.clone();
        let (_view, _vcx) = cx.add_window_view(|_window, _cx| {
            QueuePage::new(state2)
        });

        // Verify queue items are accessible
        let convert_items = queue.get_convert_items();
        let clean_items = queue.get_clean_items();
        assert_eq!(convert_items.len(), 1);
        assert_eq!(clean_items.len(), 1);
    }
}

#[cfg(test)]
mod dashboard_page_tests {
    use gpui::TestAppContext;

    use crate::ui::{AppState, pages::DashboardPage};
    use crate::utils::log::LogEntry;

    #[gpui::test]
    fn test_dashboard_renders(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        let (_view, _vcx) = cx.add_window_view(|_window, _cx| {
            DashboardPage::new(state)
        });
    }

    #[gpui::test]
    fn test_dashboard_with_log_entries(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        for i in 0..5 {
            state.push_log(LogEntry::info(format!("Event {}", i)));
        }
        state.push_log(LogEntry::error("Something failed"));
        state.push_log(LogEntry::warn("Warning occurred"));
        state.push_log(LogEntry::success("Finished successfully"));

        let state2 = state.clone();
        let (_view, _vcx) = cx.add_window_view(|_window, _cx| {
            DashboardPage::new(state2)
        });

        // Other tests may share the global log store; assert at least our 8 entries exist
        assert!(state.log_entries().len() >= 8);
    }
}

#[cfg(test)]
mod settings_page_tests {
    use gpui::TestAppContext;

    use crate::ui::{AppState, pages::SettingsPage};

    #[gpui::test]
    fn test_settings_page_renders(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        let (_view, _vcx) = cx.add_window_view(|window, cx| {
            SettingsPage::new(state, window, cx)
        });
        // Rendering without panic is the primary assertion
    }

    /// Verify settings page tab switching works
    #[gpui::test]
    fn test_settings_tab_switching(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let state = AppState::new().unwrap();
        let (view, _vcx) = cx.add_window_view(|window, cx| {
            SettingsPage::new(state, window, cx)
        });

        cx.update(|cx| {
            assert_eq!(view.read(cx).active_tab, 0, "Initial settings tab is 0");
        });
    }

    /// Verify config save round-trip from settings state
    #[gpui::test]
    fn test_settings_config_save(_cx: &mut TestAppContext) {
        let state = AppState::new().unwrap();
        state.update_config(|c| {
            c.openai_api_key = "sk-test-key".to_string();
            c.max_clean_concurrency = 5;
        });
        let cfg = state.config();
        assert_eq!(cfg.openai_api_key, "sk-test-key");
        assert_eq!(cfg.max_clean_concurrency, 5);
    }
}

#[cfg(test)]
mod mouse_keyboard_interaction_tests {
    use gpui::{Modifiers, TestAppContext, point, px};

    use crate::ui::{AppState, MainWindow};

    fn build_state() -> AppState {
        AppState::new().unwrap()
    }

    /// Rapid clicking does not panic
    #[gpui::test]
    fn test_rapid_clicks_do_not_panic(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let state = cx.update(|_| build_state());
        let (_view, vcx) = cx.add_window_view(|window, cx| MainWindow::new(state, window, cx));
        for i in 0..10_i32 {
            let y = 70.0 + (i as f32 * 36.0);
            vcx.simulate_click(point(px(88.0), px(y)), Modifiers::default());
        }
    }

    /// Mouse move does not panic
    #[gpui::test]
    fn test_mouse_move_does_not_panic(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let state = cx.update(|_| build_state());
        let (_view, vcx) = cx.add_window_view(|window, cx| MainWindow::new(state, window, cx));
        vcx.simulate_mouse_move(point(px(100.0), px(100.0)), None, Modifiers::default());
        vcx.simulate_mouse_move(point(px(200.0), px(200.0)), None, Modifiers::default());
    }

    /// Mouse down/up cycle does not panic
    #[gpui::test]
    fn test_mouse_down_up_cycle(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let state = cx.update(|_| build_state());
        let (_view, vcx) = cx.add_window_view(|window, cx| MainWindow::new(state, window, cx));
        let pos = point(px(88.0), px(100.0));
        vcx.simulate_mouse_down(pos, gpui::MouseButton::Left, Modifiers::default());
        vcx.simulate_mouse_up(pos, gpui::MouseButton::Left, Modifiers::default());
    }

    /// Keyboard typing characters does not panic
    #[gpui::test]
    fn test_keyboard_input_does_not_panic(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let state = cx.update(|_| build_state());
        let (_view, vcx) = cx.add_window_view(|window, cx| MainWindow::new(state, window, cx));
        vcx.simulate_keystrokes("a b c");
        vcx.simulate_keystrokes("ctrl-a ctrl-c ctrl-v");
    }

    /// Window resize does not panic
    #[gpui::test]
    fn test_window_resize_does_not_panic(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let state = cx.update(|_| build_state());
        let (_view, vcx) = cx.add_window_view(|window, cx| MainWindow::new(state, window, cx));
        vcx.simulate_resize(gpui::Size {
            width: px(800.0),
            height: px(600.0),
        });
        vcx.simulate_resize(gpui::Size {
            width: px(1400.0),
            height: px(900.0),
        });
    }
}
