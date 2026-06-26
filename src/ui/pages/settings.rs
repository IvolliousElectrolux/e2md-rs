use gpui::prelude::*;
use gpui::{div, Entity, MouseButton, Window};
use gpui_component::{
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    tab::{Tab, TabBar},
    ActiveTheme,
};

use crate::themes;
use crate::ui::AppState;

pub struct SettingsPage {
    pub state: AppState,
    pub active_tab: usize,
    pub mineru_token: Entity<InputState>,
    pub mineru_url: Entity<InputState>,
    pub openai_key: Entity<InputState>,
    pub openai_url: Entity<InputState>,
    pub deepseek_key: Entity<InputState>,
    pub deepseek_url: Entity<InputState>,
    pub openrouter_key: Entity<InputState>,
    pub openrouter_url: Entity<InputState>,
    pub openrouter_referer: Entity<InputState>,
    pub openrouter_title: Entity<InputState>,
    pub max_convert: Entity<InputState>,
    pub max_clean: Entity<InputState>,
    pub split_threshold: Entity<InputState>,
    pub export_dir: Entity<InputState>,
}

impl SettingsPage {
    pub fn new(state: AppState, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = state.config();

        let make_input = |val: String, cx: &mut Context<SettingsPage>, window: &mut Window| {
            cx.new(|cx| InputState::new(window, cx).default_value(val))
        };
        let make_masked = |val: String, cx: &mut Context<SettingsPage>, window: &mut Window| {
            cx.new(|cx| InputState::new(window, cx).default_value(val).masked(true))
        };

        Self {
            state,
            active_tab: 0,
            mineru_token: make_masked(config.mineru_token, cx, window),
            mineru_url: make_input(config.mineru_base_url, cx, window),
            openai_key: make_masked(config.openai_api_key, cx, window),
            openai_url: make_input(config.openai_base_url, cx, window),
            deepseek_key: make_masked(config.deepseek_api_key, cx, window),
            deepseek_url: make_input(config.deepseek_base_url, cx, window),
            openrouter_key: make_masked(config.openrouter_api_key, cx, window),
            openrouter_url: make_input(config.openrouter_base_url, cx, window),
            openrouter_referer: make_input(config.openrouter_referer, cx, window),
            openrouter_title: make_input(config.openrouter_title, cx, window),
            max_convert: make_input(config.max_convert_concurrency.to_string(), cx, window),
            max_clean: make_input(config.max_clean_concurrency.to_string(), cx, window),
            split_threshold: make_input(config.pdf_split_threshold_pages.to_string(), cx, window),
            export_dir: make_input(config.export_directory, cx, window),
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_tab;
        let entity = cx.entity();

        let (mt, mu, oak, oau, dsk, dsu, ork, oru, orref, ortit, mc, mcl, st, ed) = (
            self.mineru_token.clone(),
            self.mineru_url.clone(),
            self.openai_key.clone(),
            self.openai_url.clone(),
            self.deepseek_key.clone(),
            self.deepseek_url.clone(),
            self.openrouter_key.clone(),
            self.openrouter_url.clone(),
            self.openrouter_referer.clone(),
            self.openrouter_title.clone(),
            self.max_convert.clone(),
            self.max_clean.clone(),
            self.split_threshold.clone(),
            self.export_dir.clone(),
        );

        let state = self.state.clone();
        let mt2 = mt.clone(); let mu2 = mu.clone();
        let oak2 = oak.clone(); let oau2 = oau.clone();
        let dsk2 = dsk.clone(); let dsu2 = dsu.clone();
        let ork2 = ork.clone(); let oru2 = oru.clone();
        let orref2 = orref.clone(); let ortit2 = ortit.clone();
        let mc2 = mc.clone(); let mcl2 = mcl.clone();
        let st2 = st.clone(); let ed2 = ed.clone();

        let current_theme = state.config().theme_name.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .bg(cx.theme().background)
            // Tab bar
            .child(
                TabBar::new("settings_tabs")
                    .selected_index(active)
                    .on_click({
                        let e = entity.clone();
                        move |idx, _, cx| {
                            e.update(cx, |view, cx| {
                                view.active_tab = *idx;
                                cx.notify();
                            });
                        }
                    })
                    .child(Tab::new().label("MinerU"))
                    .child(Tab::new().label("OpenAI"))
                    .child(Tab::new().label("DeepSeek"))
                    .child(Tab::new().label("OpenRouter"))
                    .child(Tab::new().label("全局设置"))
                    .child(Tab::new().label("主题")),
            )
            // Tab content
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_3()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().popover)
                    .when(active == 0, |this| {
                        this.child(labeled_input("MinerU Token", &mt, cx))
                            .child(labeled_input("MinerU Base URL", &mu, cx))
                    })
                    .when(active == 1, |this| {
                        this.child(labeled_input("OpenAI API Key", &oak, cx))
                            .child(labeled_input("OpenAI Base URL", &oau, cx))
                    })
                    .when(active == 2, |this| {
                        this.child(labeled_input("DeepSeek API Key", &dsk, cx))
                            .child(labeled_input("DeepSeek Base URL", &dsu, cx))
                    })
                    .when(active == 3, |this| {
                        this.child(labeled_input("OpenRouter API Key", &ork, cx))
                            .child(labeled_input("OpenRouter Base URL", &oru, cx))
                            .child(labeled_input("HTTP-Referer (可选)", &orref, cx))
                            .child(labeled_input("App Title (可选)", &ortit, cx))
                    })
                    .when(active == 4, |this| {
                        this.child(labeled_input("最大转换并发数", &mc, cx))
                            .child(labeled_input("最大清洗并发数", &mcl, cx))
                            .child(labeled_input("PDF 切分页数阈值", &st, cx))
                            .child(labeled_input("导出目录", &ed, cx))
                    })
                    .when(active == 5, |this| {
                        this.child(theme_grid(&current_theme, &state, cx))
                    }),
            )
            // Save button — only show for non-theme tabs
            .when(active != 5, |this| {
                this.child(
                    Button::new("save_settings")
                        .primary()
                        .label("保存设置")
                        .on_click(move |_, _, cx| {
                            let mt_val = mt2.read(cx).value().to_string();
                            let mu_val = mu2.read(cx).value().to_string();
                            let oak_val = oak2.read(cx).value().to_string();
                            let oau_val = oau2.read(cx).value().to_string();
                            let dsk_val = dsk2.read(cx).value().to_string();
                            let dsu_val = dsu2.read(cx).value().to_string();
                            let ork_val = ork2.read(cx).value().to_string();
                            let oru_val = oru2.read(cx).value().to_string();
                            let orref_val = orref2.read(cx).value().to_string();
                            let ortit_val = ortit2.read(cx).value().to_string();
                            let mc_val = mc2.read(cx).value().parse::<usize>().unwrap_or(50);
                            let mcl_val = mcl2.read(cx).value().parse::<usize>().unwrap_or(3);
                            let st_val = st2.read(cx).value().parse::<u32>().unwrap_or(200);
                            let ed_val = ed2.read(cx).value().to_string();

                            state.update_config(|cfg| {
                                cfg.mineru_token = mt_val;
                                cfg.mineru_base_url = mu_val;
                                cfg.openai_api_key = oak_val;
                                cfg.openai_base_url = oau_val;
                                cfg.deepseek_api_key = dsk_val;
                                cfg.deepseek_base_url = dsu_val;
                                cfg.openrouter_api_key = ork_val;
                                cfg.openrouter_base_url = oru_val;
                                cfg.openrouter_referer = orref_val;
                                cfg.openrouter_title = ortit_val;
                                cfg.max_convert_concurrency = mc_val;
                                cfg.max_clean_concurrency = mcl_val;
                                cfg.pdf_split_threshold_pages = st_val;
                                cfg.export_directory = ed_val;
                            });
                            crate::utils::log::log_success("设置已保存");
                        }),
                )
            })
    }
}

/// Render a grid of theme swatches.  Clicking a swatch instantly switches the theme.
fn theme_grid(
    current_theme: &str,
    state: &AppState,
    cx: &mut gpui::App,
) -> impl IntoElement {
    let current = current_theme.to_string();

    // (display name, bg hex, accent hex)
    let swatches: Vec<(&'static str, u32, u32)> = vec![
        ("Default Light",    0xffffff, 0x171717),
        ("Default Dark",     0x0a0a0a, 0xf5f5f5),
        ("Dracula",          0x282a36, 0xbd93f9),
        ("Catppuccin Mocha", 0x1e1e2e, 0xcba6f7),
        ("Nord",             0x2e3440, 0x81a1c1),
        ("Solarized Light",  0xfdf6e3, 0x268bd2),
    ];

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("选择主题 — 点击即时应用"),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .children(swatches.into_iter().map(|(name, bg, accent)| {
                    let name_str = name.to_string();
                    let is_active = name == current;
                    let state2 = state.clone();
                    let ring_color = cx.theme().ring;
                    let border_color = cx.theme().border;
                    let fg = cx.theme().foreground;
                    let muted = cx.theme().muted_foreground;

                    // Fixed-width card: swatch preview + label, click handled by on_mouse_down
                    div()
                        .w_32()                      // fixed 128px width
                        .flex()
                        .flex_col()
                        .gap_1()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, {
                            let n = name_str.clone();
                            move |_, _, cx| {
                                themes::apply(&n, cx);
                                state2.update_config(|cfg| {
                                    cfg.theme_name = n.clone();
                                });
                            }
                        })
                        .child(
                            // Preview card
                            div()
                                .w_32()
                                .h_20()
                                .rounded_md()
                                .border_2()
                                .border_color(if is_active { ring_color } else { border_color })
                                .bg(gpui::rgb(bg))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .w_8()
                                        .h_3()
                                        .rounded_full()
                                        .bg(gpui::rgb(accent)),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(if is_active { fg } else { muted })
                                .when(is_active, |d| d.font_weight(gpui::FontWeight::BOLD))
                                .child(name_str),
                        )
                })),
        )
}

fn labeled_input(label: &str, input: &Entity<InputState>, cx: &gpui::App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(Input::new(input))
}
