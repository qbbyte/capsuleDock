//! 待办编辑：内联新增/编辑与提醒对话框的编辑状态收口。
//!
//! 这是 [`crate::app::App`] 抽出的第四个领域子模块（批 4）。胶囊坞的待办有两种编辑
//! 入口，共用同一份「正在编辑吗」的状态：
//!
//! - **内联编辑**：在待办页里直接新建或改某条（`adding` / `editing` / `buf`）。
//! - **提醒对话框**：弹出更完整的「编辑待办/设置提醒」面板（`task_dialog`），可改
//!   内容、选提醒时间、设每周重复。
//!
//! 两者都由 [`TodoEditor::is_active`] 汇总为「正在编辑」，用于决定坞体是否要保持
//! 展开、是否拦截滚轮翻页等。
//!
//! ## 为什么状态成模块、保存逻辑留 App
//!
//! 保存要把草稿写回 `todos`（持久化）、刷新 `today`、弹 `say("待办已保存")`、请求
//! 窗口焦点——这些都是 `App` 级编排。本模块只管**编辑状态**（草稿、焦点、弹簧动画）
//! 与**校验**（草稿是否合法），通过 [`SaveOutcome`] 把校验结果告诉 `App`，由 `App`
//! 执行实际落库。这与批 3 的 `NoticeStack` 同构：内聚的状态 + 窄接口 + 单向数据流。

use crate::anim::Spring;
use crate::task_picker::ReminderPicker;

/// 提醒对话框的草稿（待保存的待办字段 + 时间选择器）。
pub(crate) struct TaskDraft {
    pub index: usize,
    pub original: String,
    pub text: String,
    pub due: String,
    pub error: String,
    pub focused: bool,
    pub reminder_only: bool,
    pub repeat: u8,
    pub picker: ReminderPicker,
}

/// 内联编辑与对话框编辑的合一状态。
pub(crate) struct TodoEditor {
    /// 是否正在「新建」一条待办（待办页底部输入框）。
    pub adding: bool,
    /// 是否正在「编辑」某条已有待办（存的是索引）。
    pub editing: Option<usize>,
    /// 当前输入框文本（新建与编辑共用）。
    pub buf: String,
    /// 输入框是否已取得焦点（用于首帧请求焦点）。
    pub todo_input_focused: bool,
    /// 新增待办时的小弹跳动画。
    pub todo_pop: Spring,
    /// 提醒对话框草稿（`None` 表示对话框未开）。
    pub task_dialog: Option<TaskDraft>,
}

/// 保存草稿时本模块给出的校验结论；`App` 据此决定是否落库、是否报错。
pub(crate) enum SaveOutcome {
    /// 草稿不合法（内容空 / 时间格式错 / 任务已变）：`App` 应保留草稿并把错误显示给用户。
    Invalid,
    /// 草稿合法，可以落库。携带要写回的字段：`(索引, 正文, 到期, 重复)`。
    Valid { index: usize, text: String, due: Option<String>, repeat: u8 },
}

impl TodoEditor {
    pub(crate) fn new() -> Self {
        Self {
            adding: false,
            editing: None,
            buf: String::new(),
            todo_input_focused: false,
            todo_pop: Spring::new(0.0, 500.0, 0.45),
            task_dialog: None,
        }
    }

    /// 当前是否处于任何一种编辑状态（内联或对话框）。
    pub(crate) fn is_active(&self) -> bool {
        self.adding || self.editing.is_some() || self.task_dialog.is_some()
    }

    /// 是否正在内联新建（用于前台切换时清掉空草稿）。
    pub(crate) fn is_adding_empty(&self) -> bool {
        self.adding && self.buf.trim().is_empty()
    }

    /// 进入「新建」模式：清空编辑态与缓冲、置为新增。
    pub(crate) fn begin_add(&mut self) {
        self.adding = true;
        self.editing = None;
        self.buf.clear();
        self.todo_input_focused = false;
    }

    /// 进入「编辑某条」模式：载入该条正文。
    pub(crate) fn begin_edit(&mut self, index: usize, text: &str) {
        self.editing = Some(index);
        self.adding = false;
        self.buf = text.to_string();
        self.todo_input_focused = false;
    }

    /// 打开提醒对话框草稿。`text` / `due` / `repeat` 是待办当前字段。
    pub(crate) fn begin_dialog(&mut self, index: usize, text: &str, due: Option<&str>, repeat: u8) {
        self.task_dialog = Some(TaskDraft {
            index,
            original: text.to_string(),
            text: text.to_string(),
            due: due.unwrap_or("").to_string(),
            error: String::new(),
            focused: false,
            reminder_only: false,
            repeat,
            picker: ReminderPicker::new(due),
        });
    }

    /// 打开对话框并标记为「只设提醒」模式（隐藏正文编辑、进来就展开时间选择器）。
    pub(crate) fn begin_dialog_reminder(&mut self, index: usize, text: &str, due: Option<&str>, repeat: u8) {
        self.begin_dialog(index, text, due, repeat);
        if let Some(d) = self.task_dialog.as_mut() {
            d.reminder_only = true;
            d.picker.show();
        }
    }

    /// 退出一切编辑态（切页、收起、失焦时调用）。
    pub(crate) fn cancel(&mut self) {
        self.adding = false;
        self.editing = None;
        self.task_dialog = None;
        self.buf.clear();
        self.todo_input_focused = false;
    }

    /// 推进新增动画弹簧（每帧由 `App::tick` 调用）。
    pub(crate) fn tick(&mut self, dt: f32) {
        self.todo_pop.step(dt);
    }

    /// 给新增动画一个初速度（成功添加一条后调用）。`delta` 是要叠加的速度量。
    pub(crate) fn pop(&mut self, delta: f32) {
        self.todo_pop.vel += delta;
    }

    /// 校验对话框草稿能否落库；返回 [`SaveOutcome`]，`App` 负责实际写回 `todos`。
    /// `due_valid` 由 `App` 用 `parse_due` 判断时间格式是否合法后传入。
    pub(crate) fn validate_dialog(
        draft: &mut TaskDraft,
        original_unchanged: bool,
        due_valid: bool,
        due_is_empty: bool,
    ) -> SaveOutcome {
        if draft.text.trim().is_empty() {
            draft.error = "请填写任务内容".into();
            return SaveOutcome::Invalid;
        }
        if !due_is_empty && !due_valid {
            draft.error = "时间格式应为 18:30 或完整日期时间".into();
            return SaveOutcome::Invalid;
        }
        if !original_unchanged {
            draft.error = "任务已变化，请取消后重新打开".into();
            return SaveOutcome::Invalid;
        }
        let due = if due_is_empty { None } else { Some(draft.due.clone()) };
        SaveOutcome::Valid { index: draft.index, text: draft.text.trim().to_string(), due, repeat: draft.repeat }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_active_reflects_all_three_edit_modes() {
        let mut e = TodoEditor::new();
        assert!(!e.is_active());
        e.begin_add();
        assert!(e.is_active(), "新建模式应算编辑中");
        e.cancel();
        e.begin_edit(2, "买牛奶");
        assert!(e.is_active(), "编辑模式应算编辑中");
        assert_eq!(e.editing, Some(2));
        e.cancel();
        e.begin_dialog(0, "x", None, 0);
        assert!(e.is_active(), "对话框打开应算编辑中");
        e.cancel();
        assert!(!e.is_active());
    }

    #[test]
    fn cancel_clears_everything() {
        let mut e = TodoEditor::new();
        e.begin_add();
        e.buf = "残留".into();
        e.task_dialog = None;
        e.cancel();
        assert!(!e.adding && e.editing.is_none() && e.buf.is_empty() && e.task_dialog.is_none() && !e.todo_input_focused);
    }

    #[test]
    fn begin_edit_loads_text_and_clears_add() {
        let mut e = TodoEditor::new();
        e.begin_add();
        e.begin_edit(3, "完成任务");
        assert!(!e.adding, "进入编辑应退出新建");
        assert_eq!(e.editing, Some(3));
        assert_eq!(e.buf, "完成任务");
    }

    #[test]
    fn validate_rejects_empty_text() {
        let mut draft = TaskDraft { index: 0, original: "x".into(), text: "   ".into(), due: String::new(), error: String::new(), focused: false, reminder_only: false, repeat: 0, picker: ReminderPicker::new(None) };
        assert!(matches!(TodoEditor::validate_dialog(&mut draft, true, true, true), SaveOutcome::Invalid));
        assert_eq!(draft.error, "请填写任务内容");
    }

    #[test]
    fn validate_rejects_bad_due_format() {
        let mut draft = TaskDraft { index: 0, original: "x".into(), text: "t".into(), due: "not-a-time".into(), error: String::new(), focused: false, reminder_only: false, repeat: 0, picker: ReminderPicker::new(None) };
        // due 非空且不合法 -> Invalid
        assert!(matches!(TodoEditor::validate_dialog(&mut draft, true, false, false), SaveOutcome::Invalid));
        assert_eq!(draft.error, "时间格式应为 18:30 或完整日期时间");
    }

    #[test]
    fn validate_rejects_when_task_changed_underneath() {
        let mut draft = TaskDraft { index: 0, original: "旧".into(), text: "新".into(), due: String::new(), error: String::new(), focused: false, reminder_only: false, repeat: 0, picker: ReminderPicker::new(None) };
        // original_unchanged=false -> Invalid
        assert!(matches!(TodoEditor::validate_dialog(&mut draft, false, true, true), SaveOutcome::Invalid));
        assert_eq!(draft.error, "任务已变化，请取消后重新打开");
    }

    #[test]
    fn validate_accepts_valid_draft_and_reports_fields() {
        let mut draft = TaskDraft { index: 1, original: "x".into(), text: " 改后的内容 ".into(), due: "2026-11-07 09:00".into(), error: String::new(), focused: false, reminder_only: false, repeat: 0b0000001, picker: ReminderPicker::new(None) };
        match TodoEditor::validate_dialog(&mut draft, true, true, false) {
            SaveOutcome::Valid { index, text, due, repeat } => {
                assert_eq!(index, 1);
                assert_eq!(text, "改后的内容", "正文应去空白");
                assert_eq!(due.as_deref(), Some("2026-11-07 09:00"));
                assert_eq!(repeat, 0b0000001);
            }
            _ => panic!("合法草稿应被接受"),
        }
    }
}
