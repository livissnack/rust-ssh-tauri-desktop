# 扩展指南

## 新增 Tauri 命令（后端 → 前端）

### 1. Rust 实现

```rust
// src-tauri/src/your_module.rs
#[tauri::command]
pub fn your_command(arg: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    // ...
    Ok(result)
}
```

### 2. 注册

在 `lib.rs`：

```rust
mod your_module;
use your_module::your_command;

// invoke_handler 数组中加入 your_command
```

### 3. 前端调用

```typescript
import { invoke } from '@tauri-apps/api/core';

const result = await invoke<string>('your_command', { arg: 'value' });
```

命名约定：snake_case 命令名，与 Rust 函数名一致。

---

## 新增右侧面板

1. 创建 `src/components/YourPanel.vue`
2. 在 `App.vue`：

```typescript
const YourPanel = defineAsyncComponent(() => import('./components/YourPanel.vue'));

const panelMap = {
  // ...
  'your-panel': YourPanel,
};
```

3. 侧栏增加图标按钮，设置 `rightPanelType = 'your-panel'`
4. 样式遵循 `base.scss` CSS 变量，与现有面板保持 `height: 100%` + flex 布局

---

## 扩展 API 调试器

### 新增协议 Tab

1. **`utils/apiDebugger.ts`**：扩展 `ApiTab` 类型
2. **`utils/apiDebuggerStorage.ts`**：
   - 扩展 `RequestProtocol`
   - 添加 `toXxxSnapshot()` 工厂函数
   - 更新 `defaultRequestName` / `snapshotTagLabel`
3. **`api_debugger.rs`**：在 `HttpRequestSnapshot` 增加字段（`#[serde(default)]` 保证兼容）
4. **`ApiDebuggerPanel.vue`**：
   - Tab 配置、`currentSnapshot` computed
   - `loadRequestSnapshot` 分支
   - 连接生命周期（参考 `CloseIntent` + `finalize*` 模式）
5. **`ApiDebuggerCollections.vue`**：协议标签样式（`methodClass`）

### 连接生命周期模板

长连接（WS/SSE/SIO/MQTT）统一模式：

```typescript
type CloseIntent = 'manual' | 'remote' | 'error' | 'unmount';

const xxxSession = { intent: null as CloseIntent | null };
const xxxActive = ref(false);    // 连接进行中或已连接
const xxxConnected = ref(false); // 已成功握手

const finalizeXxx = (intent: CloseIntent, detail?: string) => {
  if (!xxxActive.value) return;
  logConnectionEnd('Protocol', intent, detail);
  // reset refs + client
};

const disconnectXxx = (intent: CloseIntent = 'manual') => {
  xxxSession.intent = intent;
  // close client; callback calls finalizeXxx
};
```

UI：`!xxxActive` → Connect；`xxxActive && !xxxConnected` → Cancel；`xxxConnected` → Disconnect。

---

## 新增 Redb 表（不参与同步）

参考 `api_debugger.rs`：

```rust
pub const YOUR_TABLE: TableDefinition<&str, &str> = TableDefinition::new("your_table");
const STORE_KEY: &str = "store";

pub fn read_store(db: &Database) -> Result<YourStore, String> { /* ... */ }
pub fn write_store(db: &Database, store: &YourStore) -> Result<(), String> { /* ... */ }
```

---

## 样式约定

- 组件 scoped SCSS，复用 `var(--accent)`、`var(--bg-input)` 等变量
- Sass 使用 `@use` 而非 `@import`（Dart Sass 3 兼容）
- 下拉：`AppSelect.vue`；Tooltip：`Tooltip.vue`；勿用原生 `title`
- API 调试共享样式：`api-debugger-manager.scss`，通过 `@use` 引入

---

## 开发命令

```bash
deno install          # 安装 npm 依赖（nodeModulesDir: auto）
deno task dev           # 仅前端
deno task tauri dev     # 完整桌面开发
deno task check         # 前端类型检查 + cargo check
deno task tauri build   # 生产打包
```

---

## 发布

1. 更新 `src-tauri/tauri.conf.json` 的 `version`
2. 更新 `update.json`（应用内更新）
3. 提交并打 tag：`git tag v1.x.x && git push origin v1.x.x`
4. GitHub Actions 自动构建 Release
