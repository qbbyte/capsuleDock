//! 本机网卡实时速率采样（Windows）。
//!
//! 只读取 `GetIfTable2` 提供的累计收发字节数，两次采样取差值除以真实时间差：
//! 不抓包、不读取数据内容、不需要管理员权限，也不发起任何网络连接。
//! 统计口径是「默认路由所在网卡」的链路速率（与任务管理器一致），
//! 这样 WSL / Docker / Hyper-V / VPN 等虚拟网卡不会和物理网卡重复累加。
//! 局域网内传输同样会计入，属于网卡层速率而非「公网下载速度」。
//!
//! 和 `sysmon` 一样由界面层每帧调用 `sample_if_ready`，采样本身用
//! `PollClock` 节流到约 1 秒；不需要后台线程，也不会主动唤醒重绘。

use std::time::{Duration, Instant};

use crate::poll_clock::PollClock;

/// 建议采样间隔。静态界面本来就每秒重绘一次，正好对齐。
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(1);

/// 收 / 发速率，单位：字节每秒（EMA 平滑后的显示值）。
#[derive(Clone, Copy, Debug, Default)]
pub struct NetSpeed {
    pub down: f64,
    pub up: f64,
    clock: PollClock,
    /// 上次采样的 (累计接收, 累计发送, 时刻, 接口索引)。
    prev: Option<(u64, u64, Instant, u32)>,
    /// 演示录制器 / 回归测试注入的合成速率；为 true 时不再读取系统计数器。
    synthetic: bool,
}

impl NetSpeed {
    /// 每隔 `interval` 采一次样；首次调用只建立基线，速率保持 0。
    pub fn sample_if_ready(&mut self, interval: Duration) {
        if self.synthetic || !self.clock.ready(interval) {
            return;
        }
        let now = Instant::now();
        self.clock = PollClock::now();
        let Some((index, received, sent)) = (unsafe { interface_counters() }) else {
            self.down = 0.0;
            self.up = 0.0;
            self.prev = None;
            return;
        };
        let next = match self.prev {
            Some((p_recv, p_sent, p_at, p_index)) if p_index == index => {
                let secs = now.saturating_duration_since(p_at).as_secs_f64();
                rates((p_recv, p_sent), (received, sent), secs, (self.down, self.up))
            }
            // 换了网卡（例如插上网线、连上 VPN）时重新建立基线。
            _ => None,
        };
        match next {
            Some((down, up)) => {
                self.down = down;
                self.up = up;
            }
            None => {
                self.down = 0.0;
                self.up = 0.0;
            }
        }
        self.prev = Some((received, sent, now, index));
    }

    /// 组件被关闭时停止采样并清空基线，重新打开后从 0 开始，不闪一下旧读数。
    pub fn pause(&mut self) {
        if !self.synthetic {
            self.prev = None;
            self.down = 0.0;
            self.up = 0.0;
        }
    }

    /// 注入合成速率（字节每秒），供演示录制器与回归测试使用。
    pub fn set_synthetic(&mut self, down: f64, up: f64) {
        self.down = down.max(0.0);
        self.up = up.max(0.0);
        self.synthetic = true;
    }

    pub fn clear_synthetic(&mut self) {
        self.synthetic = false;
        self.pause();
    }
}

/// 两次累计计数 → 平滑速率。异常样本返回 None，由调用方把速率归零并重建基线：
/// 休眠唤醒（时间差过大）、帧间隔异常短、计数器回绕或网卡重置都算异常，
/// 否则几十秒的累计字节会被摊成一瞬间的虚假峰值。
fn rates(prev: (u64, u64), now: (u64, u64), secs: f64, shown: (f64, f64)) -> Option<(f64, f64)> {
    if !(0.05..=5.0).contains(&secs) || now.0 < prev.0 || now.1 < prev.1 {
        return None;
    }
    const A: f64 = 0.35; // EMA：约 3 秒收敛，尖峰不至于被完全抹平
    let down = (now.0 - prev.0) as f64 / secs;
    let up = (now.1 - prev.1) as f64 / secs;
    Some((shown.0 + (down - shown.0) * A, shown.1 + (up - shown.1) * A))
}

/// 把字节每秒格式化为定长短文本；数值变化不会改变坞体宽度。
/// M / G 档在 >=10 时去掉小数（30.2M → 30M），把最长文本锁在 4 字符以内；
/// 否则五字符在坞上会被缩字号甚至裁成省略号（网速槽每组只有约 18 逻辑单位）。
pub fn format_rate(bytes_per_sec: f64) -> String {
    const K: f64 = 1024.0;
    let v = bytes_per_sec.max(0.0);
    if v < K {
        "0".into()
    } else if v < K * K {
        format!("{:.0}K", v / K)
    } else if v < K * K * K {
        let m = v / (K * K);
        if m >= 10.0 { format!("{:.0}M", m) } else { format!("{:.1}M", m) }
    } else {
        let g = v / (K * K * K);
        if g >= 10.0 { format!("{:.0}G", g) } else { format!("{:.1}G", g) }
    }
}

/// 返回 (接口索引, 累计接收字节, 累计发送字节)。
#[cfg(windows)]
unsafe fn interface_counters() -> Option<(u32, u64, u64)> {
    use windows::Win32::NetworkManagement::IpHelper::{
        FreeMibTable, GetBestInterface, GetIfTable2, MIB_IF_ROW2, MIB_IF_TABLE2,
    };

    // 只关心已经连上的非回环接口；1 = IfOperStatusUp，24 = IF_TYPE_SOFTWARE_LOOPBACK。
    fn usable(row: &MIB_IF_ROW2) -> bool {
        row.OperStatus.0 == 1 && row.Type != 24
    }

    // 默认路由所在接口；拿不到时 best 保持 0，下面的兜底会挑一个物理网卡。
    let mut best: u32 = 0;
    if GetBestInterface(0, &mut best) != 0 {
        best = 0;
    }
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    if GetIfTable2(&mut table).0 != 0 || table.is_null() {
        return None;
    }
    // MIB_IF_TABLE2::Table 声明为 [MIB_IF_ROW2; 1]，真实长度在 NumEntries。
    let count = (*table).NumEntries as usize;
    let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), count);
    let chosen = rows
        .iter()
        .find(|row| row.InterfaceIndex == best && usable(row))
        .or_else(|| rows.iter().filter(|row| usable(row)).max_by_key(|row| row.ReceiveLinkSpeed));
    let result = chosen.map(|row| (row.InterfaceIndex, row.InOctets, row.OutOctets));
    // 每秒一次，漏掉这一步会持续泄漏 MIB 表内存。
    FreeMibTable(table as *const core::ffi::c_void);
    result
}

#[cfg(not(windows))]
unsafe fn interface_counters() -> Option<(u32, u64, u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_rate_is_bounded_and_monotonic_in_units() {
        assert_eq!(format_rate(-5.0), "0");
        assert_eq!(format_rate(512.0), "0");
        assert_eq!(format_rate(40.0 * 1024.0), "40K");
        assert_eq!(format_rate(1.2 * 1024.0 * 1024.0), "1.2M");
        assert_eq!(format_rate(30.2 * 1024.0 * 1024.0), "30M");
        assert_eq!(format_rate(3.0 * 1024.0 * 1024.0 * 1024.0), "3.0G");
        assert_eq!(format_rate(12.0 * 1024.0 * 1024.0 * 1024.0), "12G");
        for rate in [0.0, 1023.0, 1024.0, 1024.0 * 1024.0, 5e9,
                     30.2 * 1024.0 * 1024.0, 999.0 * 1024.0 * 1024.0, 1e12] {
            assert!(format_rate(rate).len() <= 4, "{rate} 格式化后过长：{}", format_rate(rate));
        }
    }

    #[test]
    fn rates_reject_sleep_counter_reset_and_wrap() {
        let (down, up) = rates((0, 0), (1024, 512), 1.0, (0.0, 0.0)).unwrap();
        assert!((down - 358.4).abs() < 0.01 && (up - 179.2).abs() < 0.01);
        assert_eq!(rates((0, 0), (1024, 512), 30.0, (5.0, 5.0)), None); // 休眠唤醒
        assert_eq!(rates((100, 100), (99, 200), 1.0, (0.0, 0.0)), None); // 计数器回绕
        assert_eq!(rates((0, 0), (1024, 512), 0.001, (0.0, 0.0)), None); // 帧间隔异常
    }

    #[test]
    fn synthetic_value_is_not_overwritten_by_sampling() {
        let mut speed = NetSpeed::default();
        speed.set_synthetic(1.5 * 1024.0 * 1024.0, 40.0 * 1024.0);
        for _ in 0..10 {
            speed.sample_if_ready(DEFAULT_INTERVAL);
        }
        assert!(speed.down > 1.0);
        assert_eq!(format_rate(speed.down), "1.5M");
        assert_eq!(format_rate(speed.up), "40K");
        speed.clear_synthetic();
        assert_eq!(speed.down, 0.0);
    }
}
