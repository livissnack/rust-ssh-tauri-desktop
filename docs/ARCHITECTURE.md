# 架构说明

本文档描述 **hiphup-terminal** 的整体架构，便于定位代码与理解模块边界。

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面壳 | Tauri 2.0 |
| 前端 | Vue 3 + TypeScript + Vite |
| 后端 | Rust（Tokio 异步） |
| 本地存储 | Redb（嵌入式 KV） |
| 终端 | xterm.js |
| 任务运行 | Deno 2（驱动 npm scripts） |

## 运行时结构

```
┌─────────────────────────────────────────────────────────────┐
│  WebView (Vue 3)                                            │
│  ┌──────────────┐  ┌──────────────────────────────────────┐ │
│  │ Sidebar      │  │ Workspace (xterm / SFTP)             │ │
│  │ 服务器列表   │  │                                      │ │
│  └──────────────┘  └──────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────────────────────┐│
│  │ 右侧面板（按需 lazy load）                                ││
│  │ AI / Redis / API 调试 / 聊天 / 同步 / 主题 / 快捷命令     ││
│  └──────────────────────────────────────────────────────────┘│
└───────────────────────────┬─────────────────────────────────┘
                            │ invoke / events
┌───────────────────────────▼─────────────────────────────────┐
│  Rust (lib.rs + 子模块)                                      │
│  SSH · SFTP · 本地 Shell · HTTP · Redis · P2P · 同步 · DB    │
└─────────────────────────────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│  Redb 数据库 + 文件系统（密钥、下载等）                       │
└─────────────────────────────────────────────────────────────┘
```

## 前端入口与布局

| 文件 | 职责 |
|------|------|
| `src/main.ts` | 创建 Vue 应用，注册全局组件 `Tooltip` / `AppSelect` / `NumberInput` |
| `src/App.vue` | **主壳**：SSH 会话、终端实例、SFTP、右侧面板切换 |
| `src/assets/css/base.scss` | CSS 变量、主题色、暗色/亮色映射 |
| `src/assets/css/app.scss` | 布局、侧栏、工作区等全局样式 |

### 右侧面板 lazy load

`App.vue` 中通过 `defineAsyncComponent` 按需加载重量级面板，减少首屏体积：

| panel key | 组件 | 说明 |
|-----------|------|------|
| `quick` | `QuickCommandPanel.vue` | 快捷命令 |
| `ai` | `AiAssistantPanel.vue` | AI 助手 |
| `redis` | `RedisManager.vue` | Redis 管理 |
| `api` | `ApiDebuggerPanel.vue` | API 调试（HTTP/WS/SSE/SIO/MQTT） |
| `chat` | `ChatPanel.vue` | 局域网 P2P 聊天 |
| `sync-settings` | `SyncSettings.vue` | WebDAV 同步 |
| `theme-settings` | `ThemeSettings.vue` | 主题 |

新增右侧面板时：创建组件 → 加入 `panelMap` → 在侧栏图标处绑定 `rightPanelType`。

## 后端模块

| Rust 模块 | 文件 | 职责 |
|-----------|------|------|
| 核心 | `lib.rs` | Tauri 初始化、SSH/SFTP/本地文件、AI 代理、命令注册 |
| 同步 | `sync.rs` | WebDAV 双向同步、revision 冲突处理 |
| API 调试 | `api_debugger.rs` | Collections/Environments/History 持久化、Postman 导入导出 |
| Redis | `redis_manager.rs` | Redis 连接与 CRUD |
| P2P | `p2p.rs` | 局域网节点发现、消息、文件 |
| 本地终端 | `local_shell.rs` | 本机 Shell 会话 |
| 安全 | `security.rs` | 敏感字段加解密 |

所有 Tauri 命令在 `lib.rs` 末尾 `invoke_handler` 中注册。新增命令需：实现 `#[tauri::command]` → 注册 → 前端 `invoke('command_name', { ... })`。

## 功能域与文件对照

### SSH 终端 & SFTP

| 前端 | 后端 |
|------|------|
| `App.vue`, `Sidebar.vue`, `TerminalTabs.vue`, `WorkspaceHeader.vue` | `connect_ssh`, `write_to_ssh`, `disconnect_ssh`, `resize_ssh` |
| `SftpFileDialog.vue`, `ServerModal.vue` | `list_remote_dir`, `sftp_upload`, `sftp_download`, … |
| `utils/session.ts` | `local_shell.rs`, `LOCAL_SERVER_ID` |

### AI 助手

| 前端 | 后端 |
|------|------|
| `AiAssistantPanel.vue` | `ask_ai`, `get_ai_config`, `save_ai_config` |

### Redis

| 前端 | 后端 |
|------|------|
| `RedisManager.vue`, `RedisCreateModal.vue` | `redis_*`, `save_redis_config`, … |

### API 调试

| 前端 | 后端 |
|------|------|
| `ApiDebuggerPanel.vue`（主面板） | `send_http_request` |
| `ApiDebuggerCollections.vue` | `get/save_api_debugger_data` |
| `ApiDebuggerEnvironments.vue` | 同上 |
| `ApiDebuggerHistory.vue` | 同上 |
| `ApiDebuggerSaveRequestDialog.vue` | 同上 |
| `ApiDebuggerDataMenu.vue` | `export/import_api_debugger_file` |
| `utils/apiDebugger.ts` | 纯前端工具（Header、日志、Body 格式化） |
| `utils/apiDebuggerStorage.ts` | invoke 封装 + 类型定义 |

### 同步

| 前端 | 后端 |
|------|------|
| `SyncSettings.vue` | `sync_to_cloud`, `sync_from_cloud`, `get/save_sync_settings` |

### 局域网聊天

| 前端 | 后端 |
|------|------|
| `ChatPanel.vue` | `send_p2p_message`, `get_p2p_messages`, `get_online_peers`, … |

## 共享 UI 组件

| 组件 | 用途 |
|------|------|
| `Tooltip.vue` | 全局 tooltip（Teleport + fixed） |
| `AppSelect.vue` | 自定义下拉（HTTP Method、环境选择等） |
| `NumberInput.vue` | 数字输入 |
| `TitleBar.vue` | 无边框窗口标题栏 |
| `StatusBar.vue` | 底部状态栏 |
| `utils/toast.ts` + `Toast.vue` | 轻提示 |
| `utils/confirm.ts` + `Confirm.vue` | 确认对话框 |

## 构建与发布

```
deno task dev          → Vite 开发服务器 (1420)
deno task tauri dev    → Tauri + 前端热更新
deno task build        → vue-tsc + vite build → dist/
deno task tauri build  → 打包各平台安装包
```

CI：`.github/workflows/release.yml`，推送 `v*` 标签触发 Windows/macOS 构建。

## 建议的目录演进（可选）

当前组件平铺在 `src/components/`。若模块继续增大，可按域拆分子目录，例如：

```
src/components/api-debugger/   # ApiDebugger*.vue, HeaderComboInput.vue
src/components/terminal/       # Sidebar, TerminalTabs, SftpFileDialog
src/utils/api-debugger/          # apiDebugger.ts, apiDebuggerStorage.ts
```

拆分时保持 `App.vue` 的 async import 路径同步更新即可。
