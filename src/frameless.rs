//! 无边框窗口的通用处理：按标题找到本线程的原生窗口，并按圆角轮廓裁剪窗口区域。
//!
//! 无边框 + 透明的窗口（更新弹窗、设置窗口）在部分显卡上仍会画出黑底，需要把窗口
//! 区域裁成圆角，窗口外沿才不会留下直角黑边。egui 的视口回调只给 `&Context`，拿不到
//! HWND，所以这里沿用更新弹窗验证过的做法：枚举本线程的窗口，按标题匹配。

use eframe::egui;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, CreateRoundRectRgn, DeleteObject, EqualRgn, GetWindowRgn, SetWindowRgn};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{EnumThreadWindows, GetWindowTextW};

/// 玻璃诊断日志：与坞体共用同一份记录，便于对比两个窗口的模糊状态。
pub fn glass_diagnostic(message: &str) {
    use std::io::Write;
    let path = crate::model::config_dir().join("diagnostics").join("glass-diagnostics.txt");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 64 * 1024) {
        let _ = std::fs::write(&path, b"");
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{} · {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message);
    }
}

/// 按标题找到本线程的原生窗口。找到时返回 HWND，否则 None。
pub fn find_thread_window(title: &str) -> Option<HWND> {
    struct Search<'a> {
        title: &'a str,
        found: Option<HWND>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> BOOL {
        let search = &mut *(data.0 as *mut Search);
        let mut buffer = [0u16; 128];
        let len = GetWindowTextW(hwnd, &mut buffer) as usize;
        if String::from_utf16_lossy(&buffer[..len]) == search.title {
            search.found = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut search = Search { title, found: None };
    unsafe { let _ = EnumThreadWindows(GetCurrentThreadId(), Some(visit), LPARAM(&mut search as *mut _ as isize)); }
    search.found
}

/// 按逻辑点矩形与圆角直径构造窗口区域。调用方负责用 `DeleteObject` 释放。
///
/// 与 `clip_thread_window` 共用同一套取整规则：右/下边界各加 1，圆角直径按缩放取整，
/// 保证不同缩放比下圆角大小一致、边缘不露黑角。
pub fn rounded_region(rect: egui::Rect, scale: f32, corner_diameter: f32) -> windows::Win32::Graphics::Gdi::HRGN {
    unsafe {
        CreateRoundRectRgn(
            (rect.left() * scale).round() as i32,
            (rect.top() * scale).round() as i32,
            (rect.right() * scale).round() as i32 + 1,
            (rect.bottom() * scale).round() as i32 + 1,
            (corner_diameter * scale).round() as i32,
            (corner_diameter * scale).round() as i32,
        )
    }
}

/// 把本线程中标题匹配的窗口裁成圆角矩形（逻辑点坐标 + 缩放）。
///
/// 返回是否成功设置过区域；找不到窗口时返回 false，调用方据此走不透明兜底。
pub fn clip_thread_window(title: &str, rect: egui::Rect, scale: f32, corner_diameter: f32) -> bool {
    let Some(hwnd) = find_thread_window(title) else { return false };
    clip_window(hwnd, rect, scale, corner_diameter)
}

/// 已持有 HWND 时直接裁圆角，省掉按标题枚举窗口的固定开销。
pub fn clip_window(hwnd: HWND, rect: egui::Rect, scale: f32, corner_diameter: f32) -> bool {
    let region = rounded_region(rect, scale, corner_diameter);
    unsafe {
        let old = CreateRectRgn(0, 0, 0, 0);
        GetWindowRgn(hwnd, old);
        // 区域不变时不必重设；注意 EqualRgn 为真时 region 没交给系统，必须自己释放，
        // 否则设置窗口每帧都会漏一个 GDI 区域，几分钟后 GDI 句柄耗尽、整机变慢。
        let applied = if EqualRgn(old, region).as_bool() {
            let _ = DeleteObject(region);
            true
        } else {
            // SetWindowRgn 成功会接管 region 的所有权，失败时由我们释放。
            let set = SetWindowRgn(hwnd, region, true) != 0;
            if !set { let _ = DeleteObject(region); }
            set
        };
        let _ = DeleteObject(old);
        applied
    }
}
