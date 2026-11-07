//! 通知/告警队列：告警的生命周期管理（入队、去重、按配置过滤、过期、出队）与铃铛计时。
//!
//! 这是 [`crate::app::App`] 抽出的第三个领域子模块（批 3，折中方案）。它持有
//! 告警队列的全部内存状态——当前展示中的告警、HUD 层告警、待出队队列、去重历史、
//! 通知到达后的铃铛计时窗口——并把「入队去重」「按设置过滤」「到期出队」的纯逻辑
//! 收敛在这里。
//!
//! ## 为什么只搬「队列逻辑」、不搬「收发入口」
//!
//! 收发入口（`alert` / `bluetooth_alert` / `receive_notification` /
//! `receive_wechat_reminder`）每个都要读 5–7 个 settings 字段、`pending` 命令队列、
//! 信使状态和当前时刻——它们本质是「协调设置 → 构造告警 → 决定是否转发系统」的
//! **编排逻辑**，属于 `App`。把编排搬进本模块会让 `NoticeInputs` 长成十几个字段的
//! 胖接口，反而降低内聚、抬高耦合。因此本模块只暴露窄接口 [`NoticeStack::enqueue`]，
//! `App` 在入口里构造好 [`Alert`] 后喂进来。
//!
//! ## 渲染态留在 `App`
//!
//! `notice_width`（告警横幅宽度弹簧）、`hud_alpha`（HUD 透明度缓动）、`notice_details`
//! （展开详情开关）三者紧耦合 immediate-mode 绘制循环（每帧随告警存在性驱动），属于
//! 渲染层状态，留在 `App`。本模块只管告警「在不在」，不管它「画多大」。
//!
//! ## 单向数据流
//!
//! 队列过滤/转发需要的 settings 通过只读快照 [`NoticeSettings`] 传入；入队时若需要
//! 转发系统通知，通过返回的 [`EnqueueAction`] 让 `App` 往自己的 `pending` 队列推命令，
//! 本模块不持有 `Cmd` 类型。

use std::collections::VecDeque;

use crate::extras::{Alert, AlertSource};

/// 通知/告警队列的全部状态。
pub(crate) struct NoticeStack {
    /// 当前正在展示的横幅告警（`None` 表示无）。
    pub notice: Option<Alert>,
    /// HUD 层告警的最近一份副本（用于药丸态下显示音量/蓝牙等系统提示）。
    pub hud_notice: Option<Alert>,
    /// 待出队的告警队列（最多 8 条）。
    pub(crate) alerts: VecDeque<Alert>,
    /// 本次运行内见过的内部告警历史，用于去重（最多 50 条）。
    pub(crate) history: VecDeque<Alert>,
    /// 通知到达后，坞体在该时间窗口内显示铃铛动画。
    pub notification_bell_until: f64,
    /// 最近一次通知到达的时刻（用于触发动画起点）。
    pub notification_bell_at: f64,
}

/// 入队时本模块要求 `App` 执行的连带动作。
#[derive(Default)]
pub(crate) enum EnqueueAction {
    #[default]
    None,
    /// 该告警应作为系统通知转发：`App` 据此往 `pending` 推 `Cmd::Notify(title, body)`。
    Notify(String, String),
}

/// 入队/过滤需要的只读设置快照（由 `App` 从 `Settings` 现算）。
pub(crate) struct NoticeSettings {
    pub notifications_enabled: bool,
}

impl NoticeStack {
    pub(crate) fn new() -> Self {
        Self {
            notice: None,
            hud_notice: None,
            alerts: VecDeque::new(),
            history: VecDeque::new(),
            notification_bell_until: 0.0,
            notification_bell_at: -10.0,
        }
    }

    /// 入队一条告警：记录历史、去重同事件、追加到队列。
    /// 返回 [`EnqueueAction::Notify`] 表示该告警应被 `App` 转发为系统通知。
    pub(crate) fn enqueue(&mut self, a: Alert, system: bool, settings: &NoticeSettings) -> EnqueueAction {
        let mut action = EnqueueAction::None;
        // 通知仅保存在本次运行的内存中。
        if a.source == AlertSource::Internal {
            self.history.push_front(a.clone());
            self.history.truncate(50);
        }
        if !settings.notifications_enabled {
            return action;
        }
        if system && a.source == AlertSource::Internal {
            action = EnqueueAction::Notify(a.title.clone(), a.body.clone());
        }
        let same_event = |old: &Alert| {
            old.source == a.source && old.title == a.title && (a.is_hud() || old.body == a.body)
        };
        if a.source != AlertSource::Internal || a.title != "蓝牙设备" {
            self.alerts.retain(|old| !same_event(old));
        }
        if a.source != AlertSource::Internal || a.title != "蓝牙设备" {
            if self.notice.as_ref().is_some_and(&same_event) {
                self.notice = Some(a);
                return action;
            }
        }
        self.alerts.push_back(a);
        while self.alerts.len() > 8 {
            self.alerts.pop_front();
        }
        action
    }

    /// 每帧推进：按 `allowed` 过滤队列与当前告警、处理到期、把队首弹出成当前告警。
    /// `t` 是当前时刻；`allowed` 决定某条告警在当前设置下是否允许展示。
    pub(crate) fn tick(&mut self, t: f64, mut allowed: impl FnMut(&Alert) -> bool) {
        self.alerts.retain(&mut allowed);
        if self.notice.as_ref().is_some_and(|a| !allowed(a)) {
            self.notice = None;
        }
        if self.hud_notice.as_ref().is_some_and(|a| !allowed(a)) {
            self.hud_notice = None;
        }
        if self.notice.as_ref().is_some_and(|a| t - a.at > a.duration) {
            self.notice = None;
        }
        if self.notice.is_none() {
            if let Some(mut a) = self.alerts.pop_front() {
                a.at = t;
                self.notice = Some(a);
            }
        }
    }

    /// 重新对队列中所有告警套用一次「预览设置」：当关闭通知内容预览时，
    /// 把外部来源的正文替换为占位文案。`App` 在设置变化或每帧按需调用。
    pub(crate) fn apply_preview_policy(&mut self, show_preview: bool) {
        if show_preview {
            return;
        }
        for a in self.alerts.iter_mut().chain(self.notice.iter_mut()).chain(self.hud_notice.iter_mut()) {
            if matches!(&a.source, AlertSource::External(_)) {
                a.body = "收到新通知 · 内容已隐藏".into();
            }
        }
    }

    /// 清掉某信使来源的全部告警（用户已查看该来源所有账号时调用）。
    pub(crate) fn clear_for_messenger(&mut self, source: crate::app::Messenger) {
        self.alerts.retain(|a| a.messenger() != Some(source));
        if self.notice.as_ref().is_some_and(|a| a.messenger() == Some(source)) {
            self.notice = None;
            self.notification_bell_until = 0.0;
        }
    }

    /// 当前展示中的告警是否属于某信使来源（供 `App` 计算 compact 横幅时用）。
    pub(crate) fn current_is_messenger(&self) -> bool {
        self.notice.as_ref().is_some_and(|a| a.messenger().is_some())
    }

    /// 触发铃铛计时窗口：通知到达后的 `duration` 秒内坞体显示铃铛动画。
    pub(crate) fn ring_bell(&mut self, at: f64, duration: f64) {
        self.notification_bell_at = at;
        self.notification_bell_until = at + duration;
    }
}

/// 告警横幅的展示尺寸（与告警来源/类型有关，与设置无关）。
pub(crate) fn notice_size(a: &Alert, hud_w: f32) -> (f32, f32) {
    if a.source == AlertSource::Internal && a.title == "蓝牙设备" {
        (hud_w, 48.0)
    } else if a.is_hud() {
        (hud_w, 38.0)
    } else {
        (334.0, 44.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Messenger;

    fn alert(title: &str) -> Alert {
        Alert {
            title: title.into(),
            source: AlertSource::Internal,
            body: "".into(),
            page: 0,
            at: 0.0,
            duration: 5.0,
            bluetooth: None,
        }
    }

    fn enabled() -> NoticeSettings {
        NoticeSettings { notifications_enabled: true }
    }

    #[test]
    fn enqueue_dedups_same_event_and_caps_queue() {
        let mut s = NoticeStack::new();
        for _ in 0..12 {
            s.enqueue(alert("系统音量"), false, &enabled());
        }
        // 同事件（且 is_hud）只保留一条。
        assert!(s.alerts.len() <= 1, "同事件告警应被去重");
        // 不同标题不会去重。
        let mut s2 = NoticeStack::new();
        for i in 0..12 {
            s2.enqueue(
                Alert { title: format!("X{i}"), source: AlertSource::External("app".into()), body: "b".into(), page: 0, at: 0.0, duration: 4.0, bluetooth: None },
                false,
                &enabled(),
            );
        }
        assert_eq!(s2.alerts.len(), 8, "队列最多保留 8 条");
    }

    #[test]
    fn tick_pops_queue_into_current_when_empty() {
        let mut s = NoticeStack::new();
        s.enqueue(alert("网络连接"), false, &enabled());
        assert!(s.notice.is_none());
        s.tick(1.0, |_| true);
        assert!(s.notice.is_some(), "队首应在 tick 时弹出成当前告警");
    }

    #[test]
    fn tick_expires_current_alert_and_filters_disallowed() {
        let mut s = NoticeStack::new();
        // at=0, duration=5 -> t=6 时到期。
        s.enqueue(Alert { title: "T".into(), source: AlertSource::Internal, body: "".into(), page: 0, at: 0.0, duration: 5.0, bluetooth: None }, false, &enabled());
        s.tick(0.0, |_| true);
        assert!(s.notice.is_some());
        s.tick(6.0, |_| true);
        assert!(s.notice.is_none(), "到期应被清掉");
        // 不被 allowed 的告警应被过滤。
        s.enqueue(Alert { title: "QQ".into(), source: AlertSource::External("QQ".into()), body: "".into(), page: 0, at: 0.0, duration: 5.0, bluetooth: None }, false, &enabled());
        s.tick(0.0, |a| !matches!(&a.source, AlertSource::External(_)));
        assert!(s.alerts.is_empty(), "不允许的来源应被过滤掉");
    }

    #[test]
    fn clear_for_messenger_drops_that_source() {
        let mut s = NoticeStack::new();
        s.enqueue(Alert { title: "微信".into(), source: AlertSource::Wechat, body: "x".into(), page: 0, at: 0.0, duration: 4.0, bluetooth: None }, false, &enabled());
        s.tick(0.0, |_| true);
        assert!(s.notice.as_ref().is_some_and(|a| a.messenger() == Some(Messenger::Wechat)));
        s.clear_for_messenger(Messenger::Wechat);
        assert!(s.notice.is_none(), "清信使时应连带清当前告警并停铃铛");
        assert_eq!(s.notification_bell_until, 0.0);
    }

    #[test]
    fn apply_preview_masks_external_bodies() {
        let mut s = NoticeStack::new();
        s.enqueue(Alert { title: "A".into(), source: AlertSource::External("app".into()), body: "secret".into(), page: 0, at: 0.0, duration: 4.0, bluetooth: None }, false, &enabled());
        s.tick(0.0, |_| true);
        s.apply_preview_policy(false);
        assert_eq!(s.notice.as_ref().unwrap().body, "收到新通知 · 内容已隐藏");
        // 开启预览时不动。
        s.apply_preview_policy(true);
        assert_eq!(s.notice.as_ref().unwrap().body, "收到新通知 · 内容已隐藏");
    }
}
