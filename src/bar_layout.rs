//! 常驻组件按实际占位排布，并在开关变化时连续补位。
use crate::anim::{smoothstep, Spring};

pub(crate) const CLOCK: usize = 0;
pub(crate) const BATTERY: usize = 1;
pub(crate) const MUSIC: usize = 2;
pub(crate) const FOCUS: usize = 3;
pub(crate) const BELL: usize = 4;
pub(crate) const PET: usize = 5;
/// 系统状态组件：CPU / 内存双圆环，紧邻时钟。
pub(crate) const SYSMON: usize = 6;
const WIDTHS: [f32; 7] = [44.0, 20.0, 14.0, crate::focus_bar::WIDTH, 20.0, 32.0, 36.0];

pub(crate) const DEFAULT_ORDER: [usize; 6] = [BATTERY, PET, CLOCK, SYSMON, MUSIC, FOCUS];
pub(crate) fn valid_order(order: &[usize]) -> bool {
    order.len() == 6 && DEFAULT_ORDER.iter().all(|slot| order.iter().filter(|value| *value == slot).count() == 1)
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
    pub fn new(battery: bool, music: bool, focus: bool, bell: bool, pet: bool, sysmon: bool) -> Self {
        fn place(cursor: &mut f32, width: f32) -> f32 { let x = *cursor + width / 2.0; *cursor += width + 12.0; x }
        let mut x = vec![None; WIDTHS.len()];
        let mut cursor = 16.0;
        if battery { x[BATTERY] = Some(place(&mut cursor, WIDTHS[BATTERY])); }
        if pet { x[PET] = Some(place(&mut cursor, WIDTHS[PET])); }
        x[CLOCK] = Some(place(&mut cursor, WIDTHS[CLOCK]));
        if sysmon { x[SYSMON] = Some(place(&mut cursor, WIDTHS[SYSMON])); }
        for (slot, on) in [(MUSIC, music && !bell), (FOCUS, focus), (BELL, bell)] {
            if on { x[slot] = Some(place(&mut cursor, WIDTHS[slot])); }
        }
        let width = cursor - 12.0 + 16.0;
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
            cursor += right - left + 12.0;
        }
    }
    /// 组件经过吉祥物时暂时淡出，避免移动中相互遮挡。
    pub fn clearance(&self, slot: usize) -> f32 {
        self.pet_clearance(slot)
    }
    pub fn pet_clearance(&self, slot: usize) -> f32 {
        match (self.x[slot], self.x[PET]) {
            (Some(x), Some(pet)) => smoothstep(self.widths[slot] / 2.0 + 14.0, self.widths[slot] / 2.0 + 24.0, (x - pet).abs()),
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
    fn default() -> Self { Self { target: Layout::new(false, false, false, false, false, false), positions: vec![Spring::new(0.0, 360.0, 0.76); 7], initialized: false } }
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
        for order in [DEFAULT_ORDER, [FOCUS, MUSIC, CLOCK, SYSMON, PET, BATTERY]] {
            let mut layout = Layout::new(true, true, true, false, true, false);
            layout.reorder(&order);
            layout.widen_clock(40.0);
            let mut boxes: Vec<_> = layout.x.iter().enumerate().filter_map(|(i, x)| x.map(|x| {
                let width = layout.widths[i] + if i == CLOCK { 40.0 } else { 0.0 };
                (x - width / 2.0, x + width / 2.0)
            })).collect();
            boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|p| p[1].0 - p[0].1 >= 6.9));
            assert!(boxes.first().unwrap().0 >= -layout.width / 2.0 + 15.9);
            assert!(boxes.last().unwrap().1 <= layout.width / 2.0 - 15.9);
        }
    }
    #[test]
    fn custom_order_preserves_groups_and_hidden_positions() {
        assert!(!valid_order(&[0, 0, 2, 3, 9]));
        assert!(!valid_order(&[0, 1, 2]));
        let order = [FOCUS, MUSIC, CLOCK, SYSMON, PET, BATTERY];
        for battery in [false, true] {
            let mut layout = Layout::new(battery, true, true, false, true, false);
            let width = layout.width;
            layout.reorder(&order);
            assert_eq!(layout.width, width);
            let positions: Vec<_> = order.iter().filter_map(|i| layout.x[*i]).collect();
            assert!(positions.windows(2).all(|p| p[0] < p[1]));
            let mut boxes: Vec<_> = layout.x.iter().enumerate().filter_map(|(i, x)| x.map(|x| (x - layout.widths[i] / 2.0, x + layout.widths[i] / 2.0))).collect();
            boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|p| p[1].0 - p[0].1 >= 6.9));
        }
    }
    #[test]
    fn every_component_combination_fits_without_overlaps() {
        for battery in [false, true] { for music in [false, true] { for focus in [false, true] { for bell in [false, true] { for pet in [false, true] {
            let layout = Layout::new(battery, music, focus, bell, pet, false);
            let mut boxes: Vec<_> = layout.x.iter().enumerate().filter_map(|(i, x)| x.map(|x| (x - layout.widths[i] / 2.0, x + layout.widths[i] / 2.0))).collect();
            boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(boxes.first().unwrap().0 >= -layout.width / 2.0 + 15.9);
            assert!(boxes.last().unwrap().1 <= layout.width / 2.0 - 15.9);
            assert!(boxes.windows(2).all(|pair| pair[1].0 - pair[0].1 >= 6.9), "组件不能挤在一起：{layout:?}");
            assert_eq!(layout.clearance(CLOCK), 1.0);
        } } } } }
    }
    #[test]
    fn replacing_battery_moves_continuously_and_supports_an_instant_state() {
        let before = Layout::new(true, true, false, false, true, false);
        let after = Layout::new(false, true, false, false, true, false);
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
