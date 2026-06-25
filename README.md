# Hiphup Terminal

基于 **Tauri 2.0 + Vue 3 + Rust** 的现代化 SSH 终端与运维工具箱，集成 AI 助手、Redis 管理、API 调试、局域网聊天与 WebDAV 同步。

![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![Stack](https://img.shields.io/badge/Stack-Rust%20%7C%20Vue%203%20%7C%20Deno-green)

## 功能概览

| 模块 | 能力 |
|------|------|
| **SSH 终端** | 多会话、跳板机、本地 Shell、xterm.js 渲染 |
| **SFTP** | 远程/本地文件浏览、上传下载、权限修改 |
| **AI 助手** | 多模型流式对话，Markdown 代码块一键注入终端 |
| **快捷命令** | 分组管理常用 Shell 命令 |
| **Redis** | 连接管理、Key 浏览与编辑 |
| **API 调试** | HTTP / WebSocket / SSE / Socket.IO / MQTT；Collections、Environments、History；Postman 导入导出 |
| **局域网聊天** | P2P 消息与文件 |
| **同步** | WebDAV 双向同步（服务器、命令、AI、Redis 配置） |
| **主题** | 多主题切换，终端配色联动 |

## 快速开始

### 环境要求

- [Rust](https://www.rust-lang.org/tools/install)（stable）
- [Deno](https://docs.deno.com/) 2.x
- Linux 额外依赖：`libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`

### 安装与运行

```bash
# 安装依赖
deno install

# 开发（Tauri + 前端热更新）
deno task tauri dev

# 仅前端
deno task dev

# 类型检查 + Rust 编译检查
deno task check

# 生产打包
deno task tauri build
```

## 项目结构

```
hiphup-terminal/
├── src/                          # Vue 3 前端
│   ├── App.vue                   # 主壳：SSH 工作区 + 右侧面板
│   ├── main.ts                   # 入口，全局组件注册
│   ├── assets/css/               # base.scss（主题变量）、app.scss（布局）
│   ├── components/               # UI 组件（见下方分组）
│   └── utils/                    # 工具函数、invoke 封装、Toast/Confirm
│
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── lib.rs                # 入口：SSH/SFTP/AI/命令注册
│   │   ├── sync.rs               # WebDAV 同步
│   │   ├── api_debugger.rs       # API 调试持久化
│   │   ├── redis_manager.rs      # Redis
│   │   ├── p2p.rs                # 局域网聊天
│   │   ├── local_shell.rs        # 本地终端
│   │   └── security.rs           # 敏感数据加密
│   ├── tauri.conf.json           # Tauri 配置、版本号
│   └── capabilities/             # 权限声明
│
├── docs/                         # 维护文档
│   ├── ARCHITECTURE.md           # 架构与模块对照
│   ├── DATA.md                   # Redb 表、同步范围、API Store
│   └── EXTENDING.md              # 扩展新功能指南
│
├── .github/workflows/release.yml # 标签触发 CI 发布
├── deno.json                     # Deno 任务定义
├── package.json                  # npm 依赖
├── vite.config.ts                # Vite（端口 1420）
└── update.json                   # 应用内更新元数据
```

### 前端组件分组

| 分组 | 文件 |
|------|------|
| **布局/壳** | `TitleBar`, `Sidebar`, `TerminalTabs`, `WorkspaceHeader`, `StatusBar` |
| **SSH/SFTP** | `ServerModal`, `SftpFileDialog` |
| **AI** | `AiAssistantPanel` |
| **Redis** | `RedisManager`, `RedisCreateModal` |
| **API 调试** | `ApiDebuggerPanel`, `ApiDebuggerCollections`, `ApiDebuggerEnvironments`, `ApiDebuggerHistory`, `ApiDebuggerSaveRequestDialog`, `ApiDebuggerDataMenu`, `ApiDebuggerInputDialog`, `HeaderComboInput`, `api-debugger-manager.scss` |
| **聊天** | `ChatPanel` |
| **同步/主题/命令** | `SyncSettings`, `ThemeSettings`, `QuickCommandPanel` |
| **通用 UI** | `Tooltip`, `AppSelect`, `NumberInput` |

### 前端工具

| 文件 | 用途 |
|------|------|
| `utils/session.ts` | 会话类型、本地 Shell ID |
| `utils/theme.ts` / `terminalTheme.ts` | 应用与终端主题 |
| `utils/toast.ts` / `confirm.ts` | 全局反馈 |
| `utils/apiDebugger.ts` | API 调试纯前端工具 |
| `utils/apiDebuggerStorage.ts` | API 调试 invoke + 类型 + 导入导出 |

## 技术栈

| 模块 | 选型 |
|------|------|
| 桌面 | Tauri 2.0 |
| 前端 | Vue 3 Composition API + TypeScript |
| 构建 | Vite 6 + Deno 2 |
| 终端 | xterm.js + WebGL addon |
| 本地库 | Redb |
| SSH | russh + russh-sftp |
| HTTP 代理 | reqwest（`send_http_request`） |
| 实时协议 | 浏览器 WebSocket / EventSource；socket.io-client；mqtt |

## 数据与同步

- 配置持久化在 **Redb** 嵌入式数据库
- **WebDAV 同步**：SSH 服务器、快捷命令、AI 配置、Redis 连接
- **不同步**：API 调试数据、P2P 消息（有独立导入导出）

详见 [docs/DATA.md](docs/DATA.md)。

## 维护文档

| 文档 | 内容 |
|------|------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 运行时结构、前后端模块对照、命令注册 |
| [docs/DATA.md](docs/DATA.md) | Redb 表、Store 结构、同步边界 |
| [docs/EXTENDING.md](docs/EXTENDING.md) | 新增命令、面板、API 协议、发布流程 |

## 发布

```bash
git tag v1.x.x
git push origin v1.x.x
```

GitHub Actions 自动构建 Windows / macOS 安装包。发布前更新 `src-tauri/tauri.conf.json` 版本与 `update.json`。

## 开源协议

MIT License

## 致谢

[Tauri](https://tauri.app/) · [Vue 3](https://vuejs.org/) · [xterm.js](https://xtermjs.org/) · [Font Awesome](https://fontawesome.com/)
