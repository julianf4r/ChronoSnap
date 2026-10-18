# ChronoSnap / 瞬影

ChronoSnap 是一款本地时间记录工具。它会按设定间隔自动截取屏幕快照，并把截图、事件、标签和提醒组织成一条可回看的日时间轴，帮助你复盘时间分配和活动记录。

## 主要功能

- 自动按间隔截图，并按日期保存到本地目录
- 在时间轴中预览、定格和全屏查看历史截图
- 为时间段添加事件、主标签、副标签和备注
- 管理提醒，并在到点时发送系统通知
- 使用甘特图规划跨日任务，并跟踪进行中、逾期和完成状态
- 查看今日、近 7 天、近 30 天或自定义范围的时间统计
- 导出记录为 JSON，或生成可复制到 Excel 的表格数据
- 支持浅色、深色和跟随系统主题
- 支持设置截图保存目录、SQLite 数据库文件、截图间隔和保留天数

## Windows 与 macOS

支持 Windows 和 macOS 14 及以上版本。发布工作流保留 Windows MSI 与便携版，并增加适用于 Apple Silicon 和 Intel 的 macOS 通用 DMG 与 `.app.zip`。

当前继续使用原有 `xcap 0.9.3`。已通过本机编译、Apple Silicon `.app` 打包与图像合并单元测试，用户已在自己的 Mac 上实测确认角标、提醒、截图和双屏拼接正常。该库的 macOS 实现仍调用 Apple 已弃用的 `CGWindowListCreateImage`；目前保留现有实现，后续如需迁移到 ScreenCaptureKit，再另行评估并确认。

- macOS 首次使用时，点击应用内「请求屏幕录制权限」，并在「系统设置 → 隐私与安全性 → 屏幕录制（或屏幕与系统音频录制）」中允许 ChronoSnap。系统若要求重新打开应用，请从菜单栏退出后再打开。未授权时跳过自动截图，时间记录、标签和统计等功能仍可使用。
- 提醒首次触发时会请求系统通知权限；通知的显示方式由 macOS 通知设置控制。
- 未完成的逾期提醒数量在 Windows 任务栏图标上显示，在 macOS Dock 图标上使用系统数字角标显示；数量为零时清除。Windows 的覆盖图标接口只在 Windows 调用。
- macOS 点击菜单栏图标可打开菜单，选择打开主界面、暂停／恢复记录或退出。关闭窗口后应用继续记录，点击 Dock 图标可重新打开窗口。
- 保存事件和任务支持 `Ctrl+Enter`，macOS 也支持 `Command+Enter`；时间轴支持按住 `Ctrl` 或 `Command` 滚动缩放。
- 截图合并支持 Retina 与普通显示器混用，并保留各显示器的相对布局；低密度屏幕会按统一比例放大。

macOS 配置关闭隐藏窗口后的 WebView 定时器暂停，以便后台提醒和角标继续刷新。该系统能力要求 macOS 14，因此本次未扩展对更早 macOS 版本的支持。电脑睡眠期间不会截图或触发提醒。

目前发布流程未配置 Apple Developer 签名和公证；在其他 Mac 上分发时可能受 Gatekeeper 限制。本次不自动配置签名或修改系统安全设置。`scripts/register_startup.ps1` 仅用于 Windows 的手动快捷方式注册，并包含平台检查。现有自启动插件使用 macOS LaunchAgent，但应用目前没有启用自启动的设置入口，也不会自动开启。

## 本地开发与打包

需要 Node.js、pnpm 和 Rust；macOS 还需要 Xcode Command Line Tools。

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

所有平台共用 `src-tauri/tauri.conf.json`，其中的 Windows 和 macOS 专属设置只在对应平台生效。主配置的 `bundle.targets` 为 `all`，直接运行 `pnpm tauri build` 会生成当前平台支持的默认安装包格式；使用以下命令可明确限定输出格式，与发布流程保持一致。

Windows 上生成 MSI：

```sh
pnpm tauri build --bundles msi
```

Mac 上生成 `.app` 和 DMG：

```sh
pnpm tauri build --bundles app,dmg
```

在 Mac 上构建同时支持 Apple Silicon 和 Intel 的通用包：

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm tauri build --target universal-apple-darwin --bundles app,dmg
```

验证适配时，请在安装的 `.app` 中检查屏幕录制授权／拒绝、通知、Dock 角标增加及清除、关闭窗口后继续记录、Dock 和菜单栏恢复窗口，以及多显示器截图。开发进程与正式安装应用的系统授权可能不同，需要分别授权。

## 数据存储

ChronoSnap 默认只使用本地数据：

- 截图文件保存到你选择的目录，并按日期分文件夹存放
- 标签、事件和提醒保存到你选择或创建的 SQLite 数据库文件
- 配置项保存在 Tauri 的本地应用配置中

请根据截图内容的敏感程度选择合适的保存位置，并定期清理或备份数据。
