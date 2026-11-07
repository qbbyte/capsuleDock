//! 坞体形变：悬停展开 / 离开收起的弹簧动画与「意图锁」。
//!
//! 这是 [`crate::app::App`] 抽出的第一个领域子模块（批 1）。它持有形变相关的全部
//! 状态（宽高弹簧、是否展开、悬停/离开时刻、前台切换锁、强显进度），并把每帧推进
//! 形变的纯逻辑收敛在这里。
//!
//! ## 为什么形变逻辑单独成模块
//!
//! immediate-mode GUI 里，「坞体此刻长多大」由几十处状态共同决定（通知、侧边停靠、
//! 歌词条、专注、待办编辑……）。形变字段原先直接长在 `App` 上、逻辑散在 `app.rs`，
//! 使结构体膨胀到 80+ 字段。本模块把形变的状态收口成一个独立结构体，让 `App` 只持有
//! 一个 `morph: MorphController` 实例，而不是 7 个零散字段。
//!
//! ## 单向数据流（不回指 App）
//!
//! 形变推进需要读取大量跨领域的 `App` 状态（折叠尺寸、是否侧边、是否拖入感应区、
//! 是否极简、展开高度）。本模块**不持有 `App` 引用、也不回调 `App` 方法**——需要的
//! 输入全部通过 [`MorphInputs`] 以只读快照传入，需要 `App` 执行的副作用通过
//! [`MorphAction`] 返回，由 `App` 在调用点消化。这样 `morph` 永远不会循环依赖 `App`。
//!
//! `App::base_size()` 已经融合了通知/侧边/歌词/设置等多领域决策，折叠尺寸在调用方
//! 算好后放进 `inputs.base`，本模块只关心「往目标尺寸弹」这件事本身。

use crate::anim::Spring;

/// 形变状态：宽高弹簧 + 展开标志 + 悬停/离开计时 + 前台锁 + 强显进度。
pub(crate) struct MorphController {
    pub w: Spring,
    pub h: Spring,
    pub open: bool,
    pub(crate) hover_since: Option<f64>,
    pub(crate) leave_since: Option<f64>,
    /// 前台切换导致的强制收起：光标离开前不再自动展开。
    pub hover_lock: bool,
    pub(crate) reveal: f32,
}

impl MorphController {
    pub(crate) fn new(base_w: f32, base_h: f32) -> Self {
        // CLOSE 弹簧参数与收起状态一致，避免首帧出现非零速度。
        const CLOSE_STIFF: f32 = 620.0;
        const CLOSE_ZETA: f32 = 1.0;
        Self {
            w: Spring::new(base_w, CLOSE_STIFF, CLOSE_ZETA),
            h: Spring::new(base_h, CLOSE_STIFF, CLOSE_ZETA),
            open: false,
            hover_since: None,
            leave_since: None,
            hover_lock: false,
            reveal: 0.0,
        }
    }

    /// 每帧推进形变。`want` 表示是否希望展开；`inputs` 是调用方算好的只读上下文；
    /// 返回 [`MorphAction`] 让调用方执行连带副作用（收起时取消编辑、清空确认态）。
    pub(crate) fn tick(&mut self, want: bool, inputs: &MorphInputs, t: f64, dt: f32) -> MorphAction {
        let (bw, bh) = inputs.base;
        let mut action = MorphAction::None;
        if want {
            self.leave_since = None;
            let since = *self.hover_since.get_or_insert(t);
            let intent = if inputs.minimal { MINIMAL_HOVER_INTENT } else { HOVER_INTENT };
            if !self.open && (inputs.drag_hot || t - since >= intent) {
                self.open = true;
                self.w.tune(OPEN_W.0, OPEN_W.1);
                self.h.tune(OPEN_H.0, OPEN_H.1);
                self.w.target = PW;
                self.h.target = inputs.expanded_height;
                if inputs.drag_hot {
                    // 拖入不等缓冲，直接给一点初速度，30 ms 内就能看到形变
                    self.w.vel = self.w.vel.max(600.0);
                    self.h.vel = self.h.vel.max(260.0);
                }
            } else if !self.open {
                self.w.tune(SWELL_K.0, SWELL_K.1);
                self.h.tune(SWELL_K.0, SWELL_K.1);
                // 悬停时轻微胀大：厚度方向胀得多一点（侧边停靠时厚度是宽度）。
                let (sw, sh) = if inputs.side { (SWELL.1, SWELL.0) } else { SWELL };
                self.w.target = bw * sw;
                self.h.target = bh * sh;
            }
        } else {
            self.hover_since = None;
            if self.open {
                let since = *self.leave_since.get_or_insert(t);
                if t - since >= LEAVE_DELAY {
                    self.open = false;
                    self.leave_since = None;
                    self.w.tune(CLOSE.0, CLOSE.1);
                    self.h.tune(CLOSE.0, CLOSE.1);
                    self.w.target = bw;
                    self.h.target = bh;
                    // 从展开收起：连带取消编辑、清空置物架确认态（由 App 执行）。
                    action = MorphAction::Closed;
                }
            } else {
                self.w.tune(CLOSE.0, CLOSE.1);
                self.h.tune(CLOSE.0, CLOSE.1);
                self.w.target = bw;
                self.h.target = bh;
            }
        }
        if self.open { self.h.target = inputs.expanded_height; }
        let (prev_w, prev_h) = (self.w.pos, self.h.pos);
        self.w.step(dt);
        self.h.step(dt);
        // 仅在从外侧越过目标时收住，扩展提示条时不强制跳到目标宽度。
        if !self.open {
            if prev_w >= bw && self.w.pos < bw { self.w.snap(bw); }
            if prev_h >= bh && self.h.pos < bh { self.h.snap(bh); }
        }
        action
    }
}

/// 每帧喂给形变推进的只读上下文。`App` 在调用 `tick` 前算好这些值，
/// 把「坞体该多大」的多领域决策结果浓缩成几个数，形变模块只负责「弹向它」。
pub(crate) struct MorphInputs {
    /// 折叠态目标尺寸（宽, 高），由 `App::base_size()` 计算。
    pub base: (f32, f32),
    /// 是否侧边停靠（决定悬停胀大方向的轴）。
    pub side: bool,
    /// 是否拖入感应区（拖入时无需缓冲、直接展开并给初速度）。
    pub drag_hot: bool,
    /// 是否极简模式（决定悬停展开所需的停留时长）。
    pub minimal: bool,
    /// 展开态高度。
    pub expanded_height: f32,
}

/// 形变推进要求 `App` 执行的连带副作用。
#[derive(Default)]
pub(crate) enum MorphAction {
    #[default]
    None,
    /// 坞体从展开收起：应取消编辑、清空置物架确认态。
    Closed,
}

// 形变节奏常量（从 `app.rs` 顶部搬来，保持与改造前完全一致）。
const HOVER_INTENT: f64 = 0.20;
const MINIMAL_HOVER_INTENT: f64 = 0.40;
const LEAVE_DELAY: f64 = 0.12;
const SWELL: (f32, f32) = (1.075, 1.13);
const OPEN_W: (f32, f32) = (330.0, 0.70);
const OPEN_H: (f32, f32) = (290.0, 0.74);
const CLOSE: (f32, f32) = (620.0, 1.0);
const SWELL_K: (f32, f32) = (420.0, 0.8);
/// 展开面板宽（与 `app.rs::PW` 同值；这里保留一份避免模块间常量耦合）。
const PW: f32 = 419.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(base: (f32, f32)) -> MorphInputs {
        MorphInputs { base, side: false, drag_hot: false, minimal: false, expanded_height: 140.0 }
    }

    #[test]
    fn opens_after_hover_intent_and_snaps_back_on_leave() {
        let mut m = MorphController::new(163.0, 30.0);
        // 持续悬停超过 HOVER_INTENT(0.2s) 应展开到面板宽。每帧推进时间 t。
        let dt = 1.0 / 60.0;
        let mut t = 0.0;
        for _ in 0..20 { m.tick(true, &inputs((163.0, 30.0)), t, dt as f32); t += dt; }
        assert!(m.open);
        assert!(m.w.target >= PW - 1.0);
        // 光标离开：超过 LEAVE_DELAY(0.12s) 后收起，并报告 Closed 让 App 清理编辑态。
        let mut closed_seen = false;
        for _ in 0..20 {
            let act = m.tick(false, &inputs((163.0, 30.0)), t, dt as f32);
            if matches!(act, MorphAction::Closed) { closed_seen = true; }
            t += dt;
        }
        assert!(closed_seen, "收起时应返回 Closed 让 App 清理连带状态");
        assert!(!m.open);
    }

    #[test]
    fn drag_hot_opens_without_hover_buffer() {
        let mut m = MorphController::new(163.0, 30.0);
        // 拖入感应区：第一帧就应展开，不必等悬停缓冲。
        m.tick(true, &MorphInputs { base: (163.0, 30.0), side: false, drag_hot: true, minimal: false, expanded_height: 140.0 }, 0.0, 1.0 / 60.0);
        assert!(m.open, "拖入时无需缓冲应立即展开");
        assert!(m.w.vel > 0.0, "拖入展开应带正初速度");
    }

    #[test]
    fn minimal_mode_needs_longer_hover_intent() {
        let mut m = MorphController::new(44.0, 4.5);
        let dt = 1.0 / 60.0;
        // 极简模式：HOVER_INTENT(0.2s) 还不够，要 MINIMAL_HOVER_INTENT(0.4s) 才展开。
        let mut t = 0.0;
        for _ in 0..12 { m.tick(true, &MorphInputs { base: (44.0, 4.5), side: false, drag_hot: false, minimal: true, expanded_height: 140.0 }, t, dt as f32); t += dt; }
        assert!(!m.open, "极简模式下短时悬停不应展开");
        for _ in 0..16 { m.tick(true, &MorphInputs { base: (44.0, 4.5), side: false, drag_hot: false, minimal: true, expanded_height: 140.0 }, t, dt as f32); t += dt; }
        assert!(m.open, "极简模式下悬停够久应展开");
    }
}
