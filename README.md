# LocalAIRouter

LocalAIRouter 是一个本地 AI 代理桌面应用。在 Codex、Claude Code 等 AI 工具和你的多个 API 账号之间插一层——切换账号不用改配置，不用改环境变量，点一下就行。

**它不是什么：** 不是透明代理，不劫持系统流量；不是聊天客户端；不会上传你的 API Key。

**它做什么：** 你的 AI 工具都指向 `http://127.0.0.1:16321`，在 LocalAIRouter 桌面端里选择当前用哪个 Provider 和哪个账号，所有流经的请求都有日志、有统计、能看到 Token 花了多少。


## 核心特性

- **一键切账号**：每个 Provider 下配多个 Account，设默认立刻生效，tray 菜单同步更新。
- **Codex / Claude 配置同步**：点 Sync 就写进配置，不覆盖其他设置，原 `base_url` 自动备份。
- **支持Codex + Deepseek V4**：不仅可以Claude Code + Deepseek，也支持Codex + Deepseek。

## 默认本地入口

| 客户端 | 本地代理地址 |
|--------|-------------|
| Codex | `http://127.0.0.1:16321/codex` |
| Claude Code | `http://127.0.0.1:16321/claude-code` |
| Gemini | `http://127.0.0.1:16321/gemini` |
| 自定义 Provider | `http://127.0.0.1:16321/{proxy-path}` |

## 快速开始

### 1. 准备环境

需要安装：

- Rust / Cargo
- Tauri CLI

如果还没有 Tauri CLI：

```bash
cargo install tauri-cli --locked
```

### 2. 编译桌面 App

```bash
cd apps/desktop/src-tauri
cargo tauri build
```

产出：`target/release/bundle/macos/LocalAIRouter.app`

DMG 构建：

```bash
cd apps/desktop/src-tauri
cargo tauri build --bundles dmg
```

### 3. 运行

```bash
open target/release/bundle/macos/LocalAIRouter.app
```

### 4. 首次使用

1. 在 `Providers` 里确认内置 Provider，或新增自定义 Provider。
2. 在 `Accounts` 里添加账号和 API Key。
3. 在 `Accounts` 里把某个账号设为默认。
4. 在 `文档` 页面查看接入说明，或在 `Providers` 里点 Sync 同步到 Codex / Claude Code。

## 测试和检查

运行全部 Rust 测试：

```bash
cargo test --workspace
```

检查前端 JS 语法：

```bash
node --check apps/desktop/ui/app.js
```

格式化 Rust 代码：

```bash
cargo fmt --all
```

## 目录结构

```text
crates/localairouter-core/      核心模型、存储、路由和日志逻辑
crates/localairouter-daemon/    本地 HTTP daemon
apps/desktop/src-tauri/         Tauri 桌面壳
apps/desktop/ui/                桌面端前端页面
```
