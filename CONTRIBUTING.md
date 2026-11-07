# 参与开发

欢迎反馈兼容性问题、修复 bug 或完善使用说明。提交较大的功能前，先开 Issue 说明具体场景与预期行为。

## 本地开发

安装 Rust stable、Visual Studio C++ 构建工具和 Windows SDK，克隆仓库后运行：

```powershell
cargo build --locked --bin capsuledock
cargo test --locked --all-targets --features record -- --test-threads=1
```

为开发实例指定独立目录，避免覆盖日常使用数据：

```powershell
$env:CAPSULEDOCK_DATA_DIR = Join-Path $env:TEMP 'capsuledock-development'
cargo run --locked --bin capsuledock
```

常规测试使用合成数据；带 `#[ignore]` 的截图导出测试须按需要显式运行。

## 代码导航

| 目录／文件 | 内容 |
| --- | --- |
| `src/main.rs` | Windows 外壳、系统状态和后台服务衔接 |
| `src/app.rs`、`src/pages.rs` | 状态与主界面 |
| `src/otp.rs`、`src/code_mask.rs` | 验证码识别与通知打码 |
| `src/native.rs`、`src/system.rs` | 原生窗口、输入、拖放和系统事件 |
| `src/ui_tests.rs` | 无窗口交互与布局回归测试 |
| `src/bin/capsuledock-record` | 合成数据演示与软件渲染器 |
| `vendor/` | 少量必要的上游补丁，见依赖说明 |
| `installer/`、`scripts/` | 安装、卸载与打包 |

## 提交要求

修 bug 时提供能复现的测试；界面变更附不含私人信息的截图。保持改动聚焦，不把个人配置、授权码、Cookie、编译产物或真实账号数据提交到仓库。

源码贡献按仓库 MIT 许可提交；第三方素材须同时提交准确的来源与许可声明。不要修改或移除 vendored 依赖的原始版权声明。
