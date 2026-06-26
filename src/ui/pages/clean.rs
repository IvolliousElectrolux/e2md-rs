use gpui::prelude::*;
use gpui::{div, Window};
use gpui_component::{button::{Button, ButtonVariants}, checkbox::Checkbox, ActiveTheme, Disableable, Sizable, StyledExt};

use crate::models::JobStatus;
use crate::ui::AppState;
use crate::ui::pages::convert::log_panel;

pub struct CleanPage {
    pub state: AppState,
}

impl CleanPage {
    pub fn new(state: AppState, _cx: &mut gpui::Context<Self>) -> Self {
        Self { state }
    }
}

impl Render for CleanPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let jobs = self.state.jobs();
        let rules = self.state.rules();
        let selected_rule = self.state.selected_rule();
        let selected_for_clean = self.state.selected_jobs_for_clean();

        // Show tasks that are ready to clean, being cleaned, or already exported
        let cleanable: Vec<_> = jobs
            .iter()
            .filter(|j| matches!(
                j.status,
                JobStatus::Converted | JobStatus::CleanPartial
                | JobStatus::Cleaning | JobStatus::Cleaned | JobStatus::Exported
            ))
            .cloned()
            .collect();

        let has_selection = !selected_for_clean.is_empty() && selected_rule.is_some();
        let entity = cx.entity();
        let entity2 = cx.entity();
        let entity3 = cx.entity();

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
                    .child("AI 清洗"),
            )
            // Job list
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("待清洗任务 ({} 个)", cleanable.len())),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .max_h_64()
                            .overflow_hidden()
                            .rounded_md()
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().popover)
                            .p_2()
                            .gap_1()
                            // Header row
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_4()
                                    .px_2()
                                    .py_1()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(div().w_6())
                                    .child(div().flex_1().child("文件名"))
                                    .child(div().w_32().child("创建时间"))
                                    .child(div().w_20().child("状态")),
                            )
                            .children(
                                cleanable.iter().map(|job| {
                                    let is_selectable = matches!(
                                        job.status,
                                        JobStatus::Converted | JobStatus::CleanPartial | JobStatus::Exported
                                    );
                                    let is_selected = selected_for_clean.contains(&job.job_guid);
                                    let guid = job.job_guid.clone();
                                    let s = self.state.clone();
                                    let e = entity.clone();
                                    let export_path = job.export_path.clone();
                                    let has_export = !export_path.is_empty();

                                    let status_color = match job.status {
                                        JobStatus::Exported => cx.theme().success,
                                        JobStatus::Cleaning => cx.theme().warning,
                                        JobStatus::Failed => cx.theme().danger,
                                        _ => cx.theme().muted_foreground,
                                    };

                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap_3()
                                        .px_2()
                                        .py_1()
                                        .rounded_sm()
                                        .when(is_selected, |d| d.bg(cx.theme().accent))
                                        .child(
                                            Checkbox::new(format!("sel_{}", job.job_guid))
                                                .checked(is_selected)
                                                .disabled(!is_selectable)
                                                .on_click(move |_, _, cx| {
                                                    s.toggle_job_for_clean(guid.clone());
                                                    e.update(cx, |_, cx| cx.notify());
                                                }),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .overflow_hidden()
                                                .child(job.original_file_name.clone()),
                                        )
                                        .child(
                                            div()
                                                .w_32()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(job.create_time.clone()),
                                        )
                                        .child(
                                            div()
                                                .w_20()
                                                .text_xs()
                                                .text_color(status_color)
                                                .child(format!("{}", job.status)),
                                        )
                                        .when(has_export, |d| {
                                            d.child(
                                                Button::new(format!("open_{}", job.job_guid))
                                                    .ghost()
                                                    .xsmall()
                                                    .label("打开")
                                                    .on_click(move |_, _, _cx| {
                                                        crate::utils::shell::open_in_file_manager(
                                                            std::path::Path::new(&export_path),
                                                        );
                                                    }),
                                            )
                                        })
                                })
                                .collect::<Vec<_>>(),
                            ),
                    ),
            )
            // Rule selector
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("清洗规则:"),
                    )
                    .children(
                        rules.iter().map(|rule| {
                            let is_selected = selected_rule.as_deref() == Some(&rule.file_name);
                            let fname = rule.file_name.clone();
                            let s = self.state.clone();
                            let e = entity2.clone();
                            Button::new(format!("rule_{}", rule.file_name))
                                .label(&rule.name)
                                .when(is_selected, |b| b.primary())
                                .when(!is_selected, |b| b.ghost())
                                .on_click(move |_, _, cx| {
                                    s.set_selected_rule(Some(fname.clone()));
                                    e.update(cx, |_, cx| cx.notify());
                                })
                        })
                        .collect::<Vec<_>>(),
                    ),
            )
            // Start clean
            .child(
                Button::new("start_clean")
                    .primary()
                    .label("开始清洗")
                    .disabled(!has_selection)
                    .on_click({
                        let s = self.state.clone();
                        let e = entity3.clone();
                        move |_, _, cx| {
                            let selected = s.selected_jobs_for_clean();
                            let rule_name = s.selected_rule();
                            let rules = s.rules();
                            if let Some(rule) = rules.into_iter().find(|r| Some(&r.file_name) == rule_name.as_ref()) {
                                crate::utils::log::log_info_tagged("clean", format!(
                                    "开始清洗 {} 个任务, 规则: {}",
                                    selected.len(), rule.name
                                ));
                                s.spawn_clean_task(selected, rule, cx);
                            } else {
                                crate::utils::log::log_error_tagged("clean", "未找到选中的清洗规则".to_string());
                            }
                            e.update(cx, |_, cx| cx.notify());
                        }
                    }),
            )
            // Log area
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("清洗日志"),
                    )
                    .child(log_panel("clean", cx)),
            )
    }
}
