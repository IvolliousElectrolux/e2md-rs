use gpui::prelude::*;
use gpui::{div, px, Entity, Window};
use gpui_component::{button::{Button, ButtonVariants}, ActiveTheme, StyledExt};
use std::time::Duration;

#[cfg(target_os = "linux")]
use crate::ui::linux_welcome::LinuxWelcomeOverlay;

use crate::ui::{
    pages::{CleanPage, ConvertPage, DashboardPage, QueuePage, SettingsPage},
    AppState,
};

pub const TAB_DASHBOARD: usize = 0;
pub const TAB_CONVERT: usize = 1;
pub const TAB_CLEAN: usize = 2;
pub const TAB_QUEUE: usize = 3;
pub const TAB_SETTINGS: usize = 4;

pub struct MainWindow {
    state: AppState,
    pub active_tab: usize,
    dashboard: Entity<DashboardPage>,
    convert: Entity<ConvertPage>,
    clean: Entity<CleanPage>,
    queue_page: Entity<QueuePage>,
    settings: Entity<SettingsPage>,
    #[cfg(target_os = "linux")]
    linux_welcome: Entity<LinuxWelcomeOverlay>,
}

impl MainWindow {
    pub fn new(state: AppState, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dashboard = cx.new(|_cx| DashboardPage::new(state.clone()));
        let convert = cx.new(|cx| ConvertPage::new(state.clone(), window, cx));
        let clean = cx.new(|cx| CleanPage::new(state.clone(), cx));
        let queue_page = cx.new(|_cx| QueuePage::new(state.clone()));
        let settings = cx.new(|cx| SettingsPage::new(state.clone(), window, cx));

        // Poll the notifier every 500 ms and redraw if background tasks have pinged
        let notifier = state.notifier.clone();
        cx.spawn(async move |weak_self, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                if notifier.drain() {
                    weak_self.update(cx, |_, cx| cx.notify()).ok();
                }
            }
        })
        .detach();

        #[cfg(target_os = "linux")]
        let linux_welcome = cx.new(LinuxWelcomeOverlay::new);

        Self {
            active_tab: 0,
            state,
            dashboard,
            convert,
            clean,
            queue_page,
            settings,
            #[cfg(target_os = "linux")]
            linux_welcome,
        }
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_tab;
        let entity = cx.entity();

        let nav_labels = ["仪表盘", "转换", "清洗", "队列", "设置"];

        div()
            .relative()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .size_full()
                    .bg(cx.theme().background)
                    // Left sidebar
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w(px(176.0))
                            .h_full()
                            .border_r_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().sidebar)
                            .p_3()
                            .gap_1()
                            .child(
                                div()
                                    .px_2()
                                    .py_3()
                                    .text_base()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("E2MD"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .pb_2()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Everything to Markdown"),
                            )
                            .child(div().h_px().bg(cx.theme().border))
                            .child(div().h_2())
                            .children(
                                nav_labels.iter().enumerate().map(|(idx, label)| {
                                    let is_active = active == idx;
                                    let e = entity.clone();
                                    Button::new(format!("nav_{}", idx))
                                        .label(*label)
                                        .when(is_active, |b| b.primary())
                                        .when(!is_active, |b| b.ghost())
                                        .w_full()
                                        .justify_start()
                                        .on_click(move |_, _, cx| {
                                            e.update(cx, |view, cx| {
                                                view.active_tab = idx;
                                                cx.notify();
                                            });
                                        })
                                })
                                .collect::<Vec<_>>(),
                            ),
                    )
                    // Main content
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            // Content area
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .overflow_hidden()
                                    .when(active == TAB_DASHBOARD, |this| {
                                        this.child(self.dashboard.clone())
                                    })
                                    .when(active == TAB_CONVERT, |this| {
                                        this.child(self.convert.clone())
                                    })
                                    .when(active == TAB_CLEAN, |this| {
                                        this.child(self.clean.clone())
                                    })
                                    .when(active == TAB_QUEUE, |this| {
                                        this.child(self.queue_page.clone())
                                    })
                                    .when(active == TAB_SETTINGS, |this| {
                                        this.child(self.settings.clone())
                                    }),
                            )
                            // Status bar
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .px_4()
                                    .h_7()
                                    .border_t_1()
                                    .border_color(cx.theme().border)
                                    .bg(cx.theme().status_bar)
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(status_text(&self.state))
                                    .child("E2MD v0.1.0"),
                            ),
                    ),
            )
            .map(|shell| attach_linux_welcome(shell, self))
    }
}

#[cfg(target_os = "linux")]
fn attach_linux_welcome(
    shell: gpui::Div,
    view: &MainWindow,
) -> gpui::Div {
    shell.child(view.linux_welcome.clone())
}

#[cfg(not(target_os = "linux"))]
fn attach_linux_welcome(shell: gpui::Div, _view: &MainWindow) -> gpui::Div {
    shell
}

fn status_text(state: &AppState) -> String {
    let jobs = state.jobs();
    let active = jobs.iter().filter(|j| j.is_active()).count();
    if active == 0 {
        "就绪".to_string()
    } else {
        format!("正在处理 {} 个任务...", active)
    }
}
