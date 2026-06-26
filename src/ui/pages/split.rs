#![allow(dead_code)]
/// Split page (PDF chapter splitting configuration) — minimal stub
/// Full implementation follows the same pattern as ConvertPage.
use gpui::prelude::*;
use gpui::{div, Window};
use gpui_component::{ActiveTheme, StyledExt};

use crate::ui::AppState;

pub struct SplitPage {
    pub state: AppState,
}

impl SplitPage {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl Render for SplitPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_2()
            .child(
                div()
                    .text_lg()
                    .font_bold()
                    .text_color(cx.theme().foreground)
                    .child("切分配置"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("在转换页面中配置 PDF 切分阈值并提交文件后, 此处可查看切分计划。"),
            )
    }
}
