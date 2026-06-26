use gpui::prelude::*;
use gpui::{div, Window};
use gpui_component::{button::{Button, ButtonVariants}, ActiveTheme, Disableable, Sizable, StyledExt};

use crate::models::{QueueItemStatus, QueuePoolType};
use crate::ui::AppState;

pub struct QueuePage {
    pub state: AppState,
}

fn open_dir(path: &str) {
    crate::utils::shell::open_in_file_manager(std::path::Path::new(path));
}

impl QueuePage {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl Render for QueuePage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let queue = self.state.queue();
        let convert_items = queue.get_convert_items();
        let clean_items = queue.get_clean_items();
        // Build a guid→export_path map for showing "open" buttons on finished items
        let jobs = self.state.jobs();
        let export_map: std::collections::HashMap<String, String> = jobs
            .into_iter()
            .filter(|j| !j.export_path.is_empty())
            .map(|j| (j.job_guid, j.export_path))
            .collect();
        let entity = cx.entity();
        let entity2 = cx.entity();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .bg(cx.theme().background)
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child("任务队列"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .flex_1()
                    // Convert queue
                    .child(queue_panel(
                        "转换队列",
                        &convert_items,
                        QueuePoolType::Convert,
                        &queue,
                        &export_map,
                        entity.clone(),
                        cx,
                    ))
                    // Clean queue
                    .child(queue_panel(
                        "清洗队列",
                        &clean_items,
                        QueuePoolType::Clean,
                        &queue,
                        &export_map,
                        entity2.clone(),
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        Button::new("clear_done")
                            .ghost()
                            .label("清空已完成")
                            .on_click({
                                let q = self.state.queue();
                                let e = cx.entity();
                                move |_, _, cx| {
                                    q.clear_done();
                                    e.update(cx, |_, cx| cx.notify());
                                }
                            }),
                    ),
            )
    }
}

fn queue_panel(
    title: &str,
    items: &[crate::models::QueueItem],
    _pool: QueuePoolType,
    queue: &crate::work_queue::WorkQueue,
    export_map: &std::collections::HashMap<String, String>,
    entity: gpui::Entity<QueuePage>,
    cx: &gpui::App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .text_color(cx.theme().foreground)
                .child(format!("{} ({})", title, items.len())),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().popover)
                .p_2()
                .gap_1()
                .children(
                    items.iter().map(|item| {
                        let status_color = match item.status {
                            QueueItemStatus::Running => cx.theme().success,
                            QueueItemStatus::Failed => cx.theme().danger,
                            QueueItemStatus::Done => cx.theme().muted_foreground,
                            _ => cx.theme().foreground,
                        };
                        let item_id = item.id.clone();
                        let q = queue.clone();
                        let e = entity.clone();
                        let export_path = export_map.get(&item.job_guid).cloned();
                        let is_done = item.status == QueueItemStatus::Done;

                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(cx.theme().foreground)
                                    .overflow_hidden()
                                    .child(item.file_name.clone()),
                            )
                            .child(
                                div()
                                    .w_16()
                                    .text_xs()
                                    .text_color(status_color)
                                    .child(format!("{}", item.status)),
                            )
                            .child(
                                div()
                                    .w_12()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{:.0}%", item.progress * 100.0)),
                            )
                            // "打开" button — only visible for done items with a known path
                            .when(is_done && export_path.is_some(), |row| {
                                let path = export_path.clone().unwrap_or_default();
                                row.child(
                                    Button::new(format!("open_{}_{}", title, item.id))
                                        .ghost()
                                        .xsmall()
                                        .label("打开")
                                        .on_click(move |_, _, _cx| {
                                            open_dir(&path);
                                        }),
                                )
                            })
                            .child(
                                Button::new(format!("cancel_{}_{}", title, item.id))
                                    .ghost()
                                    .xsmall()
                                    .label("✕")
                                    .disabled(matches!(
                                        item.status,
                                        QueueItemStatus::Done | QueueItemStatus::Cancelled
                                    ))
                                    .on_click(move |_, _, cx| {
                                        q.cancel(&item_id);
                                        e.update(cx, |_, cx| cx.notify());
                                    }),
                            )
                    })
                    .collect::<Vec<_>>(),
                ),
        )
}
