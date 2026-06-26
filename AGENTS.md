# Agent / 维护者速查

> 给 AI 助手或新贡献者的快速上下文，详细说明见 `docs/`。

## 项目是什么

Tauri 2 桌面 SSH 终端 + 运维工具箱（AI、Redis、API 调试、P2P 聊天、WebDAV 同步）。

## 关键入口

| 层级 | 文件 |
|------|------|
| 前端主壳 | `src/App.vue` |
| 前端入口 | `src/main.ts` |
| 后端入口 | `src-tauri/src/lib.rs` |
| Tauri 配置 | `src-tauri/tauri.conf.json` |
| 构建 | `deno.json` → `deno task tauri dev` / `build` |

## 右侧面板 lazy load

`App.vue` → `panelMap`：`quick` | `ai` | `redis` | `api` | `chat` | `sync-settings` | `theme-settings`

## Redb 表（`lib.rs` 常量）

`ssh_servers` · `quick_commands` · `ai_settings` · `sync_config` · `redis_connections` · `p2p_messages` · `p2p_remarks` · `api_debugger`

**同步含**：servers, commands, ai, redis, sync_config  
**同步不含**：api_debugger, p2p

## API 调试文件簇

- 面板：`src/components/ApiDebuggerPanel.vue`（~1700 行，含 HTTP/WS/SSE/SIO/MQTT）
- 子组件：`ApiDebuggerCollections/Environments/History/SaveRequestDialog/DataMenu/InputDialog`
- 工具：`src/utils/apiDebugger.ts` + `apiDebuggerStorage.ts`
- 后端：`src-tauri/src/api_debugger.rs`
- 样式共享：`api-debugger-manager.scss`（用 `@use` 引入）

长连接统一用 `CloseIntent`（manual / remote / error / unmount）+ `finalize*()` 避免重复日志。

## 编码约定

- Vue 3 `<script setup lang="ts">`
- 全局组件：`Tooltip`, `AppSelect`, `NumberInput`（`main.ts` 注册）
- 勿用原生 `title`，用 `Tooltip`
- Sass `@use` 不用 `@import`
- 最小 diff，匹配现有风格
- 仅用户要求时 git commit

## 前端依赖安装（必读）

本项目通过 **Deno** 管理 npm 依赖（`deno.json` 中 `"nodeModulesDir": "auto"`），**禁止使用 `npm install`**。

| 场景 | 正确命令 | 禁止 |
|------|----------|------|
| 首次克隆 / 全量安装 | `deno install --allow-scripts` 或 `deno task install` | `npm install` |
| 新增生产依赖 | `deno install npm:包名` | `npm install 包名` |
| 新增开发依赖 | `deno install --dev npm:包名` | `npm install -D 包名` |
| 重装 / 更新 lock | `deno install --allow-scripts` | `npm ci` / `npm update` |

说明：

- `--allow-scripts`：允许 npm 生命周期脚本（如 `sass-embedded` 等原生模块）
- Deno 会同步更新 `package.json` 与 `deno.lock`，并写入 `node_modules/`
- 构建与开发仍通过 `deno task dev` / `deno task tauri dev` 调用 Vite，与 npm scripts 兼容

## 拖放规范（必读，勿破坏）

Tauri 桌面端存在**两套互斥**的拖放机制，混用会导致禁止图标、SFTP 上传失效、主机排序失灵。**改一处必须隔离，不得影响另一处。**

### 机制对照

| 场景 | 正确做法 | 禁止 |
|------|----------|------|
| 资源管理器 → SFTP 远程面板上传 | Tauri 原生 `onDragDropEvent`（`App.vue` → `setupNativeDragDrop`） | 不要用 HTML5 `@drop` / `dataTransfer.files` 替代 |
| SFTP 本地列表 → 远程列表 | Pointer 拖拽（`beginPointerDrag`，`onSftpFilePointerDown`） | 不要给文件行加 `draggable="true"` |
| 主机列表排序 / 改分组 | Pointer 拖拽（`Sidebar.vue` → `onHostNamePointerDown`） | 不要给主机项加 `draggable="true"` |

**原因**：Tauri 默认 `dragDropEnabled: true` 时，WebView2 上 HTML5 Drag and Drop API 与原生文件拖放冲突。业内（VS Code、SortableJS、@dnd-kit）对**应用内**排序/跨区拖也用 Pointer，不用 `draggable`。

### 共享工具

- **`src/utils/pointerDrag.ts`**：`beginPointerDrag`、`findAttrFromPoint`
- 拖动阈值默认 6px，区分点击与拖动
- 拖动中源元素设 `pointer-events: none`，便于 `elementFromPoint` 命中目标

### 涉及文件（改动时只动对应文件）

| 功能 | 文件 | 要点 |
|------|------|------|
| OS 文件上传到 SFTP | `src/App.vue`（`setupNativeDragDrop`、`handleOsFileDrop`） | 保持 `dragDropEnabled` 默认 true，勿关 |
| SFTP 面板内互拖 | `src/App.vue`（`onSftpFilePointerDown`） | 仅 `file-item` 的 `@pointerdown`，无 HTML5 drag 事件 |
| 主机排序/分组 | `src/components/Sidebar.vue` | 仅名称 `@pointerdown` → `onHostNamePointerDown`；`data-host-id` / `data-group-key` 标记落点 |
| 样式 | `src/assets/css/app.scss`（`.sftp-manager--internal-drag`） | 与 Sidebar 的 `.is-dragging` 独立 |

### 隔离 checklist（修改拖放相关代码前自检）

1. **禁止**在项目中新增 `draggable="true"`（SFTP、Sidebar、其他列表均如此）
2. **禁止**在 `document` / `window` 上监听 HTML5 的 `dragover` / `drop` / `dragend`
3. **禁止**在 Sidebar 的 `dragover` 里无条件 `preventDefault`（会拦截 Tauri 原生文件拖放）
4. **禁止**用 `text/plain` 的 `setData` 传递主机/SFTP 载荷（与系统拖放 MIME 冲突）
5. 新增「可拖动 UI」→ 只用 `pointerDrag.ts`，不要引入 HTML5 DnD
6. 改 Sidebar 拖放 → 不要动 `App.vue` 的 SFTP；改 SFTP → 不要动 Sidebar
7. 勿设置 `dragDropEnabled: false`，除非全量迁移 OS 文件上传方案（当前未采用）

### 主机列表交互

- **按住名称**拖动：排序；拖到其他主机 → 同步分组；拖到分组标题 → 移入该分组
- **单击**选中；**双击**连接；**右键**编辑/分组
- 底部提示：`按住名称拖动排序或改分组`

### SFTP 交互

- **按住本地文件行**拖到远程面板：上传
- **从电脑拖文件**到远程面板：Tauri 原生上传
- 高亮：`isDraggingOverRemote` / `drag-over`（原生与 Pointer 共用 highlight 逻辑）

## 常用命令

```bash
deno task install        # 安装前端依赖（勿用 npm install）
deno task check          # vue-tsc + cargo check
deno task tauri dev
deno task tauri build
```

## 扩展新功能

见 [docs/EXTENDING.md](docs/EXTENDING.md)。
