# 数据与持久化

## 存储位置

Redb 数据库文件位于 Tauri 应用数据目录（由系统分配，各平台路径不同）。所有结构化配置均序列化为 JSON 存入 Redb。

## Redb 表一览

| 表名 | 常量 | 内容 | WebDAV 同步 |
|------|------|------|-------------|
| `ssh_servers` | `SERVERS_TABLE` | SSH 服务器配置 | ✅ |
| `quick_commands` | `COMMANDS_TABLE` | 快捷命令 | ✅ |
| `ai_settings` | `AI_CONFIG_TABLE` | AI 提供商配置 | ✅ |
| `sync_config` | `SYNC_CONFIG_TABLE` | 同步账号设置 | ✅ |
| `redis_connections` | `REDIS_CONN_TABLE` | Redis 连接配置 | ✅ |
| `p2p_messages` | `P2P_MESSAGES_TABLE` | P2P 聊天消息 | ❌ |
| `p2p_remarks` | `P2P_REMARKS_TABLE` | P2P 备注 | ❌ |
| `api_debugger` | `API_DEBUGGER_TABLE` | API 调试数据 | ❌ |

> **设计原则**：API 调试数据体积大、变更频繁，且含 Postman 导入导出能力，** intentionally 不参与 WebDAV 全量同步**，避免污染同步包与冲突。

## 同步数据包结构

`sync.rs` 中 `FullSyncData` 包含：

- `servers`
- `commands`
- `ai_config`
- `redis_configs`
- `sync_config`
- `revision` / `device_id` / `timestamp`（冲突与版本控制）

不含 `api_debugger`、P2P 消息。

## API 调试 Store 结构

前端类型：`src/utils/apiDebuggerStorage.ts`  
后端类型：`src-tauri/src/api_debugger.rs`

```typescript
ApiDebuggerStore {
  collections: ApiCollection[]      // 集合 + 已保存请求
  environments: ApiEnvironment[]      // 环境变量
  history: HistoryEntry[]             // 最近 100 条 HTTP 历史
  activeEnvId: string | null
}
```

### RequestSnapshot（多协议）

| 字段 | 说明 |
|------|------|
| `protocol` | `http` \| `ws` \| `sse` \| `socketio` \| `mqtt`（缺省为 http） |
| `method`, `url`, `headers`, `body`, `bodyType` | HTTP 通用 |
| `message` | WebSocket 默认消息 |
| `path`, `event`, `payload` | Socket.IO |
| `clientId`, `username`, `password`, `subTopic`, `pubTopic`, `pubMessage` | MQTT |

### 导入导出格式

| 格式 | 命令 | 说明 |
|------|------|------|
| Hiphup 完整备份 | `export_api_debugger_file` / `import` | JSON，含 collections + environments + history |
| Postman Collection | 同上 | 仅 HTTP 请求 |
| Postman Environment | 同上 | 环境变量 |

Postman 导出**自动过滤**非 HTTP 协议条目。

## 敏感数据

- SSH 密码、Redis 密码、同步密码等经 `security.rs` 加密后存库
- API Key（AI）同样加密存储
- 前端 `localStorage` 曾用于 API 调试数据，已迁移至 Redb；`apiDebuggerStorage.ts` 含一次性 legacy 迁移逻辑

## 历史与上限

| 数据 | 上限 | 位置 |
|------|------|------|
| HTTP History | 100 条 | `pushHistory()` / Rust `MAX_HISTORY` |
| Console 日志 | 500 条 | `ApiDebuggerPanel.vue` `appendLog` |
| History UI 分页 | 默认 40 条/页 | `ApiDebuggerHistory.vue` |

## 新增持久化模块 checklist

1. 在 `lib.rs` 定义 `TableDefinition` 常量
2. 在 `init_db` / 启动逻辑中 `create_table`
3. 新建 `src-tauri/src/your_module.rs`，实现 read/write + `#[tauri::command]`
4. 注册到 `invoke_handler`
5. 前端 `utils/yourModuleStorage.ts` 封装 invoke
6. 决定是否加入 `FullSyncData`（参与同步）或独立导入导出
