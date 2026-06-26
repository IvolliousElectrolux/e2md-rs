use gpui::prelude::*;
use gpui::{div, ExternalPaths, PathPromptOptions, Window};
use gpui_component::{
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    ActiveTheme, Disableable, Sizable, StyledExt,
};
use gpui_component::text::TextView;

use crate::ui::AppState;

pub struct ConvertPage {
    pub state: AppState,
}

impl ConvertPage {
    pub fn new(state: AppState, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { state }
    }

    fn add_paths(&self, paths: Vec<std::path::PathBuf>, cx: &mut Context<Self>) {
        let exts = ["pdf", "docx", "pptx", "doc", "ppt", "xlsx", "xls"];
        let mut added = 0usize;
        for p in paths {
            let ext_ok = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false);
            if ext_ok {
                self.state.add_staged_file(p);
                added += 1;
            }
        }
        if added > 0 {
            crate::utils::log::log_info_tagged("convert", format!("已添加 {} 个文件", added));
        }
        cx.notify();
    }
}

/// Render a selectable log panel showing entries filtered by `tag` ("convert" or "clean").
/// Entries are shown newest-first. Text is selectable and copyable.
pub fn log_panel(tag: &'static str, cx: &gpui::App) -> impl gpui::IntoElement {
    use crate::utils::log::history_tagged;
    use gpui_component::ActiveTheme;

    let logs = history_tagged(tag);
    // Build a single plain-text string: newest entries at top
    let text: String = logs
        .iter()
        .rev()
        .take(200)
        .map(|e| e.format())
        .collect::<Vec<_>>()
        .join("\n");

    div()
        .flex_1()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().popover)
        .p_3()
        .overflow_hidden()
        .child(
            TextView::markdown(format!("log_panel_{tag}"), text)
                .selectable(true)
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
}

impl Render for ConvertPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let staged = self.state.staged_files();
        let auto_clean = self.state.auto_clean();
        let pdf_split = self.state.pdf_split_enabled();
        let has_staged = !staged.is_empty();

        let entity_btn = cx.entity();
        let entity_drop = cx.entity();
        let entity_opt1 = cx.entity();
        let entity_opt2 = cx.entity();
        let entity_submit = cx.entity();

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
                    .child("PDF / Office → Markdown 转换"),
            )
            // File add button
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("add_file")
                            .label("添加文件")
                            .on_click({
                                let entity = entity_btn.clone();
                                move |_event, window, cx| {
                                    let receiver = cx.prompt_for_paths(PathPromptOptions {
                                        files: true,
                                        directories: false,
                                        multiple: true,
                                        prompt: None,
                                    });
                                    let entity = entity.clone();
                                    window
                                        .spawn(cx, async move |async_cx| {
                                            if let Ok(Ok(Some(paths))) = receiver.await {
                                                async_cx
                                                    .update(|_window, cx| {
                                                        entity.update(cx, |view, cx| {
                                                            view.add_paths(paths, cx);
                                                        });
                                                    })
                                                    .ok();
                                            }
                                        })
                                        .detach();
                                }
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} 个文件已添加", staged.len())),
                    ),
            )
            // Drop zone + staged files list
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_h_32()
                    .max_h_48()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().popover)
                    .p_2()
                    .gap_1()
                    .drag_over::<ExternalPaths>(|this, _, _, _| {
                        this.border_color(gpui::blue())
                    })
                    .on_drop({
                        let _entity = entity_drop.clone();
                        cx.listener(
                            move |view: &mut ConvertPage,
                                  paths: &ExternalPaths,
                                  _window,
                                  cx| {
                                let all: Vec<_> = paths.0.iter().cloned().collect();
                                view.add_paths(all, cx);
                            },
                        )
                    })
                    .when(staged.is_empty(), |d| {
                        d.items_center().justify_center().child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("拖放文件到此处, 或点击上方「添加文件」"),
                        )
                    })
                    .when(!staged.is_empty(), |d| {
                        d.children(
                            staged
                                .iter()
                                .enumerate()
                                .map(|(i, path)| {
                                    let name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("?")
                                        .to_string();
                                    let s = self.state.clone();
                                    let e_rm = cx.entity();
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .px_2()
                                        .py_1()
                                        .rounded_sm()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .child(name),
                                        )
                                        .child(
                                            Button::new(format!("rm_{}", i))
                                                .ghost()
                                                .label("✕")
                                                .xsmall()
                                                .on_click(move |_e, _w, cx| {
                                                    s.remove_staged_file(i);
                                                    e_rm.update(cx, |_, cx| cx.notify());
                                                }),
                                        )
                                })
                                .collect::<Vec<_>>(),
                        )
                    }),
            )
            // Options
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_6()
                    .items_center()
                    .child(
                        Checkbox::new("auto_clean")
                            .label("转换后自动清洗")
                            .checked(auto_clean)
                            .on_click({
                                let s = self.state.clone();
                                let e = entity_opt1.clone();
                                move |v, _w, cx| {
                                    s.set_auto_clean(*v);
                                    e.update(cx, |_, cx| cx.notify());
                                }
                            }),
                    )
                    .child(
                        Checkbox::new("pdf_split")
                            .label("启用大文件切分")
                            .checked(pdf_split)
                            .on_click({
                                let s = self.state.clone();
                                let e = entity_opt2.clone();
                                move |v, _w, cx| {
                                    s.set_pdf_split_enabled(*v);
                                    e.update(cx, |_, cx| cx.notify());
                                }
                            }),
                    ),
            )
            // Submit
            .child(
                Button::new("start_convert")
                    .primary()
                    .label("开始转换")
                    .disabled(!has_staged)
                    .on_click({
                        let s = self.state.clone();
                        let e = entity_submit.clone();
                        move |_, _, cx| {
                            let guids = s.submit_staged_files(cx);
                            crate::utils::log::log_info_tagged("convert", format!(
                                "已提交 {} 个文件到转换队列",
                                guids.len()
                            ));
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
                            .child("转换日志"),
                    )
                    .child(log_panel("convert", cx)),
            )
    }
}
