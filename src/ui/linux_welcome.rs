//! Mandatory startup splash for Linux builds — shown on every launch, 5 s countdown.

use gpui::prelude::*;
use gpui::{div, px, Window};
use gpui_component::{
    button::{Button, ButtonVariants},
    ActiveTheme, Disableable,
};
use std::time::Duration;

pub const MESSAGE: &str = "specially for linux users like 刘殷睿";
const COUNTDOWN_SECS: u32 = 5;

pub struct LinuxWelcomeOverlay {
    visible: bool,
    secs_left: u32,
}

impl LinuxWelcomeOverlay {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            for _ in 0..COUNTDOWN_SECS {
                cx.background_executor()
                    .timer(Duration::from_secs(1))
                    .await;
                let _ = this.update(cx, |state, cx| {
                    if state.visible && state.secs_left > 0 {
                        state.secs_left -= 1;
                        cx.notify();
                    }
                });
            }
        })
        .detach();

        Self {
            visible: true,
            secs_left: COUNTDOWN_SECS,
        }
    }
}

impl Render for LinuxWelcomeOverlay {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div();
        }

        let can_close = self.secs_left == 0;
        let entity = cx.entity();
        let ok_label = if can_close {
            "关闭".to_string()
        } else {
            format!("请等待 {} 秒...", self.secs_left)
        };

        div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(cx.theme().overlay)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_6()
                    .w(px(420.0))
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().popover)
                    .child(
                        div()
                            .text_base()
                            .text_color(cx.theme().foreground)
                            .text_center()
                            .child(MESSAGE),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .child(
                                Button::new("linux_welcome_close")
                                    .primary()
                                    .label(ok_label)
                                    .disabled(!can_close)
                                    .on_click(move |_, _, cx| {
                                        entity.update(cx, |state, cx| {
                                            state.visible = false;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    ),
            )
    }
}
