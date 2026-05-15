# Inky

Inky 是一个 Windows 桌面专注助手，使用 Tauri 2、React 18 和 TypeScript 构建。

它以一个透明、置顶的桌面小窗呈现，会陪伴用户记录任务、进入专注模式、完成番茄钟，并通过一只会成长的章鱼宠物提供轻量提醒和反馈。

## 主要功能

- 桌面悬浮小窗：透明、无边框、始终置顶，适合放在桌面角落长期使用。
- 任务快速记录：支持创建、完成和管理待办任务。
- 专注模式：任务可以进入专注/番茄钟流程，计时状态会在主界面和迷你模式之间保留。
- 章鱼宠物 Inky：根据等级和心情展示不同动画，完成任务后获得 XP 并成长。
- 迷你模式：缩小为宠物形态，减少干扰。
- 本地持久化：任务、心情、XP 和宠物名称保存在本地。
- 可选 AI 解析：通过 Tauri 后端解析自然语言任务，不在前端暴露 API Key。
- Windows 原生能力：支持窗口拖拽、全局快捷键和系统空闲检测。

## 技术栈

- Tauri 2
- Rust
- React 18
- TypeScript
- Vite
- pnpm / Corepack
- SQLite / rusqlite

## 环境要求

请先安装：

- Node.js
- pnpm（推荐通过 Corepack 使用）
- Rust
- Tauri 2 所需的 Windows 构建环境

如果没有启用 Corepack，可以先运行：

```bash
corepack enable
```

## 安装依赖

在项目根目录运行：

```bash
corepack pnpm install
```

## 启动桌面应用

这是最完整的开发运行方式，会启动 Vite 前端和 Tauri 桌面壳：

```bash
corepack pnpm tauri dev
```

启动后会打开 Inky 桌面小窗。

## 仅启动前端预览

如果只想在浏览器里查看前端界面，可以运行：

```bash
corepack pnpm dev
```

然后打开：

```text
http://127.0.0.1:1420
```

注意：浏览器预览不能完整使用 Tauri 原生能力，例如窗口控制、系统空闲检测和全局快捷键。

## 常用检查命令

```bash
corepack pnpm typecheck
corepack pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

说明：

- `typecheck`：检查 TypeScript 类型。
- `build`：构建前端资源。
- `cargo check`：检查 Rust / Tauri 后端代码。

## 打包桌面应用

```bash
corepack pnpm tauri build
```

打包产物会生成在 Tauri 的构建输出目录中。

## AI 任务解析配置

Inky 支持可选的 AI 任务解析能力。前端不会直接调用 AI 服务，也不会接触 API Key。

AI 相关逻辑在 Tauri 后端中处理，本地配置会保存在应用数据目录中的 `ai-config.json`。如果没有配置 AI，应用的基础任务和专注功能仍然可以正常使用。

## 项目结构

```text
Inky-app/
├─ public/                 # 静态资源，包括宠物图片
│  └─ pet-assets/
├─ src/                    # React + TypeScript 前端
│  ├─ components/
│  │  ├─ FocusFlowWidget/  # 主界面和核心交互
│  │  └─ PetRenderer/      # 宠物渲染
│  ├─ types/               # 共享类型
│  └─ utils/               # 持久化、等级、AI 调用等工具
├─ src-tauri/              # Tauri / Rust 后端
│  ├─ src/
│  └─ tauri.conf.json
├─ package.json
└─ README.md
```

## 宠物资源说明

宠物图片放在：

```text
public/pet-assets/
```

前端通过 public 路径加载图片，不需要在 TypeScript 文件中直接导入 PNG。

当前宠物等级配置在：

```text
src/components/PetRenderer/petConfig.ts
```

## 常见问题

### 端口 1420 被占用

如果启动时看到类似错误：

```text
Error: Port 1420 is already in use
```

说明已有另一个 Vite 开发服务器占用了端口。可以关闭旧终端，或在 Windows 中找到占用端口的进程并结束它。

### 浏览器里可以打开，但桌面能力不可用

这是正常的。`corepack pnpm dev` 只启动前端页面；需要运行下面命令才能测试完整桌面能力：

```bash
corepack pnpm tauri dev
```

### 修改宠物图片后没有变化

请确认图片放在 `public/pet-assets/` 下，并且 `petConfig.ts` 中的路径和文件名完全一致。修改后重新启动开发服务器。

## 开发备注

- 应用名和章鱼宠物名统一为 Inky。
- 默认窗口是透明、无边框、始终置顶的小窗。
- 前端状态主要集中在 `FocusFlowWidget` 中。
- 宠物渲染逻辑集中在 `PetRenderer` 中。
- 不要把 API Key 写进前端代码或提交到仓库。
