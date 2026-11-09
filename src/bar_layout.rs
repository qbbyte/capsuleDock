//! 常驻组件按实际占位排布，并在开关变化时连续补位。
use crate::anim::{smoothstep, Spring};

pub(crate) const CLOCK: usize = 0;
pub(crate) const BATTERY: usize = 1;
pub(crate) const MUSIC: usize = 2;
pub(crate) const FOCUS: usize = 3;
pub(crate) const BELL: usize = 4;
pub(crate) const PET: usize = 5;
/// CPU 占用圆环。沿用旧版「电脑状态」槽位的数字 6，已保存的排列无需重编号。
pub(crate) const CPU: usize = 6;
/// 网速组件：下载 / 上传速率单行横排。
pub(crate) const NETSPEED: usize = 7;
/// 内存占用圆环：从旧的双环组件里拆出来的第二个独立槽位。
pub(crate) const MEM: usize = 8;
/// 槽宽固定，速率与百分比都用定长格式，数值变化不会让坞体宽度抖动。
/// CPU / MEM 按「单环 + 两位数百分比」定宽（40），网速是两组「箭头 + 速率」横排（58）；
/// `100%` 由 `Card::fit_shrink` 缩字号，速率由 `paint::netspeed_alloc` 按两组实际文本宽
/// 动态分配并统一缩字号，都不必为极端值预留空槽。
/// 公开给绘制层、展开顶栏布局与设置预览共用，避免同一宽度硬编码三份。
pub(crate) const WIDTHS: [f32; 9] = [44.0, 20.0, 14.0, crate::focus_bar::WIDTH, 20.0, 32.0, 40.0, 58.0, 40.0];
/// 相邻常驻组件之间的固定留白。`Layout::new`、`reorder` 与展开顶栏布局共用，避免多处漂移；
/// 布局计算统一按 `WIDTHS` + `GAP` 取位，调间距只需改这里。
pub(crate) const GAP: f32 = 6.0;

pub(crate) const DEFAULT_ORDER: [usize; 8] = [BATTERY, PET, CLOCK, CPU, MEM, NETSPEED, MUSIC, FOCUS];
pub(crate) fn valid_order(order: &[usize]) -> bool {
    order.len() == 8 && DEFAULT_ORDER.iter().all(|slot| order.iter().filter(|value| *value == slot).count() == 1)
}

pub(crate) fn animations_enabled() -> bool {
    #[cfg(windows)] {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ENABLED.get_or_init(|| {
            use windows::Win32::{Foundation::BOOL, UI::WindowsAndMessaging::*};
            let mut enabled = BOOL(1);
            unsafe { let _ = SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, Some((&mut enabled as *mut BOOL).cast()), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)); }
            enabled.as_bool()
        })
    }
    #[cfg(not(windows))] { true }
}

#[derive(Clone, Debug)]
pub(crate) struct Layout {
    pub width: f32,
    pub x: Vec<Option<f32>>,
    pub widths: Vec<f32>,
}

impl Layout {
    pub fn new(battery: bool, music: bool, focus: bool, bell: bool, pet: bool, cpu: bool, mem: bool, netspeed: bool) -> Self {
        fn place(cursor: &mut f32, width: f32) -> f32 { let x = *cursor + width / 2.0; *cursor += width + GAP; x }
        let mut x = vec![None; WIDTHS.len()];
        let mut cursor = 16.0;
        if battery { x[BATTERY] = Some(place(&mut cursor, WIDTHS[BATTERY])); }
        if pet { x[PET] = Some(place(&mut cursor, WIDTHS[PET])); }
        x[CLOCK] = Some(place(&mut cursor, WIDTHS[CLOCK]));
        if cpu { x[CPU] = Some(place(&mut cursor, WIDTHS[CPU])); }
        if mem { x[MEM] = Some(place(&mut cursor, WIDTHS[MEM])); }
        if netspeed { x[NETSPEED] = Some(place(&mut cursor, WIDTHS[NETSPEED])); }
        for (slot, on) in [(MUSIC, music && !bell), (FOCUS, focus), (BELL, bell)] {
            if on { x[slot] = Some(place(&mut cursor, WIDTHS[slot])); }
        }
        let width = cursor - GAP + 16.0;
        for value in x.iter_mut().flatten() { *value -= width / 2.0; }
        Self { width: width.max(96.0), x, widths: WIDTHS.to_vec() }
    }
    pub fn widen_clock(&mut self, extra: f32) {
        let clock = self.x[CLOCK].unwrap();
        for (slot, x) in self.x.iter_mut().enumerate() {
            if slot != CLOCK { if let Some(x) = x { *x += if *x < clock { -extra / 2.0 } else { extra / 2.0 }; } }
        }
        self.width += extra;
    }
    pub fn reorder(&mut self, order: &[usize]) {
        if !valid_order(order) { return; }
        let original = self.x.clone();
        let mut cursor = -self.width / 2.0 + 16.0;
        for slot in order.iter().copied().chain(std::iter::once(BELL)) {
            let Some(x) = original[slot] else { continue; };
            let left = x - self.widths[slot] / 2.0;
            let right = x + self.widths[slot] / 2.0;
            self.x[slot] = Some(cursor + x - left);
            cursor += right - left + GAP;
        }
    }
    /// 组件经过吉祥物时暂时淡出，避免移动中相互遮挡。
    pub fn clearance(&self, slot: usize) -> f32 {
        self.pet_clearance(slot)
    }
    /// 组件经过吉祥物时暂时淡出，避免移动中相互遮挡。
    ///
    /// 阈值以「两槽中心距」衡量，且**必须相对 `GAP` 表达**：原来的魔数 14 / 24
    /// 是按旧 `GAP = 12` 校准的，`GAP` 收到 6 后静止排列的中心距随之变小，
    /// 会落进淡出区间，导致时钟 / 状态组件被持续淡出（约 10%），而不是只在移动时淡出。
    ///
    /// 语义：中心距 ≥ 半宽和 + 一个 `GAP`（正常并排）完全不淡出；缩到半宽和
    /// （刚好贴住）时完全淡出。静止排列时相邻组件恰好相距「半宽和 + `GAP`」，
    /// 因此静止态恒为 1.0，与 `GAP` 取什么值无关。
    pub fn pet_clearance(&self, slot: usize) -> f32 {
        match (self.x[slot], self.x[PET]) {
            (Some(x), Some(pet)) => {
                let apart = self.widths[slot] / 2.0 + self.widths[PET] / 2.0;
                smoothstep(apart, apart + GAP, (x - pet).abs())
            }
            _ => 1.0,
        }
    }
}

pub(crate) struct Motion {
    pub target: Layout,
    positions: Vec<Spring>,
    initialized: bool,
}
impl Default for Motion {
    fn default() -> Self { Self { target: Layout::new(false, false, false, false, false, false, false, false), positions: vec![Spring::new(0.0, 360.0, 0.76); WIDTHS.len()], initialized: false } }
}
impl Motion {
    pub fn step(&mut self, target: Layout, dt: f32, animate: bool) {
        self.positions.resize(target.x.len(), Spring::new(0.0, 360.0, 0.76));
        for (i, x) in target.x.iter().enumerate() {
            if let Some(x) = x {
                let spring = &mut self.positions[i];
                spring.target = *x;
                if !self.initialized || self.target.x.get(i).copied().flatten().is_none() || !animate { spring.snap(*x); } else { spring.step(dt); }
            }
        }
        self.target = target;
        self.initialized = true;
    }
    pub fn current(&self) -> Layout {
        let mut layout = self.target.clone();
        for (i, x) in layout.x.iter_mut().enumerate() { if x.is_some() { *x = Some(self.positions[i].pos); } }
        layout
    }
    pub fn position(&self, slot: usize) -> f32 { self.positions[slot].pos }
    pub fn settled(&self) -> bool { self.target.x.iter().enumerate().all(|(i, x)| x.is_none() || self.positions[i].settled(100.0)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_12h_keeps_components_separate_in_custom_orders() {
        for order in [DEFAULT_ORDER, [FOCUS, MUSIC, CLOCK, CPU, MEM, NETSPEED, PET, BATTERY]] {
            let mut layout = Layout::new(true, true, true, false, true, false, false, true);
            layout.reorder(&order);
            layout.widen_clock(40.0);
            let mut boxes: Vec<_> = layout.x.iter().enumerate().filter_map(|(i, x)| x.map(|x| {
                let width = layout.widths[i] + if i == CLOCK { 40.0 } else { 0.0 };
                (x - width / 2.0, x + width / 2.0)
            })).collect();
            boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|p| p[1].0 - p[0].1 >= 5.9));
            assert!(boxes.first().unwrap().0 >= -layout.width / 2.0 + 15.9);
            assert!(boxes.last().unwrap().1 <= layout.width / 2.0 - 15.9);
        }
    }
    #[test]
    fn custom_order_preserves_groups_and_hidden_positions() {
        assert!(!valid_order(&[0, 0, 2, 3, 9]));
        assert!(!valid_order(&[0, 1, 2]));
        // 旧版六位 / 七位顺序已不再合法，必须靠 model::Settings::load 迁移。
        assert!(!valid_order(&[FOCUS, MUSIC, CLOCK, CPU, PET, BATTERY]));
        assert!(!valid_order(&[FOCUS, MUSIC, CLOCK, CPU, NETSPEED, PET, BATTERY]));
        let order = [FOCUS, MUSIC, CLOCK, CPU, MEM, NETSPEED, PET, BATTERY];
        for battery in [false, true] {
            let mut layout = Layout::new(battery, true, true, false, true, false, false, true);
            let width = layout.width;
            layout.reorder(&order);
            assert_eq!(layout.width, width);
            let positions: Vec<_> = order.iter().filter_map(|i| layout.x[*i]).collect();
            assert!(positions.windows(2).all(|p| p[0] < p[1]));
            let mut boxes: Vec<_> = layout.x.iter().enumerate().filter_map(|(i, x)| x.map(|x| (x - layout.widths[i] / 2.0, x + layout.widths[i] / 2.0))).collect();
            boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|p| p[1].0 - p[0].1 >= 5.9));
        }
    }
    #[test]
    fn every_component_combination_fits_without_overlaps() {
        for battery in [false, true] { for music in [false, true] { for focus in [false, true] { for bell in [false, true] { for pet in [false, true] { for netspeed in [false, true] {
            let layout = Layout::new(battery, music, focus, bell, pet, false, false, netspeed);
            let mut boxes: Vec<_> = layout.x.iter().enumerate().filter_map(|(i, x)| x.map(|x| (x - layout.widths[i] / 2.0, x + layout.widths[i] / 2.0))).collect();
            boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(boxes.first().unwrap().0 >= -layout.width / 2.0 + 15.9);
            assert!(boxes.last().unwrap().1 <= layout.width / 2.0 - 15.9);
            assert!(boxes.windows(2).all(|pair| pair[1].0 - pair[0].1 >= 5.9), "组件不能挤在一起：{layout:?}");
            assert_eq!(layout.clearance(CLOCK), 1.0);
        } } } } } }
    }
    #[test]
    fn cpu_and_mem_are_independent_fixed_width_slots() {
        let both = Layout::new(false, false, false, false, false, true, true, false);
        let cpu_only = Layout::new(false, false, false, false, false, true, false, false);
        let mem_only = Layout::new(false, false, false, false, false, false, true, false);
        assert!(both.x[CPU].is_some() && both.x[MEM].is_some());
        assert!(cpu_only.x[CPU].is_some() && cpu_only.x[MEM].is_none());
        assert!(mem_only.x[MEM].is_some() && mem_only.x[CPU].is_none());
        assert_eq!(both.widths[CPU], WIDTHS[CPU]);
        assert_eq!(both.widths[MEM], WIDTHS[MEM]);
        assert!(both.x[CPU].unwrap() < both.x[MEM].unwrap());
        assert!((both.width - cpu_only.width - (WIDTHS[CPU] + GAP)).abs() < 0.01, "恰好少一个槽宽加间距");
    }
    #[test]
    fn netspeed_slot_is_fixed_width_and_toggles_cleanly() {
        let off = Layout::new(true, true, true, false, true, true, true, false);
        let on = Layout::new(true, true, true, false, true, true, true, true);
        assert!(off.x[NETSPEED].is_none());
        assert!(on.x[NETSPEED].is_some());
        assert_eq!(on.widths[NETSPEED], WIDTHS[NETSPEED]);
        assert!((on.width - off.width - (WIDTHS[NETSPEED] + GAP)).abs() < 0.01, "恰好多一个槽宽加间距");
        let mut motion = Motion::default();
        motion.step(on, 0.0, false);
        assert!(motion.settled());
    }
    #[test]
    fn replacing_battery_moves_continuously_and_supports_an_instant_state() {
        let before = Layout::new(true, true, false, false, true, false, false, false);
        let after = Layout::new(false, true, false, false, true, false, false, false);
        let mut motion = Motion::default();
        motion.step(before.clone(), 0.0, true);
        motion.step(after.clone(), 1.0 / 60.0, true);
        let mid = motion.current().x[MUSIC].unwrap();
        assert!(mid < before.x[MUSIC].unwrap() && mid > after.x[MUSIC].unwrap());
        for _ in 0..120 { motion.step(after.clone(), 1.0 / 60.0, true); }
        assert!((motion.current().x[MUSIC].unwrap() - after.x[MUSIC].unwrap()).abs() < 0.01);
        motion.step(before.clone(), 1.0 / 60.0, false);
        assert_eq!(motion.current().x, before.x);
    }
}
