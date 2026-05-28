const DEFAULT_PORT = 16321;
const DEFAULT_MONITOR_BUFFER_LIMIT = 200;
const DEFAULT_LOG_RETENTION_DAYS = 30;
const ONBOARDING_TARGETS = [
  { id: "codex", label: "Codex" },
  { id: "claude-code", label: "Claude Code" },
  { id: "curl", label: "cURL / Manual" },
];

const ZH_MESSAGES = {
  Dashboard: "总览",
  Stats: "统计",
  Providers: "供应商",
  Accounts: "账号",
  Routes: "路由",
  Logs: "日志",
  Onboarding: "文档",
  Settings: "设置",
  "Local AI Proxy Router": "本地 AI 代理路由器",
  "Show daemon details": "显示 Daemon 详情",
  "Daemon Details": "Daemon 详情",
  Status: "状态",
  Database: "数据库",
  Started: "启动时间",
  "Log File": "日志文件",
  "Last Exit": "上次退出",
  "Last Error": "上次错误",
  Start: "启动",
  Stop: "停止",
  Restart: "重启",
  "Daemon Log": "Daemon 日志",
  Coverage: "覆盖情况",
  "Routing Snapshot": "路由快照",
  Today: "今天",
  "Today's Activity": "今日活动",
  "Last 30 Days": "最近 30 天",
  "Daily Calls": "每日调用",
  "Daily Tokens": "每日 Tokens",
  "Selected Day": "选中日期",
  "Click a column to inspect the exact day.":
    "点击柱子即可查看当天的精确数据。",
  Requests: "请求",
  Success: "成功",
  Tokens: "Tokens",
  "Grouped by your desktop local day. Restarting the app does not reset these counts.":
    "按你桌面当前时区的本地自然日分组。重启应用后这些统计不会丢失。",
  "Only persisted input + output token usage is counted here. Cached tokens are included when providers report them.":
    "这里只统计已经持久化入库的输入 + 输出 token；上游返回缓存 token 时也会计入。",
  Registry: "注册表",
  "Provider Catalog": "Provider列表",
  "Built-ins can be tuned in place. Custom providers can be edited or removed once no accounts and routes depend on them.":
    "内置 Provider 可直接调整。自定义 Provider 只有在没有账号和路由依赖时才能编辑或删除。",
  "New Provider": "新建 Provider",
  "Each account belongs to one provider. Disable accounts to keep them available for audit without letting routes hit them.":
    "每个账号都属于一个 Provider。禁用账号后仍可保留审计信息，但不会再被路由命中。",
  "New Account": "新建账号",
  Import: "导入",
  "Imported account {name}.": "已导入账号 {name}。",
  "Failed to import Codex account.": "导入 Codex 账号失败。",
  "Failed to import Claude Code account.": "导入 Claude Code 账号失败。",
  Bindings: "路由绑定",
  "Active Routes": "当前路由",
  "Default rules appear alongside model-prefix overrides. Overrides always win when a request model matches the prefix.":
    "默认规则和模型前缀覆盖规则会同时显示。请求模型命中前缀时，覆盖规则优先。",
  "New Route": "新建路由",
  Reload: "刷新",
  "Live Logs": "实时日志",
  "In-memory tail view for current traffic. Full sanitized request logs still persist to daily JSONL files under the configured log directory. Use Open Log Root for the archived files.":
    "这里显示当前流量的内存尾部视图。完整的脱敏请求日志仍会按天落到当前配置的日志目录下，格式为 JSONL。归档文件可通过 Open Log Root 打开。",
  "Open Log Root": "打开日志目录",
  "Open Logs Directory": "打开日志目录",
  "The request could not be completed.": "请求未能完成。",
  "Built-in provider {slug} cannot be deleted.":
    "内置 Provider {slug} 不能删除。",
  "Provider {slug} still has {count} account(s). Remove its accounts before deleting it.":
    "Provider {slug} 仍有关联的 {count} 个账号。请先删除这些账号。",
  "Provider {slug} still has {count} route(s). Remove its routes before deleting it.":
    "Provider {slug} 仍有关联的 {count} 条路由。请先删除这些路由。",
  "Provider {slug} was not found.": "Provider {slug} 不存在。",
  "Provider {slug} is disabled.": "Provider {slug} 已禁用。",
  "Account {id} was not found.": "账号 {id} 不存在。",
  "Account {id} is disabled.": "账号 {id} 已禁用。",
  "Account {id} does not belong to provider {slug}.":
    "账号 {id} 不属于 Provider {slug}。",
  "Selected account {id} for provider {slug} was not found.":
    "Provider {slug} 当前选中的账号 {id} 不存在。",
  "Selected account {name} is disabled.": "当前选中的账号 {name} 已禁用。",
  "Provider {slug} has no default route.": "Provider {slug} 还没有默认路由。",
  "Secret is missing for account {id}.": "账号 {id} 缺少已存储的密钥。",
  "New accounts require an API key.": "新建账号必须填写 API Key。",
  "API key must not be empty when provided.": "填写 API Key 时，内容不能为空。",
  Provider: "Provider",
  All: "全部",
  Account: "账号",
  Limit: "数量",
  "Apply Filters": "应用筛选",
  "Client Setup": "客户端接入",
  "Onboarding Guides": "文档",
  "Runtime Settings": "运行设置",
  "Update the local daemon listening port and traffic log directory here. Saving settings restarts the daemon when it is running.":
    "在这里调整本地 Daemon 的监听端口、实时日志缓冲区、日志保留周期和流量日志目录。保存后如果 Daemon 正在运行，会自动重启。",
  "Update the local daemon listening port, live log buffer, and traffic log directory here. Saving settings restarts the daemon when it is running.":
    "在这里调整本地 Daemon 的监听端口、实时日志缓冲区、日志保留周期和流量日志目录。保存后如果 Daemon 正在运行，会自动重启。",
  "Update the local daemon listening port, live log buffer, log retention window, and traffic log directory here. Saving settings restarts the daemon when it is running.":
    "在这里调整本地 Daemon 的监听端口、局域网访问、实时日志缓冲区、日志保留周期和流量日志目录。保存后如果 Daemon 正在运行，会自动重启。",
  "Listening Port": "监听端口",
  "Allow LAN Access": "允许局域网访问",
  "When enabled, the daemon binds to 0.0.0.0 so other devices can reach it through this machine's LAN IP.":
    "启用后，Daemon 会绑定到 0.0.0.0，其他设备可通过本机局域网 IP 访问。",
  "LAN access URL: {url}": "局域网访问地址：{url}",
  "Unable to determine LAN IP. Check your network connection.":
    "无法确定局域网 IP，请检查网络连接。",
  "Live Log Buffer": "实时日志缓冲",
  "Log Retention Days": "日志保留天数",
  "Traffic Logs Directory": "流量日志目录",
  "Temporary Maintenance": "临时维护",
  "Rebuild historical token totals from archived daily JSONL logs. Use this only to repair older stats after the token calculation logic changed.":
    "从归档的每日 JSONL 日志里重建历史 token 总数。仅在 token 计算逻辑变更后，用它修复旧统计。",
  "Rebuild Token Stats": "重建 Token 统计",
  "Rebuilding historical token stats…": "正在重建历史 Token 统计…",
  "Token stats rebuilt. Updated {updated} / {total} logs.":
    "Token 统计已重建。共更新 {updated} / {total} 条日志。",
  "Token stats rebuild finished. Updated {updated} logs, skipped {skipped}.":
    "Token 统计重建完成。已更新 {updated} 条，跳过 {skipped} 条。",
  "Failed to rebuild token stats.": "重建 Token 统计失败。",
  Browse: "浏览",
  "Default logs directory will appear here.": "默认日志目录会显示在这里。",
  "Default logs directory: {path}": "默认日志目录：{path}",
  "Data Root": "数据根目录",
  "Database Path": "数据库路径",
  "Use Default Logs Directory": "使用默认日志目录",
  "Save Settings": "保存设置",
  "Generate localhost instructions for built-in and custom providers. Codex and Claude Code presets follow provider protocol compatibility, while every provider still exposes a manual cURL entrypoint.":
    "为内置和自定义 Provider 生成 localhost 接入说明。Codex 和 Claude Code 预设遵循 Provider 协议兼容性，同时每个 Provider 仍保留手动 cURL 入口。",
  Refresh: "刷新",
  "Control Plane": "控制面",
  Unlock: "解锁",
  "Upstream Registry": "上游注册表",
  "Close provider dialog": "关闭 Provider 对话框",
  "Display Name": "显示名称",
  Protocol: "协议",
  "Proxy Path": "代理路径",
  "Base URL": "Base URL",
  "Auth Header": "鉴权 Header",
  "Auth Prefix": "鉴权前缀",
  "Provider enabled": "启用 Provider",
  "Save Provider": "保存",
  Credentials: "凭证",
  "Close account dialog": "关闭账号对话框",
  Name: "名称",
  "Primary Account": "主账号",
  Reveal: "查看",
  "Copy API Key": "复制 API Key",
  "Stored API Key": "已存储的 API Key",
  "Base URL Override": "Base URL 覆盖",
  "Optional. Defaults to the provider base URL":
    "可选，默认使用 Provider 的 Base URL",
  "Default Model": "默认模型",
  "Optional. Overrides client request model for this account":
    "可选。配置后此账号会覆盖客户端请求里的模型。",
  "Optional. Overrides client request model for this provider":
    "可选。配置后此 Provider 会覆盖客户端请求里的模型。",
  "default model {model}": "默认模型 {model}",
  Note: "备注",
  "Optional note": "可选备注",
  "Account enabled": "启用账号",
  "Save Account": "保存",
  Routing: "路由",
  "Close route dialog": "关闭路由对话框",
  "Model Prefix": "模型前缀",
  "Leave empty for the provider default": "留空则表示 Provider 默认路由",
  "Save Route": "保存",
  "Confirm Action": "确认操作",
  "Close confirmation dialog": "关闭确认对话框",
  Cancel: "取消",
  Confirm: "确认",
  "Nothing to show yet.": "暂时没有内容。",
  Unavailable: "不可用",
  enabled: "已启用",
  disabled: "已禁用",
  default: "默认",
  override: "覆盖",
  "provider default": "Provider 默认",
  "built-in": "内置",
  custom: "自定义",
  streamed: "流式",
  sync: "同步",
  "Open LocalAIRouter": "打开 LocalAIRouter",
  Quit: "退出",
  "No Accounts": "无帐号",
  Disabled: "已禁用",
  "just now": "刚刚",
  unknown: "未知",
  error: "错误",
  live: "实时",
  routing: "路由中",
  upstream: "请求上游",
  response: "响应中",
  streaming: "流式中",
  failed: "失败",
  completed: "完成",
  Available: "可用",
  Locked: "已锁定",
  Offline: "离线",
  "Daemon offline": "Daemon 离线",
  "Daemon starting": "Daemon 启动中",
  "Daemon unreachable": "Daemon 不可达",
  "Daemon Online": "Daemon 在线",
  "Daemon online": "Daemon 在线",
  "Daemon Starting": "Daemon 启动中",
  "Daemon Offline": "Daemon 离线",
  "Setup required": "需要初始化",
  Initialize: "初始化",
  "Waiting for the local daemon health endpoint to come online.":
    "正在等待本地 Daemon 的健康检查接口上线。",
  Default: "默认",
  "Set Default": "设为默认",
  "Provider default": "Provider 默认",
  "Generic HTTP": "通用 HTTP",
  Edit: "编辑",
  Delete: "删除",
  Disable: "禁用",
  Copy: "复制",
  "Copy Full Log": "复制完整日志",
  "Copy URL": "复制 URL",
  "Copy Snippet": "复制片段",
  "Sync Config": "同步配置",
  "Sync Codex": "同步 Codex",
  "Sync Claude": "同步 Claude",
  Usage: "用法",
  Req: "请求",
  Res: "响应",
  "account base url": "账号 Base URL",
  "provider base url": "Provider Base URL",
  "No account": "无账号",
  "model unavailable": "模型不可用",
  unassigned: "未分配",
  "No providers configured": "尚未配置 Provider",
  "No enabled accounts": "没有可用账号",
  "All providers": "全部 Providers",
  "All accounts": "全部账号",
  "Uses provider base URL.": "使用 Provider 的 Base URL。",
  "{count} route": "{count} 条路由",
  "{count} routes": "{count} 条路由",
  "{count} acct": "{count} 个账号",
  "{count} ovrd": "{count} 个覆盖",
  "{label} ({count})": "{label} ({count})",
  "{name} (disabled)": "{name}（已禁用）",
  "updated {time}": "更新于 {time}",
  "session {id}": "会话 {id}",
  provider: "provider",
  account: "account",
  model: "model",
  phase: "phase",
  status: "status",
  duration: "duration",
  mode: "mode",
  updated: "updated",
  request: "request",
  response: "response",
  "No request body preview.": "没有请求体预览。",
  "Resolving provider route and active account.":
    "正在解析 Provider 路由和当前账号。",
  "Forwarded upstream. Waiting for headers.": "已转发到上游，等待响应头。",
  "Receiving upstream response body.": "正在接收上游响应体。",
  "Streaming response chunks.": "正在处理流式响应分片。",
  "Request failed before a response preview was captured.":
    "请求在捕获到响应预览前就失败了。",
  "Response completed with no preview payload.":
    "响应已完成，但没有可展示的预览内容。",
  "Copy failed. Clipboard access is unavailable.":
    "复制失败，当前环境无法访问剪贴板。",
  "Full interaction log copied.": "完整交互日志已复制。",
  "Full log is still being written. Try again after the request completes.":
    "完整日志仍在写入，请在请求完成后重试。",
  "Desktop integration is unavailable in this context.":
    "当前上下文中无法使用桌面集成功能。",
  "Cannot reach the local daemon on {address}.":
    "无法连接本地 Daemon：{address}。",
  "Cannot read daemon process status from the desktop host.":
    "无法从桌面宿主读取 Daemon 进程状态。",
  "Provider saved.": "Provider 已保存。",
  "Provider updated.": "Provider 已更新。",
  "Provider {name} deleted.": "Provider {name} 已删除。",
  "Define a built-in override or register a custom upstream with its own proxy path, auth header, and protocol shape.":
    "可以覆盖内置 Provider，也可以注册带有独立代理路径、鉴权 Header 和协议形态的自定义上游。",
  "Tune Built-In: {name}": "调整内置 Provider：{name}",
  "Edit Provider: {name}": "编辑 Provider：{name}",
  "Built-in providers keep their internal identity. You can still adjust endpoint, auth header, proxy path, and enabled state.":
    "内置 Provider 会保留其内部标识。你仍然可以调整 endpoint、鉴权 Header、代理路径和启用状态。",
  "Editing a custom provider updates the existing registry entry in place.":
    "编辑自定义 Provider 时，会直接更新现有注册项。",
  "Update Provider": "更新",
  "Local ingress demo will appear here.": "本地入口示例会显示在这里。",
  "Local ingress: {url}": "本地入口：{url}",
  "Account saved.": "账号已保存。",
  "Account updated.": "账号已更新。",
  "Account {name} deleted.": "账号 {name} 已删除。",
  "Account {name} disabled.": "账号 {name} 已禁用。",
  "Route saved.": "路由已保存。",
  "Route updated.": "路由已更新。",
  "Route deleted.": "路由已删除。",
  "Logs refreshed.": "日志已刷新。",
  "Routes refreshed.": "路由已刷新。",
  "Onboarding guides refreshed.": "接入说明已刷新。",
  "Log root opened.": "日志目录已打开。",
  "Daemon started.": "Daemon 已启动。",
  "Daemon stopped.": "Daemon 已停止。",
  "Daemon restarted.": "Daemon 已重启。",
  "Daemon log opened.": "Daemon 日志已打开。",
  "Settings saved.": "设置已保存。",
  "Settings saved. Daemon restarted.": "设置已保存，Daemon 已重启。",
  "Failed to start daemon.": "启动 Daemon 失败。",
  "Failed to stop daemon.": "停止 Daemon 失败。",
  "Failed to restart daemon.": "重启 Daemon 失败。",
  "Failed to save settings.": "保存设置失败。",
  "Failed to choose logs directory.": "选择日志目录失败。",
  "Failed to open daemon log.": "打开 Daemon 日志失败。",
  "Failed to open log root.": "打开日志目录失败。",
  "Monitor item copied.": "日志条目已复制。",
  "Local base URL copied.": "本地 Base URL 已复制。",
  "Onboarding snippet copied.": "接入片段已复制。",
  "Codex config synced.": "Codex 配置已同步。",
  "Codex config synced by updating the existing model_provider base_url.":
    "Codex 配置已同步，已直接更新现有 model_provider 的 base_url。",
  "Claude config synced.": "Claude 配置已同步。",
  "Failed to sync Codex config.": "同步 Codex 配置失败。",
  "Failed to sync Claude config.": "同步 Claude 配置失败。",
  "Copied {value} URL.": "已复制 {value} URL。",
  "Provider ID could not be generated from the display name.":
    "无法根据显示名称生成 Provider ID。",
  "Provider ID may only use lowercase letters, digits, and dashes.":
    "Provider ID 只能包含小写字母、数字和短横线。",
  "Display name is required.": "显示名称不能为空。",
  "Proxy path is required.": "代理路径不能为空。",
  "Proxy path must be one lowercase path segment with letters, digits, or dashes.":
    "代理路径必须是单段小写路径，只能包含字母、数字和短横线。",
  "Base URL must start with http:// or https://.":
    "Base URL 必须以 http:// 或 https:// 开头。",
  "Auth header is required and cannot contain spaces.":
    "鉴权 Header 不能为空，且不能包含空格。",
  "Choose an enabled provider before saving an account.":
    "保存账号前，请先选择一个已启用的 Provider。",
  "Account name is required.": "账号名称不能为空。",
  "New accounts require an API key.": "新建账号必须填写 API Key。",
  "Account base URL must start with http:// or https://.":
    "账号 Base URL 必须以 http:// 或 https:// 开头。",
  "Choose a provider before saving a route.": "保存路由前请先选择 Provider。",
  "The selected provider has no enabled accounts to bind.":
    "当前 Provider 没有可绑定的已启用账号。",
  "Delete Provider": "删除",
  "Delete Account": "删除",
  "Delete Route": "删除",
  "Delete Provider: {name}": "删除 Provider：{name}",
  "Delete Account: {name}": "删除账号：{name}",
  "Delete Route: {name}": "删除路由：{name}",
  "Delete this provider definition? This only succeeds after all dependent accounts and routes are removed.":
    "确认删除这个 Provider 定义吗？只有在其依赖的账号和路由都被删除后才能成功。",
  "No accounts under {name} yet.": "{name} 下还没有账号。",
  "{name} is now the default account for {provider}.":
    "{name} 现已成为 {provider} 的默认账号。",
  "This route currently points at a disabled account. Choose an enabled account before saving.":
    "当前路由指向了一个已禁用账号。保存前请先选择可用账号。",
  "Routes apply immediately to new requests.": "路由会立即应用到后续新请求。",
  "This provider has no enabled accounts. Add or re-enable one before binding routes.":
    "这个 Provider 没有可用账号。请先新增或重新启用账号再绑定路由。",
  "Select a provider to see enabled accounts.":
    "先选择一个 Provider，才能看到可用账号。",
  "View Stored Key": "查看已存密钥",
  "Hide Stored Key": "隐藏已存密钥",
  "Update Account": "更新",
  "Set one default account per provider, then add optional model-prefix overrides for fine-grained account selection.":
    "每个 Provider 先设置一个默认账号，再按需添加模型前缀覆盖，实现更细粒度的账号选择。",
  "Edit Route: {name}": "编辑路由：{name}",
  "Update Route": "更新",
  "Update the provider, prefix, or account binding. Changing provider or prefix will replace the previous binding.":
    "可以修改 Provider、前缀或账号绑定。更改 Provider 或前缀会替换原有绑定。",
  "This row is the provider default account. Updating the bound account here changes the provider default used by non-matching requests.":
    "这一行是 Provider 的默认账号。这里修改绑定账号后，所有未命中覆盖规则的请求都会使用新的默认账号。",
  "Process running but health endpoint unavailable":
    "进程正在运行，但健康检查接口不可用",
  "No compatible provider is available for the selected client profile.":
    "当前所选客户端配置没有可兼容的 Provider。",
  "Add a provider to generate onboarding instructions.":
    "请先新增 Provider，再生成接入说明。",
  "default -> {name}": "默认 -> {name}",
  "default missing": "默认路由缺失",
  "{count} overrides": "{count} 个覆盖",
  "no overrides": "没有覆盖规则",
  "{count} enabled accounts": "{count} 个已启用账号",
  "This provider is enabled in the catalog.":
    "这个 Provider 当前已在目录中启用。",
  "This provider is currently disabled in the catalog. Re-enable it before relying on this namespace.":
    "这个 Provider 当前在目录中已禁用。正式使用前请先重新启用。",
  "Default traffic currently resolves to {name}.":
    "默认流量当前会路由到 {name}。",
  "No provider default account is configured yet. Set one in Accounts or Routes before using this namespace as the catch-all path.":
    "当前还没有配置 Provider 默认账号。请先在 Accounts 或 Routes 中设置默认账号，再把这个命名空间作为兜底入口使用。",
  "{count} model-prefix overrides are active for this provider.":
    "这个 Provider 当前有 {count} 条模型前缀覆盖规则生效。",
  "No model-prefix overrides are active for this provider.":
    "这个 Provider 当前没有生效的模型前缀覆盖规则。",
  "Current local daemon address: {daemonUrl}. This provider namespace resolves at {baseUrl}.":
    "当前本地 Daemon 地址：{daemonUrl}。这个 Provider 命名空间会解析到 {baseUrl}。",
  "Client credentials shown here are placeholders only. LocalAIRouter strips them and injects the real upstream secret from the selected account.":
    "这里展示的客户端凭证只是占位符。LocalAIRouter 会剥离这些占位值，并注入所选账号对应的真实上游密钥。",
  "provider enabled": "Provider 已启用",
  "provider disabled": "Provider 已禁用",
  "Codex via {name}": "通过 {name} 接入 Codex",
  "Claude Code via {name}": "通过 {name} 接入 Claude Code",
  "OpenAI-Compatible Client via {name}": "通过 {name} 接入 OpenAI 兼容客户端",
  "Anthropic-Compatible Client via {name}":
    "通过 {name} 接入 Anthropic 兼容客户端",
  "Manual HTTP via {name}": "通过 {name} 手动接入 HTTP",
  "Use this namespace for Codex or any coding CLI that reads OpenAI-compatible base URL settings. Current target: {baseUrl}.":
    "将这个命名空间用于 Codex 或任何支持 OpenAI 兼容 base URL 的编码 CLI。当前目标地址：{baseUrl}。",
  "Use this namespace for Claude Code or other Anthropic-style clients that support an alternate base URL. Current target: {baseUrl}.":
    "将这个命名空间用于 Claude Code 或其他支持自定义 base URL 的 Anthropic 风格客户端。当前目标地址：{baseUrl}。",
  "Use these settings for SDKs, tools, or scripts that speak the OpenAI API surface but are not Codex-specific. Current target: {baseUrl}.":
    "将这些设置用于支持 OpenAI API 形状、但并非 Codex 专用的 SDK、工具或脚本。当前目标地址：{baseUrl}。",
  "Use these settings for SDKs or CLIs that expect Anthropic-compatible request shapes without being Claude Code itself. Current target: {baseUrl}.":
    "将这些设置用于期望 Anthropic 兼容请求格式、但并非 Claude Code 本身的 SDK 或 CLI。当前目标地址：{baseUrl}。",
  "Generic HTTP providers stay manual-only. Append the upstream-specific path and payload after this namespace. Current target: {baseUrl}.":
    "Generic HTTP Provider 仍保持手动接入。请在这个命名空间后面追加上游特定的路径和负载。当前目标地址：{baseUrl}。",
  "Use this for smoke tests, quick probes, or custom scripts against the local provider namespace. Current target: {baseUrl}.":
    "将其用于冒烟测试、快速探测，或访问本地 Provider 命名空间的自定义脚本。当前目标地址：{baseUrl}。",
  "LAN access is enabled. On other devices, replace 127.0.0.1 with this machine's LAN IP.":
    "已启用局域网访问。在其他设备上，请把 127.0.0.1 替换成当前机器的局域网 IP。",
  "Generic HTTP providers do not have a Codex or Claude Code preset.":
    "Generic HTTP Provider 不提供 Codex 或 Claude Code 预设。",
  "Today's Requests": "今日请求数",
  "Today Calls": "今日调用",
  "Today Tokens": "今日 Tokens",
  "30d Calls": "30天调用",
  "30d Tokens": "30天 Tokens",
  "Active Days": "活跃天数",
  "Local day request count": "本地当天请求总数",
  "Today's Success": "今日成功率",
  "HTTP status below 400": "HTTP 状态码低于 400",
  "Today's Tokens": "今日 Tokens",
  "Total Tokens": "总 Tokens",
  "Summed from today's upstream input + output usage":
    "汇总今天上游返回的输入 + 输出 usage",
  "Input and output tokens are counted. Cached tokens are included when providers report them.":
    "统计输入和输出 token；上游返回缓存 token 时也会计入。",
  "Avg Latency": "平均延迟",
  "Across today's successful requests": "仅基于今天成功请求统计",
  "P95 Latency": "P95 延迟",
  "Today's successful-request slow-tail indicator": "今日成功请求慢尾指标",
  "Today's Errors": "今日错误数",
  "HTTP 400+ or proxy failure": "HTTP 400+ 或代理失败",
  "Accounts Used": "使用账号数",
  "Distinct routed accounts today": "今天命中过的不同账号数",
  "Show metric note": "显示指标说明",
  "No daily stats yet.": "还没有每日统计数据。",
  "Max {value}": "峰值 {value}",
  "Stats refreshed.": "统计已刷新。",
  "Default missing": "缺少默认路由",
  "Default {name}": "默认 {name}",
};

const state = {
  daemonStatus: null,
  health: null,
  providers: [],
  accounts: [],
  routes: [],
  monitor: [],
  dashboardLogs: [],
  dailyStats: [],
  statsSelectedDay: null,
  onboarding: [],
  appSettings: null,
  lanIp: "",
  locale: detectInitialLocale(),
  activeTab: "dashboard",
  accountProviderFilter: "",
  accountProviderFilterTouched: false,
  onboardingTarget: "codex",
  onboardingProvider: null,
  providerEditor: null,
  accountEditor: null,
  routeEditor: null,
  openMetricTooltip: null,
  settingsDirty: false,
  rebuildingTokenStats: false,
  tokenRebuildStatus: "",
};
let pendingConfirmation = null;

const elements = {
  daemonChip: document.querySelector("#daemon-chip"),
  dbPath: document.querySelector("#db-path"),
  daemonPort: document.querySelector("#daemon-port"),
  startedAt: document.querySelector("#started-at"),
  detailStatus: document.querySelector("#detail-status"),
  detailPid: document.querySelector("#detail-pid"),
  daemonLogPath: document.querySelector("#daemon-log-path"),
  daemonLastExit: document.querySelector("#daemon-last-exit"),
  daemonLastError: document.querySelector("#daemon-last-error"),
  detailsButton: document.querySelector("#details-button"),
  detailsPanel: document.querySelector("#details-panel"),
  startDaemonButton: document.querySelector("#start-daemon-button"),
  stopDaemonButton: document.querySelector("#stop-daemon-button"),
  restartDaemonButton: document.querySelector("#restart-daemon-button"),
  openDaemonLogButton: document.querySelector("#open-daemon-log-button"),
  localeSelect: document.querySelector("#locale-select"),
  tabButtons: Array.from(document.querySelectorAll(".tab-button")),
  tabPanels: Array.from(document.querySelectorAll(".tab-panel")),
  metricsList: document.querySelector("#metrics-list"),
  statsSummaryList: document.querySelector("#stats-summary-list"),
  statsRequestsChart: document.querySelector("#stats-requests-chart"),
  statsTokensChart: document.querySelector("#stats-tokens-chart"),
  refreshStats: document.querySelector("#refresh-stats"),
  routeSummaryList: document.querySelector("#route-summary-list"),
  recentActivityList: document.querySelector("#recent-activity-list"),
  onboardingList: document.querySelector("#onboarding-list"),
  onboardingTargetTabs: document.querySelector("#onboarding-target-tabs"),
  onboardingProviderTabs: document.querySelector("#onboarding-provider-tabs"),
  refreshOnboarding: document.querySelector("#refresh-onboarding"),
  settingsForm: document.querySelector("#settings-form"),
  settingsDaemonPort: document.querySelector("#settings-daemon-port"),
  settingsAllowLan: document.querySelector("#settings-allow-lan"),
  settingsMonitorBuffer: document.querySelector("#settings-monitor-buffer"),
  settingsLogRetentionDays: document.querySelector(
    "#settings-log-retention-days",
  ),
  settingsLogsDir: document.querySelector("#settings-logs-dir"),
  settingsPickLogsDir: document.querySelector("#settings-pick-logs-dir"),
  settingsLanAddress: document.querySelector("#settings-lan-address"),
  settingsDefaultLogsDir: document.querySelector("#settings-default-logs-dir"),
  settingsDataRoot: document.querySelector("#settings-data-root"),
  settingsDatabasePath: document.querySelector("#settings-database-path"),
  settingsUseDefaultLogs: document.querySelector("#settings-use-default-logs"),
  settingsSubmit: document.querySelector("#settings-submit"),
  openSettingsLogsDir: document.querySelector("#open-settings-logs-dir"),
  rebuildTokenStats: document.querySelector("#rebuild-token-stats"),
  rebuildTokenStatus: document.querySelector("#rebuild-token-status"),
  openProviderDialog: document.querySelector("#open-provider-dialog"),
  providerDialog: document.querySelector("#provider-dialog"),
  closeProviderDialog: document.querySelector("#close-provider-dialog"),
  providerForm: document.querySelector("#provider-form"),
  providerFormError: document.querySelector("#provider-form-error"),
  providerFormTitle: document.querySelector("#provider-form-title"),
  providerFormCopy: document.querySelector("#provider-form-copy"),
  providerSubmit: document.querySelector("#provider-submit"),
  providerSlug: document.querySelector("#provider-slug"),
  providerName: document.querySelector("#provider-name"),
  providerProtocol: document.querySelector("#provider-protocol"),
  providerBaseUrl: document.querySelector("#provider-base-url"),
  providerDefaultModel: document.querySelector("#provider-default-model"),
  providerPath: document.querySelector("#provider-path"),
  providerPathDemo: document.querySelector("#provider-path-demo"),
  providerAuthHeader: document.querySelector("#provider-auth-header"),
  providerAuthPrefix: document.querySelector("#provider-auth-prefix"),
  providerEnabled: document.querySelector("#provider-enabled"),
  providersList: document.querySelector("#providers-list"),
  openAccountDialog: document.querySelector("#open-account-dialog"),
  accountDialog: document.querySelector("#account-dialog"),
  closeAccountDialog: document.querySelector("#close-account-dialog"),
  accountForm: document.querySelector("#account-form"),
  accountFormError: document.querySelector("#account-form-error"),
  accountFormTitle: document.querySelector("#account-form-title"),
  accountFormCopy: document.querySelector("#account-form-copy"),
  accountSubmit: document.querySelector("#account-submit"),
  accountId: document.querySelector("#account-id"),
  accountProvider: document.querySelector("#account-provider"),
  accountName: document.querySelector("#account-name"),
  accountApiKey: document.querySelector("#account-api-key"),
  accountKeyToggleView: document.querySelector("#account-key-toggle-view"),
  accountBaseUrl: document.querySelector("#account-base-url"),
  accountDefaultModel: document.querySelector("#account-default-model"),
  accountNote: document.querySelector("#account-note"),
  accountEnabled: document.querySelector("#account-enabled"),
  accountsProviderTabs: document.querySelector("#accounts-provider-tabs"),
  accountsList: document.querySelector("#accounts-list"),
  openRouteDialog: document.querySelector("#open-route-dialog"),
  routeDialog: document.querySelector("#route-dialog"),
  closeRouteDialog: document.querySelector("#close-route-dialog"),
  routeForm: document.querySelector("#route-form"),
  routeFormError: document.querySelector("#route-form-error"),
  routeFormTitle: document.querySelector("#route-form-title"),
  routeFormCopy: document.querySelector("#route-form-copy"),
  routeProvider: document.querySelector("#route-provider"),
  routePrefix: document.querySelector("#route-prefix"),
  routeAccount: document.querySelector("#route-account"),
  routeHint: document.querySelector("#route-hint"),
  routeSubmit: document.querySelector("#route-submit"),
  routesList: document.querySelector("#routes-list"),
  refreshRoutes: document.querySelector("#refresh-routes"),
  confirmDialog: document.querySelector("#confirm-dialog"),
  closeConfirmDialog: document.querySelector("#close-confirm-dialog"),
  confirmForm: document.querySelector("#confirm-form"),
  confirmDialogTitle: document.querySelector("#confirm-dialog-title"),
  confirmDialogCopy: document.querySelector("#confirm-dialog-copy"),
  confirmCancel: document.querySelector("#confirm-cancel"),
  confirmSubmit: document.querySelector("#confirm-submit"),
  monitorFilterForm: document.querySelector("#monitor-filter-form"),
  monitorProvider: document.querySelector("#monitor-provider"),
  monitorAccount: document.querySelector("#monitor-account"),
  monitorLimit: document.querySelector("#monitor-limit"),
  monitorList: document.querySelector("#monitor-list"),
  refreshMonitor: document.querySelector("#refresh-monitor"),
  openLogsRoot: document.querySelector("#open-logs-root"),
  toastStack: document.querySelector("#toast-stack"),
  emptyTemplate: document.querySelector("#empty-template"),
};
let liveMonitorRefreshing = false;
let liveDashboardRefreshing = false;
let liveStatsRefreshing = false;
let daemonDataRefreshing = false;

window.addEventListener("DOMContentLoaded", async () => {
  startUiDevPolling();
  initializeLocale();
  bindEvents();
  resetProviderForm();
  resetAccountForm();
  resetRouteForm();
  setActiveTab(state.activeTab);
  await syncDesktopLocale();
  await refreshAll();
  window.setInterval(async () => {
    await refreshDaemonStatus(true);
    await refreshHealth();
  }, 3000);
  window.setInterval(async () => {
    if (state.activeTab !== "monitor" || liveMonitorRefreshing) {
      return;
    }
    liveMonitorRefreshing = true;
    try {
      await refreshMonitor(true);
    } finally {
      liveMonitorRefreshing = false;
    }
  }, 1000);
  window.setInterval(async () => {
    if (state.activeTab !== "dashboard" || liveDashboardRefreshing) {
      return;
    }
    liveDashboardRefreshing = true;
    try {
      await refreshDashboardLogs();
    } finally {
      liveDashboardRefreshing = false;
    }
  }, 5000);
  window.setInterval(async () => {
    if (state.activeTab !== "stats" || liveStatsRefreshing) {
      return;
    }
    liveStatsRefreshing = true;
    try {
      await refreshDailyStats(true);
    } finally {
      liveStatsRefreshing = false;
    }
  }, 10000);
});

function sleep(ms) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function detectInitialLocale() {
  try {
    const stored = window.localStorage.getItem("localairouter.locale");
    if (stored) {
      return normalizeLocale(stored);
    }
    const legacyStored = window.localStorage.getItem("localopenrouter.locale");
    if (legacyStored) {
      return normalizeLocale(legacyStored);
    }
  } catch {
    // Ignore localStorage access failures.
  }
  return normalizeLocale(
    navigator.language || navigator.languages?.[0] || "en",
  );
}

function normalizeLocale(value) {
  return String(value || "")
    .toLowerCase()
    .startsWith("zh")
    ? "zh-CN"
    : "en";
}

function t(message, vars = {}) {
  const template =
    state.locale === "zh-CN" ? ZH_MESSAGES[message] || message : message;
  return template.replace(/\{(\w+)\}/g, (_, key) => String(vars[key] ?? ""));
}

function initializeLocale() {
  elements.localeSelect.value = state.locale;
  applyLocale(false);
}

async function setLocale(locale) {
  state.locale = normalizeLocale(locale);
  try {
    window.localStorage.setItem("localairouter.locale", state.locale);
  } catch {
    // Ignore localStorage access failures.
  }
  applyLocale(true);
  await syncDesktopLocale();
}

function applyLocale(rerender) {
  document.documentElement.lang = state.locale;
  document.querySelectorAll("[data-i18n]").forEach((node) => {
    node.textContent = t(node.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((node) => {
    node.setAttribute("placeholder", t(node.dataset.i18nPlaceholder));
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach((node) => {
    node.setAttribute("aria-label", t(node.dataset.i18nAriaLabel));
  });
  elements.localeSelect.value = state.locale;
  if (!rerender) {
    return;
  }
  renderChrome();
  renderDashboard();
  renderStats();
  renderProviders();
  renderAccountProviderTabs();
  renderAccounts();
  renderRoutes();
  renderMonitor();
  renderOnboarding();
  renderSettings();
  renderProviderPathDemo();
}

function hasDesktopIntegration() {
  return Boolean(
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke,
  );
}

async function syncDesktopLocale() {
  if (!hasDesktopIntegration()) {
    return;
  }
  try {
    await invokeDesktop("set_app_locale", { locale: state.locale });
  } catch (error) {
    console.error(error);
  }
}

async function refreshDesktopTrayMenu() {
  if (!hasDesktopIntegration()) {
    return;
  }
  try {
    await invokeDesktop("refresh_tray_menu");
  } catch (error) {
    console.error(error);
  }
}

function startUiDevPolling() {
  if (!isUiDevOrigin()) {
    return;
  }

  let currentVersion = null;
  const poll = async () => {
    try {
      const response = await fetch("/__dev__/version", { cache: "no-store" });
      if (!response.ok) {
        return;
      }
      const nextVersion = (await response.text()).trim();
      if (!nextVersion) {
        return;
      }
      if (currentVersion && currentVersion !== nextVersion) {
        window.location.reload();
        return;
      }
      currentVersion = nextVersion;
    } catch {
      // Ignore transient dev server polling failures.
    }
  };

  void poll();
  window.setInterval(poll, 900);
}

function isUiDevOrigin() {
  return (
    window.location.protocol === "http:" &&
    (window.location.hostname === "127.0.0.1" ||
      window.location.hostname === "localhost")
  );
}

function bindEvents() {
  elements.localeSelect.addEventListener("change", async () => {
    await setLocale(elements.localeSelect.value);
  });
  elements.tabButtons.forEach((button) => {
    button.addEventListener("click", () => setActiveTab(button.dataset.tab));
  });
  window.addEventListener("localairouter:navigate", (event) => {
    if (typeof event.detail === "string") {
      setActiveTab(event.detail);
    }
  });
  window.addEventListener("localopenrouter:navigate", (event) => {
    if (typeof event.detail === "string") {
      setActiveTab(event.detail);
    }
  });
  window.addEventListener("localairouter:refresh", async () => {
    await refreshProviders();
    await refreshAccounts();
    await refreshRoutes();
    await refreshOnboarding();
    await refreshDashboardLogs();
    await refreshDailyStats(true);
  });
  window.addEventListener("localopenrouter:refresh", async () => {
    await refreshProviders();
    await refreshAccounts();
    await refreshRoutes();
    await refreshOnboarding();
    await refreshDashboardLogs();
    await refreshDailyStats(true);
  });

  elements.detailsButton.addEventListener("click", (event) => {
    event.stopPropagation();
    toggleDetailsPanel();
  });

  document.addEventListener("click", (event) => {
    if (!event.target.closest(".details-anchor")) {
      closeDetailsPanel();
    }
    if (!event.target.closest(".metric-anchor")) {
      closeMetricTooltips();
    }
  });

  elements.accountDialog.addEventListener("click", (event) => {
    if (event.target === elements.accountDialog) {
      closeAccountDialog();
    }
  });
  elements.routeDialog.addEventListener("click", (event) => {
    if (event.target === elements.routeDialog) {
      closeRouteDialog();
    }
  });
  elements.confirmDialog.addEventListener("click", (event) => {
    if (event.target === elements.confirmDialog) {
      closeConfirmDialog();
    }
  });
  elements.confirmDialog.addEventListener("close", () => {
    pendingConfirmation = null;
  });
  elements.providerDialog.addEventListener("close", resetProviderForm);
  elements.accountDialog.addEventListener("close", resetAccountForm);
  elements.routeDialog.addEventListener("close", resetRouteForm);
  elements.closeProviderDialog.addEventListener("click", closeProviderDialog);
  elements.closeAccountDialog.addEventListener("click", closeAccountDialog);
  elements.closeRouteDialog.addEventListener("click", closeRouteDialog);
  elements.closeConfirmDialog.addEventListener("click", closeConfirmDialog);
  elements.confirmCancel.addEventListener("click", closeConfirmDialog);
  elements.confirmForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const action = pendingConfirmation;
    pendingConfirmation = null;
    closeDialog(elements.confirmDialog);
    if (action) {
      await action();
    }
  });

  elements.startDaemonButton.addEventListener("click", async () => {
    const status = await performDesktop(
      () => invokeDesktop("start_daemon"),
      t("Daemon started."),
      t("Failed to start daemon."),
    );
    if (!status) {
      await refreshDaemonStatus(false);
      return;
    }
    state.daemonStatus = status;
    await refreshHealth({ attempts: 8, delayMs: 350 });
    await refreshAll();
  });

  elements.stopDaemonButton.addEventListener("click", async () => {
    const status = await performDesktop(
      () => invokeDesktop("stop_daemon"),
      t("Daemon stopped."),
      t("Failed to stop daemon."),
    );
    if (!status) {
      await refreshDaemonStatus(false);
      return;
    }
    state.daemonStatus = status;
    state.health = null;
    syncDaemonPanels();
    renderChrome();
    renderDashboard();
  });

  elements.restartDaemonButton.addEventListener("click", async () => {
    const status = await performDesktop(
      () => invokeDesktop("restart_daemon"),
      t("Daemon restarted."),
      t("Failed to restart daemon."),
    );
    if (!status) {
      await refreshDaemonStatus(false);
      return;
    }
    state.daemonStatus = status;
    await refreshHealth({ attempts: 8, delayMs: 350 });
    await refreshAll();
  });

  elements.openDaemonLogButton.addEventListener("click", async () => {
    await performDesktop(
      () => invokeDesktop("open_daemon_log"),
      t("Daemon log opened."),
      t("Failed to open daemon log."),
    );
  });

  elements.refreshOnboarding.addEventListener("click", async () => {
    await refreshOnboarding();
    notify(t("Onboarding guides refreshed."), "info");
  });

  elements.refreshRoutes.addEventListener("click", async () => {
    await refreshRoutes();
    renderDashboard();
    await refreshDesktopTrayMenu();
    notify(t("Routes refreshed."), "info");
  });

  elements.refreshMonitor.addEventListener("click", async () => {
    await refreshMonitor(false);
    notify(t("Logs refreshed."), "info");
  });
  elements.refreshStats?.addEventListener("click", async () => {
    await refreshDailyStats(false);
    notify(t("Stats refreshed."), "info");
  });
  elements.monitorFilterForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    await refreshMonitor(true);
  });

  elements.openLogsRoot.addEventListener("click", async () => {
    await performDesktop(
      () => invokeDesktop("open_logs_root"),
      t("Log root opened."),
      t("Failed to open log root."),
    );
  });

  elements.openSettingsLogsDir.addEventListener("click", async () => {
    await performDesktop(
      () => invokeDesktop("open_logs_root"),
      t("Log root opened."),
      t("Failed to open log root."),
    );
  });
  elements.settingsDaemonPort.addEventListener("input", () => {
    state.settingsDirty = true;
    renderSettings();
  });
  elements.settingsAllowLan.addEventListener("change", async () => {
    state.settingsDirty = true;
    if (elements.settingsAllowLan.checked && !state.lanIp) {
      await refreshLanIp(false);
    }
    renderSettings();
  });
  elements.settingsMonitorBuffer.addEventListener("input", () => {
    state.settingsDirty = true;
  });
  elements.settingsLogRetentionDays.addEventListener("input", () => {
    state.settingsDirty = true;
  });
  elements.settingsLogsDir.addEventListener("input", () => {
    state.settingsDirty = true;
  });
  elements.settingsPickLogsDir.addEventListener("click", async () => {
    const selected = await performDesktop(
      () =>
        invokeDesktop("pick_logs_directory", {
          initialPath:
            normalizeOptional(elements.settingsLogsDir.value) ||
            state.appSettings?.logsDir ||
            state.appSettings?.defaultLogsDir ||
            null,
        }),
      null,
      t("Failed to choose logs directory."),
    );
    if (!selected) {
      return;
    }
    elements.settingsLogsDir.value = selected;
    state.settingsDirty = true;
  });
  elements.settingsUseDefaultLogs.addEventListener("click", () => {
    elements.settingsLogsDir.value = state.appSettings?.defaultLogsDir || "";
    state.settingsDirty = true;
  });
  elements.rebuildTokenStats?.addEventListener("click", async () => {
    if (state.rebuildingTokenStats) {
      return;
    }
    state.rebuildingTokenStats = true;
    state.tokenRebuildStatus = t("Rebuilding historical token stats…");
    renderSettings();
    try {
      const report = await rebuildTokenStats();
      state.tokenRebuildStatus = t(
        "Token stats rebuild finished. Updated {updated} logs, skipped {skipped}.",
        {
          updated: report?.updatedLogs ?? 0,
          skipped: report?.skippedLogs ?? 0,
        },
      );
      await refreshDashboardLogs();
      await refreshDailyStats(true);
      notify(
        t("Token stats rebuilt. Updated {updated} / {total} logs.", {
          updated: report?.updatedLogs ?? 0,
          total: report?.totalLogs ?? 0,
        }),
        "success",
      );
    } catch (error) {
      console.error(error);
      state.tokenRebuildStatus =
        error?.message || t("Failed to rebuild token stats.");
    } finally {
      state.rebuildingTokenStats = false;
      renderSettings();
    }
  });
  elements.settingsForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const daemonPort = Number(
      elements.settingsDaemonPort.value || DEFAULT_PORT,
    );
    const allowLanAccess = Boolean(elements.settingsAllowLan.checked);
    const monitorBufferLimit = Number(
      elements.settingsMonitorBuffer.value || DEFAULT_MONITOR_BUFFER_LIMIT,
    );
    const logRetentionDays = Number(
      elements.settingsLogRetentionDays.value || DEFAULT_LOG_RETENTION_DAYS,
    );
    const logsDir = normalizeOptional(elements.settingsLogsDir.value);
    const saved = await performDesktop(
      () =>
        invokeDesktop("save_app_settings_command", {
          input: {
            daemonPort,
            allowLanAccess,
            monitorBufferLimit,
            logRetentionDays,
            logsDir,
          },
        }),
      null,
      t("Failed to save settings."),
    );
    if (!saved) {
      return;
    }
    state.appSettings = saved;
    state.settingsDirty = false;
    const wasRunning = Boolean(state.daemonStatus?.running);
    if (wasRunning) {
      const status = await performDesktop(
        () => invokeDesktop("restart_daemon"),
        null,
        t("Failed to restart daemon."),
      );
      if (!status) {
        await refreshDaemonStatus(false);
        renderSettings();
        return;
      }
      state.daemonStatus = status;
      state.health = null;
      await refreshHealth({ attempts: 8, delayMs: 350 });
    } else {
      await refreshDaemonStatus(true);
    }
    await refreshAppSettings(true);
    await refreshOnboarding();
    renderProviderPathDemo();
    notify(
      wasRunning
        ? t("Settings saved. Daemon restarted.")
        : t("Settings saved."),
      "success",
    );
  });

  elements.openProviderDialog.addEventListener("click", () => {
    resetProviderForm();
    openDialog(elements.providerDialog, elements.providerName);
  });
  elements.providerProtocol.addEventListener("change", () =>
    applyProviderProtocolDefaults(true),
  );
  elements.providerName.addEventListener("input", () => syncProviderIdentity());
  elements.providerPath.addEventListener("input", () => {
    elements.providerPath.dataset.autofill = "off";
    syncGeneratedProviderSlug();
    renderProviderPathDemo();
  });

  elements.providerForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    clearFormError(elements.providerFormError);
    const payload = buildProviderPayload();
    if (!payload) {
      return;
    }

    const isEditing = Boolean(state.providerEditor);
    const provider = await perform(
      () =>
        api("/admin/providers", {
          method: "POST",
          body: payload,
          silent: true,
        }),
      isEditing ? t("Provider updated.") : t("Provider saved."),
      (error) => {
        showFormError(elements.providerFormError, error);
      },
    );
    if (!provider) {
      return;
    }

    closeProviderDialog();
    await refreshProviders();
    await refreshAccounts();
    await refreshRoutes();
    await refreshOnboarding();
    await refreshDesktopTrayMenu();
    renderDashboard();
  });

  elements.openAccountDialog.addEventListener("click", () => {
    resetAccountForm();
    openDialog(elements.accountDialog, elements.accountName);
  });
  elements.accountProvider.addEventListener("change", () => {});

  if (elements.accountKeyToggleView) {
    elements.accountKeyToggleView.addEventListener("click", () => {
      toggleAccountApiKeyVisibility();
    });
  }

  elements.accountForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    clearFormError(elements.accountFormError);
    const payload = buildAccountPayload();
    if (!payload) {
      return;
    }

    const isEditing = Boolean(state.accountEditor);
    const account = await perform(
      () =>
        api("/admin/accounts", {
          method: "POST",
          body: payload,
          silent: true,
        }),
      isEditing ? t("Account updated.") : t("Account saved."),
      (error) => {
        showFormError(elements.accountFormError, error);
      },
    );
    if (!account) {
      return;
    }

    state.accountProviderFilter = account.provider;
    closeAccountDialog();
    await refreshAccounts();
    await refreshRoutes();
    await refreshDesktopTrayMenu();
    renderDashboard();
  });

  elements.openRouteDialog.addEventListener("click", () => {
    resetRouteForm();
    openDialog(elements.routeDialog, elements.routeProvider);
  });

  elements.routeForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    clearFormError(elements.routeFormError);
    const payload = buildRoutePayload();
    if (!payload) {
      return;
    }

    const previousId = state.routeEditor?.id;
    const nextId = routeBindingId(payload.provider, payload.modelPrefix);
    const route = await perform(
      () =>
        api("/admin/routes", {
          method: "POST",
          body: payload,
          silent: true,
        }),
      state.routeEditor ? t("Route updated.") : t("Route saved."),
      (error) => {
        showFormError(elements.routeFormError, error);
      },
    );
    if (!route) {
      return;
    }

    if (previousId && previousId !== nextId) {
      await perform(
        () => api(`/admin/routes/${previousId}`, { method: "DELETE" }),
        null,
      );
    }

    closeRouteDialog();
    await refreshRoutes();
    await refreshDesktopTrayMenu();
    renderDashboard();
  });

  elements.routeProvider.addEventListener("change", syncRouteAccountOptions);
}

async function refreshAll() {
  await refreshAppSettings(true);
  await refreshDaemonStatus(true);
  const healthy = await refreshHealth({
    attempts: 20,
    delayMs: 500,
    refreshDataOnOnline: false,
  });
  if (!healthy) {
    return;
  }
  await refreshDaemonBackedData();
}

async function refreshDaemonBackedData() {
  if (daemonDataRefreshing) {
    return;
  }
  daemonDataRefreshing = true;
  try {
    await refreshProviders();
    await refreshAccounts();
    await refreshRoutes();
    await refreshOnboarding();
    await refreshMonitor(true);
    await refreshDashboardLogs();
    await refreshDailyStats(true);
    await refreshDesktopTrayMenu();
    renderDashboard();
    renderStats();
  } finally {
    daemonDataRefreshing = false;
  }
}

async function refreshAppSettings(silent = true) {
  if (!hasDesktopIntegration()) {
    state.appSettings = null;
    state.lanIp = "";
    renderSettings();
    return;
  }
  try {
    state.appSettings = await invokeDesktop("get_app_settings");
    state.settingsDirty = false;
    if (state.appSettings?.allowLanAccess) {
      await refreshLanIp(true);
    } else {
      state.lanIp = "";
    }
  } catch (error) {
    if (!silent) {
      notify(error?.message || t("Failed to save settings."), "error");
    }
    console.error(error);
  }
  renderSettings();
}

async function refreshLanIp(silent = true) {
  if (!hasDesktopIntegration()) {
    state.lanIp = "";
    return;
  }
  try {
    state.lanIp = (await invokeDesktop("local_lan_ip")) || "";
  } catch (error) {
    state.lanIp = "";
    if (!silent) {
      notify(
        error?.message ||
          t("Unable to determine LAN IP. Check your network connection."),
        "error",
      );
    }
    console.error(error);
  }
}

async function refreshDaemonStatus(silent = true) {
  try {
    state.daemonStatus = await invokeDesktop("daemon_status");
  } catch (error) {
    state.daemonStatus = null;
    if (!silent) {
      notify(
        t("Cannot read daemon process status from the desktop host."),
        "error",
      );
    }
    console.error(error);
  }
  if (!state.health) {
    if (state.daemonStatus?.running) {
      setDaemonChip(t("Daemon starting"), "warn");
    } else {
      setDaemonChip(t("Daemon offline"), "bad");
    }
  }
  syncDaemonPanels();
  renderChrome();
  renderDashboard();
  renderSettings();
}

async function refreshHealth(options = {}) {
  const attempts = options.attempts || 1;
  const delayMs = options.delayMs || 0;

  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      const wasOffline = !state.health;
      state.health = await api("/health", { silent: true });
      const tone = "ok";
      const chipText = t("Daemon Online");
      setDaemonChip(chipText, tone);
      syncDaemonPanels();
      renderChrome();
      renderDashboard();
      renderSettings();
      if (wasOffline && options.refreshDataOnOnline !== false) {
        void refreshDaemonBackedData();
      }
      return true;
    } catch (error) {
      state.health = null;
      if (attempt < attempts - 1) {
        await sleep(delayMs);
        continue;
      }
      await refreshDaemonStatus(true);
      if (!state.daemonStatus?.running) {
        setDaemonChip(t("Daemon offline"), "bad");
      } else {
        setDaemonChip(t("Daemon unreachable"), "warn");
      }
      syncDaemonPanels();
      console.error(error);
    }
  }
  renderChrome();
  renderDashboard();
  renderSettings();
  return false;
}

async function refreshProviders() {
  try {
    state.providers = await api("/admin/providers", { silent: true });
  } catch (error) {
    console.error(error);
  }
  renderProviders();
  renderAccountProviderTabs();
  syncProviderOptions();
  syncMonitorProviderOptions();
  await refreshOnboarding();
  renderDashboard();
}

async function refreshAccounts() {
  try {
    state.accounts = await api("/admin/accounts", { silent: true });
  } catch (error) {
    console.error(error);
  }
  renderAccountProviderTabs();
  renderAccounts();
  syncProviderOptions();
  syncRouteAccountOptions();
  syncMonitorAccountOptions();
  await refreshOnboarding();
  renderDashboard();
}

async function refreshRoutes() {
  try {
    state.routes = await api("/admin/routes", { silent: true });
  } catch (error) {
    console.error(error);
  }
  renderAccounts();
  renderRoutes();
  await refreshOnboarding();
  renderDashboard();
}

async function refreshDashboardLogs() {
  try {
    state.dashboardLogs = await fetchLogs(todayLogQuery(), true);
  } catch (error) {
    state.dashboardLogs = [];
    console.error(error);
  }
  try {
    state.dailyStats = await fetchDailyStats(30, true);
  } catch (error) {
    state.dailyStats = [];
    console.error(error);
  }
  renderDashboard();
}

async function refreshDailyStats(silent = true) {
  try {
    state.dailyStats = await fetchDailyStats(30, silent);
  } catch (error) {
    state.dailyStats = [];
    console.error(error);
  }
  renderStats();
}

async function refreshMonitor(silent = true) {
  try {
    state.monitor = await fetchMonitor(
      {
        provider: elements.monitorProvider.value,
        accountId: elements.monitorAccount.value,
        limit: elements.monitorLimit.value || "60",
      },
      silent,
    );
  } catch (error) {
    state.monitor = [];
    console.error(error);
  }
  renderMonitor();
}

async function refreshOnboarding() {
  const availableTargets = availableOnboardingTargets();
  if (!availableTargets.length) {
    state.onboarding = [];
    state.onboardingProvider = null;
    renderOnboarding();
    return;
  }

  if (
    !availableTargets.some((target) => target.id === state.onboardingTarget)
  ) {
    state.onboardingTarget = availableTargets[0].id;
  }

  const providers = onboardingProvidersForTarget(state.onboardingTarget);
  if (
    !providers.some((provider) => provider.slug === state.onboardingProvider)
  ) {
    state.onboardingProvider = providers[0]?.slug ?? null;
  }

  const selectedProvider = providers.find(
    (provider) => provider.slug === state.onboardingProvider,
  );
  state.onboarding = selectedProvider
    ? [buildOnboardingGuide(state.onboardingTarget, selectedProvider)]
    : [];
  renderOnboarding();
}

async function fetchLogs(filters, silent) {
  const params = new URLSearchParams();
  if (filters.provider) {
    params.set("provider", filters.provider);
  }
  if (filters.accountId) {
    params.set("accountId", filters.accountId);
  }
  if (filters.createdFrom) {
    params.set("createdFrom", filters.createdFrom);
  }
  if (filters.createdTo) {
    params.set("createdTo", filters.createdTo);
  }
  if (filters.limit) {
    params.set("limit", String(filters.limit));
  }
  return api(`/admin/logs?${params.toString()}`, { silent });
}

async function fetchLog(logId, silent) {
  return api(`/admin/logs/${encodeURIComponent(logId)}`, { silent });
}

async function fetchDailyStats(days, silent) {
  const params = new URLSearchParams();
  params.set("days", String(days || 30));
  params.set("utcOffsetMinutes", String(currentUtcOffsetMinutes()));
  return api(`/admin/stats/daily?${params.toString()}`, { silent });
}

async function rebuildTokenStats() {
  return api("/admin/stats/rebuild-tokens", {
    method: "POST",
    silent: false,
  });
}

async function fetchMonitor(filters, silent) {
  const params = new URLSearchParams();
  if (filters.provider) {
    params.set("provider", filters.provider);
  }
  if (filters.accountId) {
    params.set("accountId", filters.accountId);
  }
  params.set("limit", String(filters.limit || 60));
  return api(`/admin/monitor?${params.toString()}`, { silent });
}

function renderDashboard() {
  renderMetrics();
  renderRouteSummary();
  renderRecentActivity();
}

function renderStats() {
  const series = buildDailyStatsSeries(30);
  syncSelectedStatsDay(series);
  renderStatsSummary(series);
  renderStatsChart(
    elements.statsRequestsChart,
    series,
    "requestCount",
    (value) => String(value),
    "requests",
  );
  renderStatsChart(
    elements.statsTokensChart,
    series,
    "totalTokens",
    (value) => formatTokenCount(value),
    "tokens",
  );
}

function syncSelectedStatsDay(series) {
  if (!series.length) {
    state.statsSelectedDay = null;
    return null;
  }
  const selected = series.find((point) => point.day === state.statsSelectedDay);
  if (selected) {
    return selected;
  }
  state.statsSelectedDay = series[series.length - 1].day;
  return series[series.length - 1];
}

function renderStatsSummary(series) {
  if (!elements.statsSummaryList) {
    return;
  }
  if (!state.dailyStats.length) {
    elements.statsSummaryList.replaceChildren(
      emptyNode(t("No daily stats yet.")),
    );
    return;
  }

  const today = series[series.length - 1] || {
    requestCount: 0,
    totalTokens: 0,
  };
  const totalRequests = series.reduce(
    (sum, point) => sum + point.requestCount,
    0,
  );
  const totalTokens = series.reduce((sum, point) => sum + point.totalTokens, 0);
  const summaryItems = [
    { label: t("Today Calls"), value: String(today.requestCount) },
    { label: t("Today Tokens"), value: formatTokenCount(today.totalTokens) },
    { label: t("30d Calls"), value: String(totalRequests) },
    { label: t("30d Tokens"), value: formatTokenCount(totalTokens) },
  ];

  const items = summaryItems.map((item) => {
    const node = document.createElement("article");
    node.className = "stats-summary-card";
    node.innerHTML = `
      <p class="stats-summary-label">${escapeHtml(item.label)}</p>
      <div class="stats-summary-value">${escapeHtml(item.value)}</div>
    `;
    return node;
  });
  elements.statsSummaryList.replaceChildren(...items);
}

function renderStatsChart(container, series, key, formatter, tone) {
  if (!container) {
    return;
  }
  if (!state.dailyStats.length) {
    container.replaceChildren(emptyNode(t("No daily stats yet.")));
    return;
  }

  const selectedPoint = syncSelectedStatsDay(series);
  const maxValue = Math.max(
    1,
    ...series.map((point) => finiteNumber(point[key]) ?? 0),
  );
  const detail = document.createElement("div");
  detail.className = "stats-chart-detail";
  const detailValue = selectedPoint
    ? (finiteNumber(selectedPoint[key]) ?? 0)
    : 0;
  const detailMeta =
    key === "requestCount"
      ? `${selectedPoint?.successCount ?? 0} ${t("Success")}`
      : t("Click a column to inspect the exact day.");
  const detailValueLabel =
    key === "requestCount"
      ? `${formatter(detailValue)} ${t("Requests")}`
      : `${formatter(detailValue)} ${t("Tokens")}`;
  detail.innerHTML = `
    <span class="stats-chart-detail-label">${escapeHtml(t("Selected Day"))}</span>
    <strong class="stats-chart-detail-day">${escapeHtml(selectedPoint?.day || "--")}</strong>
    <span class="stats-chart-detail-value">${escapeHtml(detailValueLabel)}</span>
    <span class="stats-chart-detail-meta">${escapeHtml(detailMeta)}</span>
  `;
  const wrapper = document.createElement("div");
  wrapper.className = "stats-chart-bars";

  series.forEach((point, index) => {
    const value = finiteNumber(point[key]) ?? 0;
    const height = value > 0 ? Math.max((value / maxValue) * 100, 4) : 2;
    const showDay =
      index === 0 || index === series.length - 1 || index % 5 === 4;
    const selected = point.day === state.statsSelectedDay;
    const tooltipValue =
      key === "requestCount"
        ? `${formatter(value)} ${t("Requests")}`
        : `${formatter(value)} ${t("Tokens")}`;
    const tooltipMeta =
      key === "requestCount" ? `${point.successCount} ${t("Success")}` : "";
    const column = document.createElement("button");
    column.type = "button";
    column.className = `stats-chart-column ${tone}${selected ? " is-active" : ""}`;
    column.setAttribute("aria-pressed", String(selected));
    column.setAttribute("title", `${point.day}: ${formatter(value)}`);
    column.addEventListener("click", () => {
      state.statsSelectedDay = point.day;
      renderStats();
    });
    column.innerHTML = `
      <div class="stats-chart-tooltip" role="presentation">
        <strong class="stats-chart-tooltip-day">${escapeHtml(point.day)}</strong>
        <span class="stats-chart-tooltip-value">${escapeHtml(tooltipValue)}</span>
        ${tooltipMeta ? `<span class="stats-chart-tooltip-meta">${escapeHtml(tooltipMeta)}</span>` : ""}
      </div>
      <div class="stats-chart-column-inner">
        <div class="stats-chart-bar" style="height: ${height}%"></div>
      </div>
      <span class="stats-chart-day">${escapeHtml(showDay ? point.day.slice(5) : "")}</span>
    `;
    wrapper.appendChild(column);
  });

  const meta = document.createElement("p");
  meta.className = "muted compact-note";
  meta.textContent = t("Max {value}", { value: formatter(maxValue) });
  container.replaceChildren(detail, wrapper, meta);
}

function renderMetrics() {
  const todayLogs = state.dashboardLogs;
  const todayStats = todayDailyStats();
  const latencyLogs = latencyMetricLogs(todayLogs);
  const tokenUsage = aggregateTokenUsage(todayLogs);
  const requestCount = todayStats?.requestCount ?? todayLogs.length;
  const successCount =
    todayStats?.successCount ??
    todayLogs.filter((log) => isSuccessStatus(log.statusCode)).length;
  const totalTokens = todayStats?.totalTokens ?? tokenUsage.total;
  const accountsUsed = new Set(
    todayLogs
      .map((log) => log.accountId)
      .filter((accountId) => typeof accountId === "string" && accountId),
  ).size;
  const metrics = [
    {
      key: "requests",
      label: t("Today's Requests"),
      value: String(requestCount),
      note: t("Local day request count"),
      tone: "accent",
    },
    {
      key: "success",
      label: t("Today's Success"),
      value: formatSuccessRateFromCounts(requestCount, successCount),
      note: t("HTTP status below 400"),
      tone: "accent",
    },
    {
      key: "total-tokens",
      label: t("Today's Tokens"),
      value: formatTokenCount(totalTokens),
      note: t(
        "Input and output tokens are counted. Cached tokens are included when providers report them.",
      ),
      tone: "warm",
    },
    {
      key: "avg-latency",
      label: t("Avg Latency"),
      value: formatLatency(averageLatency(latencyLogs)),
      note: t("Across today's successful requests"),
    },
    {
      key: "p95-latency",
      label: t("P95 Latency"),
      value: formatLatency(percentileLatency(latencyLogs, 0.95)),
      note: t("Today's successful-request slow-tail indicator"),
      tone: "warm",
    },
    {
      key: "accounts-used",
      label: t("Accounts Used"),
      value: String(accountsUsed),
      note: t("Distinct routed accounts today"),
      tone: "warm",
    },
  ];

  elements.metricsList.replaceChildren(
    ...(metrics.length
      ? metrics.map((metric) => {
          const isOpen = state.openMetricTooltip === metric.key;
          const card = document.createElement("article");
          card.className = `metric-card${metric.tone ? ` ${metric.tone}` : ""}`;
          card.innerHTML = `
            <div class="metric-head">
              <p class="metric-label">${escapeHtml(metric.label)}</p>
              <div class="metric-anchor">
                <button
                  type="button"
                  class="metric-help"
                  aria-label="${escapeHtml(t("Show metric note"))}"
                  aria-expanded="${String(isOpen)}"
                >i</button>
                <div class="metric-tooltip"${isOpen ? "" : " hidden"}>${escapeHtml(metric.note)}</div>
              </div>
            </div>
            <div class="metric-value">${escapeHtml(metric.value)}</div>
          `;
          const helpButton = card.querySelector(".metric-help");
          const tooltip = card.querySelector(".metric-tooltip");
          helpButton?.classList.toggle("is-open", isOpen);
          helpButton?.addEventListener("click", (event) => {
            event.stopPropagation();
            const shouldOpen = state.openMetricTooltip !== metric.key;
            closeMetricTooltips(false);
            if (!tooltip) {
              state.openMetricTooltip = shouldOpen ? metric.key : null;
              return;
            }
            state.openMetricTooltip = shouldOpen ? metric.key : null;
            tooltip.hidden = !shouldOpen;
            helpButton.classList.toggle("is-open", shouldOpen);
            helpButton.setAttribute("aria-expanded", String(shouldOpen));
          });
          tooltip?.addEventListener("click", (event) => {
            event.stopPropagation();
          });
          return card;
        })
      : [emptyNode()]),
  );
}

function renderRouteSummary() {
  const items = state.providers.map((provider) => {
    const enabledAccounts = state.accounts.filter(
      (account) => account.provider === provider.slug && account.enabled,
    );
    const defaultRoute = state.routes.find(
      (route) => route.provider === provider.slug && !route.modelPrefix,
    );
    const defaultAccount = defaultRoute
      ? state.accounts.find((account) => account.id === defaultRoute.accountId)
      : null;
    const overrideCount = state.routes.filter(
      (route) => route.provider === provider.slug && route.modelPrefix,
    ).length;
    const localUrl = providerLocalBaseUrl(provider);
    const routeSummary = [
      defaultRoute
        ? t("Default {name}", {
            name: defaultAccount ? defaultAccount.name : t("unassigned"),
          })
        : t("Default missing"),
      t("{count} acct", { count: enabledAccounts.length }),
      t("{count} ovrd", { count: overrideCount }),
      ...(provider.defaultModel
        ? [t("default model {model}", { model: provider.defaultModel })]
        : []),
      provider.isBuiltin ? t("built-in") : t("custom"),
      `/${provider.proxyPath}`,
    ].join(" · ");

    const item = document.createElement("article");
    item.className = "summary-item";
    item.innerHTML = `
      <div class="summary-head">
        <h3 title="${escapeHtml(`${provider.displayName} · ${provider.protocol}`)}">
          ${escapeHtml(provider.displayName)} <span class="summary-protocol">${escapeHtml(protocolDisplayLabel(provider.protocol))}</span>
        </h3>
        <span class="summary-state ${provider.enabled ? "ok" : "bad"}">${provider.enabled ? t("enabled") : t("disabled")}</span>
      </div>
      <p class="summary-line muted" title="${escapeHtml(provider.baseUrl)}">${escapeHtml(provider.baseUrl)}</p>
      <p class="summary-line" title="${escapeHtml(routeSummary)}">${escapeHtml(routeSummary)}</p>
      <p class="summary-url" title="${escapeHtml(localUrl)}">${escapeHtml(localUrl)}</p>
    `;
    return item;
  });

  elements.routeSummaryList.replaceChildren(
    ...(items.length ? items : [emptyNode()]),
  );
}

function renderRecentActivity() {
  const items = state.dashboardLogs.slice(0, 5).map((log) => {
    const provider = getProvider(log.provider);
    const account = state.accounts.find(
      (candidate) => candidate.id === log.accountId,
    );
    const detail = `${account?.name || log.accountId || t("No account")} · ${log.model || t("model unavailable")}`;
    const item = document.createElement("article");
    item.className = "activity-item";
    item.innerHTML = `
      <div class="activity-inline-row">
        <div class="meta-row">
          <span class="pill">${escapeHtml(provider ? provider.displayName : log.provider)}</span>
          <span class="pill ${isSuccessStatus(log.statusCode) ? "ok" : "bad"}">${escapeHtml(String(log.statusCode ?? "error"))}</span>
          <span class="pill">${escapeHtml(formatLatency(log.durationMs))}</span>
          <span class="pill">${escapeHtml(formatRelativeTime(log.createdAt))}</span>
        </div>
        <p class="activity-line" title="${escapeHtml(`${log.path} · ${detail}`)}">
          ${escapeHtml(log.path)} · ${escapeHtml(detail)}
        </p>
      </div>
    `;
    return item;
  });

  elements.recentActivityList.replaceChildren(
    ...(items.length ? items : [emptyNode()]),
  );
}

function renderProviders() {
  const items = state.providers.map((provider) => {
    const localIngressUrl = providerLocalBaseUrl(provider);
    const syncActionLabel =
      hasDesktopIntegration() && provider.protocol === "openai"
        ? t("Sync Codex")
        : hasDesktopIntegration() && provider.protocol === "anthropic"
          ? t("Sync Claude")
          : null;
    const providerDetails = [
      provider.baseUrl,
      ...(provider.defaultModel
        ? [t("default model {model}", { model: provider.defaultModel })]
        : []),
    ].join(" · ");
    const item = document.createElement("article");
    item.className = "data-item";
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(provider.displayName)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(providerDetails)}</p>
        </div>
        <span class="pill ${provider.enabled ? "ok" : "bad"}">${provider.enabled ? t("enabled") : t("disabled")}</span>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(provider.slug)}</span>
        <span class="pill">${escapeHtml(protocolDisplayLabel(provider.protocol))}</span>
        <span class="pill" title="${escapeHtml(localIngressUrl)}">${escapeHtml(localIngressUrl)}</span>
        <span class="pill">${escapeHtml(provider.authHeader)}${provider.authPrefix ? `: ${escapeHtml(provider.authPrefix)}` : ""}</span>
        <span class="pill">${escapeHtml(t("updated {time}", { time: formatRelativeTime(provider.updatedAt) }))}</span>
        ${provider.isBuiltin ? `<span class="pill warm">${escapeHtml(t("built-in"))}</span>` : ""}
      </div>
      <div class="actions">
        <button type="button" class="ghost">${escapeHtml(t("Edit"))}</button>
        ${
          syncActionLabel
            ? `<button type="button" class="ghost provider-sync-config">${escapeHtml(syncActionLabel)}</button>`
            : ""
        }
        ${provider.isBuiltin ? "" : `<button type="button" class="ghost">${escapeHtml(t("Delete"))}</button>`}
      </div>
    `;
    const buttons = item.querySelectorAll("button");
    buttons[0].addEventListener("click", () => {
      fillProviderForm(provider);
      openDialog(elements.providerDialog, elements.providerName);
    });
    const syncButton = item.querySelector(".provider-sync-config");
    syncButton?.addEventListener("click", async () => {
      await syncProviderConfig(provider);
    });
    const deleteButton = provider.isBuiltin
      ? null
      : item.querySelectorAll(".actions button")[syncActionLabel ? 2 : 1];
    if (!provider.isBuiltin && deleteButton) {
      deleteButton.addEventListener("click", async () => {
        requestConfirmation({
          title: t("Delete Provider: {name}", { name: provider.displayName }),
          message: t(
            "Delete this provider definition? This only succeeds after all dependent accounts and routes are removed.",
          ),
          confirmLabel: t("Delete Provider"),
          onConfirm: async () => {
            const response = await perform(
              () =>
                api(`/admin/providers/${provider.slug}`, { method: "DELETE" }),
              t("Provider {name} deleted.", { name: provider.displayName }),
            );
            if (!response) {
              return;
            }
            if (state.providerEditor?.slug === provider.slug) {
              closeProviderDialog();
            }
            await refreshProviders();
            await refreshAccounts();
            await refreshRoutes();
            renderDashboard();
          },
        });
      });
    }
    return item;
  });

  elements.providersList.replaceChildren(
    ...(items.length ? items : [emptyNode()]),
  );
}

function renderAccountProviderTabs() {
  const activeFilter = normalizeAccountProviderFilter();
  const providers = state.providers.map((provider) => ({
    slug: provider.slug,
    label: provider.displayName,
    count: state.accounts.filter(
      (account) => account.provider === provider.slug,
    ).length,
  }));

  const buttons = providers.map((entry) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `filter-tab${entry.slug === activeFilter ? " is-active" : ""}`;
    button.textContent = t("{label} ({count})", {
      label: entry.label,
      count: entry.count,
    });
    button.addEventListener("click", () => {
      if (state.accountProviderFilter === entry.slug) {
        return;
      }
      state.accountProviderFilterTouched = true;
      state.accountProviderFilter = entry.slug;
      renderAccountProviderTabs();
      renderAccounts();
    });
    return button;
  });

  elements.accountsProviderTabs.replaceChildren(...buttons);
}

function renderAccounts() {
  const visibleAccounts = filteredAccounts();
  const items = visibleAccounts.map((account) => {
    const provider = getProvider(account.provider);
    const defaultRoute = defaultRouteForProvider(account.provider);
    const isDefaultAccount = defaultRoute?.accountId === account.id;
    const routeCount = countRoutesForAccount(account.id);
    const upstreamSummary =
      account.baseUrl || provider?.baseUrl || t("Uses provider base URL.");
    const detailParts = [upstreamSummary];
    if (account.defaultModel) {
      detailParts.push(
        t("default model {model}", { model: account.defaultModel }),
      );
    }
    if (account.note) {
      detailParts.push(account.note);
    }
    const noteSummary = detailParts.join(" · ");
    const item = document.createElement("article");
    item.className = `data-item${isDefaultAccount ? " default-account-item" : ""}`;
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(account.name)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(noteSummary)}</p>
        </div>
        <span class="pill ${account.enabled ? "ok" : "bad"}">${account.enabled ? t("enabled") : t("disabled")}</span>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(provider ? provider.displayName : account.provider)}</span>
        ${
          account.apiKeyMasked
            ? `<span class="pill ok">${escapeHtml(t("secret stored"))}</span>`
            : `<span class="pill warn">${escapeHtml(t("missing secret"))}</span>`
        }
        <span class="pill ${account.baseUrl ? "warm" : ""}">${escapeHtml(account.baseUrl ? t("account base url") : t("provider base url"))}</span>
        ${isDefaultAccount ? `<span class="pill ok">${escapeHtml(t("default"))}</span>` : ""}
        <span class="pill">${escapeHtml(routeCountLabel(routeCount))}</span>
        <span class="pill">${escapeHtml(t("updated {time}", { time: formatRelativeTime(account.updatedAt) }))}</span>
      </div>
      <div class="actions">
        <button type="button" class="ghost" ${isDefaultAccount || !account.enabled ? "disabled" : ""}>${escapeHtml(isDefaultAccount ? t("Default") : t("Set Default"))}</button>
        <button type="button" class="ghost">${escapeHtml(t("Edit"))}</button>
        <button type="button" class="ghost" ${account.enabled ? "" : "disabled"}>${escapeHtml(t("Disable"))}</button>
        <button type="button" class="ghost">${escapeHtml(t("Delete"))}</button>
      </div>
    `;

    const [defaultButton, editButton, disableButton, deleteButton] =
      item.querySelectorAll("button");
    defaultButton.addEventListener("click", async () => {
      const response = await perform(
        () =>
          api("/admin/routes", {
            method: "POST",
            body: {
              provider: account.provider,
              modelPrefix: null,
              accountId: account.id,
            },
          }),
        t("{name} is now the default account for {provider}.", {
          name: account.name,
          provider: provider?.displayName || account.provider,
        }),
      );
      if (!response) {
        return;
      }
      await refreshRoutes();
      await refreshDesktopTrayMenu();
      renderDashboard();
    });
    editButton.addEventListener("click", () => {
      fillAccountForm(account);
      openDialog(elements.accountDialog, elements.accountName);
    });
    disableButton.addEventListener("click", async () => {
      const response = await perform(
        () => api(`/admin/accounts/${account.id}/disable`, { method: "POST" }),
        t("Account {name} disabled.", { name: account.name }),
      );
      if (!response) {
        return;
      }
      await refreshAccounts();
      await refreshRoutes();
      await refreshDesktopTrayMenu();
      renderDashboard();
    });
    deleteButton.addEventListener("click", async () => {
      requestConfirmation({
        title: t("Delete Account: {name}", { name: account.name }),
        message: t(
          "Delete this account and its stored secret? Any route bindings pointing at it will also be removed.",
        ),
        confirmLabel: t("Delete Account"),
        onConfirm: async () => {
          const response = await perform(
            () => api(`/admin/accounts/${account.id}`, { method: "DELETE" }),
            t("Account {name} deleted.", { name: account.name }),
          );
          if (!response) {
            return;
          }
          if (state.accountEditor?.id === account.id) {
            closeAccountDialog();
          }
          await refreshAccounts();
          await refreshRoutes();
          await refreshDashboardLogs();
          await refreshDesktopTrayMenu();
          renderDashboard();
        },
      });
    });

    return item;
  });

  if (items.length) {
    elements.accountsList.replaceChildren(...items);
    return;
  }

  const provider = getProvider(normalizeAccountProviderFilter());
  if (provider && canImportAccountFromClientConfig(provider)) {
    elements.accountsList.replaceChildren(
      renderAccountImportEmptyState(provider),
    );
    return;
  }

  elements.accountsList.replaceChildren(
    emptyNode(
      provider
        ? t("No accounts under {name} yet.", { name: provider.displayName })
        : t("Nothing to show yet."),
    ),
  );
}

function canImportAccountFromClientConfig(provider) {
  return provider?.slug === "codex" || provider?.slug === "claude-code";
}

function renderAccountImportEmptyState(provider) {
  const node = document.createElement("div");
  node.className = "empty empty-actions";
  const copy = document.createElement("p");
  copy.className = "empty-copy";
  copy.textContent = t("No accounts under {name} yet.", {
    name: provider.displayName,
  });
  const button = document.createElement("button");
  button.type = "button";
  button.className = "ghost";
  button.textContent = t("Import");
  button.addEventListener("click", () => {
    void importAccountFromClientConfig(provider);
  });
  node.append(copy, button);
  return node;
}

async function importAccountFromClientConfig(provider) {
  const command =
    provider.slug === "claude-code"
      ? "import_claude_account"
      : "import_codex_account";
  const failureMessage =
    provider.slug === "claude-code"
      ? t("Failed to import Claude Code account.")
      : t("Failed to import Codex account.");
  const imported = await performDesktop(
    () => invokeDesktop(command),
    null,
    failureMessage,
  );
  if (!imported) {
    return;
  }

  const account = await perform(
    () =>
      api("/admin/accounts", {
        method: "POST",
        body: imported,
      }),
    t("Imported account {name}.", {
      name: imported.name || provider.displayName,
    }),
  );
  if (!account) {
    return;
  }

  if (!defaultRouteForProvider(provider.slug)) {
    await perform(() =>
      api("/admin/routes", {
        method: "POST",
        body: {
          provider: provider.slug,
          modelPrefix: null,
          accountId: account.id,
        },
      }),
    );
  }

  await refreshAccounts();
  await refreshRoutes();
  await refreshDashboardLogs();
  await refreshDesktopTrayMenu();
  renderDashboard();
}

function renderRoutes() {
  const items = state.routes.map((route) => {
    const account = state.accounts.find(
      (candidate) => candidate.id === route.accountId,
    );
    const provider = getProvider(route.provider);
    const isDefaultRoute = !route.modelPrefix;
    const label =
      route.modelPrefix || `${provider?.displayName || route.provider} default`;
    const localUrl = providerLocalBaseUrl(provider, route.provider);

    const item = document.createElement("article");
    item.className = "data-item";
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(label)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(
            t("{kind} ingress {url}", {
              kind: route.modelPrefix ? t("Override") : t("Provider default"),
              url: localUrl,
            }),
          )}</p>
        </div>
        <span class="pill ${route.modelPrefix ? "warm" : "ok"}">${route.modelPrefix ? t("override") : t("default")}</span>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(provider ? provider.displayName : route.provider)}</span>
        <span class="pill">${escapeHtml(providerIngress(provider, route.provider))}</span>
        <span class="pill">${escapeHtml(account?.name || route.accountId)}</span>
        ${isDefaultRoute ? `<span class="pill ok">${escapeHtml(t("provider default"))}</span>` : ""}
        <span class="pill">${escapeHtml(t("updated {time}", { time: formatRelativeTime(route.updatedAt) }))}</span>
      </div>
      <div class="actions">
        <button type="button" class="ghost">${escapeHtml(t("Copy URL"))}</button>
        <button type="button" class="ghost">${escapeHtml(t("Edit"))}</button>
        <button type="button" class="ghost" ${isDefaultRoute ? "disabled" : ""}>${escapeHtml(t("Delete"))}</button>
      </div>
    `;
    const [copyButton, editButton, deleteButton] =
      item.querySelectorAll("button");
    copyButton.addEventListener("click", async () => {
      await copyText(
        localUrl,
        t("Copied {value} URL.", {
          value: providerIngress(provider, route.provider),
        }),
      );
    });
    editButton.addEventListener("click", () => {
      fillRouteForm(route);
      openDialog(elements.routeDialog, elements.routeProvider);
    });
    deleteButton.addEventListener("click", async () => {
      if (isDefaultRoute) {
        return;
      }
      requestConfirmation({
        title: t("Delete Route: {name}", { name: label }),
        message: t("Delete this model override route?"),
        confirmLabel: t("Delete Route"),
        onConfirm: async () => {
          const response = await perform(
            () => api(`/admin/routes/${route.id}`, { method: "DELETE" }),
            t("Route deleted."),
          );
          if (!response) {
            return;
          }
          if (state.routeEditor?.id === route.id) {
            closeRouteDialog();
          }
          await refreshRoutes();
          await refreshDesktopTrayMenu();
          renderDashboard();
        },
      });
    });
    return item;
  });

  elements.routesList.replaceChildren(
    ...(items.length ? items : [emptyNode()]),
  );
}

function renderMonitor() {
  const items = state.monitor.map((entry) => {
    const account = state.accounts.find(
      (candidate) => candidate.id === entry.accountId,
    );
    const provider = getProvider(entry.provider);
    const providerName = provider ? provider.displayName : entry.provider;
    const accountName = account?.name || entry.accountId || t("routing");
    const monitorCopyLabel = t("Copy Full Log");
    const monitorCopyTitle = entry.logId
      ? t("Copy Full Log")
      : t(
          "Full log is still being written. Try again after the request completes.",
        );
    const item = document.createElement("article");
    item.className = "data-item monitor-item";
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(`${entry.method} ${entry.path}`)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(entry.model || t("model unavailable"))}</p>
        </div>
        <div class="monitor-status">
          <span class="pill ${monitorPhaseTone(entry)}">${escapeHtml(monitorPhaseLabel(entry))}</span>
          <span class="pill ${monitorStatusTone(entry)}">${escapeHtml(monitorStatusLabel(entry))}</span>
          <button type="button" class="ghost monitor-copy-button" title="${escapeHtml(monitorCopyTitle)}">${escapeHtml(monitorCopyLabel)}</button>
        </div>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(providerName)}</span>
        <span class="pill">${escapeHtml(accountName)}</span>
        ${entry.upstreamUrl ? `<span class="pill" title="${escapeHtml(entry.upstreamUrl)}">${escapeHtml(truncateMiddle(entry.upstreamUrl, 42))}</span>` : ""}
        <span class="pill">${escapeHtml(entry.streamed ? t("streamed") : t("sync"))}</span>
        <span class="pill">${escapeHtml(monitorDurationLabel(entry))}</span>
        <span class="pill">${escapeHtml(formatRelativeTime(entry.updatedAt || entry.startedAt))}</span>
      </div>
      <div class="monitor-preview-stack">
        <div class="monitor-preview-row">
          <span class="monitor-preview-label">${escapeHtml(t("Req"))}</span>
          <p class="monitor-preview-text clamp-2">${escapeHtml(monitorRequestSummary(entry))}</p>
        </div>
        <div class="monitor-preview-row">
          <span class="monitor-preview-label">${escapeHtml(t("Res"))}</span>
          <p class="monitor-preview-text clamp-2">${escapeHtml(monitorResponseSummary(entry))}</p>
        </div>
      </div>
    `;
    const copyButton = item.querySelector(".monitor-copy-button");
    copyButton.addEventListener("click", async () => {
      await copyMonitorEntry(entry, providerName, accountName);
    });
    return item;
  });

  elements.monitorList.replaceChildren(
    ...(items.length
      ? items
      : [emptyNode(t("No live traffic in memory right now."))]),
  );
}

function renderOnboarding() {
  renderOnboardingTargetTabs();
  renderOnboardingProviderTabs();

  if (!state.onboarding.length) {
    const message = state.providers.length
      ? t(
          "No compatible provider is available for the selected client profile.",
        )
      : t("Add a provider to generate onboarding instructions.");
    elements.onboardingList.replaceChildren(emptyNode(message));
    return;
  }

  const items = state.onboarding.map((guide) => {
    const item = document.createElement("article");
    item.className = "guide";
    item.innerHTML = `
      <div class="guide-head">
        <div>
          <p class="panel-kicker">${escapeHtml(guide.targetLabel)}</p>
          <h3>${escapeHtml(guide.title)}</h3>
          <p class="muted">${escapeHtml(guide.baseUrl)}</p>
        </div>
        <div class="guide-actions">
          <button type="button" class="ghost onboarding-copy-url">${escapeHtml(t("Copy URL"))}</button>
          <button type="button" class="ghost onboarding-copy-snippet">${escapeHtml(t("Copy Snippet"))}</button>
        </div>
      </div>
      <div class="guide-meta">
        ${guide.meta.map((entry) => `<span class="pill ${entry.tone || ""}">${escapeHtml(entry.label)}</span>`).join("")}
      </div>
      ${
        guide.env.length
          ? `<div class="guide-meta">
              ${guide.env
                .map(
                  (envVar) =>
                    `<span class="pill">${escapeHtml(envVar.key)}=${escapeHtml(envVar.value)}</span>`,
                )
                .join("")}
            </div>`
          : ""
      }
      <div class="guide-notes">
        ${guide.notes?.map((note) => `<p class="muted">${escapeHtml(note)}</p>`).join("") || ""}
      </div>
      <div class="guide-section">
        <p class="panel-kicker">${escapeHtml(t("Usage"))}</p>
        <p class="muted">${escapeHtml(guide.summary)}</p>
      </div>
      <pre>${escapeHtml(guide.snippet)}</pre>
    `;
    const copyUrlButton = item.querySelector(".onboarding-copy-url");
    const copySnippetButton = item.querySelector(".onboarding-copy-snippet");
    copyUrlButton.addEventListener("click", async () => {
      await copyText(guide.baseUrl, t("Local base URL copied."));
    });
    copySnippetButton.addEventListener("click", async () => {
      await copyText(guide.snippet, t("Onboarding snippet copied."));
    });
    return item;
  });

  elements.onboardingList.replaceChildren(
    ...(items.length ? items : [emptyNode()]),
  );
}

function renderOnboardingTargetTabs() {
  const targets = availableOnboardingTargets();
  const buttons = targets.map((target) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `filter-tab${target.id === state.onboardingTarget ? " is-active" : ""}`;
    button.textContent = t(target.label);
    button.addEventListener("click", async () => {
      if (state.onboardingTarget === target.id) {
        return;
      }
      state.onboardingTarget = target.id;
      state.onboardingProvider = null;
      await refreshOnboarding();
    });
    return button;
  });
  elements.onboardingTargetTabs.replaceChildren(...buttons);
}

function renderOnboardingProviderTabs() {
  const providers = onboardingProvidersForTarget(state.onboardingTarget);
  const buttons = providers.map((provider) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `filter-tab${provider.slug === state.onboardingProvider ? " is-active" : ""}`;
    button.textContent = provider.enabled
      ? provider.displayName
      : t("{name} (disabled)", { name: provider.displayName });
    button.addEventListener("click", async () => {
      if (state.onboardingProvider === provider.slug) {
        return;
      }
      state.onboardingProvider = provider.slug;
      await refreshOnboarding();
    });
    return button;
  });
  elements.onboardingProviderTabs.replaceChildren(...buttons);
}

function availableOnboardingTargets() {
  return ONBOARDING_TARGETS.filter((target) =>
    state.providers.some((provider) =>
      providerSupportsOnboardingTarget(provider, target.id),
    ),
  );
}

function onboardingProvidersForTarget(targetId) {
  return state.providers.filter((provider) =>
    providerSupportsOnboardingTarget(provider, targetId),
  );
}

function providerSupportsOnboardingTarget(provider, targetId) {
  if (targetId === "curl") {
    return true;
  }
  if (targetId === "codex") {
    return provider.protocol === "openai";
  }
  if (targetId === "claude-code") {
    return provider.protocol === "anthropic";
  }
  return false;
}

function buildOnboardingGuide(targetId, provider) {
  const baseUrl = providerLocalBaseUrl(provider);
  const routeState = onboardingRouteState(provider.slug);
  const daemonUrl = daemonBaseUrl();
  const defaultAccountName =
    routeState.defaultAccount?.name || routeState.defaultRoute?.accountId;
  const defaultRouteLabel = routeState.defaultRoute
    ? t("default -> {name}", { name: defaultAccountName })
    : t("default missing");
  const overrideLabel = routeState.overrideCount
    ? t("{count} overrides", { count: routeState.overrideCount })
    : t("no overrides");
  const enabledAccountLabel = t("{count} enabled accounts", {
    count: routeState.enabledAccounts,
  });
  const providerStatusNote = provider.enabled
    ? t("This provider is enabled in the catalog.")
    : t(
        "This provider is currently disabled in the catalog. Re-enable it before relying on this namespace.",
      );
  const routeStatusNote = routeState.defaultRoute
    ? t("Default traffic currently resolves to {name}.", {
        name: defaultAccountName,
      })
    : t(
        "No provider default account is configured yet. Set one in Accounts or Routes before using this namespace as the catch-all path.",
      );
  const overrideNote = routeState.overrideCount
    ? t("{count} model-prefix overrides are active for this provider.", {
        count: routeState.overrideCount,
      })
    : t("No model-prefix overrides are active for this provider.");
  const listenerNote = t(
    "Current local daemon address: {daemonUrl}. This provider namespace resolves at {baseUrl}.",
    { daemonUrl, baseUrl },
  );
  const lanNote = state.appSettings?.allowLanAccess
    ? t(
        "LAN access is enabled. On other devices, replace 127.0.0.1 with this machine's LAN IP.",
      )
    : null;
  const credentialNote = t(
    "Client credentials shown here are placeholders only. LocalAIRouter strips them and injects the real upstream secret from the selected account.",
  );

  const meta = [
    { label: protocolDisplayLabel(provider.protocol) },
    { label: providerIngress(provider) },
    {
      label: provider.enabled ? t("provider enabled") : t("provider disabled"),
      tone: provider.enabled ? "ok" : "bad",
    },
    { label: enabledAccountLabel },
    { label: defaultRouteLabel, tone: routeState.defaultRoute ? "ok" : "warn" },
    { label: overrideLabel, tone: routeState.overrideCount ? "warm" : "" },
  ];

  switch (targetId) {
    case "codex": {
      const env = openAiEnv(baseUrl);
      return {
        target: targetId,
        providerSlug: provider.slug,
        providerName: provider.displayName,
        targetLabel: t("Codex"),
        title: t("Codex via {name}", { name: provider.displayName }),
        baseUrl,
        meta,
        env,
        summary: t(
          "Use this namespace for Codex or any coding CLI that reads OpenAI-compatible base URL settings. Current target: {baseUrl}.",
          { baseUrl },
        ),
        snippet: buildEnvSnippet(env),
        notes: [
          listenerNote,
          ...(lanNote ? [lanNote] : []),
          providerStatusNote,
          routeStatusNote,
          credentialNote,
        ],
      };
    }
    case "claude-code": {
      const env = anthropicEnv(baseUrl);
      return {
        target: targetId,
        providerSlug: provider.slug,
        providerName: provider.displayName,
        targetLabel: t("Claude Code"),
        title: t("Claude Code via {name}", { name: provider.displayName }),
        baseUrl,
        meta,
        env,
        summary: t(
          "Use this namespace for Claude Code or other Anthropic-style clients that support an alternate base URL. Current target: {baseUrl}.",
          { baseUrl },
        ),
        snippet: buildEnvSnippet(env),
        notes: [
          listenerNote,
          ...(lanNote ? [lanNote] : []),
          providerStatusNote,
          routeStatusNote,
          credentialNote,
        ],
      };
    }
    case "curl":
    default:
      return {
        target: targetId,
        providerSlug: provider.slug,
        providerName: provider.displayName,
        targetLabel: t("cURL / Manual"),
        title: t("Manual HTTP via {name}", { name: provider.displayName }),
        baseUrl,
        meta,
        env: [],
        summary:
          provider.protocol === "generic"
            ? t(
                "Generic HTTP providers stay manual-only. Append the upstream-specific path and payload after this namespace. Current target: {baseUrl}.",
                { baseUrl },
              )
            : t(
                "Use this for smoke tests, quick probes, or custom scripts against the local provider namespace. Current target: {baseUrl}.",
                { baseUrl },
              ),
        snippet: buildCurlSnippet(provider, baseUrl),
        notes: [
          listenerNote,
          ...(lanNote ? [lanNote] : []),
          providerStatusNote,
          routeStatusNote,
          overrideNote,
          provider.protocol === "generic"
            ? t(
                "Generic HTTP providers do not have a Codex or Claude Code preset.",
              )
            : credentialNote,
        ],
      };
  }
}

function syncProviderOptions() {
  const enabledProviders = state.providers.filter(
    (provider) => provider.enabled,
  );
  const accountSelection =
    state.accountEditor?.provider ||
    elements.accountProvider.value ||
    normalizeAccountProviderFilter();
  const routeSelection =
    state.routeEditor?.provider || elements.routeProvider.value;

  const accountProviders = providerOptionsForSelect(accountSelection);
  const routeProviders = providerOptionsForSelect(routeSelection);

  if (accountProviders.length) {
    elements.accountProvider.replaceChildren(
      ...accountProviders.map((provider) =>
        optionNode(provider.slug, providerOptionLabel(provider)),
      ),
    );
    elements.accountProvider.disabled = false;
  } else {
    elements.accountProvider.replaceChildren(
      optionNode("", t("No providers configured")),
    );
    elements.accountProvider.disabled = true;
    elements.accountSubmit.disabled = true;
  }

  if (routeProviders.length) {
    elements.routeProvider.replaceChildren(
      ...routeProviders.map((provider) =>
        optionNode(provider.slug, providerOptionLabel(provider)),
      ),
    );
    elements.routeProvider.disabled = false;
  } else {
    elements.routeProvider.replaceChildren(
      optionNode("", t("No providers configured")),
    );
    elements.routeProvider.disabled = true;
  }

  if (
    accountSelection &&
    accountProviders.some((provider) => provider.slug === accountSelection)
  ) {
    elements.accountProvider.value = accountSelection;
  } else if (accountProviders[0]) {
    elements.accountProvider.value = accountProviders[0].slug;
  }

  if (
    routeSelection &&
    routeProviders.some((provider) => provider.slug === routeSelection)
  ) {
    elements.routeProvider.value = routeSelection;
  } else if (routeProviders[0]) {
    elements.routeProvider.value = routeProviders[0].slug;
  }

  elements.accountSubmit.disabled = !accountProviders.length;
  syncRouteAccountOptions();
}

function syncRouteAccountOptions() {
  const provider = elements.routeProvider.value;
  const preferredAccountId =
    state.routeEditor?.provider === provider
      ? state.routeEditor.accountId
      : null;
  const candidates = routeAccountOptionsForProvider(
    provider,
    preferredAccountId,
  );
  const current = elements.routeAccount.value;

  if (candidates.length) {
    elements.routeAccount.replaceChildren(
      ...candidates.map((account) =>
        optionNode(
          account.id,
          account.enabled
            ? account.name
            : t("{name} (disabled)", { name: account.name }),
        ),
      ),
    );
    if (current && candidates.some((account) => account.id === current)) {
      elements.routeAccount.value = current;
    } else if (
      preferredAccountId &&
      candidates.some((account) => account.id === preferredAccountId)
    ) {
      elements.routeAccount.value = preferredAccountId;
    } else if (candidates[0]) {
      elements.routeAccount.value = candidates[0].id;
    }

    const selectedAccount = candidates.find(
      (account) => account.id === elements.routeAccount.value,
    );
    elements.routeAccount.disabled = false;
    if (selectedAccount?.enabled === false) {
      elements.routeSubmit.disabled = true;
      elements.routeHint.textContent = t(
        "This route currently points at a disabled account. Choose an enabled account before saving.",
      );
    } else {
      elements.routeSubmit.disabled = false;
      elements.routeHint.textContent = t(
        "Routes apply immediately to new requests.",
      );
    }
  } else {
    elements.routeAccount.replaceChildren(
      optionNode("", t("No enabled accounts")),
    );
    elements.routeAccount.disabled = true;
    elements.routeSubmit.disabled = true;
    elements.routeHint.textContent = provider
      ? t(
          "This provider has no enabled accounts. Add or re-enable one before binding routes.",
        )
      : t("Select a provider to see enabled accounts.");
  }
}

function syncProviderFilterOptions(select, allLabel) {
  const selected = select.value;
  const options = [
    optionNode("", allLabel),
    ...state.providers.map((provider) =>
      optionNode(
        provider.slug,
        `${provider.displayName} (${provider.protocol})`,
      ),
    ),
  ];
  select.replaceChildren(...options);
  if (selected) {
    select.value = selected;
  }
}

function syncAccountFilterOptions(select, allLabel) {
  const selected = select.value;
  const options = [
    optionNode("", allLabel),
    ...state.accounts.map((account) => {
      const provider = getProvider(account.provider);
      return optionNode(
        account.id,
        `${account.name} (${provider ? provider.displayName : account.provider})`,
      );
    }),
  ];
  select.replaceChildren(...options);
  if (selected) {
    select.value = selected;
  }
}

function syncMonitorProviderOptions() {
  syncProviderFilterOptions(elements.monitorProvider, t("All providers"));
}

function syncMonitorAccountOptions() {
  syncAccountFilterOptions(elements.monitorAccount, t("All accounts"));
}

function clearFormError(node) {
  if (!node) {
    return;
  }
  node.hidden = true;
  node.textContent = "";
}

function showFormError(node, error) {
  const message =
    typeof error === "string"
      ? error
      : error?.message || t("The request could not be completed.");
  if (!node) {
    notify(message, "error");
    return;
  }
  node.textContent = message;
  node.hidden = false;
  window.requestAnimationFrame(() => {
    node.scrollIntoView({ block: "nearest" });
  });
}

function resetProviderForm() {
  state.providerEditor = null;
  clearFormError(elements.providerFormError);
  elements.providerFormTitle.textContent = t("New Provider");
  elements.providerFormCopy.textContent = t(
    "Define a built-in override or register a custom upstream with its own proxy path, auth header, and protocol shape.",
  );
  elements.providerSubmit.textContent = t("Save Provider");
  elements.providerSlug.value = "";
  elements.providerProtocol.disabled = false;
  elements.providerName.value = "";
  elements.providerProtocol.value = "openai";
  elements.providerBaseUrl.value = "";
  elements.providerDefaultModel.value = "";
  elements.providerPath.value = "";
  elements.providerPath.dataset.autofill = "on";
  elements.providerAuthHeader.value = "";
  elements.providerAuthPrefix.value = "";
  elements.providerEnabled.checked = true;
  applyProviderProtocolDefaults(true);
  syncProviderIdentity();
}

function fillProviderForm(provider) {
  state.providerEditor = { slug: provider.slug, isBuiltin: provider.isBuiltin };
  clearFormError(elements.providerFormError);
  elements.providerFormTitle.textContent = provider.isBuiltin
    ? t("Tune Built-In: {name}", { name: provider.displayName })
    : t("Edit Provider: {name}", { name: provider.displayName });
  elements.providerFormCopy.textContent = provider.isBuiltin
    ? t(
        "Built-in providers keep their internal identity. You can still adjust endpoint, auth header, proxy path, and enabled state.",
      )
    : t(
        "Editing a custom provider updates the existing registry entry in place.",
      );
  elements.providerSubmit.textContent = t("Update Provider");
  elements.providerSlug.value = provider.slug;
  elements.providerProtocol.disabled = provider.isBuiltin;
  elements.providerName.value = provider.displayName;
  elements.providerProtocol.value = provider.protocol;
  elements.providerBaseUrl.value = provider.baseUrl;
  elements.providerDefaultModel.value = provider.defaultModel || "";
  elements.providerPath.value = provider.proxyPath;
  elements.providerPath.dataset.autofill = "off";
  elements.providerAuthHeader.value = provider.authHeader;
  elements.providerAuthPrefix.value = provider.authPrefix || "";
  elements.providerEnabled.checked = provider.enabled;
  syncGeneratedProviderSlug();
  renderProviderPathDemo();
}

function buildProviderPayload() {
  const displayName = elements.providerName.value.trim();
  const slug = currentProviderSlug();
  const protocol = elements.providerProtocol.value;
  const baseUrl = elements.providerBaseUrl.value.trim();
  const defaultModel = normalizeOptional(elements.providerDefaultModel.value);
  const proxyPath = normalizeSegment(elements.providerPath.value || slug);
  const authHeader = elements.providerAuthHeader.value.trim();
  const authPrefix = normalizeOptional(elements.providerAuthPrefix.value);

  if (!slug) {
    showFormError(
      elements.providerFormError,
      t("Provider ID could not be generated from the display name."),
    );
    return null;
  }
  if (!isValidSlug(slug)) {
    showFormError(
      elements.providerFormError,
      t("Provider ID may only use lowercase letters, digits, and dashes."),
    );
    return null;
  }
  if (!displayName) {
    showFormError(elements.providerFormError, t("Display name is required."));
    return null;
  }
  if (!proxyPath) {
    showFormError(elements.providerFormError, t("Proxy path is required."));
    return null;
  }
  if (!isValidSlug(proxyPath)) {
    showFormError(
      elements.providerFormError,
      t(
        "Proxy path must be one lowercase path segment with letters, digits, or dashes.",
      ),
    );
    return null;
  }
  if (!baseUrl.startsWith("http://") && !baseUrl.startsWith("https://")) {
    showFormError(
      elements.providerFormError,
      t("Base URL must start with http:// or https://."),
    );
    return null;
  }
  if (!authHeader || /\s/.test(authHeader)) {
    showFormError(
      elements.providerFormError,
      t("Auth header is required and cannot contain spaces."),
    );
    return null;
  }

  elements.providerSlug.value = slug;
  elements.providerPath.value = proxyPath;
  syncGeneratedProviderSlug();

  return {
    slug,
    displayName,
    protocol,
    baseUrl,
    defaultModel,
    proxyPath,
    authHeader,
    authPrefix,
    enabled: elements.providerEnabled.checked,
  };
}

function currentProviderSlug() {
  const existingSlug =
    elements.providerSlug.value || state.providerEditor?.slug || "";
  if (state.providerEditor) {
    return normalizeSegment(existingSlug);
  }
  return (
    normalizeSegment(elements.providerName.value || "") ||
    normalizeSegment(elements.providerPath.value || "") ||
    normalizeSegment(existingSlug)
  );
}

function syncGeneratedProviderSlug() {
  elements.providerSlug.value = currentProviderSlug();
}

function syncProviderIdentity() {
  syncGeneratedProviderSlug();
  syncProviderProxyPath();
}

function applyProviderProtocolDefaults(force) {
  const protocol = elements.providerProtocol.value;
  if (protocol === "openai") {
    if (force || !elements.providerAuthHeader.value) {
      elements.providerAuthHeader.value = "Authorization";
    }
    if (force || !elements.providerAuthPrefix.value) {
      elements.providerAuthPrefix.value = "Bearer";
    }
  } else if (protocol === "anthropic") {
    if (
      force ||
      !elements.providerAuthHeader.value ||
      elements.providerAuthHeader.value === "Authorization"
    ) {
      elements.providerAuthHeader.value = "x-api-key";
    }
    if (force || elements.providerAuthPrefix.value === "Bearer") {
      elements.providerAuthPrefix.value = "";
    }
  } else if (force) {
    elements.providerAuthHeader.value = "Authorization";
    elements.providerAuthPrefix.value = "Bearer";
  }
}

function syncProviderProxyPath() {
  if (elements.providerPath.dataset.autofill === "off") {
    renderProviderPathDemo();
    return;
  }
  elements.providerPath.value = currentProviderSlug();
  renderProviderPathDemo();
}

function resetAccountForm() {
  state.accountEditor = null;
  clearFormError(elements.accountFormError);
  elements.accountFormTitle.textContent = t("New Account");
  elements.accountFormCopy.textContent = t(
    "Store one encrypted credential set per account. Leave Base URL Override empty to inherit the provider upstream endpoint.",
  );
  elements.accountSubmit.textContent = t("Save Account");
  elements.accountId.value = "";
  elements.accountName.value = "";
  elements.accountApiKey.value = "";
  setAccountApiKeyVisible(false);
  elements.accountBaseUrl.value = "";
  elements.accountDefaultModel.value = "";
  elements.accountNote.value = "";
  elements.accountEnabled.checked = true;
  const preferredProvider =
    state.providers.find(
      (provider) =>
        provider.slug === normalizeAccountProviderFilter() && provider.enabled,
    ) || state.providers.find((provider) => provider.enabled);
  if (preferredProvider) {
    elements.accountProvider.value = preferredProvider.slug;
  } else {
    elements.accountProvider.value = "";
  }
}

function fillAccountForm(account) {
  state.accountEditor = {
    id: account.id,
    provider: account.provider,
    name: account.name,
  };
  clearFormError(elements.accountFormError);
  elements.accountFormTitle.textContent = t("Edit Account: {name}", {
    name: account.name,
  });
  elements.accountFormCopy.textContent = t(
    "Store one API key per account. Override Base URL only when the account uses a different upstream endpoint.",
  );
  elements.accountSubmit.textContent = t("Update Account");
  elements.accountId.value = account.id;
  syncProviderOptions();
  elements.accountProvider.value = account.provider;
  elements.accountName.value = account.name;
  elements.accountApiKey.value = account.apiKey || "";
  setAccountApiKeyVisible(false);
  elements.accountBaseUrl.value = account.baseUrl || "";
  elements.accountDefaultModel.value = account.defaultModel || "";
  elements.accountNote.value = account.note || "";
  elements.accountEnabled.checked = account.enabled;
}

function toggleAccountApiKeyVisibility() {
  const input = elements.accountApiKey;
  if (!input) {
    return;
  }
  setAccountApiKeyVisible(input.type === "password");
}

function setAccountApiKeyVisible(visible) {
  const input = elements.accountApiKey;
  const button = elements.accountKeyToggleView;
  if (!input || !button) {
    return;
  }
  if (visible) {
    input.type = "text";
    button.innerHTML =
      '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M13.5 13.5L2.5 2.5"/><path d="M7 3.5C6 3.5 4.5 3.5 2 7.5C3.5 10 5.8 12 8 12C9.5 12 11.5 10.5 14 7.5C12.5 5 10.5 3.5 7 3.5z"/></svg>';
    button.title = "Hide API key";
    return;
  }
  input.type = "password";
  button.innerHTML =
    '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z"/><circle cx="8" cy="8" r="2"/></svg>';
  button.title = "Toggle API key visibility";
}

function buildAccountPayload() {
  const provider = elements.accountProvider.value;
  const name = elements.accountName.value.trim();
  const apiKey = normalizeOptional(elements.accountApiKey.value);
  const baseUrl = normalizeOptional(elements.accountBaseUrl.value);
  const defaultModel = normalizeOptional(elements.accountDefaultModel.value);
  const note = normalizeOptional(elements.accountNote.value);
  const isEditing = Boolean(state.accountEditor);

  if (!provider) {
    showFormError(
      elements.accountFormError,
      t("Choose an enabled provider before saving an account."),
    );
    return null;
  }
  if (!name) {
    showFormError(elements.accountFormError, t("Account name is required."));
    return null;
  }
  if (!isEditing && !apiKey) {
    showFormError(
      elements.accountFormError,
      t("New accounts require an API key."),
    );
    return null;
  }
  if (
    baseUrl &&
    !baseUrl.startsWith("http://") &&
    !baseUrl.startsWith("https://")
  ) {
    showFormError(
      elements.accountFormError,
      t("Account base URL must start with http:// or https://."),
    );
    return null;
  }

  return {
    id: elements.accountId.value || null,
    provider,
    name,
    baseUrl,
    defaultModel,
    apiKey,
    note,
    enabled: elements.accountEnabled.checked,
  };
}

function resetRouteForm() {
  state.routeEditor = null;
  clearFormError(elements.routeFormError);
  elements.routeFormTitle.textContent = t("New Route");
  elements.routeFormCopy.textContent = t(
    "Set one default account per provider, then add optional model-prefix overrides for fine-grained account selection.",
  );
  elements.routeSubmit.textContent = t("Save Route");
  elements.routePrefix.value = "";
  const firstEnabled = state.providers.find((provider) => provider.enabled);
  if (firstEnabled) {
    elements.routeProvider.value = firstEnabled.slug;
  } else {
    elements.routeProvider.value = "";
  }
  syncRouteAccountOptions();
}

function fillRouteForm(route) {
  state.routeEditor = {
    id: route.id,
    provider: route.provider,
    modelPrefix: route.modelPrefix,
    accountId: route.accountId,
  };
  clearFormError(elements.routeFormError);
  elements.routeFormTitle.textContent = route.modelPrefix
    ? t("Edit Route: {name}", { name: route.modelPrefix })
    : t("Edit Route: {name}", { name: `${route.provider} ${t("default")}` });
  elements.routeFormCopy.textContent = route.modelPrefix
    ? t(
        "Update the provider, prefix, or account binding. Changing provider or prefix will replace the previous binding.",
      )
    : t(
        "This row is the provider default account. Updating the bound account here changes the provider default used by non-matching requests.",
      );
  elements.routeSubmit.textContent = t("Update Route");
  syncProviderOptions();
  elements.routeProvider.value = route.provider;
  elements.routePrefix.value = route.modelPrefix || "";
  syncRouteAccountOptions();
  if (
    [...elements.routeAccount.options].some(
      (option) => option.value === route.accountId,
    )
  ) {
    elements.routeAccount.value = route.accountId;
  }
}

function buildRoutePayload() {
  const provider = elements.routeProvider.value;
  const accountId = elements.routeAccount.value;
  const modelPrefix = normalizeOptional(elements.routePrefix.value);
  if (!provider) {
    showFormError(
      elements.routeFormError,
      t("Choose a provider before saving a route."),
    );
    return null;
  }
  if (!accountId) {
    showFormError(
      elements.routeFormError,
      t("The selected provider has no enabled accounts to bind."),
    );
    return null;
  }

  return {
    provider,
    modelPrefix,
    accountId,
  };
}

function setActiveTab(tab) {
  state.activeTab = tab;
  elements.tabButtons.forEach((button) => {
    const active = button.dataset.tab === tab;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-selected", String(active));
  });
  elements.tabPanels.forEach((panel) => {
    panel.classList.toggle("is-active", panel.dataset.panel === tab);
  });
  if (tab === "monitor") {
    void refreshMonitor(true);
  } else if (tab === "stats") {
    void refreshDailyStats(true);
  } else if (tab === "settings") {
    renderSettings();
  }
}

function setDaemonChip(text, tone) {
  elements.daemonChip.textContent = text;
  elements.daemonChip.dataset.tone = tone;
}

function syncDaemonPanels() {
  const daemon = state.daemonStatus;
  const health = state.health;
  const daemonRunning = Boolean(daemon?.running);

  elements.startDaemonButton.disabled = daemonRunning;
  elements.stopDaemonButton.disabled = !daemonRunning;
  elements.restartDaemonButton.disabled = false;
  elements.openDaemonLogButton.disabled = !daemon?.logFilePath;

  elements.detailPid.textContent = daemon?.pid
    ? String(daemon.pid)
    : t("Unavailable");
  elements.daemonLogPath.textContent = daemon?.logFilePath || t("Unavailable");
  elements.daemonLastExit.textContent = daemon?.lastExit || t("Unavailable");
  elements.daemonLastError.textContent = daemon?.lastError || t("Unavailable");

  if (health) {
    elements.detailStatus.textContent = t("Daemon online");
    renderProviderPathDemo();
    return;
  }

  elements.detailStatus.textContent = daemonRunning
    ? t("Process running but health endpoint unavailable")
    : t("Daemon offline");
  elements.dbPath.textContent = t("Unavailable");
  elements.daemonPort.textContent = String(daemon?.port || DEFAULT_PORT);
  elements.startedAt.textContent = daemon?.startedAt
    ? formatDateTime(daemon.startedAt)
    : t("Unavailable");
  renderProviderPathDemo();
}

function renderChrome() {
  elements.openProviderDialog.disabled = !state.health;
  elements.openAccountDialog.disabled = !state.health;
  elements.openRouteDialog.disabled = !state.health;
}

function renderSettings() {
  const settings = state.appSettings;
  const enabled = hasDesktopIntegration();
  const daemonAvailable =
    Boolean(state.health) || Boolean(state.daemonStatus?.running);
  const savedPort = settings?.daemonPort || DEFAULT_PORT;
  const draftPort = Number(elements.settingsDaemonPort.value || savedPort);
  const currentPort =
    state.settingsDirty && Number.isFinite(draftPort) && draftPort > 0
      ? draftPort
      : savedPort;
  const allowLanAccess = state.settingsDirty
    ? Boolean(elements.settingsAllowLan.checked)
    : Boolean(settings?.allowLanAccess);
  const currentMonitorBuffer =
    settings?.monitorBufferLimit || DEFAULT_MONITOR_BUFFER_LIMIT;
  const currentLogRetentionDays =
    settings?.logRetentionDays || DEFAULT_LOG_RETENTION_DAYS;
  const currentLogsDir = settings?.logsDir || "";
  const defaultLogsDir = settings?.defaultLogsDir || "";

  if (!state.settingsDirty) {
    elements.settingsDaemonPort.value = String(currentPort);
    elements.settingsAllowLan.checked = allowLanAccess;
    elements.settingsMonitorBuffer.value = String(currentMonitorBuffer);
    elements.settingsLogRetentionDays.value = String(currentLogRetentionDays);
    elements.settingsLogsDir.value = currentLogsDir;
  }
  elements.settingsDefaultLogsDir.textContent = defaultLogsDir
    ? t("Default logs directory: {path}", { path: defaultLogsDir })
    : t("Default logs directory will appear here.");
  if (elements.settingsLanAddress) {
    if (allowLanAccess) {
      const lanUrl = state.lanIp ? `http://${state.lanIp}:${currentPort}/` : "";
      elements.settingsLanAddress.textContent = lanUrl
        ? t("LAN access URL: {url}", { url: lanUrl })
        : t("Unable to determine LAN IP. Check your network connection.");
      elements.settingsLanAddress.hidden = false;
    } else {
      elements.settingsLanAddress.textContent = "";
      elements.settingsLanAddress.hidden = true;
    }
  }
  elements.settingsDataRoot.value = settings?.dataRoot || "";
  elements.settingsDatabasePath.value = settings?.databasePath || "";
  elements.settingsDaemonPort.disabled = !enabled;
  elements.settingsAllowLan.disabled = !enabled;
  elements.settingsMonitorBuffer.disabled = !enabled;
  elements.settingsLogRetentionDays.disabled = !enabled;
  elements.settingsLogsDir.disabled = !enabled;
  elements.settingsPickLogsDir.disabled = !enabled;
  elements.settingsUseDefaultLogs.disabled = !enabled;
  elements.settingsSubmit.disabled = !enabled;
  elements.openSettingsLogsDir.disabled = !enabled;
  if (elements.rebuildTokenStats) {
    elements.rebuildTokenStats.disabled =
      !enabled || !daemonAvailable || state.rebuildingTokenStats;
  }
  if (elements.rebuildTokenStatus) {
    elements.rebuildTokenStatus.textContent = state.tokenRebuildStatus || "";
    elements.rebuildTokenStatus.hidden = !state.tokenRebuildStatus;
  }
}

function toggleDetailsPanel() {
  const hidden = elements.detailsPanel.hasAttribute("hidden");
  if (hidden) {
    elements.detailsPanel.removeAttribute("hidden");
    elements.detailsButton.setAttribute("aria-expanded", "true");
  } else {
    closeDetailsPanel();
  }
}

function closeDetailsPanel() {
  elements.detailsPanel.setAttribute("hidden", "");
  elements.detailsButton.setAttribute("aria-expanded", "false");
}

function closeProviderDialog() {
  closeDialog(elements.providerDialog);
}

function closeAccountDialog() {
  closeDialog(elements.accountDialog);
}

function closeRouteDialog() {
  closeDialog(elements.routeDialog);
}

function closeConfirmDialog() {
  pendingConfirmation = null;
  closeDialog(elements.confirmDialog);
}

function openDialog(dialog, focusTarget) {
  if (typeof dialog.showModal === "function" && !dialog.open) {
    dialog.showModal();
  } else {
    dialog.setAttribute("open", "");
  }
  if (focusTarget) {
    window.setTimeout(() => focusTarget.focus(), 0);
  }
}

function closeDialog(dialog) {
  if (typeof dialog.close === "function" && dialog.open) {
    dialog.close();
  } else {
    dialog.removeAttribute("open");
    dialog.dispatchEvent(new Event("close"));
  }
}

function getProvider(slug) {
  return state.providers.find((provider) => provider.slug === slug);
}

function requestConfirmation({ title, message, confirmLabel, onConfirm }) {
  pendingConfirmation = onConfirm;
  elements.confirmDialogTitle.textContent = title;
  elements.confirmDialogCopy.textContent = message;
  elements.confirmSubmit.textContent = confirmLabel;
  elements.confirmSubmit.dataset.tone = "danger";
  openDialog(elements.confirmDialog, elements.confirmCancel);
}

function optionNode(value, label) {
  const option = document.createElement("option");
  option.value = value;
  option.textContent = label;
  return option;
}

function configuredDaemonPort() {
  const port = Number(
    state.health?.port ||
      state.daemonStatus?.port ||
      state.appSettings?.daemonPort ||
      elements.daemonPort.textContent,
  );
  return Number.isFinite(port) && port > 0 ? port : DEFAULT_PORT;
}

function daemonAddress() {
  return `127.0.0.1:${configuredDaemonPort()}`;
}

function daemonBaseUrl() {
  return `http://${daemonAddress()}`;
}

function providerIngress(provider, fallbackSlug = provider?.slug || "") {
  return `/${provider?.proxyPath || fallbackSlug}`;
}

function providerLocalBaseUrl(provider, fallbackSlug = provider?.slug || "") {
  return `${daemonBaseUrl()}${providerIngress(provider, fallbackSlug)}`;
}

function protocolDisplayLabel(protocol) {
  switch (protocol) {
    case "openai":
      return "OpenAI";
    case "anthropic":
      return "Anthropic";
    case "generic":
      return t("Generic HTTP");
    default:
      return protocol;
  }
}

function routeCountLabel(count) {
  return count === 1
    ? t("{count} route", { count })
    : t("{count} routes", { count });
}

function renderProviderPathDemo() {
  const path = normalizeSegment(elements.providerPath.value || "");
  elements.providerPathDemo.textContent = path
    ? t("Local ingress: {url}", { url: `${daemonBaseUrl()}/${path}` })
    : t("Local ingress: {url}", { url: `${daemonBaseUrl()}/{proxy-path}` });
}

function storageRootPath() {
  const dbPath = state.health?.dbPath;
  if (!dbPath) {
    return null;
  }
  return dbPath.replace(/[/\\][^/\\]+$/, "");
}

function logArtifactRelativePath(log) {
  if (log.logFilePath) {
    return log.logFilePath;
  }
  const day = log.createdAt?.slice(0, 10) || "undated";
  return `logs/${day}/${log.id}`;
}

function logArtifactPath(log) {
  const root = storageRootPath();
  const relativePath = logArtifactRelativePath(log);
  return root ? `${root}/${relativePath}` : relativePath;
}

function formatSessionLabel(sessionId) {
  return t("session {id}", { id: truncateMiddle(sessionId, 20) });
}

function monitorPhaseLabel(entry) {
  switch (entry.phase) {
    case "routing":
      return t("routing");
    case "upstream":
      return t("upstream");
    case "response":
      return t("response");
    case "streaming":
      return t("streaming");
    case "failed":
      return t("failed");
    case "completed":
    default:
      return t("completed");
  }
}

function monitorPhaseTone(entry) {
  switch (entry.phase) {
    case "routing":
    case "upstream":
    case "response":
      return "warn";
    case "streaming":
      return "warm";
    case "failed":
      return "bad";
    case "completed":
    default:
      return isSuccessStatus(entry.statusCode) ? "ok" : "bad";
  }
}

function monitorStatusLabel(entry) {
  if (typeof entry.statusCode === "number") {
    return String(entry.statusCode);
  }
  return entry.phase === "failed" ? t("error") : t("live");
}

function monitorStatusTone(entry) {
  if (typeof entry.statusCode === "number") {
    return isSuccessStatus(entry.statusCode) ? "ok" : "bad";
  }
  return entry.phase === "failed" ? "bad" : "warm";
}

function monitorRequestSummary(entry) {
  return entry.requestPreview || t("No request body preview.");
}

function monitorResponseSummary(entry) {
  if (entry.errorText) {
    return entry.errorText;
  }
  if (entry.responsePreview) {
    return entry.responsePreview;
  }
  switch (entry.phase) {
    case "routing":
      return t("Resolving provider route and active account.");
    case "upstream":
      return t("Forwarded upstream. Waiting for headers.");
    case "response":
      return t("Receiving upstream response body.");
    case "streaming":
      return t("Streaming response chunks.");
    case "failed":
      return t("Request failed before a response preview was captured.");
    case "completed":
    default:
      return t("Response completed with no preview payload.");
  }
}

function monitorDurationLabel(entry) {
  return typeof entry.durationMs === "number"
    ? formatLatency(entry.durationMs)
    : t("live");
}

function buildFullLogClipboardText(
  log,
  fallbackProviderName,
  fallbackAccountName,
) {
  const provider = getProvider(log.provider);
  const account = state.accounts.find(
    (candidate) => candidate.id === log.accountId,
  );
  const providerName =
    provider?.displayName || fallbackProviderName || log.provider;
  const accountName =
    account?.name || log.accountId || fallbackAccountName || t("No account");
  const metadata = [
    ["id", log.id],
    ["createdAt", formatDateTime(log.createdAt)],
    ["provider", providerName],
    ["providerSlug", log.provider],
    ["account", accountName],
    ["accountId", log.accountId],
    ["upstreamUrl", log.upstreamUrl],
    ["model", log.model || t("model unavailable")],
    ["method", log.method],
    ["path", log.path],
    ["status", log.statusCode ?? "error"],
    ["duration", formatLatency(log.durationMs)],
    ["streamed", log.streamed ? "true" : "false"],
    ["totalTokens", formatTokenCount(log.totalTokens)],
    ["sessionId", log.sessionId],
    ["logFilePath", log.logFilePath],
  ];
  if (log.errorText) {
    metadata.push(["error", log.errorText]);
  }

  return [
    "LocalAIRouter Interaction Log",
    "",
    "Metadata",
    ...metadata.map(([key, value]) => `${key}: ${formatMetadataValue(value)}`),
    "",
    "Request Headers",
    formatLogPayload(log.requestHeaders),
    "",
    "Request Body",
    formatLogPayload(log.requestBody),
    "",
    "Response Headers",
    formatLogPayload(log.responseHeaders),
    "",
    "Response Body",
    formatLogPayload(log.responseBody),
  ].join("\n");
}

function formatMetadataValue(value) {
  return value === null || value === undefined || value === "" ? "--" : value;
}

function formatLogPayload(value) {
  const text = typeof value === "string" ? value : "";
  if (!text) {
    return "(empty)";
  }
  const parsed = safeJsonParse(text);
  return parsed ? JSON.stringify(parsed, null, 2) : text;
}

function countRoutesForAccount(accountId) {
  return state.routes.filter((route) => route.accountId === accountId).length;
}

function onboardingRouteState(providerSlug) {
  const providerRoutes = state.routes.filter(
    (route) => route.provider === providerSlug,
  );
  const defaultRoute =
    providerRoutes.find((route) => !route.modelPrefix) || null;
  const defaultAccount = defaultRoute
    ? state.accounts.find((account) => account.id === defaultRoute.accountId) ||
      null
    : null;
  return {
    defaultRoute,
    defaultAccount,
    overrideCount: providerRoutes.filter((route) => route.modelPrefix).length,
    enabledAccounts: state.accounts.filter(
      (account) => account.provider === providerSlug && account.enabled,
    ).length,
  };
}

function openAiEnv(baseUrl) {
  return [
    { key: "OPENAI_BASE_URL", value: baseUrl },
    { key: "OPENAI_API_KEY", value: "localairouter-managed" },
  ];
}

function anthropicEnv(baseUrl) {
  return [
    { key: "ANTHROPIC_BASE_URL", value: baseUrl },
    { key: "ANTHROPIC_API_KEY", value: "localairouter-managed" },
  ];
}

function buildEnvSnippet(env) {
  return env.map((entry) => `export ${entry.key}="${entry.value}"`).join("\n");
}

function buildCurlSnippet(provider, baseUrl) {
  if (provider.protocol === "openai") {
    return [
      `curl "${baseUrl}/chat/completions" \\`,
      `  -H "Content-Type: application/json" \\`,
      `  -H "Authorization: Bearer localairouter-managed" \\`,
      `  -d '{`,
      `    "model": "gpt-5.4",`,
      `    "messages": [{"role": "user", "content": "Hello from LocalAIRouter"}]`,
      `  }'`,
    ].join("\n");
  }

  if (provider.protocol === "anthropic") {
    return [
      `curl "${baseUrl}/messages" \\`,
      `  -H "Content-Type: application/json" \\`,
      `  -H "x-api-key: localairouter-managed" \\`,
      `  -H "anthropic-version: 2023-06-01" \\`,
      `  -d '{`,
      `    "model": "claude-3-7-sonnet-latest",`,
      `    "max_tokens": 256,`,
      `    "messages": [{"role": "user", "content": "Hello from LocalAIRouter"}]`,
      `  }'`,
    ].join("\n");
  }

  return [
    `curl "${baseUrl}/<upstream-path>" \\`,
    `  -H "Content-Type: application/json" \\`,
    `  -d '{`,
    `    "replace": "with provider-specific payload"`,
    `  }'`,
  ].join("\n");
}

async function copyText(text, successMessage) {
  try {
    await writeClipboardText(String(text ?? ""));
    notify(successMessage, "success");
  } catch (error) {
    console.error(error);
    notify(t("Copy failed. Clipboard access is unavailable."), "error");
  }
}

async function writeClipboardText(text) {
  if (hasDesktopIntegration()) {
    try {
      await invokeDesktop("write_clipboard_text", { text });
      return;
    } catch (error) {
      console.warn(
        "Native clipboard copy failed; trying web clipboard.",
        error,
      );
    }
  }

  let clipboardError = null;
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch (error) {
      clipboardError = error;
      console.warn(
        "Web clipboard copy failed; trying textarea fallback.",
        error,
      );
    }
  }

  if (copyTextWithTextarea(text)) {
    return;
  }

  throw clipboardError || new Error("clipboard access is unavailable");
}

function copyTextWithTextarea(text) {
  const buffer = document.createElement("textarea");
  buffer.value = text;
  buffer.setAttribute("readonly", "");
  buffer.style.position = "fixed";
  buffer.style.left = "-9999px";
  buffer.style.top = "0";
  document.body.appendChild(buffer);
  buffer.focus();
  buffer.select();
  let copied = false;
  try {
    copied = document.execCommand("copy");
  } catch (error) {
    console.warn("Textarea clipboard fallback failed.", error);
  } finally {
    buffer.remove();
  }
  return copied;
}

async function copyMonitorEntry(entry, providerName, accountName) {
  let logId = entry.logId;
  if (!logId && ["completed", "failed"].includes(entry.phase)) {
    await refreshMonitor(true);
    logId = state.monitor.find((candidate) => candidate.id === entry.id)?.logId;
  }
  if (!logId) {
    notify(
      t(
        "Full log is still being written. Try again after the request completes.",
      ),
      "info",
    );
    return;
  }
  const log = await perform(() => fetchLog(logId, false));
  if (!log) {
    return;
  }
  await copyText(
    buildFullLogClipboardText(log, providerName, accountName),
    t("Full interaction log copied."),
  );
}

async function syncProviderConfig(provider) {
  if (!provider) {
    return null;
  }
  const guide = buildOnboardingGuide(
    provider.protocol === "anthropic" ? "claude-code" : "codex",
    provider,
  );
  return syncOnboardingGuideConfig(guide);
}

async function syncOnboardingGuideConfig(guide) {
  if (!guide?.baseUrl) {
    const failureMessage =
      guide?.target === "claude-code"
        ? t("Failed to sync Claude config.")
        : t("Failed to sync Codex config.");
    notify(failureMessage, "error");
    return null;
  }

  if (guide.target === "claude-code") {
    try {
      const result = await invokeDesktop("sync_claude_config", {
        baseUrl: guide.baseUrl,
      });
      notify(t("Claude config synced."), "success");
      return result;
    } catch (error) {
      console.error(error);
      notify(error?.message || t("Failed to sync Claude config."), "error");
      return null;
    }
  }

  if (!guide?.providerSlug) {
    notify(t("Failed to sync Codex config."), "error");
    return null;
  }

  try {
    const result = await invokeDesktop("sync_codex_config", {
      providerSlug: guide.providerSlug,
      providerName: guide.providerName || guide.title || guide.providerSlug,
      baseUrl: guide.baseUrl,
    });
    notify(
      result?.defaultProviderUpdated === false
        ? t(
            "Codex config synced by updating the existing model_provider base_url.",
          )
        : t("Codex config synced."),
      "success",
    );
    return result;
  } catch (error) {
    console.error(error);
    notify(error?.message || t("Failed to sync Codex config."), "error");
    return null;
  }
}

function normalizeAccountProviderFilter() {
  if (!state.providers.length) {
    state.accountProviderFilter = "";
    return state.accountProviderFilter;
  }

  const filterExists = state.providers.some(
    (provider) => provider.slug === state.accountProviderFilter,
  );
  const selectedHasAccounts = state.accounts.some(
    (account) => account.provider === state.accountProviderFilter,
  );
  if (
    !filterExists ||
    (!state.accountProviderFilterTouched &&
      state.accounts.length > 0 &&
      !selectedHasAccounts)
  ) {
    state.accountProviderFilter =
      state.accounts.find((account) =>
        state.providers.some((provider) => provider.slug === account.provider),
      )?.provider || state.providers[0].slug;
  }
  return state.accountProviderFilter;
}

function filteredAccounts() {
  const activeFilter = normalizeAccountProviderFilter();
  const accounts = state.accounts.filter(
    (account) => account.provider === activeFilter,
  );

  return accounts
    .map((account, index) => ({ account, index }))
    .sort((left, right) => {
      const leftDefault = isDefaultAccount(left.account);
      const rightDefault = isDefaultAccount(right.account);
      if (leftDefault !== rightDefault) {
        return leftDefault ? -1 : 1;
      }
      return left.index - right.index;
    })
    .map(({ account }) => account);
}

function providerOptionsForSelect(preferredSlug) {
  const candidates = [];
  const pushProvider = (provider) => {
    if (
      !provider ||
      candidates.some((candidate) => candidate.slug === provider.slug)
    ) {
      return;
    }
    candidates.push(provider);
  };

  if (preferredSlug) {
    pushProvider(
      state.providers.find((provider) => provider.slug === preferredSlug),
    );
  }

  state.providers
    .filter((provider) => provider.enabled)
    .forEach((provider) => pushProvider(provider));

  return candidates;
}

function providerOptionLabel(provider) {
  return provider.enabled
    ? `${provider.displayName} (${protocolDisplayLabel(provider.protocol)})`
    : `${provider.displayName} (${protocolDisplayLabel(provider.protocol)}, ${t("disabled")})`;
}

function defaultRouteForProvider(provider) {
  return (
    state.routes.find(
      (route) => route.provider === provider && !route.modelPrefix,
    ) || null
  );
}

function isDefaultAccount(account) {
  return defaultRouteForProvider(account.provider)?.accountId === account.id;
}

function routeAccountOptionsForProvider(provider, preferredAccountId) {
  const candidates = [];
  const pushAccount = (account) => {
    if (
      !account ||
      candidates.some((candidate) => candidate.id === account.id)
    ) {
      return;
    }
    candidates.push(account);
  };

  if (preferredAccountId) {
    pushAccount(
      state.accounts.find(
        (account) =>
          account.provider === provider && account.id === preferredAccountId,
      ),
    );
  }

  state.accounts
    .filter((account) => account.provider === provider && account.enabled)
    .forEach((account) => pushAccount(account));

  return candidates;
}

async function perform(action, successMessage, onError) {
  try {
    const response = await action();
    if (successMessage) {
      notify(successMessage, "success");
    }
    return response;
  } catch (error) {
    console.error(error);
    if (typeof onError === "function") {
      onError(error);
    }
    return null;
  }
}

async function performDesktop(action, successMessage, failureMessage) {
  try {
    const response = await action();
    if (successMessage) {
      notify(successMessage, "success");
    }
    return response;
  } catch (error) {
    console.error(error);
    notify(error?.message || failureMessage, "error");
    return null;
  }
}

async function invokeDesktop(command, args = {}) {
  const invoke = window.__TAURI__?.core?.invoke
    ? window.__TAURI__.core.invoke.bind(window.__TAURI__.core)
    : window.__TAURI_INTERNALS__?.invoke?.bind(window.__TAURI_INTERNALS__);
  if (!invoke) {
    throw new Error(t("Desktop integration is unavailable in this context."));
  }
  return invoke(command, args);
}

async function api(path, options = {}) {
  let response;
  try {
    response = await fetch(`${daemonBaseUrl()}${path}`, {
      method: options.method || "GET",
      headers: {
        "Content-Type": "application/json",
        ...(options.headers || {}),
      },
      body: options.body ? JSON.stringify(options.body) : undefined,
    });
  } catch (error) {
    if (!options.silent) {
      notify(
        t("Cannot reach the local daemon on {address}.", {
          address: daemonAddress(),
        }),
        "error",
      );
    }
    throw error;
  }

  const text = await response.text();
  const payload = text ? safeJson(text) : null;
  if (!response.ok) {
    const rawMessage =
      payload?.error || `${response.status} ${response.statusText}`;
    const message = translateApiError(path, rawMessage);
    if (!options.silent || isCompatibilityError(message)) {
      notify(message, "error");
    }
    throw new Error(message);
  }
  return payload;
}

function translateApiError(path, message) {
  if (message === `resource not found: ${path}` && path.startsWith("/admin/")) {
    return t(
      "Daemon on {address} is running, but it does not support {path}. This usually means an older LocalAIRouter daemon is still occupying the port. Stop that process and restart the desktop app.",
      { address: daemonAddress(), path },
    );
  }
  const domainMessage = translateDomainError(message);
  if (domainMessage) {
    return domainMessage;
  }
  return message;
}

function translateDomainError(message) {
  const normalized = message.startsWith("validation error: ")
    ? message.slice("validation error: ".length)
    : message;

  let match = normalized.match(
    /^built-in provider `([^`]+)` cannot be deleted$/,
  );
  if (match) {
    return t("Built-in provider {slug} cannot be deleted.", {
      slug: match[1],
    });
  }

  match = normalized.match(/^provider `([^`]+)` still has (\d+) account\(s\)$/);
  if (match) {
    return t(
      "Provider {slug} still has {count} account(s). Remove its accounts before deleting it.",
      {
        slug: match[1],
        count: match[2],
      },
    );
  }

  match = normalized.match(/^provider `([^`]+)` still has (\d+) route\(s\)$/);
  if (match) {
    return t(
      "Provider {slug} still has {count} route(s). Remove its routes before deleting it.",
      {
        slug: match[1],
        count: match[2],
      },
    );
  }

  match = normalized.match(/^provider `([^`]+)` is disabled$/);
  if (match) {
    return t("Provider {slug} is disabled.", {
      slug: match[1],
    });
  }

  match = normalized.match(
    /^account `([^`]+)` does not belong to provider `([^`]+)`$/,
  );
  if (match) {
    return t("Account {id} does not belong to provider {slug}.", {
      id: match[1],
      slug: match[2],
    });
  }

  match = normalized.match(/^account `([^`]+)` is disabled$/);
  if (match) {
    return t("Account {id} is disabled.", {
      id: match[1],
    });
  }

  match = normalized.match(/^selected account `([^`]+)` is disabled$/);
  if (match) {
    return t("Selected account {name} is disabled.", {
      name: match[1],
    });
  }

  match = normalized.match(
    /^selected account `([^`]+)` for provider `([^`]+)`$/,
  );
  if (match) {
    return t("Selected account {id} for provider {slug} was not found.", {
      id: match[1],
      slug: match[2],
    });
  }

  match = normalized.match(/^default route for provider `([^`]+)`$/);
  if (match) {
    return t("Provider {slug} has no default route.", {
      slug: match[1],
    });
  }

  match = normalized.match(/^secret missing for account `([^`]+)`$/);
  if (match) {
    return t("Secret is missing for account {id}.", {
      id: match[1],
    });
  }

  if (normalized === "new accounts require an API key") {
    return t("New accounts require an API key.");
  }

  if (normalized === "API key must not be empty when provided") {
    return t("API key must not be empty when provided.");
  }

  match = message.match(/^resource not found: provider `([^`]+)`$/);
  if (match) {
    return t("Provider {slug} was not found.", {
      slug: match[1],
    });
  }

  match = message.match(/^resource not found: account `([^`]+)`$/);
  if (match) {
    return t("Account {id} was not found.", {
      id: match[1],
    });
  }

  return null;
}

function isCompatibilityError(message) {
  return message.includes("older LocalAIRouter daemon");
}

function safeJson(text) {
  try {
    return JSON.parse(text);
  } catch {
    return { raw: text };
  }
}

function notify(message, tone = "info") {
  const toast = document.createElement("div");
  toast.className = `toast ${tone}`;
  toast.textContent = message;
  elements.toastStack.appendChild(toast);
  window.setTimeout(() => {
    toast.remove();
  }, 3200);
}

function emptyNode(message = t("Nothing to show yet.")) {
  const node = elements.emptyTemplate.content.firstElementChild.cloneNode(true);
  node.textContent = message;
  return node;
}

function normalizeOptional(value) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

function routeBindingId(provider, modelPrefix) {
  return modelPrefix ? `${provider}::${modelPrefix}` : `${provider}::*`;
}

function normalizeSegment(value) {
  return value
    .trim()
    .toLowerCase()
    .replaceAll("/", "-")
    .replaceAll("_", "-")
    .replace(/\s+/g, "-");
}

function isValidSlug(value) {
  return /^[a-z0-9-]+$/.test(value);
}

function isSuccessStatus(statusCode) {
  return typeof statusCode === "number" && statusCode < 400;
}

function currentUtcOffsetMinutes() {
  return -new Date().getTimezoneOffset();
}

function formatLocalDay(date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function buildDailyStatsSeries(days) {
  const count = Math.max(1, days || 30);
  const end = new Date();
  end.setHours(0, 0, 0, 0);
  const source = new Map(
    state.dailyStats.map((point) => [
      point.day,
      {
        requestCount: finiteNumber(point.requestCount) ?? 0,
        successCount: finiteNumber(point.successCount) ?? 0,
        totalTokens: finiteNumber(point.totalTokens) ?? 0,
      },
    ]),
  );

  const series = [];
  for (let offset = count - 1; offset >= 0; offset -= 1) {
    const day = new Date(end);
    day.setDate(end.getDate() - offset);
    const key = formatLocalDay(day);
    const point = source.get(key);
    series.push({
      day: key,
      requestCount: point?.requestCount ?? 0,
      successCount: point?.successCount ?? 0,
      totalTokens: point?.totalTokens ?? 0,
    });
  }
  return series;
}

function todayDailyStats() {
  const key = formatLocalDay(new Date());
  return state.dailyStats.find((point) => point.day === key) || null;
}

function todayLogQuery() {
  const now = new Date();
  const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const end = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  return {
    createdFrom: start.toISOString(),
    createdTo: end.toISOString(),
  };
}

function averageLatency(logs) {
  if (!logs.length) {
    return null;
  }
  const total = logs.reduce((sum, log) => sum + log.durationMs, 0);
  return Math.round(total / logs.length);
}

function percentileLatency(logs, percentile) {
  if (!logs.length) {
    return null;
  }
  const sorted = logs
    .map((log) => log.durationMs)
    .sort((left, right) => left - right);
  const index = Math.min(
    sorted.length - 1,
    Math.ceil(sorted.length * percentile) - 1,
  );
  return sorted[index];
}

function latencyMetricLogs(logs) {
  return logs.filter(
    (log) =>
      isSuccessStatus(log.statusCode) &&
      !log.errorText &&
      typeof log.durationMs === "number" &&
      Number.isFinite(log.durationMs) &&
      log.durationMs >= 0,
  );
}

function aggregateTokenUsage(logs) {
  return logs.reduce(
    (aggregate, log) => {
      const persistedTotal = finiteNumber(log.totalTokens);
      if (persistedTotal != null) {
        aggregate.total += persistedTotal;
        return aggregate;
      }
      const usage = extractLogTokenUsage(log);
      if (!usage) {
        return aggregate;
      }
      aggregate.input += usage.input ?? 0;
      aggregate.output += usage.output ?? 0;
      aggregate.total +=
        usage.total ?? (usage.input ?? 0) + (usage.output ?? 0);
      return aggregate;
    },
    { input: 0, output: 0, total: 0 },
  );
}

function extractLogTokenUsage(log) {
  if (
    !isSuccessStatus(log.statusCode) ||
    log.errorText ||
    typeof log.responseBody !== "string"
  ) {
    return null;
  }
  const body = log.responseBody.trim();
  if (!body) {
    return null;
  }

  const directPayload = safeJsonParse(body);
  const directUsage = directPayload
    ? extractTokenUsageFromPayload(directPayload)
    : null;
  if (hasTokenUsage(directUsage)) {
    return normalizeTokenUsage(directUsage);
  }

  const sseUsage = extractTokenUsageFromSse(body);
  return hasTokenUsage(sseUsage) ? normalizeTokenUsage(sseUsage) : null;
}

function extractTokenUsageFromSse(body) {
  return body.split(/\r?\n/).reduce((usage, rawLine) => {
    const line = rawLine.trim();
    if (!line.startsWith("data:")) {
      return usage;
    }
    const data = line.slice(5).trim();
    if (!data || data === "[DONE]") {
      return usage;
    }
    const payload = safeJsonParse(data);
    if (!payload) {
      return usage;
    }
    return mergeTokenUsage(usage, extractTokenUsageFromPayload(payload));
  }, null);
}

function extractTokenUsageFromPayload(payload) {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const usageCandidates = [];
  if (payload.usage && typeof payload.usage === "object") {
    usageCandidates.push(payload.usage);
  }
  if (payload.message?.usage && typeof payload.message.usage === "object") {
    usageCandidates.push(payload.message.usage);
  }
  if (payload.response?.usage && typeof payload.response.usage === "object") {
    usageCandidates.push(payload.response.usage);
  }

  for (const usage of usageCandidates) {
    const promptTokens = finiteNumber(usage.prompt_tokens);
    const inputTokens = finiteNumber(usage.input_tokens);
    const promptCacheHitTokens = finiteNumber(usage.prompt_cache_hit_tokens);
    const promptCacheMissTokens = finiteNumber(usage.prompt_cache_miss_tokens);
    const cacheCreationTokens = finiteNumber(usage.cache_creation_input_tokens);
    const cacheReadTokens = finiteNumber(usage.cache_read_input_tokens);
    const completionTokens = finiteNumber(usage.completion_tokens);
    const outputTokens = finiteNumber(usage.output_tokens);
    const totalTokens = finiteNumber(usage.total_tokens);

    const cacheInput = (cacheCreationTokens ?? 0) + (cacheReadTokens ?? 0);
    const deepseekInput =
      promptCacheHitTokens != null || promptCacheMissTokens != null
        ? (promptCacheHitTokens ?? 0) + (promptCacheMissTokens ?? 0)
        : null;
    const inputBase = promptTokens ?? inputTokens ?? deepseekInput ?? null;
    const input =
      inputBase != null
        ? inputBase + cacheInput
        : cacheInput > 0
          ? cacheInput
          : null;
    const output = completionTokens ?? outputTokens ?? null;
    const total =
      input != null || output != null
        ? (input ?? 0) + (output ?? 0)
        : (totalTokens ?? null);

    const normalized = normalizeTokenUsage({ input, output, total });
    if (hasTokenUsage(normalized)) {
      return normalized;
    }
  }

  return null;
}

function mergeTokenUsage(currentUsage, nextUsage) {
  if (!hasTokenUsage(nextUsage)) {
    return currentUsage;
  }
  if (!hasTokenUsage(currentUsage)) {
    return normalizeTokenUsage(nextUsage);
  }
  return normalizeTokenUsage({
    input: Math.max(currentUsage.input ?? 0, nextUsage.input ?? 0),
    output: Math.max(currentUsage.output ?? 0, nextUsage.output ?? 0),
    total: Math.max(currentUsage.total ?? 0, nextUsage.total ?? 0),
  });
}

function normalizeTokenUsage(usage) {
  if (!usage) {
    return null;
  }
  return {
    input: finiteNumber(usage.input),
    output: finiteNumber(usage.output),
    total: finiteNumber(usage.total),
  };
}

function hasTokenUsage(usage) {
  return Boolean(
    usage &&
    [usage.input, usage.output, usage.total].some(
      (value) => typeof value === "number" && Number.isFinite(value),
    ),
  );
}

function safeJsonParse(value) {
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function finiteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function formatSuccessRate(logs) {
  if (!logs.length) {
    return "--";
  }
  const successCount = logs.filter((log) =>
    isSuccessStatus(log.statusCode),
  ).length;
  return formatSuccessRateFromCounts(logs.length, successCount);
}

function formatSuccessRateFromCounts(totalCount, successCount) {
  if (!totalCount) {
    return "--";
  }
  return `${Math.round((successCount / totalCount) * 100)}%`;
}

function formatLatency(value) {
  return typeof value === "number" ? `${value} ms` : "--";
}

function formatTokenCount(value) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.round(value).toLocaleString(state.locale)
    : "--";
}

function closeMetricTooltips(resetState = true) {
  if (resetState) {
    state.openMetricTooltip = null;
  }
  document.querySelectorAll(".metric-tooltip").forEach((tooltip) => {
    tooltip.hidden = true;
  });
  document.querySelectorAll(".metric-help").forEach((button) => {
    button.classList.remove("is-open");
    button.setAttribute("aria-expanded", "false");
  });
}

function formatDateTime(value) {
  if (!value) {
    return t("Unavailable");
  }
  return new Date(value).toLocaleString(state.locale);
}

function formatRelativeTime(value) {
  const timestamp = new Date(value).getTime();
  if (Number.isNaN(timestamp)) {
    return t("unknown");
  }
  const deltaMs = Date.now() - timestamp;
  const deltaMinutes = Math.round(deltaMs / 60000);
  if (deltaMinutes < 1) {
    return t("just now");
  }
  if (deltaMinutes < 60) {
    return state.locale === "zh-CN"
      ? `${deltaMinutes} 分钟前`
      : `${deltaMinutes}m ago`;
  }
  const deltaHours = Math.round(deltaMinutes / 60);
  if (deltaHours < 24) {
    return state.locale === "zh-CN"
      ? `${deltaHours} 小时前`
      : `${deltaHours}h ago`;
  }
  const deltaDays = Math.round(deltaHours / 24);
  return state.locale === "zh-CN" ? `${deltaDays} 天前` : `${deltaDays}d ago`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function truncateMiddle(value, maxChars) {
  if (!value || value.length <= maxChars) {
    return value;
  }
  const lead = Math.max(4, Math.floor((maxChars - 1) / 2));
  const tail = Math.max(4, maxChars - lead - 1);
  return `${value.slice(0, lead)}…${value.slice(-tail)}`;
}
