//! 置物架视图状态：滚动位置、条目进出动画、分栏选择、清空确认、图标缓存。
//!
//! 这是 [`crate::app::App`] 抽出的第五个领域子模块（批 5）。胶囊坞的「置物架」页
//! 把电脑上的文件「只记位置、不复制」地摆成一排，并附带一个「剪贴板」分栏。本模块
//! 持有这个页面全部的**视图状态**——当前分栏、滚动偏移、每条文件进出动画的弹簧、
//! 清空操作的二次确认计时、右键菜单的命中矩形、文件图标缩略图缓存。
//!
//! ## 视图状态 vs 绘制
//!
//! 与前几批一致：本模块只管「状态」，不管「怎么画」。`page_shelf` / `page_clipboard`
//! / `shelf_row` 等 immediate-mode 绘制方法留在 `App`（它们要直接操作 `Card`、产生
//! `Cmd`），通过 `self.shelf_view.*` 访问本模块的字段。本模块提供 [`ShelfView::tick`]
//! 推进动画弹簧与清空确认的过期，其余都是公开字段供绘制层读写。
//!
//! ## 与 `morph` 收起的协作
//!
//! 坞体从展开收起时（见 `App::morph` 消化 `MorphAction::Closed`）会把
//! [`ShelfView::confirm_clear`] 清空，以免下次展开时残留一个红色「确认清空」按钮。
//! 这是 `App` 直接写本模块字段，不需要额外接口——和 `todo.cancel()` 同样的轻量协作。

use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui::Rect;

use crate::anim::Spring;
use crate::icons::Cache as IconCache;

/// 置物架页面的视图状态。
pub(crate) struct ShelfView {
    /// 当前分栏：0 = 文件，1 = 剪贴板。
    pub shelf_tab: usize,
    /// 文件分栏的滚动偏移（以「第几条」为单位，弹簧驱动）。
    pub shelf_scroll: Spring,
    /// 文件分栏当前可见的第一条索引（滚轮 / 删除后裁剪）。
    pub shelf_first: usize,
    /// 剪贴板分栏当前可见的第一条索引。
    pub clip_first: usize,
    /// 每条文件的水平位置弹簧（路径 -> 弹簧），负责条目横向进出动画。
    pub shelf_x: HashMap<PathBuf, Spring>,
    /// 每条文件的「放入时刻」，用于淡入动画。
    pub shelf_born: HashMap<PathBuf, f64>,
    /// 「清空置物架」的二次确认计时（按下「清空」时记下时刻，3 秒内再点才执行）。
    pub confirm_clear: Option<f64>,
    /// 「清空剪贴板未固定项」的二次确认计时。
    pub clip_confirm_clear: Option<f64>,
    /// 右键菜单当前的命中矩形（用于命中测试时把菜单区域并入可交互区）。
    pub shelf_menu_rect: Option<Rect>,
    /// 文件图标缩略图缓存（按路径取预览）。
    pub icons: IconCache,
}

impl ShelfView {
    pub(crate) fn new() -> Self {
        Self {
            shelf_tab: 0,
            shelf_scroll: Spring::new(0.0, 300.0, 0.9),
            shelf_first: 0,
            clip_first: 0,
            shelf_x: HashMap::new(),
            shelf_born: HashMap::new(),
            confirm_clear: None,
            clip_confirm_clear: None,
            shelf_menu_rect: None,
            icons: IconCache::default(),
        }
    }

    /// 每帧推进：滚动弹簧跟上 `shelf_first`，并让超时的清空确认失效。
    pub(crate) fn tick(&mut self, dt: f32, t: f64) {
        self.shelf_scroll.target = self.shelf_first as f32;
        self.shelf_scroll.step(dt);
        // 「清空置物架」确认超过 3 秒未复点则取消。
        if let Some(at) = self.confirm_clear {
            if t - at > 3.0 {
                self.confirm_clear = None;
            }
        }
    }

    /// 切换分栏时清掉两个分栏各自的清空确认态（避免带着「确认」按钮切换）。
    pub(crate) fn switch_tab(&mut self, tab: usize) {
        if self.shelf_tab != tab {
            self.shelf_tab = tab;
            self.confirm_clear = None;
            self.clip_confirm_clear = None;
        }
    }

    /// 清空置物架后重置所有文件相关的视图状态（弹簧、淡入时刻、图标、滚动位置）。
    pub(crate) fn reset_after_clear(&mut self) {
        self.icons.clear();
        self.shelf_born.clear();
        self.shelf_x.clear();
        self.shelf_first = 0;
        self.confirm_clear = None;
    }

    /// 删除某条文件后裁剪滚动偏移，并清掉它的视图痕迹。
    pub(crate) fn remove_path(&mut self, p: &PathBuf, item_count_after: usize) {
        self.icons.remove(p);
        self.shelf_x.remove(p);
        self.shelf_born.remove(p);
        self.shelf_first = self.shelf_first.min(item_count_after.saturating_sub(5));
    }

    /// 记录一条新放入文件的淡入起点（用于 `appear` 动画从右侧滑入）。
    pub(crate) fn note_drop(&mut self, p: PathBuf, at: f64) {
        self.shelf_born.insert(p, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_tab_clears_both_confirm_states() {
        let mut v = ShelfView::new();
        v.confirm_clear = Some(0.0);
        v.clip_confirm_clear = Some(0.0);
        v.switch_tab(1);
        assert_eq!(v.shelf_tab, 1);
        assert!(v.confirm_clear.is_none() && v.clip_confirm_clear.is_none(), "切分栏应清掉两个清空确认");
        // 切到同一分栏不应改动。
        v.clip_confirm_clear = Some(1.0);
        v.switch_tab(1);
        assert!(v.clip_confirm_clear.is_some(), "切到当前已选分栏不应清状态");
    }

    #[test]
    fn tick_expires_confirm_clear_after_three_seconds() {
        let mut v = ShelfView::new();
        v.confirm_clear = Some(0.0);
        v.tick(0.0, 2.9);
        assert!(v.confirm_clear.is_some(), "3 秒内确认态应保留");
        v.tick(0.0, 3.1);
        assert!(v.confirm_clear.is_none(), "超过 3 秒确认态应失效");
    }

    #[test]
    fn tick_advances_scroll_spring_toward_first() {
        let mut v = ShelfView::new();
        v.shelf_first = 3;
        v.tick(1.0 / 60.0, 0.0);
        assert_eq!(v.shelf_scroll.target, 3.0, "滚动目标应跟随 shelf_first");
        for _ in 0..120 { v.tick(1.0 / 60.0, 0.0); }
        assert!((v.shelf_scroll.pos - 3.0).abs() < 0.5, "弹簧应收敛到目标");
    }

    #[test]
    fn reset_after_clear_drops_all_file_state() {
        let mut v = ShelfView::new();
        v.shelf_first = 4;
        v.shelf_x.insert(PathBuf::from("a"), Spring::new(0.0, 1.0, 1.0));
        v.shelf_born.insert(PathBuf::from("a"), 0.0);
        v.confirm_clear = Some(0.0);
        v.reset_after_clear();
        assert!(v.shelf_x.is_empty() && v.shelf_born.is_empty(), "清空应抹掉所有文件视图状态");
        assert_eq!(v.shelf_first, 0);
        assert!(v.confirm_clear.is_none());
    }

    #[test]
    fn remove_path_clamps_first_and_drops_state() {
        let mut v = ShelfView::new();
        v.shelf_first = 5;
        v.shelf_x.insert(PathBuf::from("a"), Spring::new(0.0, 1.0, 1.0));
        v.shelf_born.insert(PathBuf::from("a"), 0.0);
        // 删后只剩 2 条 -> first 应回到 0（2.saturating_sub(5)=0）。
        v.remove_path(&PathBuf::from("a"), 2);
        assert!(v.shelf_x.is_empty() && v.shelf_born.is_empty());
        assert_eq!(v.shelf_first, 0);
    }
}
