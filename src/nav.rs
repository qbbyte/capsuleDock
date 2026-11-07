//! 切页：当前页索引、页面过渡弹簧、页码圆点、滚轮累积与页面顺序。
//!
//! 这是 [`crate::app::App`] 抽出的第二个领域子模块（批 2）。它持有切页相关的全部
//! 状态——当前页、上一页、过渡方向与起点时刻、横向滑动弹簧、四个页码圆点弹簧、
//! 滚轮累积量与上次翻页时刻——并把「带过渡地切到某页」「滚轮累积判定翻页」的纯逻辑
//! 收敛在这里。
//!
//! ## 与 [`crate::morph::MorphController`] 的边界
//!
//! 形变（坞体大小）与切页（面板内容横向滑动、页码圆点）是两套独立的弹簧，原本都塞在
//! `App::morph()` 里推进。批 1 把形变弹簧搬进了 `morph` 模块；本批把切页弹簧
//! （`slide` / `dots`）搬进 `nav` 模块，由各自的 `tick` 推进。两者互不依赖。
//!
//! ## 单向数据流（不回指 App）
//!
//! 切页需要读取「哪些页可见」「页顺序」（属于设置领域）。本模块**不持有 `App` 引用**：
//! 需要的设置以只读快照 [`PageLayout`] 传入；需要 `App` 执行的副作用（切页时取消编辑）
//! 通过返回的 [`NavAction`] 表达，由 `App` 在调用点消化。
//!
//! 注意：滚轮事件除了翻页，还可能滚动列表（待办/置物/剪贴板）。列表滚动属于各列表
//! 自身的状态，不在本模块。本模块只负责「累积滚轮、判定是否达到翻页阈值」；具体把
//! 这一格滚轮用于翻页还是滚列表，由 `App` 的帧逻辑根据光标位置决定，调用
//! [`PageNavigator::consume_scroll`] 返回是否真正翻页。

use crate::anim::Spring;

/// 切页状态：当前页 + 过渡参数 + 滑动/圆点弹簧 + 滚轮累积。
pub(crate) struct PageNavigator {
    pub page: usize,
    pub(crate) page_from: usize,
    pub(crate) page_dir: f32,
    pub(crate) page_t0: f64,
    pub(crate) slide: Spring,
    pub(crate) dots: [Spring; 4],
    pub(crate) wheel_delta: f32,
    pub(crate) wheel_at: f64,
    pub(crate) wheel_page_at: f64,
}

/// 每次切页/翻页需要的只读上下文：可见页位图与页顺序（来自 `Settings`）。
pub(crate) struct PageLayout {
    pub visible: [bool; 4],
    pub order: [usize; 4],
}

/// 切页推进要求 `App` 执行的连带副作用。
#[derive(Default, Debug, PartialEq)]
pub(crate) enum NavAction {
    #[default]
    None,
    /// 切到了新页：应取消编辑态。
    Changed,
}

impl PageNavigator {
    /// `initial` 是初始可见页索引；四个圆点弹簧以当前页为高亮初值。
    pub(crate) fn new(initial: usize) -> Self {
        Self {
            page: initial,
            page_from: initial,
            page_dir: 1.0,
            page_t0: -10.0,
            slide: Spring::new(0.0, 260.0, 0.92),
            dots: std::array::from_fn(|i| {
                Spring::new(if i == initial { 16.0 } else { 4.0 }, 380.0, 0.86)
            }),
            wheel_delta: 0.0,
            wheel_at: -1.0,
            wheel_page_at: -1.0,
        }
    }

    /// 过渡已进行的秒数。
    pub(crate) fn page_e(&self, t: f64) -> f32 {
        (t - self.page_t0) as f32
    }

    /// 带过渡地切到某页。`dir` > 0 表示内容从右侧进入。返回 [`NavAction::Changed`]
    /// 表示真的发生了切页（调用方据此取消编辑）；若目标页不可见或与当前页相同则不动。
    pub(crate) fn go(&mut self, page: usize, dir: f32, t: f64, layout: &PageLayout) -> NavAction {
        if page == self.page || !layout.visible[page] {
            return NavAction::None;
        }
        // 连点：上一段过渡还没结束时，以当前页为起点重新开始，位移速度保留。
        self.page_from = self.page;
        self.page = page;
        self.page_dir = dir;
        self.page_t0 = t;
        self.slide.pos =
            dir * PAGE_SLIDE + if self.page_e(t) < 0.4 { self.slide.pos * 0.3 } else { 0.0 };
        self.slide.target = 0.0;
        self.dots_target();
        NavAction::Changed
    }

    /// 按页顺序循环跳到下一个/上一个**可见**页；无可见页时不动作。
    pub(crate) fn navigate(&mut self, next: bool, t: f64, layout: &PageLayout) -> NavAction {
        let Some(position) = layout.order.iter().position(|page| *page == self.page) else {
            return NavAction::None;
        };
        for offset in 1..=layout.order.len() {
            let idx = (position + if next { offset } else { layout.order.len() - offset }) % layout.order.len();
            let page = layout.order[idx];
            if layout.visible[page] {
                return self.go(page, if next { 1.0 } else { -1.0 }, t, layout);
            }
        }
        NavAction::None
    }

    /// 累积一格滚轮输入，返回累积是否达到翻页阈值且应翻向何方（仅判定，不执行翻页）。
    /// `t` 是当前时间，用于「停顿或反向时丢弃上一段余量」。
    pub(crate) fn accumulate_scroll(&mut self, delta: f32, t: f64) -> Option<bool> {
        if t - self.wheel_at > 0.18 || self.wheel_delta * delta < 0.0 {
            self.wheel_delta = 0.0;
        }
        self.wheel_at = t;
        self.wheel_delta += delta;
        if self.wheel_delta.abs() >= 8.0 {
            let next = self.wheel_delta < 0.0;
            self.wheel_delta = 0.0;
            Some(next)
        } else {
            None
        }
    }

    /// 在滚轮确实触发翻页时调用：检查翻页节流（避免连续滚动连翻），翻页并刷新节流时刻。
    pub(crate) fn consume_scroll_to_page(
        &mut self,
        next: bool,
        t: f64,
        layout: &PageLayout,
    ) -> NavAction {
        if t - self.wheel_page_at >= 0.20 {
            let action = self.navigate(next, t, layout);
            if action == NavAction::Changed {
                self.wheel_page_at = t;
            }
            return action;
        }
        NavAction::None
    }

    /// 左右方向键 / 点击导航箭头：清掉滚轮累积后直接翻页。
    pub(crate) fn step_key(&mut self, next: bool, t: f64, layout: &PageLayout) -> NavAction {
        self.wheel_delta = 0.0;
        self.navigate(next, t, layout)
    }

    /// 推进切页弹簧（每帧由 `App::tick` 调用一次）。
    pub(crate) fn tick(&mut self, dt: f32) {
        for d in &mut self.dots {
            d.step(dt);
        }
        self.slide.step(dt);
    }

    fn dots_target(&mut self) {
        for (i, d) in self.dots.iter_mut().enumerate() {
            d.target = if i == self.page { 16.0 } else { 4.0 };
        }
    }
}

/// 切页位移幅度（与 `app.rs::PAGE_SLIDE` 同值；保留一份避免跨模块常量耦合）。
const PAGE_SLIDE: f32 = 46.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> PageLayout {
        PageLayout { visible: [true; 4], order: [0, 1, 2, 3] }
    }

    #[test]
    fn go_advances_and_reports_change_only_for_visible_target() {
        let mut nav = PageNavigator::new(0);
        assert_eq!(nav.go(2, 1.0, 1.0, &layout()), NavAction::Changed);
        assert_eq!(nav.page, 2);
        assert_eq!(nav.page_from, 0);
        // 同页不重复切。
        assert_eq!(nav.go(2, 1.0, 2.0, &layout()), NavAction::None);
        // 不可见的目标页不切。
        let hidden = PageLayout { visible: [true, true, false, true], order: [0, 1, 2, 3] };
        assert_eq!(nav.go(2, 1.0, 3.0, &hidden), NavAction::None);
        assert_eq!(nav.page, 2, "不可见页不会成为当前页");
    }

    #[test]
    fn navigate_wraps_and_skips_hidden_pages() {
        let mut nav = PageNavigator::new(3);
        let hidden = PageLayout { visible: [true, true, false, true], order: [0, 1, 2, 3] };
        // 3 -> 下一可见页应是 0（跳过不可见的 2）。
        assert_eq!(nav.navigate(true, 1.0, &hidden), NavAction::Changed);
        assert_eq!(nav.page, 0);
        // 上一页应回到 3（循环、跳过 2）。
        assert_eq!(nav.navigate(false, 2.0, &hidden), NavAction::Changed);
        assert_eq!(nav.page, 3);
    }

    #[test]
    fn scroll_accumulates_then_throttles_repeated_flips() {
        let mut nav = PageNavigator::new(0);
        // 单格滚轮未达阈值不翻页。
        for _ in 0..7 {
            assert!(nav.accumulate_scroll(-1.0, 0.0).is_none());
        }
        // 第 8 格达到阈值，返回翻向。
        assert_eq!(nav.accumulate_scroll(-1.0, 0.0), Some(true));
        // 真正翻页且刷新节流；紧接着再翻应被 0.2s 节流挡住。
        assert_eq!(nav.consume_scroll_to_page(true, 0.0, &layout()), NavAction::Changed);
        assert_eq!(nav.page, 1);
        assert_eq!(nav.consume_scroll_to_page(true, 0.1, &layout()), NavAction::None);
        assert_eq!(nav.page, 1, "节流期内不应连续翻页");
        // 超过节流窗口后可再翻。
        assert_eq!(nav.consume_scroll_to_page(true, 0.25, &layout()), NavAction::Changed);
        assert_eq!(nav.page, 2);
    }

    #[test]
    fn reverse_scroll_discards_accumulation() {
        let mut nav = PageNavigator::new(0);
        // 正向累积几格，再反向——反向时应清零而不是触发翻页。
        for _ in 0..5 { nav.accumulate_scroll(-1.0, 0.0); }
        assert!(nav.accumulate_scroll(1.0, 0.0).is_none(), "反向滚轮应丢弃累积");
    }

    #[test]
    fn dots_and_slide_springs_advance() {
        let mut nav = PageNavigator::new(0);
        nav.go(3, 1.0, 1.0, &layout());
        // 切页后高亮圆点目标变大；推进几帧后弹簧应当趋近目标。
        assert_eq!(nav.dots[3].target, 16.0);
        for _ in 0..60 { nav.tick(1.0 / 60.0); }
        assert!(nav.dots[3].pos > 8.0, "高亮圆点应当趋近 16");
        assert!(nav.slide.pos.abs() < 4.0, "滑动弹簧应已收敛回 0");
    }
}
