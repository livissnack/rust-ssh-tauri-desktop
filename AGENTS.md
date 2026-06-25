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

## 常用命令

```bash
deno task check          # vue-tsc + cargo check
deno task tauri dev
deno task tauri build
```

## 扩展新功能

见 [docs/EXTENDING.md](docs/EXTENDING.md)。
