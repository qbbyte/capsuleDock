# 胶囊坞 CapsuleDock

> Windows 顶部的桌面伙伴与信息坞。

胶囊坞常驻在屏幕顶部，鼠标靠近时展开功能面板。它把音乐控制、专注计时、置物架、待办提醒和系统通知整合进一个收起时只剩刘海或胶囊形状的小坞，配合桌面吉祥物和实时毛玻璃效果。

![胶囊坞](assets/capsuledock-logo.png)

## 功能

- **顶部的灵动坞**：收起时是屏幕顶部一条刘海或胶囊，鼠标靠近展开面板；支持顶部、左侧、右侧三种停靠，自动避让盖住边缘的全屏应用。
- **音乐与歌词**：兼容网易云音乐、酷狗音乐、汽水音乐等本机播放器，显示同步歌词和专辑封面；支持坞上歌词（仅顶部停靠）。
- **专注与休息**：番茄钟式计时，专注结束后进入休息阶段。
- **置物架**：收藏常用文件路径，新增「剪贴板」分栏记录最近 30 条复制内容，可固定常用项；历史保存在本机。
- **待办与提醒**：任务提醒，支持每周按周几重复，到点提醒并在通知里定位到对应任务。
- **通知**：微信 / QQ 未读消息在面板外侧以胶囊形式展示，点击跳转到对应账号窗口；验证码自动识别并给敏感内容打码。
- **桌面吉祥物**：可切换的桌面宠物，跟随配色和大小换色。
- **可定制外观**：五种配色（经典、奶油、毛玻璃·浅、毛玻璃·深、悬浮云）× 两种形状（刘海、胶囊），坞体大小 80%–140% 可调，水平位置可调。毛玻璃使用 Windows 合成引擎的实时模糊。

## 安装

### 下载安装包

1. 前往 [Releases](https://github.com/qbbyte/capsuleDock/releases) 下载最新版 `CapsuleDock-Setup-v版本号-x64.exe`。
2. 双击运行，可选择安装位置、是否创建桌面快捷方式、是否开机自启。
3. 默认安装到 `%LOCALAPPDATA%\Programs\CapsuleDock`，用户数据保存在 `%APPDATA%\CapsuleDock`。
4. 升级时直接运行新版安装包，覆盖安装并保留设置和数据。

### 卸载

在 Windows「已安装的应用」中卸载，会移除程序和快捷方式，但保留用户数据目录。

## 数据目录

| 内容 | 默认位置 |
| --- | --- |
| 设置、待办、置物架路径、剪贴板历史 | `%APPDATA%\CapsuleDock` |
| 诊断日志 | `%APPDATA%\CapsuleDock\diagnostics\last-error.log` |
| 更新下载缓存 | `%LOCALAPPDATA%\CapsuleDock\Updates` |

可用环境变量 `CAPSULEDOCK_DATA_DIR` 指定其他数据目录。

## 从源码构建

### 依赖

- Rust stable
- Visual Studio C++ 构建工具（MSVC 链接器）
- Windows SDK（提供 `rc.exe`，用于把图标编进 exe）

### 编译与运行

```powershell
cargo build --locked --bin capsuledock
```

为开发实例指定独立的数据目录，避免覆盖日常使用的数据：

```powershell
$env:CAPSULEDOCK_DATA_DIR = Join-Path $env:TEMP 'capsuledock-development'
cargo run --locked --bin capsuledock
```

如果笔记本接扩展坞或外接显示器时提示 OpenGL 不可用，胶囊坞会自动改用 Direct3D 12 启动。

### 打包

打包脚本使用 NSIS 生成安装包：

```powershell
./scripts/package.ps1
```

需要先通过 `./scripts/get-nsis.ps1` 获取 NSIS 编译器。

## 开发

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

运行测试：

```powershell
cargo test --locked --all-targets --features record -- --test-threads=1
```

常规测试使用合成数据；标记为 `#[ignore]` 的测试需要本机真实的微信 / QQ 窗口，按需显式运行。

## 项目结构

| 目录／文件 | 内容 |
| --- | --- |
| `src/main.rs` | Windows 外壳、系统状态和后台服务 |
| `src/lib.rs` `src/app.rs` `src/pages.rs` | 状态机与主界面、功能页面 |
| `src/model.rs` `src/lyrics.rs` `src/smtc.rs` `src/icons.rs` | 数据、歌词、系统媒体、文件图标 |
| `src/native.rs` `src/system.rs` `src/tray.rs` | 原生窗口、命中穿透、拖放、托盘 |
| `src/otp.rs` `src/code_mask.rs` | 验证码识别与通知打码 |
| `src/ui_tests.rs` | 无窗口交互与布局回归测试 |
| `src/bin/capsuledock-record` | 合成数据演示与软件渲染器 |
| `vendor/` | 少量必要的上游补丁（基于 eframe 0.31，修复 Windows WGL 上下文切换） |
| `installer/` `scripts/` | 安装、卸载与打包 |
| `assets/` | 图标、皮肤素材与收款码 |

## 隐私

- 胶囊坞默认在 `%APPDATA%\CapsuleDock` 保存用户设置与运行数据，不使用统一云端账号。
- 音乐歌词功能会读取本机播放器（网易云、酷狗、汽水）的缓存与播放日志，只保留歌词和歌曲元数据，不保存或上传日志正文、音频内容或登录信息。
- 检查更新时只访问 GitHub 公开发布接口并在请求头中包含版本号，不发送本地设置或个人数据。
- 诊断日志保存在本机，不会自动上传。

发现安全问题请通过 GitHub 的私密漏洞报告入口反馈，不要在公开 Issue 中附带凭据或账号数据。详见 [SECURITY.md](SECURITY.md)。

## 许可

本项目原创代码采用 [MIT 许可](LICENSE)。第三方依赖、字体、图标及品牌标识遵循各自许可，MIT 不授予第三方商标权；完整归属与许可文本见 [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt)。

## 更新记录

见 [CHANGELOG.md](CHANGELOG.md)。
