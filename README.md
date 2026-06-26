# e2md-rs

**Everything to Markdown** — 将 PDF / Office 文档转换为 Markdown, 并通过 AI 进行排版清洗的跨平台桌面应用.

本项目使用 **Rust + [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui)** 独立实现, 灵感与功能设计参考自 [CatMeo2430](https://github.com/CatMeo2430) 的 **E2MD** (C# / WPF 原版). 两份代码之间没有复用或移植关系, 仅为同一产品思路的平行实现.

> **关于原版作者**
>
> [CatMeo2430](https://github.com/CatMeo2430) 是 E2MD 概念的原创作者, 也是我的同学与好友. 他的 C# 版本尚未公开发布, 但已允许我先发布本 Rust 实现. 本仓库采用 [MIT License](LICENSE); 他后续推送自己的版本时, 计划选用 **EPL (Eclipse Public License)**.

## 功能概览

| 模块 | 说明 |
| :--- | :--- |
| **仪表盘** | 任务统计 (成功 / 失败 / 进行中 / 等待), API 额度监控, 最近活动日志 |
| **转换** | 添加 PDF / DOCX / PPTX 等文件, 调用 [MinerU](https://mineru.net) API 转为 Markdown, 支持大 PDF 自动切分与合并 |
| **清洗** | 对转换结果按 YAML 规则进行多阶段 AI 排版修复 (OpenAI / DeepSeek / OpenRouter) |
| **队列** | 转换与清洗双队列, 支持并发控制, 优先级调整, 暂停与取消 |
| **设置** | API 密钥, 并发数, 切分阈值, 导出目录, 主题等 |

## 支持格式

**输入**: PDF, DOC, DOCX, PPT, PPTX, XLS, XLSX

**输出**: Markdown (含 `images/` 资源目录)

## 技术栈

- **语言**: Rust (nightly toolchain)
- **GUI**: [GPUI](https://github.com/zed-industries/zed) + [gpui-component](https://github.com/longbridge/gpui-component)
- **异步运行时**: Tokio
- **HTTP**: reqwest
- **配置 / 规则**: JSON + YAML

## 平台支持

| 平台 | 状态 |
| :--- | :--- |
| Windows | 支持 |
| macOS | 支持 |
| Linux (X11 / Wayland) | 支持 (首次启动有环境检测提示) |

## 快速开始

### 前置要求

- [Rust nightly](https://rustup.rs/) (见 `rust-toolchain.toml`)
- MinerU API Token ([mineru.net](https://mineru.net))
- 至少一个 AI 提供商 API Key (OpenAI / DeepSeek / OpenRouter, 用于清洗阶段)

**Linux 额外依赖** (Arch 示例):

```bash
sudo pacman -S base-devel pkg-config openssl alsa-lib vulkan-headers vulkan-icd-loader \
  libxkbcommon wayland-protocols libwayland libxcb xcb-util-wm xcb-util-keysyms
```

### 构建与运行

```bash
git clone https://github.com/IvolliousElectrolux/e2md-rs.git
cd e2md-rs
cargo run --release
```

首次运行会在程序目录下生成 `e2md.json` 配置文件, 请在 **设置** 页面填入 API 密钥.

### 清洗规则

内置规则位于 `rules/` 目录, 可按 YAML 格式自定义多阶段 Prompt. 示例:

```yaml
Name: "通用排版修复"
Stages:
  - Name: "排版修复"
    Provider: "OpenRouter"
    Model: "google/gemma-4-31b-it:free"
    Prompt: |
      你是一个专业的 Markdown 文档排版助手...
      {{CONTENT}}
```

## 项目结构

```text
src/
├── main.rs          # 入口, GPUI 应用初始化
├── ui/              # 界面 (仪表盘 / 转换 / 清洗 / 队列 / 设置)
├── modules/         # 核心业务 (MinerU 转换, AI 清洗, 切分, 任务生命周期)
├── work_queue.rs    # 双队列调度
├── providers/       # OpenAI / DeepSeek / OpenRouter 客户端
├── models/          # 配置, 任务, 规则等数据模型
└── utils/           # 文件, 日志, 网络等工具
rules/               # 清洗规则 YAML
```

## 与原版 E2MD 的关系

| | 原版 E2MD (C#) | e2md-rs (Rust) |
| :--- | :--- | :--- |
| 作者 | [CatMeo2430](https://github.com/CatMeo2430) | [IvolliousElectrolux](https://github.com/IvolliousElectrolux) |
| 语言 / GUI | C# / WPF (.NET Framework 4.8) | Rust / GPUI |
| 许可证 | 计划 EPL (尚未公开) | MIT |
| 代码关系 | — | 独立重写, 无代码复用 |

## 许可证

本仓库以 [MIT License](LICENSE) 发布.

E2MD 产品概念与功能设计归功于 [CatMeo2430](https://github.com/CatMeo2430). 其 C# 实现尚未公开, 发布后将采用 EPL.
