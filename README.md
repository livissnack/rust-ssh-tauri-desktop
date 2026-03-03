# 🤖 AI SSH Assistant (Tauri + Vue 3 + Rust)

这是一个基于 **Tauri 2.0** 构建的现代化 SSH 终端工具，集成了 AI 智能运维助手。它能够通过流式对话为你提供 Linux 命令建议，并支持一键将命令发送至终端执行。

![Platform Support](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![Tech Stack](https://img.shields.io/badge/Stack-Rust%20%7C%20Vue%203%20%7C%20Deno-green)

## ✨ 功能特性

- 🖥️ **全功能 SSH 终端**：基于 Rust 后端驱动的稳定连接，支持多会话管理。
- 🧠 **多模型 AI 支持**：集成 DeepSeek, 通义千问, 豆包, Gemini 等主流大模型。
- ⚡ **智能命令执行**：AI 建议的 Markdown 代码块支持一键注入终端，告别手动复制粘贴。
- 📝 **Markdown 渲染**：支持流式输出渲染，代码高亮采用 Tokyo Night 深色主题。
- 🔒 **本地安全存储**：使用 Redb 嵌入式数据库存储配置，数据持久化且安全。
- 📦 **自动化发布**：集成 GitHub Actions，支持全平台（Windows, macOS, Linux）自动编译。

---

## 🚀 快速开始

### 1. 环境依赖
确保你的开发环境已安装以下工具：
* **Rust**: [安装指引](https://www.rust-lang.org/tools/install) (最新稳定版)
* **Deno**: [安装指引](https://docs.deno.com/runtime/manual/getting_started/installation) (v2.0+)
* **系统库 (仅 Linux 编译需手动安装)**:
  ```bash
  sudo apt-get update
  sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libjavascriptcoregtk-4.1-dev
  ```

### 2. 安装前端依赖
本项目使用 Deno 2.0 驱动，在根目录下运行：

  ```bash
  deno install
  ```

### 3. 启动开发模式

  ```bash
  deno task tauri dev
  ```

### 4. 生产环境构建

  ```bash
  deno task tauri build
  ```

## 🛠️ 技术栈

| 模块 | 技术选型 |
|------|----------|
| **前端框架** | Vue 3 (Composition API) + TypeScript |
| **构建工具** | Vite + Deno 2.0 |
| **跨平台引擎** | Tauri 2.0 (Rust 后端) |
| **Markdown 渲染** | Marked + Highlight.js |
| **本地数据库** | Redb (高性能 Rust KV 存储) |
| **UI 样式** | SCSS + FontAwesome 图标库 |
| **终端组件** | xterm.js + xterm-addon-fit |

## ⚙️持续集成 (GitHub Actions)
本项目配置了自动化的 CI/CD 流程。只需在本地推送版本标签，GitHub Actions 就会自动打包各平台安装文件：

  ```bash
  # 1. 提交代码
  git add .
  git commit -m "feat: 完善 AI 渲染逻辑"
  git push origin main

  # 2. 推送版本标签触发编译
  git tag v1.0.0
  git push origin v1.0.0
  ```

编译完成后，请前往 GitHub 仓库的 Releases 页面下载对应的 .msi, .dmg 或 .deb 文件。

## 📄 开源协议

本项目采用 MIT License 协议。

## 🙏 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Vue 3](https://vuejs.org/) - 渐进式 JavaScript 框架
- [xterm.js](https://xtermjs.org/) - Web 终端组件
- [Font Awesome](https://fontawesome.com/) - 图标库

---