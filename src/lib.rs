//! 胶囊坞：Windows 顶部「灵动坞」。
//!
//! - `app` / `pages`：与平台无关的界面与动效（外壳与录制器共用）
//! - `anim` / `paint` / `shape` / `theme`：弹簧、矢量绘制、面板轮廓、配色字体
//! - `model` / `lyrics` / `smtc` / `icons`：数据、歌词、系统媒体、文件图标
//! - `native` / `tray`：仅 Windows 的窗口区域、命中穿透与托盘

pub mod display_layout;
pub mod poll_clock;
pub mod resources;
pub mod anim;
pub(crate) mod morph;
pub(crate) mod nav;
pub(crate) mod notice;
pub(crate) mod todo;
pub(crate) mod shelf_view;
pub mod app;
pub mod bt;
pub mod glass;
pub mod skin_paint;
pub mod icons;
pub mod input;
pub mod lyrics;
pub mod clipboard;
pub mod side_dock;
mod task_picker;
pub mod model;
mod pages;
mod extras;
mod pet;
mod bar_layout;
mod resident_settings;
pub mod sysmon;
mod avoidance;
mod focus_bar;
#[cfg(windows)]
pub mod backdrop;
#[cfg(windows)]
pub mod composition;
#[cfg(windows)]
pub mod system;
pub mod paint;
pub mod shape;
mod fluid;
pub mod smtc;
pub mod theme;
pub mod settings_theme;
mod updates;

#[cfg(windows)]
pub mod native;
#[cfg(windows)]
pub mod tray;

#[cfg(test)]
mod ui_tests;
#[cfg(test)]
#[path = "bin/capsuledock-record/raster.rs"]
mod preview_raster;
