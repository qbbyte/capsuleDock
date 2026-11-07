//! 本机 CPU 与内存占用采样（Windows）。
//!
//! 内存直接用 `GlobalMemoryStatusEx` 的 `dwMemoryLoad`（系统已算好的近似百分比）；
//! CPU 用 `GetSystemTimes` 两次采样的空闲 / 总时间差推算，避免引入额外依赖。
//! 采样很轻量，由界面层每隔约 1 秒调用一次。
use std::time::Duration;

use crate::poll_clock::PollClock;

/// 当前 CPU / 内存占用（0..100）。
#[derive(Clone, Copy, Debug, Default)]
pub struct SysMon {
    /// CPU 占用率 0..100。
    pub cpu: f32,
    /// 内存占用率 0..100。
    pub mem: f32,
    clock: PollClock,
    /// 上一次采样得到的 (空闲时间, 总时间)，单位 100ns；用于推算区间占用。
    cpu_prev: Option<(u64, u64)>,
}

impl SysMon {
    /// 每隔 `interval` 采一次样；首次调用立即采样（首帧 CPU 记为 0，下一帧起才有差值）。
    pub fn sample_if_ready(&mut self, interval: Duration) {
        if self.clock.ready(interval) {
            self.clock = PollClock::now();
            self.mem = unsafe { memory_load() };
            let (idle, total) = unsafe { cpu_times() };
            if let Some((p_idle, p_total)) = self.cpu_prev {
                let idle_delta = idle.saturating_sub(p_idle);
                let total_delta = total.saturating_sub(p_total);
                self.cpu = if total_delta == 0 {
                    0.0
                } else {
                    (1.0 - idle_delta as f32 / total_delta as f32).clamp(0.0, 1.0) * 100.0
                };
            }
            self.cpu_prev = Some((idle, total));
        }
    }
}

/// 返回 (空闲时间, 总时间)，单位 100ns；`total` 已包含内核时间（其中含空闲）。
unsafe fn cpu_times() -> (u64, u64) {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetSystemTimes;
    let mut idle = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let _ = GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user));
    let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    (ft(idle), ft(kernel) + ft(user))
}

unsafe fn memory_load() -> f32 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut info = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if GlobalMemoryStatusEx(&mut info).is_ok() {
        info.dwMemoryLoad as f32
    } else {
        0.0
    }
}
