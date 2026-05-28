# Inky

Inky 是一个 Windows 桌面专注助手。它以一个透明、置顶的小窗陪你记录任务、收集突然冒出的想法、进入番茄钟专注，并通过一只会成长的宠物提供轻量反馈。

## 下载使用

普通用户请到本仓库的 **Releases** 页面下载最新版 Windows 安装包。

下载后运行安装包即可使用。如果 Windows SmartScreen 提示风险，这是因为当前安装包还没有代码签名证书，可以选择 **更多信息** → **仍要运行**。

## 主要功能

- **桌面悬浮小窗**：透明、无边框、始终置顶，适合放在桌面角落长期使用。
- **快速记录任务**：输入任务后选择工作、学习、生活或想法分类。
- **Inbox 想法收集**：把临时冒出的想法先放进 Inbox，之后再整理。
- **拖拽整理 Inbox**：把便签拖到转任务区或删除区，减少按钮干扰。
- **番茄钟专注**：任务可进入专注计时，完成后记录番茄数量。
- **回神引导**：专注中记录想法后，轻轻提示你回到当前任务。
- **宠物陪伴**：宠物默认叫 Inky，可首次进入时命名，也可之后修改。
- **本地宠物包**：支持从本地应用数据目录加载自定义宠物资源。
- **本地持久化**：任务、Inbox、心情、XP、宠物名称等保存在本机 SQLite 中。
- **可选 AI 解析**：可配置个人 Key 或邀请码代理来解析自然语言任务。

## 隐私说明

Inky 是本地优先应用。任务、Inbox、宠物状态和 AI 配置默认保存在本机应用数据目录。

个人 API Key 只保存在本地配置文件中，不会暴露给前端界面。只有在你主动配置并使用 AI 解析时，相关文本才会发送到对应 AI 服务或代理服务。

## 开发运行

### 环境要求

请先安装：

- Windows 10/11
- Node.js
- Corepack / pnpm
- Rust 工具链
- Tauri 2 所需的 Windows 构建环境

如果没有启用 Corepack，可以先运行：

```powershell
corepack enable
```

安装依赖：

```powershell
corepack pnpm install
```

启动完整桌面应用：

```powershell
corepack pnpm tauri dev
```

只启动浏览器预览：

```powershell
corepack pnpm dev
```

然后打开：

```text
http://127.0.0.1:1420
```

浏览器预览适合快速检查 UI，但窗口拖拽、全局快捷键、本地数据目录、SQLite 持久化等桌面能力需要用 `corepack pnpm tauri dev` 测试。

## 本地构建安装包

```powershell
corepack pnpm tauri build
```

Windows 安装包会生成在：

```text
src-tauri/target/release/bundle/
```

## 常用检查命令

```powershell
corepack pnpm typecheck
corepack pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

说明：

- `typecheck`：检查 TypeScript 类型。
- `build`：构建前端资源。
- `cargo check`：检查 Rust / Tauri 后端代码。
- `cargo test`：运行 Rust 后端测试。

## 项目结构

```text
Inky-app/
├─ public/                 # 静态资源，包括内置宠物图片
│  └─ pet-assets/
├─ src/                    # React + TypeScript 前端
│  ├─ components/
│  │  ├─ FocusFlowWidget/  # 主界面、任务、Inbox、弹窗和专注流
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

内置宠物图片放在：

```text
public/pet-assets/
```

本地自定义宠物包由桌面端从应用数据目录读取，读取范围被限制在应用自己的 pet-packs 目录内。

当前宠物等级和资源配置在：

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

```powershell
corepack pnpm tauri dev
```

### 安装包被 Windows 提醒风险

当前安装包未做代码签名，Windows 可能显示 SmartScreen 提示。确认来源是本仓库 Release 后，可以选择 **更多信息** → **仍要运行**。
