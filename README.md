# LocalAIRouter

LocalAIRouter 是一个本地 AI 代理路由器，用来把 Codex、Claude Code、Gemini 或自定义客户端的请求转发到你配置的上游账号。

默认监听地址：

- 本机访问：`http://127.0.0.1:16321`
- Codex 入口：`http://127.0.0.1:16321/codex`
- Claude Code 入口：`http://127.0.0.1:16321/claude-code`

主要能力：

- 内置 `codex`、`claude-code`、`gemini` Provider，也支持自定义 Provider
- 支持按 Provider、模型前缀、默认账号路由请求
- 支持 Provider / Account 默认模型
- 本地加密保存账号密钥
- 桌面端管理 Provider、Account、Route、运行设置和完整请求日志
- 可选开启局域网访问，让其他设备通过本机 IP 访问代理

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

推荐新人直接编译 `.app`，这样在 macOS 上运行时不会弹出 Terminal 窗口。

```bash
cd apps/desktop/src-tauri
cargo tauri build
```

编译成功后，产物在仓库根目录：

```text
target/release/bundle/macos/LocalAIRouter.app
```

如果需要生成 macOS DMG 安装包，可以显式指定 bundle 类型：

```bash
cd apps/desktop/src-tauri
cargo tauri build --bundles dmg
```

DMG 输出在仓库根目录：

```text
target/release/bundle/dmg/
```

### 3. 运行 App

```bash
open ../../../target/release/bundle/macos/LocalAIRouter.app
```

如果你已经回到仓库根目录，则运行：

```bash
open target/release/bundle/macos/LocalAIRouter.app
```

### 4. 首次使用

打开桌面端后：

1. 初始化或解锁 Vault
2. 在 `Providers` 里确认或新增上游 Provider
3. 在 `Accounts` 里添加账号和 API Key
4. 在 `Routes` 里选择默认账号
5. 在 `Onboarding` 页面复制客户端配置

本机客户端通常使用这些地址：

- Codex：`http://127.0.0.1:16321/codex`
- Claude Code：`http://127.0.0.1:16321/claude-code`

## 编译方式

### 编译完整 workspace

如果只想得到 release 二进制：

```bash
cargo build --workspace --release
```

输出包括：

- `target/release/localairouter`
- `target/release/localairouter-daemon`

注意：`target/release/localairouter` 是裸二进制，不是 macOS `.app`。从 Finder 或 `open` 运行它时，macOS 可能会打开 Terminal。想要正常桌面应用体验，请使用 `cargo tauri build` 生成的 `.app`。

### 构建 macOS DMG

默认配置只构建 `.app`，避免日常开发时被 DMG 打包耗时或环境问题影响。发布时可以单独构建 DMG：

```bash
cd apps/desktop/src-tauri
cargo tauri build --bundles dmg
```

如果你的本机没有签名证书，或只是本地分发测试，可以跳过签名：

```bash
cargo tauri build --bundles dmg --no-sign
```

DMG 产物位置：

```text
target/release/bundle/dmg/
```

如果想让 `cargo tauri build` 默认同时构建 `.app` 和 `.dmg`，可以把 `apps/desktop/src-tauri/tauri.conf.json` 里的 `bundle.targets` 改成：

```json
["app", "dmg"]
```

### 编译调试版本

```bash
cargo build --workspace
```

输出包括：

- `target/debug/localairouter`
- `target/debug/localairouter-daemon`

## 开发者运行

### 普通开发运行

在仓库根目录运行：

```bash
cargo run -p localairouter
```

桌面端启动时会自动拉起 `localairouter-daemon`。

### UI 开发模式

如果只改 `apps/desktop/ui` 下的 HTML / CSS / JS，可以开启轻量 UI dev server：

```bash
LOCALAIROUTER_UI_DEV=1 cargo run -p localairouter
```

这个模式下：

- 桌面窗口直接加载 `apps/desktop/ui`
- HTML / CSS / JS 修改后会自动刷新
- Rust 代码修改仍需要重新运行

### 单独运行 daemon

```bash
cargo run -p localairouter-daemon
```

默认监听：

```text
http://127.0.0.1:16321
```

也可以指定端口：

```bash
LOCALAIROUTER_PORT=18000 cargo run -p localairouter-daemon
```

桌面端同样会把该端口传给它拉起的 daemon：

```bash
LOCALAIROUTER_PORT=18000 cargo run -p localairouter
```

## 常用运行设置

### 修改默认数据目录

```bash
LOCALAIROUTER_DATA_DIR=/path/to/data cargo run -p localairouter
```

不设置时会使用系统默认本地数据目录。

### 指定 daemon 路径

桌面端默认会从 workspace 的 `target` 目录查找 `localairouter-daemon`。如果你想指定 daemon：

```bash
export LOCALAIROUTER_DAEMON_PATH=/absolute/path/to/localairouter-daemon
cargo run -p localairouter
```

### 允许局域网访问

在桌面端 `Settings` 页面勾选 `Allow LAN Access`。

开启后 daemon 会绑定到：

```text
0.0.0.0:<port>
```

设置页会显示当前机器的局域网访问地址。其他设备应使用本机局域网 IP，例如：

```text
http://192.168.1.10:16321/claude-code
```

也可以通过环境变量开启：

```bash
LOCALAIROUTER_ALLOW_LAN=1 cargo run -p localairouter
```

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
