use gpui::prelude::*;
use gpui::{div, Window};
use gpui_component::{ActiveTheme, StyledExt};

use crate::models::JobStatus;
use crate::ui::AppState;
use crate::utils::{
    api_usage::get_usage,
    log::LogLevel,
};

pub struct DashboardPage {
    state: AppState,
}

impl DashboardPage {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl Render for DashboardPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let jobs = self.state.jobs();
        let success = jobs.iter().filter(|j| j.status == JobStatus::Exported).count();
        let failed = jobs.iter().filter(|j| j.status == JobStatus::Failed).count();
        let active = jobs.iter().filter(|j| j.is_active()).count();
        let waiting = jobs.iter().filter(|j| j.status == JobStatus::Pending).count();

        let usage = get_usage();
        let logs = self.state.log_entries();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_4()
            .bg(cx.theme().background)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .child(stat_card("今日导出", &success.to_string(), cx.theme().success, cx))
                    .child(stat_card("失败", &failed.to_string(), cx.theme().danger, cx))
                    .child(stat_card("执行中", &active.to_string(), cx.theme().warning, cx))
                    .child(stat_card("等待中", &waiting.to_string(), cx.theme().muted_foreground, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().popover)
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("API 用量统计"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_6()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "OpenRouter: {} tokens / ${:.4}",
                                usage.openrouter.tokens, usage.openrouter.cost_usd
                            ))
                            .child(format!("DeepSeek: {} tokens", usage.deepseek.tokens))
                            .child(format!("OpenAI: {} tokens", usage.openai.tokens)),
                    ),
            )
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
                            .child("最近日志"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .overflow_hidden()
                            .rounded_lg()
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().popover)
                            .p_3()
                            .gap_px()
                            .children(
                                logs.iter()
                                    .rev()
                                    .take(50)
                                    .map(|entry| {
                                        let color = match entry.level {
                                            LogLevel::Error => cx.theme().danger,
                                            LogLevel::Warn => cx.theme().warning,
                                            LogLevel::Success => cx.theme().success,
                                            LogLevel::Info => cx.theme().muted_foreground,
                                        };
                                        div()
                                            .text_xs()
                                            .text_color(color)
                                            .child(entry.format())
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                    ),
            )
    }
}

fn stat_card(
    label: &str,
    value: &str,
    color: gpui::Hsla,
    cx: &gpui::App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .p_4()
        .w_32()
        .h_24()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().popover)
        .gap_2()
        .child(
            div()
                .text_2xl()
                .font_semibold()
                .text_color(color)
                .child(value.to_string()),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
}
