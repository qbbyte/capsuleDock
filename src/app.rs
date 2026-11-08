//! 灵动坞主体：状态、悬停形变、切页、顶栏与公共层。与平台无关——
//! Windows 外壳（`main.rs`）和无窗口录制器（`capsuledock-record`）共用同一份代码。
//!
//! 时间一律来自 `Env::t`（秒），不直接读系统时钟，所以录屏可逐帧复现。

use crate::extras::{Alert, PetState};
use crate::anim::{self, lerp, smoothstep, Spring};
use crate::bar_layout::{self as bar, Layout};
use crate::glass;
use crate::lyrics;
use crate::model;
use crate::paint::{self, Card, H};
use crate::shape;
use crate::smtc::{NowPlaying, Transport};
use crate::theme::{self, size, W};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::path::PathBuf;

// ---------- 几何（逻辑点；原版 125% 截图 ÷ 1.25） ----------
pub const PW: f32 = 419.0; // 展开面板宽
pub const PH: f32 = 140.0; // 展开面板高
pub const BAR_W: f32 = 163.0; // 收起药丸宽
pub const BAR_H: f32 = 30.0; // 收起药丸高
pub(crate) const HUD_W: f32 = 260.0;
pub(crate) const LYRIC_BAR_W: f32 = 300.0;
pub(crate) const HUD_TRACK_X: f32 = 48.0;
pub(crate) const HUD_TRACK_W: f32 = 150.0;
pub const BAR_W_MIN: f32 = 44.0; // 极简小条
pub const BAR_H_MIN: f32 = 4.5;
pub const FLARE: f32 = 17.6; // 顶部外翻肩宽
pub const FLARE_H: f32 = 16.0; // 外翻收敛高度
pub const RAD: f32 = 33.0; // 底部圆角
pub const WIN_W: f32 = 640.0; // 窗口（透明画布）
pub const CANVAS_W: f32 = WIN_W + 320.0; // 保留右侧通知及阴影的完整空间，原坞仍按 WIN_W 定位。
pub const WIN_H: f32 = 270.0;
/// 胶囊皮肤与屏幕顶边的间隙。
pub const PILL_GAP: f32 = 6.0;

/// 胶囊的柔和投影：四周都有，多层叠加近似高斯。
fn pill_shadow(painter:&egui::Painter,rect:Rect,radius:f32,strength:f32){
    if strength<=0.01 {return;}
    for (grow,a) in [(10.0f32,0.035f32),(6.5,0.05),(3.5,0.07),(1.5,0.09)] {
        let r=Rect::from_min_max(rect.min-Vec2::new(grow*0.6,grow*0.3),rect.max+Vec2::new(grow*0.6,grow));
        painter.rect_filled(r,radius+grow*0.8,Color32::from_black_alpha((255.0*a*strength) as u8));
    }
}
pub(crate) const PAGE_NAMES: [&str; 4] = ["音乐", "专注", "置物架", "待办"];
/// 待办页：铃铛与提醒跳转的落点。
pub(crate) const TODO_PAGE: usize = 3;

// ---------- 动效参数 ----------
// 悬停/收起的节奏常量（HOVER_INTENT / MINIMAL_HOVER_INTENT / LEAVE_DELAY / SWELL_K）
// 已随形变状态机迁入 `morph.rs`，这里只保留展开/收起/胀大的几何参数。
/// 悬停时轻微胀大量（厚度轴、长边轴）；用于预测悬停态下的命中框，避免视觉与命中错位。
const SWELL: (f32, f32) = (1.075, 1.13);
// 展开：欠阻尼，末段约 5% 过冲后回落；高度略软于宽度，像「垂落」下来。
const OPEN_W: (f32, f32) = (330.0, 0.70);
const OPEN_H: (f32, f32) = (290.0, 0.74);
// 收起：临界阻尼，约 0.18 s 干净收回，无回弹。
const CLOSE: (f32, f32) = (620.0, 1.0);
// 切页过渡位移幅度（PAGE_SLIDE）已随切页状态机迁入 `nav.rs`。

pub struct Options {
    /// 是否联网查歌词（录制/测试时关掉）。
    pub fetch_lyrics: bool,
    /// 吉祥物在多久无操作后出现（秒）。
    pub pet_idle: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self { fetch_lyrics: true, pet_idle: 45.0 }
    }
}

/// 每帧由外壳提供的外部世界。
#[derive(Clone, Default)]
pub struct Env {
    pub t: f64,
    /// `HH:MM:SS`
    pub clock: String,
    /// `YYYY-MM-DD`
    pub today: String,
    /// 顶栏日期，例如 `10月28日 周三`
    pub date: String,
    pub battery: Option<u8>,
    pub charging: bool,
    /// 互联网连接状态；未知时不触发表情提醒。
    pub network: Option<bool>,
    /// 光标位置（与 `frame` 传入的 `win` 同一坐标系）；不在窗口附近时为 None。
    pub cursor: Option<Pos2>,
    pub idle_seconds: Option<f64>,
    /// 前台浏览器的顶部是否与坞重叠；胶囊坞自身窗口或暂不可读时为 None。
    pub browser_at_top: Option<bool>,
    pub media: NowPlaying,
    /// 系统主音量（0..100, 是否静音）；v0.5.4 音量条与系统双向同步。
    pub volume: Option<(u32, bool)>,
    /// 已连接的蓝牙音频设备（名称 + 可选电量）。
    pub bt: Vec<crate::bt::Device>,
    /// 蓝牙服务是否已经完成首次扫描。
    pub bt_ready: bool,
    /// 坞所在屏幕（与 `win` 同一坐标系，逻辑点）；用于让通知详情留在屏幕内。
    pub screen: Option<Rect>,
    /// 毛玻璃皮肤：坞体四周采样得到的背景色（sRGB 0..1）。
    pub backdrop: Option<[f32; 3]>,
    /// 毛玻璃皮肤：系统背景模糊当前是否生效。
    pub glass_blur: bool,
    /// 不支持逐像素透明的兼容后端需要按内容轮廓裁剪窗口。
    pub opaque_surface: bool,
}

impl Env {
    pub fn hour(&self) -> usize {
        self.clock.get(0..2).and_then(|s| s.parse().ok()).unwrap_or(12)
    }
    pub fn minute(&self) -> u32 {
        self.clock.get(3..5).and_then(|s| s.parse().ok()).unwrap_or(0)
    }
}

/// 交给外壳执行的副作用。
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Media(Transport),
    Open(PathBuf),
    /// 需要键盘输入：外壳应让窗口可激活并取得焦点。
    Focus,
    Reveal(PathBuf),
    Copy(String),
    Notify(String, String),
    ToggleAutostart,
    AddFiles,
    DragFile(PathBuf),
    /// 把系统主音量设为 0..100（v0.5.4 音量条）。
    SetVolume(u32),
    /// 切换系统静音。
    ToggleMute,
}

/// 毛玻璃皮肤下原生窗口区域的组成部分（相对 `win` 左上角，逻辑点）。
/// 系统模糊只出现在窗口区域内，所以区域要贴合坞体轮廓，而不是外接矩形。
#[derive(Clone, Debug, PartialEq)]
pub enum GlassShape {
    /// 任意闭合轮廓。
    Outline(Vec<Pos2>),
    /// 坞体 / 通知胶囊：顶部外翻肩部 + 竖直侧边 + 底部圆角（参数同 `shape::fill`）。
    Notch { rect: Rect, flare: f32, flare_h: f32, radius: f32 },
    /// 停靠在左右边缘的坞体：肩部沿屏幕边缘展开（参数同 `shape::fill_docked`）。
    Docked { rect: Rect, dock: model::Dock, flare: f32, flare_h: f32, radius: f32 },
    /// 圆角矩形（通知详情、提示、浮动确认）。
    Round(Rect, f32),
}

impl GlassShape {
    /// 闭合轮廓（逻辑点）；窗口区域按它裁剪。
    pub fn outline(&self) -> Vec<Pos2> {
        match self {
            Self::Outline(points) => points.clone(),
            Self::Notch { rect, flare, flare_h, radius } => shape::outline(*rect, *flare, *flare_h, *radius),
            Self::Docked { rect, dock, flare, flare_h, radius } => shape::outline_docked(*rect, *dock, *flare, *flare_h, *radius),
            Self::Round(r, radius) => shape::outline(*r, 0.0, 0.5, *radius),
        }
    }
}

/// 一块系统模糊：物理像素的窗口坐标与圆角半径（0 = 直角）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlurPiece {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
}

/// 把毛玻璃轮廓拆成合成引擎能直接表达的圆角矩形块：
/// 主体向上多延伸一个圆角半径，让顶部两角落在窗口外，只留下底部圆角；
/// 外翻的肩部按物理像素逐行拼出（相邻同宽的行合并），边缘误差不超过半个像素。
pub fn blur_pieces(shapes: &[GlassShape], ppp: f32) -> Vec<BlurPiece> {
    let mut out = Vec::new();
    for shape in shapes {
        match shape {
            GlassShape::Docked { rect, dock, flare, flare_h, radius } => {
                if rect.width() <= 0.0 || rect.height() <= 0.0 { continue; }
                // 在贴顶坐标系里按刘海拆块，再把每块转到左 / 右边缘（宽高互换）。
                let (map, frame) = shape::EdgeMap::of(*rect, *dock);
                let mut pieces = blur_pieces(&[GlassShape::Notch { rect: frame, flare: *flare, flare_h: *flare_h, radius: *radius }], ppp);
                let edge = (map.apply(Pos2::ZERO).x * ppp).round();
                for p in &mut pieces {
                    let (x, y, w, h) = (p.x, p.y, p.w, p.h);
                    *p = BlurPiece { x: if *dock == model::Dock::Left { edge + y } else { edge - y - h }, y: x, w: h, h: w, radius: p.radius };
                }
                out.extend(pieces);
            }
            GlassShape::Notch { rect, flare, flare_h, radius } => {
                if rect.width() <= 0.0 || rect.height() <= 0.0 { continue; }
                let r = radius.min(rect.height() * 0.5).min(rect.width() * 0.5).max(0.0) * ppp;
                let fh = flare_h.min(rect.height() * 0.5).max(0.5) * ppp;
                let (x0, x1, y0, y1) = ((rect.left() * ppp).round(), (rect.right() * ppp).round(), (rect.top() * ppp).round(), (rect.bottom() * ppp).round());
                out.push(BlurPiece { x: x0, y: y0 - r, w: x1 - x0, h: y1 - y0 + r, radius: r });
                let rows = fh.ceil() as i32;
                let mut i = 0;
                while i < rows {
                    let wing = |row: i32| (flare * ppp * (1.0 - ((row as f32 + 0.5) / fh).min(1.0)).powf(2.4)).round();
                    let w = wing(i);
                    if w < 1.0 { break; }
                    let mut h = 1;
                    while i + h < rows && wing(i + h) == w { h += 1; }
                    let y = y0 + i as f32;
                    out.push(BlurPiece { x: x0 - w, y, w, h: h as f32, radius: 0.0 });
                    out.push(BlurPiece { x: x1, y, w, h: h as f32, radius: 0.0 });
                    i += h;
                }
            }
            GlassShape::Round(r, radius) => {
                out.push(BlurPiece { x: (r.left() * ppp).round(), y: (r.top() * ppp).round(), w: (r.width() * ppp).round(), h: (r.height() * ppp).round(), radius: radius * ppp });
            }
            GlassShape::Outline(points) => {
                // 任意轮廓只能退化为外接矩形；坞体本身走 Notch，不会用到这里。
                let b = Rect::from_points(points);
                if b.is_positive() {
                    out.push(BlurPiece { x: (b.left() * ppp).round(), y: (b.top() * ppp).round(), w: (b.width() * ppp).round(), h: (b.height() * ppp).round(), radius: 0.0 });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod blur_piece_tests {
    use super::*;

    fn covered(pieces: &[BlurPiece], x: f32, y: f32) -> bool {
        pieces.iter().any(|p| {
            if !(x >= p.x && x < p.x + p.w && y >= p.y && y < p.y + p.h) { return false; }
            // 圆角：只判断四角的圆弧外侧
            let r = p.radius;
            let cx = x.clamp(p.x + r, p.x + p.w - r);
            let cy = y.clamp(p.y + r, p.y + p.h - r);
            (x - cx).powi(2) + (y - cy).powi(2) <= r * r + 1e-3
        })
    }

    #[test]
    fn blur_follows_the_island_outline_not_its_bounding_box() {
        for ppp in [1.0, 1.25, 1.5, 2.0] {
            let rect = Rect::from_min_size(Pos2::new(110.0, 0.0), Vec2::new(419.0, 140.0));
            let shape = GlassShape::Notch { rect, flare: 17.6, flare_h: 16.0, radius: 33.0 };
            let pieces = blur_pieces(std::slice::from_ref(&shape), ppp);
            let outline = shape.outline();
            // 轮廓内部的点都被模糊覆盖，外部（含底部圆角外、肩部外）都不覆盖；边缘留 1.5 像素容差。
            let inside = |x: f32, y: f32| {
                let mut c = false;
                let mut prev = *outline.last().unwrap();
                for &next in &outline {
                    if (next.y > y) != (prev.y > y) && x < (prev.x - next.x) * (y - next.y) / (prev.y - next.y) + next.x { c = !c; }
                    prev = next;
                }
                c
            };
            let mut checked = 0;
            for yi in 0..(150.0 * ppp) as i32 {
                for xi in ((80.0 * ppp) as i32..(560.0 * ppp) as i32).step_by(3) {
                    let (x, y) = (xi as f32 + 0.5, yi as f32 + 0.5);
                    let (lx, ly) = (x / ppp, y / ppp);
                    let deep_in = inside(lx, ly) && [(-1.5, 0.0), (1.5, 0.0), (0.0, 1.5), (0.0, -1.5)].iter().all(|(dx, dy)| inside(lx + dx / ppp, (ly + dy / ppp).max(0.01)));
                    let far_out = !inside(lx, ly) && [(-1.5, 0.0), (1.5, 0.0), (0.0, 1.5), (0.0, -1.5)].iter().all(|(dx, dy)| !inside(lx + dx / ppp, ly + dy / ppp));
                    if deep_in { assert!(covered(&pieces, x, y), "{ppp} 缩放下坞体内 ({x},{y}) 没有模糊"); checked += 1; }
                    if far_out && y >= 0.0 { assert!(!covered(&pieces, x, y), "{ppp} 缩放下坞体外 ({x},{y}) 不能出现模糊"); }
                }
            }
            assert!(checked > 1000);
            assert!(pieces.len() < 60, "肩部合并后块数应很少：{}", pieces.len());
        }
    }

    #[test]
    fn side_docked_blur_follows_the_rotated_outline() {
        for dock in [model::Dock::Left, model::Dock::Right] {
            for ppp in [1.0, 1.5] {
                let x = if dock == model::Dock::Left { 0.0 } else { 81.0 };
                let shape = GlassShape::Docked { rect: Rect::from_min_size(Pos2::new(x, 200.0), Vec2::new(419.0, 140.0)), dock, flare: 17.6, flare_h: 16.0, radius: 33.0 };
                let pieces = blur_pieces(std::slice::from_ref(&shape), ppp);
                let outline = shape.outline();
                let inside = |x: f32, y: f32| {
                    let mut c = false;
                    let mut prev = *outline.last().unwrap();
                    for &next in &outline {
                        if (next.y > y) != (prev.y > y) && x < (prev.x - next.x) * (y - next.y) / (prev.y - next.y) + next.x { c = !c; }
                        prev = next;
                    }
                    c
                };
                let mut checked = 0;
                for yi in ((170.0 * ppp) as i32..(370.0 * ppp) as i32).step_by(2) {
                    for xi in (0..(500.0 * ppp) as i32).step_by(3) {
                        let (px, py) = (xi as f32 + 0.5, yi as f32 + 0.5);
                        let (lx, ly) = (px / ppp, py / ppp);
                        let near = [(-1.5, 0.0), (1.5, 0.0), (0.0, 1.5), (0.0, -1.5)];
                        let deep_in = inside(lx, ly) && near.iter().all(|(dx, dy)| inside((lx + dx / ppp).clamp(0.01, 499.99), ly + dy / ppp));
                        let far_out = !inside(lx, ly) && near.iter().all(|(dx, dy)| !inside(lx + dx / ppp, ly + dy / ppp));
                        if deep_in { assert!(covered(&pieces, px, py), "{dock:?} {ppp}: 坞体内 ({px},{py}) 没有模糊"); checked += 1; }
                        let on_canvas = lx > 0.5 && lx < 499.5;
                        if far_out && on_canvas { assert!(!covered(&pieces, px, py), "{dock:?} {ppp}: 坞体外 ({px},{py}) 不能有模糊"); }
                    }
                }
                assert!(checked > 1000);
            }
        }
    }

    #[test]
    fn rounded_rectangles_and_empty_shapes() {
        let pieces = blur_pieces(&[GlassShape::Round(Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(100.0, 40.0)), 10.0)], 1.5);
        assert_eq!(pieces, vec![BlurPiece { x: 15.0, y: 30.0, w: 150.0, h: 60.0, radius: 15.0 }]);
        assert!(blur_pieces(&[GlassShape::Notch { rect: Rect::NOTHING, flare: 5.0, flare_h: 8.0, radius: 12.0 }], 1.0).is_empty());
        // 极简细条：没有肩部，只有一块
        let tiny = blur_pieces(&[GlassShape::Notch { rect: Rect::from_min_size(Pos2::new(298.0, 0.0), Vec2::new(44.0, 4.5)), flare: 0.0, flare_h: 8.0, radius: 2.25 }], 1.25);
        assert_eq!(tiny.len(), 1);
    }
}

pub struct FrameOut {
    /// 原生窗口应保留的可见区域（相对 `win` 左上角）。
    pub region: Rect,
    /// 毛玻璃 + 系统模糊时，替代 `region` 的贴合轮廓；其余情况为 None。
    pub glass: Option<Vec<GlassShape>>,
    /// 鼠标命中区（相对 `win` 左上角），区域外点击穿透到下层窗口。
    pub hit: Rect,
    pub notice_hit: Option<Rect>,
    /// 文件拖入感应区；独立于普通鼠标命中区。
    pub file_drop_zone: Option<Rect>,
    /// 展开状态立即生效，不等待展开动画结束。
    pub expanded: bool,
    pub editing: bool,
    pub animating: bool,
    pub repaint_after: std::time::Duration,
    pub cmds: Vec<Cmd>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Focus,
    Rest,
}

impl Phase {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Phase::Focus => "专注时间",
            Phase::Rest => "休息",
        }
    }
}

pub(crate) struct Pomo {
    pub phase: Phase,
    pub remaining: f32,
    pub running: bool,
}

/// 本地歌词（酷狗、汽水）每 5 秒复查，最多约 1 分钟；网络失败每 30 秒重试，最多约 5 分钟。手动点重试会重新计数。
const NATIVE_RETRIES:u32=12;
const NETWORK_RETRIES:u32=10;

/// 与 `format!("{title}|{artist}|{duration:.0}")` 等价，但不分配内存：界面每帧要比较好几次。
fn lyric_key_matches(key:&str,media:&NowPlaying)->bool {
    use std::io::Write;
    let Some(rest)=key.strip_prefix(media.title.as_str()).and_then(|rest|rest.strip_prefix('|')) else{return false;};
    let Some(rest)=rest.strip_prefix(media.artist.as_str()).and_then(|rest|rest.strip_prefix('|')) else{return false;};
    let mut buffer=[0u8;32];
    let mut cursor=std::io::Cursor::new(&mut buffer[..]);
    if write!(cursor,"{:.0}",media.duration).is_err(){return rest==format!("{:.0}",media.duration);}
    let written=cursor.position() as usize;
    rest.as_bytes()==&buffer[..written]
}

#[test]
fn lyric_key_matches_the_formatted_key_without_allocating(){
    for (title,artist,duration) in [("歌","手",120.4),("A|B","C",0.0),("歌","",239.5),("歌","手",240.5),("","",f64::NAN),("t","a",1e30)] {
        let media=NowPlaying{title:title.into(),artist:artist.into(),duration,..Default::default()};
        let key=format!("{}|{}|{:.0}",title,artist,duration);
        assert!(lyric_key_matches(&key,&media),"{key}");
        assert!(!lyric_key_matches(&format!("{key}0"),&media));
        assert!(!lyric_key_matches(&format!("x{key}"),&media));
    }
}

#[derive(Default)]
pub(crate) struct LyricsState {
    pub key: String,
    pub island_animation_started: bool,
    pub scroll_animation_started: bool,
    pub loading: bool,
    pub error: Option<lyrics::FetchError>,
    pub retry_at: Option<f64>,
    /// 发起本次查询时已知的歌曲 ID；之后才拿到 ID 且还没有同步歌词时，按 ID 再查一次。
    pub song_id: Option<u64>,
    /// 同一首歌已自动重试的次数；达到上限后停止，避免整首歌期间反复读盘或联网。
    pub attempts: u32,
    pub lyrics: Option<lyrics::Lyrics>,
    pub rx: Option<std::sync::mpsc::Receiver<Result<(lyrics::Lyrics,bool), lyrics::FetchError>>>,
    pub native_rx: Option<std::sync::mpsc::Receiver<Option<lyrics::Lyrics>>>,
}

pub struct App {
    pub opts: Options,
    pub settings: model::Settings,
    pub(crate) todos: model::TodoList,
    pub(crate) shelf: model::Shelf,

    // 形变：宽高弹簧、展开标志、悬停/离开计时、前台锁、强显进度，全部收口在子模块。
    pub(crate) morph: crate::morph::MorphController,

    // 切页：当前页、过渡、滑动/圆点弹簧、滚轮累积，全部收口在子模块。
    pub(crate) nav: crate::nav::PageNavigator,

    // 拖入
    pub(crate) drag_hot: bool,
    pub(crate) drag_a: f32,
    drag_saved: Option<usize>,

    // 置物架页面视图状态：分栏、滚动、条目进出动画、清空确认、图标缓存，收口在子模块。
    pub(crate) shelf_view: crate::shelf_view::ShelfView,
    pub(crate) avoidance: crate::avoidance::Picker,
    pub clipboard: crate::clipboard::Shared,

    // 专注
    pub(crate) pomo: Pomo,
    pub(crate) ring_shown: Spring,
    pub(crate) reset_spin: Spring,

    // 待办编辑：内联新增/编辑与提醒对话框的状态，全部收口在子模块。
    pub(crate) todo: crate::todo::TodoEditor,

    // 音乐
    pub(crate) ly: LyricsState,
    pub(crate) ly_worker: Option<std::thread::JoinHandle<()>>,
    pub(crate) show_lyrics: bool,
    island_lyric_layout: Option<std::sync::Arc<egui::Galley>>,
    media_anchor: (String, f64, f64, bool),
    pub(crate) media: NowPlaying,
    pub(crate) music_artwork: Option<(std::sync::Arc<egui::ColorImage>,egui::TextureHandle)>,

    forced_open_until:f64,
    pub settings_open:bool,
    pub displays:Vec<crate::display_layout::Display>,
    browser_at_top:bool,
    browser_minimal_active:bool,
    /// 毛玻璃着色量（平滑过渡，避免背景变化时闪烁）；0 表示尚未初始化。
    glass_tint:f32,
    pub(crate) fluid:crate::fluid::Fluid,
    pub(crate) fluid_origin:Pos2,
    fluid_art:Option<egui::TextureId>,
    fluid_title:String,
    screen:Option<Rect>,
    pub(crate) settings_focus_requested:bool,
    /// 设置窗口的合成引擎模糊底板；无边框透明窗口靠它显示真实背景。
    #[cfg(windows)]
    pub(crate) settings_backdrop:Option<crate::composition::Backdrop>,
    #[cfg(windows)]
    pub(crate) settings_blur_unavailable:bool,
    pub(crate) settings_tab:usize,
    pub(crate) settings_category:usize,
    pub(crate) subpage:Option<usize>,
    // 通知/告警队列：当前告警、HUD 告警、待出队队列、去重历史、铃铛计时，收口在子模块。
    pub(crate) notices: crate::notice::NoticeStack,
    hud_alpha: f32,
    notice_width: Spring,
    notice_details: bool,
    pub(crate) pending: Vec<Cmd>,
    pub(crate) todo_first: usize,
    pub(crate) pet: crate::pet::Pet,
    // 其它
    toast: Option<(String, f64)>,
    toast_below: bool,
    last_input: f64,
    system_idle: Option<f64>,
    last_cursor: Option<Pos2>,
    pet_a: f32,
    last_t: Option<f64>,
    pub(crate) t: f64,
    pub(crate) dt: f32,
    pub(crate) today: String,
    pub(crate) current_hour: usize,
    pub(crate) cursor_local: Option<Pos2>,
    started: bool,
    /// v0.5.4：音量条展示值（拖动时先用本地值，随后跟上系统回报）。
    pub(crate) volume_ui: Option<(u32, bool)>,
    /// 拖动/滚轮刚改过音量，这段时间内系统回声不再单独弹一条。
    pub(crate) volume_quiet_until: f64,
    /// 音量条填充的缓动量（0..1），让数值变化有「量感」。
    pub(crate) volume_a: f32,
    /// 上一次看到的蓝牙设备，用来只在连接/断开时提示一次。
    pub(crate) bt_shown: Vec<crate::bt::Device>,
    bt_initialized: bool,
    pub(crate) updates: crate::updates::State,
    pub(crate) resident: bar::Motion,
    pub(crate) resident_power: (Option<u8>,bool),
    pub(crate) sysmon: crate::sysmon::SysMon,
    /// CPU / 内存的平滑显示值（0..100），让圆环填充有「量感」。
    cpu_shown: f32,
    mem_shown: f32,
}

impl App {
    pub fn new(ctx: &egui::Context, opts: Options) -> Self {
        theme::load_fonts(ctx);
        egui_extras::install_image_loaders(ctx);
        ctx.set_theme(egui::ThemePreference::Dark);
        ctx.set_visuals(egui::Visuals::dark());
        ctx.style_mut(|s| {
            s.interaction.selectable_labels = false;
            s.visuals.text_cursor.stroke = egui::Stroke::new(1.5_f32, theme::BLUE);
            s.visuals.selection.bg_fill = theme::alpha_blue(0.45);
        });
        let settings = model::Settings::load();
        let clipboard_on = settings.clipboard_history;
        let (bw, bh) = if settings.minimal { (BAR_W_MIN, BAR_H_MIN) } else { (BAR_W, BAR_H) };
        let focus_seconds=settings.focus_minutes as f32*60.0;
        let initial_lyrics=settings.lyrics_enabled;
        let initial_page=settings.page_order.iter().copied().find(|page|settings.visible_pages[*page]).unwrap_or(0);
        let mut app = Self {
            opts,
            settings,
            todos: model::TodoList::load(),
            shelf: model::Shelf::load(),
            morph: crate::morph::MorphController::new(bw, bh),
            nav: crate::nav::PageNavigator::new(initial_page),
            drag_hot: false,
            drag_a: 0.0,
            drag_saved: None,
            shelf_view: crate::shelf_view::ShelfView::new(),
            avoidance: Default::default(),
            clipboard: crate::clipboard::Shared::new(clipboard_on),
            pomo: Pomo { phase: Phase::Focus, remaining: focus_seconds, running: false },
            ring_shown: Spring::new(1.0, 160.0, 0.95),
            reset_spin: Spring::new(0.0, 160.0, 0.82),
            todo: crate::todo::TodoEditor::new(),
            ly: LyricsState::default(),
            ly_worker: None,
            show_lyrics: initial_lyrics,
            island_lyric_layout: None,
            media_anchor: (String::new(), 0.0, 0.0, false),
            media: NowPlaying::default(),
            music_artwork: None,
            forced_open_until:0.0, settings_open:false, displays:Vec::new(), browser_at_top:false, browser_minimal_active:false, glass_tint:0.0, fluid:Default::default(), fluid_origin:Pos2::ZERO, fluid_art:None, fluid_title:String::new(), screen:None, settings_focus_requested:false, settings_tab:0, settings_category:0, subpage:None,
            #[cfg(windows)]
            settings_backdrop:None,
            #[cfg(windows)]
            settings_blur_unavailable:false,
            notices: crate::notice::NoticeStack::new(), hud_alpha: 0.0, notice_width:Spring::new(0.0,CLOSE.0,CLOSE.1), notice_details:false, pending: vec![],
            todo_first: 0, pet: Default::default(),
            toast: None, toast_below: false,
            last_input: 0.0, system_idle: None,
            last_cursor: None,
            pet_a: 0.0,
            last_t: None,
            t: 0.0,
            dt: 0.0,
            today: String::new(), current_hour: 0,
            cursor_local: None,
            started: false,
            volume_ui: None,
            volume_quiet_until: 0.0,
            volume_a: 0.0,
            bt_shown: Vec::new(),
            bt_initialized: false,
            updates: Default::default(),
            resident: Default::default(), resident_power:(None,false), sysmon: Default::default(), cpu_shown: 0.0, mem_shown: 0.0,
        };
        if std::env::var("CAPSULEDOCK_DEBUG_OPEN_SETTINGS").is_ok() { app.open_settings(); }
        app
    }

    pub fn say(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), self.t));
        self.toast_below = false;
    }

    /// v0.5.4：把系统音量与蓝牙设备状态同步进界面；只在连接/断开时提示一次。
    fn sync_device(&mut self, env: &Env) {
        if self.t >= self.volume_quiet_until {
            if let Some(v) = env.volume {
                self.volume_ui = Some(v);
            }
        }
        if !self.settings.bluetooth_notifications {
            self.bt_shown.clear();
            self.bt_initialized=false;
            self.notices.alerts.retain(|a|a.source!=crate::extras::AlertSource::Internal || a.title!="蓝牙设备");
            if self.notices.notice.as_ref().is_some_and(|a|a.source==crate::extras::AlertSource::Internal && a.title=="蓝牙设备") {self.notices.notice=None;}
            return;
        }
        if !env.bt_ready { return; }
        // 只把首次完整扫描作为基线，之后的连接变化都应提示。
        if !self.bt_initialized {
            self.bt_initialized = true;
            self.bt_shown = env.bt.clone();
            return;
        }
        let connected: Vec<_> = env.bt.iter().filter(|d| !self.bt_shown.iter().any(|old| old.id == d.id)).cloned().collect();
        let disconnected: Vec<_> = self.bt_shown.iter().filter(|d| !env.bt.iter().any(|new| new.id == d.id)).cloned().collect();
        for device in &connected { self.bluetooth_alert(device, true); }
        for device in &disconnected { self.bluetooth_alert(device, false); }
        self.bt_shown = env.bt.clone();
    }

    /// 距离上次交互的秒数（小精灵出现的依据）。
    pub fn idle_for(&self) -> f64 {
        self.system_idle.unwrap_or(self.t - self.last_input)
    }

    pub fn show_page(&mut self,page:usize) {
        if !self.settings.visible_pages[page] {
            self.settings_tab=[1,2,4,3][page];
            self.open_settings();
            return;
        }
        self.notices.notice=None;
        self.go(page,1.0);
        self.morph.open=true; self.forced_open_until=self.t+4.0;
        self.morph.w.tune(OPEN_W.0,OPEN_W.1);self.morph.h.tune(OPEN_H.0,OPEN_H.1);
        self.morph.w.target=PW;self.morph.h.target=self.expanded_height();
    }
    pub fn open_settings(&mut self){
        self.settings_open=true;
        self.settings_focus_requested=true;
    }
    pub fn open_settings_category(&mut self,category:usize){
        self.settings_category=category.min(2);
        if category!=1 {self.settings_tab=if category==0{0}else{5};self.open_settings();return;}
        self.show_page(1);
        if self.settings.visible_pages[1] {self.subpage=Some(1);}
    }
    pub fn page(&self) -> usize {
        self.nav.page
    }

    pub(crate) fn set_page_visible(&mut self,page:usize,visible:bool) {
        if !visible && self.settings.visible_pages.iter().filter(|enabled|**enabled).count()==1
            && self.settings.visible_pages[page] { return; }
        self.settings.visible_pages[page]=visible;
        if !self.settings.visible_pages[self.nav.page] {
            if let Some(first)=self.settings.page_order.iter().copied().find(|page|self.settings.visible_pages[*page]) { self.go(first,1.0); }
        }
    }

    fn nav_layout(&self) -> crate::nav::PageLayout {
        crate::nav::PageLayout { visible: self.settings.visible_pages, order: self.settings.page_order }
    }

    fn navigate_page(&mut self,next:bool) {
        if self.nav.navigate(next, self.t, &self.nav_layout()) == crate::nav::NavAction::Changed {
            self.subpage = None;
            self.cancel_edit();
        }
    }

    pub fn is_editing(&self) -> bool {
        self.todo.is_active()
    }

    pub fn foreground_changed(&mut self) {
        self.morph.hover_lock=true;
        if self.todo.is_adding_empty(){self.cancel_edit();}
    }

    /// 带过渡地切到某页。`dir` > 0 表示内容从右侧进入。
    /// 切页的纯状态机收敛在 [`crate::nav::PageNavigator`]；这里消化它的连带副作用
    /// （清子页、取消编辑）。
    pub(crate) fn go(&mut self, page: usize, dir: f32) {
        if self.nav.go(page, dir, self.t, &self.nav_layout()) == crate::nav::NavAction::Changed {
            self.subpage = None;
            self.cancel_edit();
        }
    }

    /// 过渡已进行的秒数（委托给 `PageNavigator`，渲染层读取）。
    fn page_e(&self) -> f32 {
        self.nav.page_e(self.t)
    }

    pub(crate) fn cancel_edit(&mut self) {
        self.todo.cancel();
    }

    fn island_lyrics_active(&self)->bool {
        if self.side() {return false;}
        self.settings.island_lyrics && self.settings.visible_pages[0] && self.media.is_music()
            && !self.browser_minimal_active
            && lyric_key_matches(&self.ly.key,&self.media)
            && self.ly.lyrics.as_ref().is_some_and(|lyrics|lyrics.synced && lyrics.lines.iter().any(|line|!line.text.trim().is_empty()))
    }
    pub(crate) fn compact_notices(&self)->bool {self.island_lyrics_active()&&!self.morph.open}

    /// 换句、分段和展开高亮共用时间调整；正数延后，负数提前。
    pub(crate) fn lyric_position(&self)->f32 {(self.media.position-self.settings.lyric_delay_ms as f64/1000.0)as f32}

    pub(crate) fn island_lyric(&self) -> Option<(usize,&str)> {
        if !self.island_lyrics_active(){return None;}
        if !lyric_key_matches(&self.ly.key,&self.media) {return None;}
        let lyrics=self.ly.lyrics.as_ref().filter(|lyrics|lyrics.synced)?;
        let index=lyrics.index_at(self.lyric_position())?;
        let text=lyrics.lines.get(index)?.text.trim();
        (!text.is_empty()).then_some((index,text))
    }

    fn update_island_lyric_layout(&mut self,ctx:&egui::Context) {
        if !self.island_lyrics_active(){self.island_lyric_layout=None;return;}
        let current=self.island_lyric();
        let text=if let Some((_,line))=current {line}
            else if self.ly.loading || !lyric_key_matches(&self.ly.key,&self.media) {"正在查找歌词…"}
            else if let Some(lyrics)=&self.ly.lyrics {
                if !lyrics.synced {"暂无同步歌词"}
                else if lyrics.index_at(self.lyric_position()).is_none() {"前奏"}
                else {"间奏"}
            } else {""};
        let ink=if current.is_some(){theme::WHITE}else{theme::GRAY};
        let layout=|text:String| {
            let mut job=egui::text::LayoutJob::simple(text,theme::font(11.0,W::Semibold),ink,LYRIC_BAR_W-80.0);
            job.halign=egui::Align::Center;
            job.wrap.break_anywhere=true;
            ctx.fonts(|fonts|fonts.layout_job(job))
        };
        let mut galley=layout(text.to_owned());
        if let Some((index,_))=current.filter(|_|galley.rows.len()>1) {
            let lyrics=self.ly.lyrics.as_ref().unwrap();
            let start=lyrics.lines[index].t;
            let end=lyrics.lines.iter().skip(index+1).find(|line|line.t>start).map_or(self.media.duration as f32,|line|line.t);
            let progress=if end>start {((self.lyric_position()-start)/(end-start)).clamp(0.0,1.0)}else{0.0};
            // 按当前句的时间范围与各段字数切换，暂停和跳转直接跟随播放位置。
            let total:usize=galley.rows.iter().map(|row|row.glyphs.len()).sum();
            let mut count=0;
            let page=galley.rows.iter().position(|row|{count+=row.glyphs.len();progress*(total as f32)<count as f32}).unwrap_or(galley.rows.len()-1);
            let segment=galley.rows[page].glyphs.iter().map(|glyph|glyph.chr).collect::<String>();
            galley=layout(segment.trim().to_owned());
        }
        self.island_lyric_layout=Some(galley);
    }

    fn effective_minimal(&self)->bool {self.settings.minimal || self.browser_minimal_active}
    pub(crate) fn pet_enabled(&self)->bool {self.settings.pet && !self.effective_minimal() && !self.side()}
    pub(crate) fn pet_space(&self)->f32 {if self.pet_enabled(){44.0}else{0.0}}
    fn lyric_focus_space(&self)->f32 {if self.pomo.running{crate::focus_bar::LYRIC_WIDTH+10.0}else{0.0}}
    fn lyric_bar_width(&self)->f32 {LYRIC_BAR_W+if self.pet_enabled(){48.0}else{0.0}+self.lyric_focus_space()}
    fn notice_size(&self,a:&Alert)->(f32,f32) {
        crate::notice::notice_size(a, HUD_W)
    }
    pub fn set_pet(&mut self,enabled:bool) {
        self.settings.pet=enabled;if enabled {self.settings.minimal=false;}self.settings.save();
    }
    pub fn set_minimal(&mut self,enabled:bool) {
        self.settings.minimal=enabled;if enabled {self.settings.pet=false;}self.settings.save();
    }
    fn base_size(&self) -> (f32, f32) {
        if !self.compact_notices()&&!self.morph.open {
            if let Some(a)=&self.notices.notice {let (width,height)=self.notice_size(a);return (width+self.pet_space(),height);}
        }
        if self.side() {
            // 左右停靠：竖向胶囊（通知仍是贴边滑出的横向卡片，见上）。
            return self.side_base_size(self.effective_minimal()&&self.t>=self.notices.notification_bell_until);
        }
        if self.island_lyrics_active() {
            (self.lyric_bar_width(),BAR_H)
        } else if self.effective_minimal()&&self.t>=self.notices.notification_bell_until {
            (BAR_W_MIN, BAR_H_MIN)
        } else {
            (self.resident.target.width.min(WIN_W-32.0), BAR_H)
        }
    }

    /// 0 = 药丸，1 = 完整面板（展开时可略超过 1）。
    #[cfg(test)]
    pub(crate) fn current_width(&self) -> f32 { self.morph.w.pos+self.notice_width.pos }

    pub fn progress(&self) -> f32 {
        let (bw, bh) = self.base_size();
        // 左右停靠时面板沿水平方向从边缘滑出，用宽度衡量展开程度。
        if self.side() {return (self.morph.w.pos - bw) / (PW - bw).max(1.0);}
        (self.morph.h.pos - bh) / (self.expanded_height() - bh)
    }

    pub(crate) fn expanded_height(&self)->f32 {PH}

    // ---------------------------------------------------------------
    // 每帧入口
    // ---------------------------------------------------------------
    pub fn frame(&mut self, ui: &mut egui::Ui, win: Rect, env: &Env) -> FrameOut {
        let ctx = ui.ctx().clone();
        let dt = match self.last_t {
            Some(t0) => (env.t - t0).clamp(0.0, 0.1) as f32,
            None => 0.0,
        };
        let elapsed = self.last_t.map(|at| (env.t - at).max(0.0) as f32).unwrap_or(0.0);
        self.last_t = Some(env.t);
        self.t = env.t;
        self.current_hour=env.hour();
        self.system_idle=env.idle_seconds;
        self.dt = dt;
        if let Some(at_top)=env.browser_at_top {self.browser_at_top=at_top;}
        self.screen=env.screen;
        if self.settings.skin.is_glass() {
            let target=if self.settings.skin==model::Skin::GlassLight {glass::light_tint(env.backdrop,env.glass_blur)}else{glass::glass_tint(env.backdrop,env.glass_blur)};
            self.glass_tint=if self.glass_tint<=0.0 {target}else{anim::approach(self.glass_tint,target,3.0,dt)};
        }
        let browser_minimal=self.settings.browser_minimal && self.browser_at_top;
        if browser_minimal && !self.browser_minimal_active {
            self.morph.open=false;
            self.morph.hover_lock=true;
            self.morph.hover_since=None;
            self.morph.leave_since=None;
            self.forced_open_until=0.0;
        }
        self.browser_minimal_active=browser_minimal;
        if !self.started {
            self.started = true;
            self.last_input = env.t;
            self.nav.page_t0 = env.t - 10.0;
        }
        let mut cmds = Vec::new();

        self.tick(env, dt, elapsed);
        self.sync_device(env);
        self.tick_alerts();
        let hud_active=self.notices.notice.clone();
        if hud_active.as_ref().map(|a|&a.title)!=self.notices.hud_notice.as_ref().map(|a|&a.title)||hud_active.is_none(){self.notice_details=false;}
        if let Some(a)=&hud_active {self.notices.hud_notice=Some(a.clone());}
        self.hud_alpha=anim::approach(self.hud_alpha,if hud_active.is_some(){1.0}else{0.0},if hud_active.is_some(){16.0}else{8.0},dt);
        if hud_active.is_none() && self.hud_alpha<0.003 {self.notices.hud_notice=None;}
        let notice_target=if self.compact_notices()&&(self.notices.notice.is_some()||self.t<self.notices.notification_bell_until) {44.0}else{0.0};
        self.notice_width.target=notice_target;
        if self.compact_notices(){self.notice_width.step(dt);}else{self.notice_width.snap(0.0);self.notice_details=false;}
        self.update_media(env,&ctx);
        let bell=self.t<self.notices.notification_bell_until&&!self.compact_notices();
        self.resident_power=(env.battery,env.charging);
        let mut layout=Layout::new(self.settings.show_battery&&env.battery.is_some(),self.media.is_music()&&!bell,self.pomo.running,bell,self.pet_enabled(),self.settings.show_sysmon);
        layout.reorder(&self.settings.resident_order);
        if self.settings.clock_period!=0 { layout.widen_clock(40.0); }
        if self.pet_enabled() {
            if let Some(a)=self.notices.notice.as_ref().filter(|_|!self.compact_notices()&&!self.morph.open){layout.x[bar::PET]=Some(22.0-(self.notice_size(a).0+self.pet_space())/2.0);}
            else if self.island_lyrics_active() {layout.x[bar::PET]=Some(60.0-self.lyric_bar_width()/2.0);}
        }
        self.resident.step(layout,dt,ctx.style().animation_time>0.0&&bar::animations_enabled());
        if self.opts.fetch_lyrics {
            self.poll_lyrics();
        }
        self.update_island_lyric_layout(&ctx);

        let cx = win.center().x;
        // 胶囊皮肤悬浮在屏幕顶边下方。
        let pill = self.settings.island_shape==model::IslandShape::Pill;
        let top = win.top() + if pill { PILL_GAP } else { 0.0 };
        let side = self.side();
        let dock = self.dock();

        // ---------- 输入：拖放 ----------
        let (hovering_files, dropped) = ctx.input(|i| {
            (
                self.settings.visible_pages[2] && !i.raw.hovered_files.is_empty(),
                i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect::<Vec<_>>(),
            )
        });
        if !dropped.is_empty() && self.settings.visible_pages[2] {
            self.on_drop(dropped);
            self.show_page(2);
        } else if hovering_files && !self.drag_hot {
            self.drag_hot = true;
            if self.nav.page != 2 {
                self.drag_saved = Some(self.nav.page);
                self.go(2, if self.nav.page < 2 { 1.0 } else { -1.0 });
            }
        } else if !hovering_files && self.drag_hot {
            // 拖到外面松手 / 取消：安静地退回原页
            self.drag_hot = false;
            if let Some(p) = self.drag_saved.take() {
                self.go(p, if p < 3 { -1.0 } else { 1.0 });
            }
        }
        self.drag_a = anim::approach(self.drag_a, if self.drag_hot { 1.0 } else { 0.0 }, 16.0, dt);

        // ---------- 输入：悬停 ----------
        let p_prev = self.progress();
        let hit = if side {self.side_hit_rect(win,p_prev)} else {self.hit_rect(cx, top, p_prev)};
        let previous_menu=self.shelf_view.shelf_menu_rect.take();
        let inside = env.cursor.is_some_and(|c| hit.contains(c) || previous_menu.is_some_and(|r|r.contains(c)));
        let on_notice=env.cursor.is_some_and(|c|self.notice_rect(cx,top).is_some_and(|r|r.contains(c))||self.notice_details_rect(cx,top).is_some_and(|r|r.contains(c)));
        if !inside {
            self.morph.hover_lock = false;
        }
        if env.cursor != self.last_cursor || ctx.input(|i| !i.events.is_empty()) {
            if inside || hovering_files || ctx.input(|i| !i.events.is_empty()) {
                self.last_input = env.t;
            }
            self.last_cursor = env.cursor;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.is_editing() {
            self.cancel_edit();
        }
        if !self.is_editing() && inside && self.morph.open && self.morph.reveal > 0.95 {
            let (scroll, left, right, esc) = ctx.input(|i| (i.raw_scroll_delta.y, i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::Escape)));
            if esc { self.morph.hover_lock = true; }
            let layout=self.nav_layout();
            if left || right {
                // 方向键：清掉滚轮累积后直接翻页。
                if self.nav.step_key(right, self.t, &layout) == crate::nav::NavAction::Changed {
                    self.subpage=None;
                    self.cancel_edit();
                }
            } else if scroll!=0.0 {
                let cursor=env.cursor.unwrap()-self.content_origin(win).to_vec2();
                // 累积高精度滚轮的小幅输入；停顿或反向时丢弃上一段余量。
                if let Some(next)=self.nav.accumulate_scroll(scroll,self.t) {
                    let clipboard_tab=self.nav.page==2 && self.shelf_view.shelf_tab==1;
                    let list=match self.nav.page {
                        3 if Rect::from_min_max(Pos2::new(196.0,31.0),Pos2::new(377.0,124.0)).contains(cursor)
                            =>Some((&mut self.todo_first,self.todos.items.len().saturating_sub(3))),
                        2 if clipboard_tab
                            =>Some((&mut self.shelf_view.clip_first,self.clipboard.lock().items.len().saturating_sub(crate::pages::CLIP_ROWS))),
                        2 if Rect::from_min_max(Pos2::new(36.0,28.0),Pos2::new(383.0,128.0)).contains(cursor)
                            =>Some((&mut self.shelf_view.shelf_first,self.shelf.items.len().saturating_sub(5))),
                        _=>None,
                    };
                    let scrolled=list.is_some_and(|(first,max)|{
                        let target=if next {(*first+1).min(max)}else{first.saturating_sub(1)};
                        let changed=target!=*first;*first=target;changed
                    });
                    // 列表未滚动且非剪贴板内浏览时，滚轮触发翻页（带节流）。
                    if !scrolled && !clipboard_tab {
                        if self.nav.consume_scroll_to_page(next,self.t,&layout) == crate::nav::NavAction::Changed {
                            self.subpage=None;
                            self.cancel_edit();
                        }
                    }
                    ctx.input_mut(|i|{i.raw_scroll_delta=Vec2::ZERO;i.smooth_scroll_delta=Vec2::ZERO;});
                }
            }
        } else {self.nav.wheel_delta=0.0;}
        let want = (inside && !self.morph.hover_lock && (self.compact_notices()||self.notices.notice.is_none()||self.morph.open)) || self.morph.open && on_notice || (self.morph.open && self.t<self.forced_open_until) || self.drag_hot || (self.is_editing() && self.morph.open);
        self.morph(want, inside, env.t, dt);
        self.update_pet(env, cx);

        // ---------- 绘制 ----------
        let p = self.progress();
        let (w, h) = (self.morph.w.pos, self.morph.h.pos);
        let panel = self.island_rect(win, w, h);
        let body=panel;
        let pc = anim::clamp01(p);
        let painter = ui.painter_at(ui.max_rect().expand(40.0));
        let feather = 1.0 / ctx.pixels_per_point();
        let minimal_pill = self.effective_minimal() && !self.island_lyrics_active() && self.notices.notice.is_none() && self.t>=self.notices.notification_bell_until && pc < 0.02;
        let flare = if minimal_pill || pill { 0.0 } else { lerp(5.0, FLARE, pc) };
        let k = (w / PW).clamp(0.3, 1.1);
        let thickness = if side { crate::side_dock::SIDE_BAR_W } else { BAR_H };
        let radius = if minimal_pill { BAR_H_MIN / 2.0 } else if pill { (w.min(h) * 0.5).min(lerp(thickness * 0.5, 26.0, pc)) } else { lerp(12.5, RAD * k.min(1.0), pc) };
        let flare_h = lerp(8.0, FLARE_H, pc);
        let thick = glass::thickness(PH);
        let glass_skin = self.settings.skin.is_glass();
        let layer = painter.layer_id();
        let palette = self.skin_palette();
        // 换色时跳过的图形范围（坞体底色、阴影、吉祥物）。
        let mut keep_colors:Vec<(usize,usize)>=Vec::new();
        // 毛玻璃：真实模糊下保留柔和投影增加层次；无模糊时阴影再减淡一些，保持轻盈。
        let skin_shadow = if !glass_skin {1.0} else if env.glass_blur {0.75} else {0.7};
        let shadow_k = glass::shadow_alpha(0.0, thick) / 0.12 * skin_shadow;
        let island_shadow=|r:Rect,flare:f32,flare_h:f32,radius:f32,strength:f32| if pill {pill_shadow(&painter,r,radius,strength)} else if side {shape::shadow_docked(&painter,r,dock,flare,flare_h,radius,strength)} else {shape::shadow(&painter,r,flare,flare_h,radius,strength)};
        island_shadow(body, flare, flare_h, radius, smoothstep(0.1, 0.9, pc) * shadow_k);
        let notice_rect=self.notice_rect(cx,top);
        let notice_shape=notice_rect.map(|r|Rect::from_min_max(Pos2::new(r.left()-36.0,r.top()),r.max));
        if let Some(r)=notice_shape {island_shadow(r,5.0,8.0,12.5,0.5*shadow_k);}
        if self.settings.skin==model::Skin::Fluid {
            self.fluid_origin=body.min;
            let art=self.music_artwork.as_ref().map(|(_,texture)|texture.id());
            if art!=self.fluid_art || self.media.title!=self.fluid_title {
                let tint=if self.media.title.is_empty(){crate::fluid::Fluid::neutral()}
                    else if let Some((image,_))=&self.music_artwork{crate::fluid::Fluid::tint_from_image(image)}
                    else{crate::fluid::Fluid::tint_from_title(&self.media.title)};
                self.fluid.set_tint(tint);
                self.fluid_art=art;
                self.fluid_title.clone_from(&self.media.title);
            }
            self.fluid.set_cursor(env.cursor.filter(|p|body.contains(*p)).map(|p|(p.x-body.left(),p.y-body.top())));
            if !self.morph.open || self.nav.page!=0 || self.media.title.is_empty(){self.fluid.set_seek(None,0.0);}
            self.fluid.advance(env.t,dt);
        }
        self.paint_body(&painter, body, flare, flare_h, radius, feather);
        if let Some(r)=notice_shape {self.paint_body(&painter,r,5.0,8.0,12.5,feather);}
        let content_from=crate::skin_paint::shape_count(&ctx,layer);

        // 内容只随形体「揭开」：不缩放，按面板裁剪，到 55% 才开始淡入
        let reveal = if self.morph.open { smoothstep(0.50, 0.97, p) } else { smoothstep(0.55, 0.92, p) };
        self.morph.reveal = reveal;
        let origin = self.content_origin(win);
        let clip = panel.intersect(win);
        let base = Card::new(ui, origin, clip);
        self.cursor_local = env.cursor.map(|p| base.to_local(p));

        let banner=!self.compact_notices()&&self.notices.notice.is_some()&&!self.morph.open;
        let restoring_lyrics=!self.morph.open&&self.notices.notice.is_none()&&self.island_lyrics_active()&&self.notices.hud_notice.as_ref().is_some_and(|a|a.source!=crate::extras::AlertSource::Internal);
        let fading_hud=!self.compact_notices()&&!self.morph.open&&self.notices.hud_notice.as_ref().is_some_and(|a|a.is_hud())&&(banner||self.hud_alpha>0.003);
        if fading_hud {
            let from=self.resident.target.width;let to=HUD_W+self.pet_space();
            let width_progress=if (to-from).abs()<1.0 {if banner{1.0}else{0.0}}else{((self.morph.w.pos-from)/(to-from)).clamp(0.0,1.0)};
            let (opacity,header_opacity)=if banner {(self.hud_alpha*smoothstep(0.45,0.85,width_progress),1.0-smoothstep(0.05,0.40,width_progress))}
                else {(self.hud_alpha*smoothstep(0.20,0.60,width_progress),1.0-smoothstep(0.05,0.25,width_progress))};
            if !minimal_pill||pc>0.0 {self.resident_header(&base.faded(header_opacity),panel,cx,pc,reveal,env);}
            if let Some(a)=self.notices.hud_notice.clone(){
                let overlay=Card::new(ui,panel.min+Vec2::new(self.pet_space(),0.0),panel.intersect(ui.max_rect())).faded(opacity);
                let overlay=if opacity<0.85||!banner{overlay.ghost()}else{overlay};
                self.draw_notice(&overlay,panel,&a,&mut cmds);
            }
        }else if banner {
            if let Some(a)=self.notices.notice.clone(){self.draw_notice(&Card::new(ui,panel.min+Vec2::new(self.pet_space(),0.0),panel.intersect(ui.max_rect())),panel,&a,&mut cmds);}
        }else if !minimal_pill || pc > 0.0 {
            self.resident_header(&base, panel, cx, if restoring_lyrics{0.0}else{pc}, reveal, env);
        }
        if let Some(rect)=notice_rect {
            let c=Card::new(ui,rect.min,rect.intersect(ui.max_rect()));
            if let Some(a)=self.notices.hud_notice.clone() {
                let opacity=self.hud_alpha*smoothstep(0.45,0.95,self.notice_width.pos/44.0);
                let c=c.faded(opacity);let c=if self.notices.notice.is_none()||opacity<0.85 {c.ghost()}else{c};
                if a.source!=crate::extras::AlertSource::Internal {
                    paint::bell(&c.scaled(18.0,15.0,0.85),18.0,15.0,theme::WHITE,paint::bell_swing(self.t-a.at));
                    c.circle(22.0,10.5,2.0,Color32::from_rgb(255,72,72));
                }else{crate::extras::notice_badge(&c,18.0,15.0,&a);}
                let button=c.hit(0.0,0.0,36.0,BAR_H,"notice-badge").on_hover_text(format!("{}\n{}",a.title,a.body));
                if button.clicked(){self.notice_details=!self.notice_details;}
            }else if self.t<self.notices.notification_bell_until {
                paint::bell(&c.scaled(20.0,14.0,1.2),20.0,14.0,theme::WHITE,paint::bell_swing(self.t-self.notices.notification_bell_at));
                c.circle(24.0,9.5,2.0,Color32::from_rgb(255,72,72));
                if c.hit(6.0,0.0,28.0,30.0,"bell").clicked(){self.show_page(TODO_PAGE);}
            }
        }
        let notice_details=self.notice_details_rect(cx,top);
        if let Some(rect)=notice_details {
            let radius=10.0;
            let from=crate::skin_paint::shape_count(&ctx,layer);
            shape::shadow(&painter,rect,0.0,0.0,radius,0.5*shadow_k);
            painter.rect_filled(rect,radius,self.glass_fill());
            if glass_skin {
                painter.rect_stroke(rect,radius,egui::Stroke::new(1.0_f32,self.glass_rim()),egui::StrokeKind::Inside);
            }
            keep_colors.push((from,crate::skin_paint::shape_count(&ctx,layer)));
            self.draw_notice(&Card::new(ui,rect.min,rect.intersect(ui.max_rect())),rect,&self.notices.hud_notice.clone().unwrap(),&mut cmds);
            if ctx.input(|i|i.pointer.any_pressed()&&i.pointer.interact_pos().is_some_and(|p|!rect.contains(p)&&!notice_rect.is_some_and(|r|r.contains(p)))){self.notice_details=false;}
        }
        let pet_alpha=self.pet_a*(1.0-smoothstep(0.0,0.3,pc));
        if pet_alpha>0.004 && !minimal_pill {
            let y=(h.min(self.base_size().1)-BAR_H).max(0.0)/2.0;
            // 吉祥物跟随皮肤换色：浅色底上变成深色版本，保持清晰。
            let fit=if self.island_lyrics_active()||self.notices.notice.is_some(){1.0}else{((WIN_W-32.0)/self.resident.target.width).min(1.0)};
            self.pet.draw(&base.scaled(cx-origin.x,15.0,fit).shifted(0.0,y).faded(pet_alpha),cx-origin.x+self.resident.position(bar::PET),self.t);
        }
        if reveal > 0.004 && !banner && !restoring_lyrics {
            let c = base.shifted(0.0, (1.0 - reveal) * -5.0).faded(reveal);
            let c = if reveal < 0.6 { c.ghost() } else { c };
            if self.todo.task_dialog.is_some(){self.draw_task_editor(&c);}else{self.panel_body(&c, env, &mut cmds);}
            if !self.toast_below {self.toast_layer(&c,106.0);}
        }
        // 坞下的确认提示使用独立裁剪区域，收起时跟随坞体下沿。
        let floating_toast=if self.toast_below && self.toast.is_some() {
            // 侧边停靠：提示出现在胶囊正下方，靠边时向内挪，保证完整显示。
            let toast_origin=if side {
                let center=if dock==model::Dock::Left {panel.center().x.max(win.left()+90.0)} else {panel.center().x.min(win.right()-90.0)};
                Pos2::new(center-PW/2.0,panel.top())
            } else {origin};
            self.toast_layer(&Card::new(ui,toast_origin,win),h+8.0)
        }else{None};
        if (self.todo.adding||self.todo.editing.is_some()) && reveal > 0.9 && self.nav.page == TODO_PAGE && self.page_e() > 0.25 {
            self.input_field(ui, origin, &mut cmds);
        }

        if self.todo.task_dialog.is_some() && reveal>0.9 {self.task_editor_inputs(ui,origin,&mut cmds);}

        // ---------- 输出 ----------
        let rel = |r: Rect| r.translate(-win.min.to_vec2());
        let mut region = if side {
            // 侧边：面板从边缘滑出，区域包含肩部（沿边缘上下）与外侧阴影，并一直延伸到屏幕边缘。
            let r=if p > 0.02 || self.morph.open {
                let height=self.expanded_height().max(self.morph.h.pos)*1.08+FLARE*2.0+24.0;
                self.island_rect(win,PW+20.0,height).union(panel.expand(20.0))
            } else {
                let (bw,bh)=self.base_size();
                self.island_rect(win,bw*SWELL.1+14.0,bh*SWELL.0+FLARE+16.0).union(panel.expand(12.0))
            };
            if dock==model::Dock::Left {Rect::from_min_max(Pos2::new(win.left(),r.top()),r.max)} else {Rect::from_min_max(r.min,Pos2::new(win.right(),r.bottom()))}
        } else if p > 0.02 || self.morph.open {
            let height=self.expanded_height().max(self.morph.h.pos)*1.08+16.0;
            Rect::from_center_size(Pos2::new(cx, top + height/2.0), Vec2::new(PW + FLARE * 2.0 + 56.0, height))
        } else {
            let (bw, bh) = self.base_size();
            Rect::from_center_size(Pos2::new(cx, top + (bh * SWELL.1 + 12.0) / 2.0), Vec2::new(bw * SWELL.0 + 24.0, bh * SWELL.1 + 12.0))
        };
        if let Some(rect)=notice_shape {region=region.union(rect.expand2(Vec2::new(12.0,8.0)));}
        if let Some(rect)=notice_details {region=region.union(rect.expand(12.0));}
        // 胶囊上方留空的那一小条也要在窗口区域内，阴影才不会被截断。
        if pill && !side {region.min.y=region.min.y.min(win.top());}
        // 浅色皮肤：把这一帧坞体内容统一换成皮肤配色。
        if let Some(palette)=palette {
            let end=crate::skin_paint::shape_count(&ctx,layer);
            let mut ranges=Vec::new();let mut at=content_from;
            keep_colors.sort();
            for (from,to) in keep_colors {if from>at {ranges.push((at,from));} at=at.max(to);}
            if end>at {ranges.push((at,end));}
            crate::skin_paint::recolor_layer(&ctx,layer,&ranges,&palette);
        }
        // 原生裁剪还需包含悬停提示；只扩大绘制区域，保持桌面点击穿透。
        let mut tooltips=Vec::new();
        ctx.memory(|memory|{
            for layer in memory.areas().visible_layer_ids() {
                if layer.order==egui::Order::Tooltip {
                    if let Some(rect)=memory.area_rect(layer.id){region=region.union(rect.expand(12.0));tooltips.push(rect);}
                }
            }
        });
        // 毛玻璃：窗口区域只取坞体与浮层的真实轮廓，系统模糊不会溢出到透明画布上。
        let glass_region=(glass_skin && self.settings.glass_blur || env.opaque_surface).then(||{
            let outline=|r:Rect,flare:f32,flare_h:f32,radius:f32| if pill {GlassShape::Round(rel(r),radius.min(r.height().min(r.width())*0.5))}
                else if side {GlassShape::Docked{rect:rel(r),dock,flare,flare_h,radius}} else {GlassShape::Notch{rect:rel(r),flare,flare_h,radius}};
            let mut shapes=vec![outline(body,flare,flare_h,radius)];
            if let Some(r)=notice_shape {shapes.push(outline(r,5.0,8.0,12.5));}
            if let Some(r)=notice_details {
                let radius=10.0;
                shapes.push(GlassShape::Round(rel(r),radius));
            }
            if let Some(r)=floating_toast {shapes.push(GlassShape::Round(rel(r),11.0));}
            shapes.extend(tooltips.iter().map(|r|GlassShape::Round(rel(*r),6.0)));
            shapes
        });
        let transitioning = !self.morph.w.settled(PW)
            || !self.morph.h.settled(self.expanded_height())
            || self.page_e() < 0.7
            || self.nav.dots.iter().any(|d| !d.settled(16.0))
            || self.morph.hover_since.is_some() && !self.morph.open
            || self.morph.leave_since.is_some()
            || (self.drag_a > 0.001 && self.drag_a < 0.999)
            || self.drag_hot
            || (self.pet_a > 0.001 && self.pet_a < 0.999)
            || !self.resident.settled()
            || self.morph.open && self.nav.page==2 && self.shelf_view.shelf_x.values().any(|s| !s.settled(68.0))
            || self.toast.is_some()
            || self.t < self.notices.notification_bell_until && self.t-self.notices.notification_bell_at < 0.95
            || self.notices.notice.as_ref().is_some_and(|a|a.source!=crate::extras::AlertSource::Internal&&self.t-a.at<0.95)
            || self.hud_alpha > 0.003 && self.hud_alpha < 0.997
            || !self.notice_width.settled(1.0);
        let pet_visible=self.pet_a>0.01 && !self.morph.open && !minimal_pill;
        let ambient=self.settings.skin==model::Skin::Fluid && self.fluid.animating() || self.pomo.running && (self.morph.open && self.nav.page==1 || !self.morph.open && !minimal_pill && self.notices.notice.is_none() && bar::animations_enabled())
            || self.media.playing && (self.morph.open && self.nav.page==0 || !self.morph.open && !minimal_pill && self.notices.notice.is_none() && self.media.is_music());
        let animating=transitioning || ambient || pet_visible && self.pet.state!=PetState::Sleeping;
        // 交互形变保持 60 帧，媒体 30 帧；小表情 20 帧，睡眠 10 帧，静态界面每秒核对提醒。
        let repaint_after=std::time::Duration::from_secs_f64(if transitioning{1.0/60.0}else if ambient{1.0/30.0}else if pet_visible{if self.pet.state==PetState::Sleeping{0.1}else{0.05}}else{1.0});
        cmds.append(&mut self.pending);
        FrameOut {
            region: rel(floating_toast.into_iter().chain(self.shelf_view.shelf_menu_rect.map(|r|r.union(r + ctx.style().visuals.popup_shadow.margin()).expand(1.0 / ctx.pixels_per_point()))).fold(region,Rect::union).intersect(ui.max_rect())),
            glass: glass_region,
            hit: rel(if side {self.side_hit_rect(win,self.progress())} else {self.hit_rect(cx, top, self.progress())}),
            notice_hit:notice_shape.into_iter().chain(notice_details).chain(self.shelf_view.shelf_menu_rect).reduce(|a,b|a.union(b)).map(rel),
            file_drop_zone:self.settings.visible_pages[2].then(||rel(if side {self.island_rect(win,PW,self.expanded_height())} else {Rect::from_min_size(Pos2::new(cx-PW/2.0,top),Vec2::new(PW,96.0))})),
            expanded: self.morph.open,
            editing: self.is_editing(),
            animating,
            repaint_after,
            cmds,
        }
    }

    fn hit_rect(&self, cx: f32, top: f32, p: f32) -> Rect {
        let r=self.hit_rect_base(cx,top,p);
        // 胶囊离屏幕顶边有一段空隙：把光标推到屏幕最顶端也要能唤出坞。
        if self.settings.island_shape==model::IslandShape::Pill {Rect::from_min_max(Pos2::new(r.left(),r.top()-PILL_GAP),r.max)} else {r}
    }

    fn hit_rect_base(&self, cx: f32, top: f32, p: f32) -> Rect {
        if self.effective_minimal() && !self.morph.open {
            let (width,height)=self.base_size();
            let width=self.morph.w.pos.min(width);
            let height=self.morph.h.pos.min(height);
            Rect::from_min_size(Pos2::new(cx-width/2.0,top),Vec2::new(width,height))
        } else if self.notices.notice.is_some()&&!self.compact_notices()&&!self.morph.open {
            let (w,h)=self.base_size();Rect::from_min_size(Pos2::new(cx-(w+8.0)/2.0,top),Vec2::new(w+8.0,h+6.0))
        } else if p > 0.04 || self.morph.open {
            Rect::from_min_size(Pos2::new(cx - PW / 2.0 - 4.0, top), Vec2::new(PW + 8.0, self.expanded_height() + 2.0))
        } else {
            let (bw,bh)=self.base_size();
            let (hit_w,hit_h)=(bw+12.0,bh+4.0);
            Rect::from_min_size(Pos2::new(cx-hit_w/2.0,top),Vec2::new(hit_w,hit_h))
        }
    }

    fn notice_rect(&self,cx:f32,top:f32)->Option<Rect>{
        if !self.compact_notices(){return None;}
        let width=self.notice_width.pos-8.0;
        (width>0.5).then(||Rect::from_min_size(Pos2::new(cx+self.morph.w.pos/2.0+8.0,top),Vec2::new(width,BAR_H)))
    }
    fn notice_details_rect(&self,cx:f32,top:f32)->Option<Rect>{
        let a=self.notices.hud_notice.as_ref().filter(|_|self.notice_details&&self.notices.notice.is_some())?;
        let badge=self.notice_rect(cx,top)?;let (width,height)=self.notice_size(a);
        let mut x=badge.left();
        // 坞体靠近屏幕右缘时，详情卡片向左展开，始终完整留在屏幕内。
        if let Some(screen)=self.screen {x=x.min(screen.right()-width-8.0).max(screen.left()+8.0);}
        Some(Rect::from_min_size(Pos2::new(x,badge.bottom()+6.0),Vec2::new(width,height)))
    }

    /// 当前皮肤的换色调色板（经典为 None）。
    pub(crate) fn skin_palette(&self)->Option<crate::skin_paint::Palette> {
        match self.settings.skin {model::Skin::Cream=>Some(crate::skin_paint::CREAM),model::Skin::GlassLight=>Some(crate::skin_paint::GLASS_LIGHT),model::Skin::GlassDark=>Some(crate::skin_paint::GLASS_DARK),model::Skin::Classic|model::Skin::Fluid=>None}
    }

    /// 设置页预览用的坞体底色：没有真实模糊，毛玻璃按较实的玻璃表现。
    pub(crate) fn preview_fill(&self)->Color32 {
        match self.settings.skin {
            model::Skin::GlassDark=>{let (r,g,b)=glass::GROUND_DARK;Color32::from_rgba_unmultiplied(r,g,b,215)}
            model::Skin::GlassLight=>{let [r,g,b]=glass::LIGHT_GROUND.map(|v|(v*255.0).round() as u8);Color32::from_rgba_unmultiplied(r,g,b,225)}
            _=>self.glass_fill(),
        }
    }

    /// 坞体底色：经典为纯黑；奶油为暖白；毛玻璃按当前着色量叠加深色或浅色玻璃。
    fn glass_fill(&self)->Color32 {
        let alpha=(self.glass_tint.clamp(0.0,1.0)*255.0).round() as u8;
        match self.settings.skin {
            model::Skin::GlassDark=>{let (r,g,b)=glass::GROUND_DARK;Color32::from_rgba_unmultiplied(r,g,b,alpha)}
            model::Skin::GlassLight=>{let [r,g,b]=glass::LIGHT_GROUND.map(|v|(v*255.0).round() as u8);Color32::from_rgba_unmultiplied(r,g,b,alpha)}
            model::Skin::Cream=>{let [r,g,b]=crate::skin_paint::CREAM.paper;Color32::from_rgb(r,g,b)}
            model::Skin::Classic=>Color32::BLACK,
            model::Skin::Fluid=>self.fluid.compose(0.5,0.5,1.0,1.0,0.0),
        }
    }

    /// 毛玻璃边缘高光：浅色玻璃更亮，像苹果的玻璃边缘。
    fn glass_rim(&self)->Color32 {
        let strength=if self.settings.skin==model::Skin::GlassLight {0.55} else {glass::rim_alpha()*0.7};
        Color32::from_white_alpha((255.0*strength).round() as u8)
    }

    /// 绘制坞体（或通知胶囊）的本体：经典皮肤为纯黑；毛玻璃为着色底 + 顶部柔光 + 边缘高光。
    fn paint_body(&self,painter:&egui::Painter,rect:Rect,flare:f32,flare_h:f32,radius:f32,feather:f32){
        let skin=self.settings.skin;
        if skin==model::Skin::Fluid {
            let pill=self.settings.island_shape==model::IslandShape::Pill;
            let (map,frame)=shape::EdgeMap::of(rect,if pill {model::Dock::Top}else{self.dock()});
            let mut rows=shape::sections(frame,flare,flare_h,radius);
            if pill {
                let mid=frame.center().y;
                rows.retain(|row|row.0>mid && row.0>=frame.bottom()-radius);
                let top=rows.iter().rev().map(|&(y,l,r)|(2.0*mid-y,l,r)).collect::<Vec<_>>();
                rows.splice(0..0,top);
            }
            let expand=self.progress().clamp(0.0,1.0);
            let mut mesh=shape::shade_mesh(frame,rows,feather,&|x,y|{
                let p=map.apply(Pos2::new(x,y))-rect.min;
                self.fluid.compose(p.x,p.y,rect.width(),rect.height(),expand)
            });
            if self.dock().is_side() && !pill {
                for vertex in &mut mesh.vertices {vertex.pos=map.apply(vertex.pos);}
            }
            painter.add(egui::Shape::mesh(mesh));
            return;
        }
        if self.settings.island_shape==model::IslandShape::Pill {
            let radius=radius.min(rect.height().min(rect.width())*0.5);
            painter.rect_filled(rect,radius,self.glass_fill());
            if skin.is_glass() && rect.height()>=10.0 {
                painter.rect_stroke(rect,radius,egui::Stroke::new(1.0_f32,self.glass_rim()),egui::StrokeKind::Inside);
            }
            return;
        }
        let dock=self.dock();
        if dock.is_side() {
            shape::fill_docked(painter,rect,dock,flare,flare_h,radius,self.glass_fill(),feather);
            if !skin.is_glass() {return;}
            if skin==model::Skin::GlassLight {
                let sheen=Color32::from_white_alpha(15);
                shape::fill_gradient_docked(painter,rect,dock,flare,flare_h,radius,sheen,Color32::TRANSPARENT,(rect.width()*0.6).min(64.0),feather);
            }
            if rect.height()>=20.0 && rect.width()>=10.0 {
                let edge=if dock==model::Dock::Left {Rect::from_min_max(rect.min+Vec2::new(0.0,0.5),rect.max-Vec2::new(0.5,0.5))}
                    else {Rect::from_min_max(rect.min+Vec2::new(0.5,0.5),rect.max-Vec2::new(0.0,0.5))};
                painter.add(egui::Shape::line(shape::outline_docked(edge,dock,flare,flare_h,(radius-0.5).max(0.0)),egui::Stroke::new(1.0_f32,self.glass_rim())));
            }
            return;
        }
        shape::fill(painter,rect,flare,flare_h,radius,self.glass_fill(),feather);
        if !skin.is_glass() {return;}
        // 玻璃厚度带来的上亮下暗：顶部白光在前 60% 高度（最多 64 点）内淡出；浅色玻璃更亮。
        if skin==model::Skin::GlassLight {
            let sheen=Color32::from_white_alpha(15);
            shape::fill_gradient(painter,rect,flare,flare_h,radius,sheen,Color32::TRANSPARENT,(rect.height()*0.6).min(64.0),feather);
        }
        if rect.height()>=10.0 && rect.width()>=20.0 {
            let edge=Rect::from_min_max(rect.min+Vec2::new(0.5,0.0),rect.max-Vec2::new(0.5,0.5));
            painter.add(egui::Shape::line(shape::outline(edge,flare,flare_h,(radius-0.5).max(0.0)),egui::Stroke::new(1.0_f32,self.glass_rim())));
        }
    }

    /// 悬停状态机：意图缓冲 → 弹簧展开 → 离开延迟 → 临界阻尼收起；全程可打断。
    /// 形变的纯状态机收敛在 [`crate::morph::MorphController`]；这里只负责喂入
    /// 由 `base_size` / `side` / `effective_minimal` 等多领域决策算出的只读上下文，
    /// 并消化收起时的连带副作用（取消编辑、清空置物架确认态）。
    /// 切页弹簧（`dots` / `slide`）的推进仍留在 `App`，属于「切页」领域（见 PageNavigator）。
    fn morph(&mut self, want: bool, _inside: bool, t: f64, dt: f32) {
        let inputs = crate::morph::MorphInputs {
            base: self.base_size(),
            side: self.side(),
            drag_hot: self.drag_hot,
            minimal: self.effective_minimal(),
            expanded_height: self.expanded_height(),
        };
        if matches!(self.morph.tick(want, &inputs, t, dt), crate::morph::MorphAction::Closed) {
            self.cancel_edit();
            self.shelf_view.confirm_clear = None;
        }
        // 切页弹簧（滑动 + 圆点）由 PageNavigator 自己推进。
        self.nav.tick(dt);
    }

    // ---------------------------------------------------------------
    // 顶栏：电池 + 日期 + 时钟。药丸里的电池与时钟会「飞」到顶栏位置（共享元素过渡）。
    // ---------------------------------------------------------------
    fn shared_header(&mut self, base: &Card, cx: f32, p: f32, reveal: f32, env: &Env) {
        let local_cx = cx - base.origin.x;
        let fit=((WIN_W-32.0)/self.resident.target.width).min(1.0);
        let fitted=base.scaled(local_cx,15.0,lerp(fit,1.0,p));
        let base=&fitted;
        let lyric=self.island_lyric().map(|(index,_)|index);
        let lyric_alpha=if self.island_lyrics_active() {1.0-smoothstep(0.0,0.35,p)}else{0.0};
        if let Some(galley)=self.island_lyric_layout.clone().filter(|_|lyric_alpha>0.003) {
            let c=base.faded(lyric_alpha);
            let content_width=self.lyric_bar_width()-self.lyric_focus_space();
            let width=self.morph.w.pos.max(content_width);
            let left=local_cx-width/2.0;
            let center_y=BAR_H/2.0;
            if let Some((image,texture))=&self.music_artwork {
                let [width,height]=image.size;
                let uv_size=if width>height {egui::vec2(height as f32/width as f32,1.0)}else{egui::vec2(1.0,width as f32/height as f32)};
                let uv=Rect::from_center_size(Pos2::new(0.5,0.5),uv_size);
                c.painter.add(egui::epaint::RectShape::filled(c.rect(left+12.0,center_y-10.0,20.0,20.0),5.0*c.s,Color32::WHITE).with_texture(texture.id(),uv));
            } else {
                c.fill(left+12.0,center_y-10.0,20.0,20.0,5.0,theme::CARD);
                paint::note(&c,left+22.0,center_y,13.0,theme::WHITE);
            }
            let index=lyric.unwrap_or(0);
            let id=egui::Id::new("island-lyric-line");
            if !self.ly.island_animation_started {
                c.ui.ctx().animate_value_with_time(id,index as f32,0.0);
                self.ly.island_animation_started=true;
            }
            let shown=c.ui.ctx().animate_value_with_time(id,index as f32,0.18);
            let fade=1.0-(index as f32-shown).abs().min(1.0);
            let position=c.pos(left+content_width-LYRIC_BAR_W/2.0,center_y-galley.size().y/2.0+(1.0-fade)*3.0);
            c.painter.galley(position,galley,if lyric.is_some(){theme::WHITE}else{theme::GRAY});
            paint::playback_bars(&c,left+content_width-21.0,center_y,self.t,self.media.playing,theme::WHITE);
            if self.pomo.running {
                let alpha=smoothstep(0.88,1.0,(width-content_width)/self.lyric_focus_space());
                crate::focus_bar::draw(&c.faded(alpha),left+width-14.0-crate::focus_bar::LYRIC_WIDTH/2.0,center_y,self.t,&self.pomo,self.phase_secs(self.pomo.phase),true);
            }
        }
        if lyric_alpha>=0.997 {return;}
        let header=base.faded(1.0-lyric_alpha);
        let base=&header;
        let m = anim::ease_in_out(smoothstep(0.0, 0.85, p));
        let clock = self.settings.clock_text(&env.clock);
        let show_battery=self.settings.show_battery && env.battery.is_some();

        // 顶栏终点布局：[电池][日期][时钟][状态] 作为一组整体居中
        let date_w = base.text_w(&env.date, size::DATE, W::Regular);
        let clock_w = paint::clock_width(base,&clock,size::CLOCK);
        let battery_space=if show_battery {24.0}else{0.0};
        let sysmon_w = if self.settings.show_sysmon {36.0}else{0.0};
        let group = battery_space + date_w + 8.0 + clock_w + if self.settings.show_sysmon {8.0 + sysmon_w}else{0.0};
        let gx = local_cx - group / 2.0 + if show_battery {6.0}else{0.0};
        let hb = (gx + 8.5, 18.6);
        let hd = gx + battery_space;
        let hc = hd + date_w + 8.0;
        let hs_left = hc + clock_w + 8.0;
        let hs_center = hs_left + sysmon_w / 2.0;
        // 药丸起点布局
        let has_music=self.media.is_music();
        let layout=self.resident.current();
        let pb=(local_cx+layout.x[bar::BATTERY].unwrap_or(0.0),15.0);
        let pcx=local_cx+layout.x[bar::CLOCK].unwrap()-paint::clock_width(base,&clock,size::PILL_CLOCK)/2.0;

        if self.effective_minimal() && self.t>=self.notices.notification_bell_until && p < 0.02 { return; }
        // 电池：位置/大小连续插值
        let bk = lerp(1.0, 0.8, m);
        let bpos = (lerp(pb.0, hb.0, m), lerp(pb.1, hb.1, m));
        if show_battery {paint::battery(&base.faded(lerp(layout.clearance(bar::BATTERY),1.0,m)), bpos.0, bpos.1, env.battery, env.charging, bk);}

        // 时钟：两个字号沿同一路径交叉淡化，避免逐帧换字号
        let cpos = (lerp(pcx, hc, m), lerp(21.0, 23.0, m));
        let a_pill = (1.0 - smoothstep(0.25, 0.6, m)) * layout.clearance(bar::CLOCK);
        let a_head = smoothstep(0.35, 0.75, m);
        if a_pill > 0.0 {
            paint::clock(&base.faded(a_pill),cpos.0,cpos.1,&clock,size::PILL_CLOCK);
        }
        if a_head > 0.0 {
            paint::clock(&base.faded(a_head),cpos.0,cpos.1-0.5,&clock,size::CLOCK);
        }
        // 日期随内容淡入，从药丸方向轻轻滑出
        if reveal > 0.0 {
            base.faded(reveal).text(hd + (1.0 - reveal) * 10.0, 22.5, &env.date, size::DATE, W::Regular, Color32::from_rgb(210, 210, 214), H::Left);
        }
        // 系统状态：CPU / 内存双圆环，紧邻时钟（药丸里在时钟与音乐之间）。
        if self.settings.show_sysmon {
            if let Some(slot_x) = layout.x[bar::SYSMON] {
                let sx = lerp(local_cx + slot_x, hs_center, m);
                let sy = lerp(15.0, 18.0, m);
                let a = lerp(layout.clearance(bar::SYSMON), 1.0, m).clamp(0.0, 1.0);
                self.draw_sysmon(&base.faded(a), sx, sy);
                base.hit(sx - 17.0, sy - 9.0, 34.0, 18.0, "sysmon")
                    .on_hover_ui(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(80, 200, 230), format!("CPU {:.0}%", self.cpu_shown));
                            ui.colored_label(Color32::from_rgb(235, 120, 200), format!("MEM {:.0}%", self.mem_shown));
                        });
                    });
            }
        }
        if has_music && (p>0.1 || layout.x[bar::MUSIC].is_some()) {
            paint::playback_bars(base,lerp(local_cx+layout.x[bar::MUSIC].unwrap_or(0.0),367.0,m),lerp(15.0,18.0,m),self.t,self.media.playing,theme::WHITE);
        }
        let compact_alpha=1.0-smoothstep(0.0,0.22,p);
        if self.t<self.notices.notification_bell_until&&!self.compact_notices()&&p<0.1 {
            if let Some(x)=layout.x[bar::BELL]{
                let x=local_cx+x;paint::bell(&base.scaled(x,14.0,1.2),x,14.0,theme::WHITE,paint::bell_swing(self.t-self.notices.notification_bell_at));
                base.circle(x+4.0,9.5,2.0,Color32::from_rgb(255,72,72));
                if base.hit(x-14.0,3.0,28.0,25.0,"bell").clicked(){self.show_page(TODO_PAGE);}
            }
        }
        if self.pomo.running && compact_alpha>0.004 {
            crate::focus_bar::draw(&base.faded(compact_alpha),local_cx+layout.x[bar::FOCUS].unwrap(),15.0,self.t,&self.pomo,self.phase_secs(self.pomo.phase),false);
        }
    }

    /// 常驻内容：顶部停靠走共享元素过渡（药丸里的电池、时钟飞到顶栏）；
    /// 左右停靠时竖排内容随展开淡出，顶栏随面板内容淡入。
    /// 系统状态：CPU / 内存双圆环，圆心放两位数字。
    /// 底环用类型色区分（CPU 青 / MEM 粉）；进度弧正常用类型色，偏高/危险时切告警色（琥珀→红）。
    ///
    /// `k` 是圆环自身的本地缩放系数，只用来让半径、线宽、间距、字号整体联动，
    /// 便于单独微调圆环相对其他组件的比例。坞体大小（80%–140%）不在这里处理：
    /// 它由 `eframe::set_root_viewport_scale` 在视口层统一放大整个画布，绘制阶段的
    /// 坐标一律是逻辑单位。因此这里**不要**乘 `Settings::island_scale()`，
    /// 否则圆环会被缩放两次。
    fn draw_sysmon(&self, c: &Card, cx: f32, cy: f32) {
        let k = 1.0;
        let r = 8.2 * k;
        let w = 2.6 * k;
        let off = 11.0 * k;
        // (圆心 x 偏移, 显示值, 类型)：0 = CPU（青），1 = MEM（粉）
        let items = [(-off, self.cpu_shown, 0usize), (off, self.mem_shown, 1usize)];
        for &(dx, val, kind) in &items {
            let frac = ((val / 100.0).clamp(0.0, 1.0)).max(0.02);
            let col = Self::sysmon_arc(kind, val);
            // 底环（轨道）：类型色半透明，与电量圆环一致的视觉语言
            c.ring(cx + dx, cy, r, w, crate::paint::alpha(col, 0.22));
            let a0 = -std::f32::consts::FRAC_PI_2;
            // 进度弧：从正上方顺时针；满进度画完整环，否则画弧
            if frac >= 0.999 {
                c.ring(cx + dx, cy, r, w, col);
            } else {
                c.arc(cx + dx, cy, r, a0, a0 + std::f32::consts::TAU * frac, w, col);
            }
            c.centered_text(cx + dx, cy, &format!("{:.0}", val.min(100.0)), 6.6 * k, W::Bold, col, H::Center);
        }
    }

    /// 类型色：0 = CPU（青），1 = MEM（粉），用于一眼区分两个圆环。
    fn sysmon_type(kind: usize) -> Color32 {
        if kind == 0 { Color32::from_rgb(80, 200, 230) }
        else { Color32::from_rgb(235, 120, 200) }
    }

    /// 进度弧颜色：正常负载用类型色（青/粉），偏高（≥60%）琥珀，危险（≥85%）红。
    fn sysmon_arc(kind: usize, v: f32) -> Color32 {
        if v >= 85.0 { Color32::from_rgb(255, 69, 58) }
        else if v >= 60.0 { Color32::from_rgb(255, 159, 10) }
        else { Self::sysmon_type(kind) }
    }

    fn resident_header(&mut self, base: &Card, panel: Rect, cx: f32, p: f32, reveal: f32, env: &Env) {
        if !self.side() {self.shared_header(base,cx,p,reveal,env);return;}
        let bar_alpha=1.0-smoothstep(0.0,0.3,p);
        let minimal=self.effective_minimal()&&self.t>=self.notices.notification_bell_until;
        if bar_alpha>0.004 && !minimal {
            let c=base.shifted(panel.min.x-base.origin.x,panel.min.y-base.origin.y).faded(bar_alpha);
            let c=if bar_alpha<0.8 {c.ghost()} else {c};
            self.side_bar(&c,panel.width(),panel.height(),env);
        }
        if reveal>0.004 {self.shared_header(&base.faded(reveal),base.origin.x+PW/2.0,1.0,reveal,env);}
    }

    fn update_pet(&mut self, env: &Env, cx: f32) {
        let cx=cx+self.resident.position(bar::PET);
        let idle = self.idle_for();
        if self.pet.last_idle >= self.opts.pet_idle && idle < 1.0
            && matches!(self.pet.state, PetState::Dozing | PetState::Sleeping) {
            self.pet.waking_until = self.t + 2.2;
        }
        self.pet.last_idle = idle;
        self.pet.hovered = env.cursor.is_some_and(|p| (p.x-cx).abs() <= BAR_W/2.0+24.0 && (0.0..70.0).contains(&p.y));
        self.pet.worried = self.settings.show_battery && env.battery.is_some_and(|v|v<=20) && !env.charging || env.network == Some(false);
        let allowed = self.pet_enabled();
        if !allowed { self.pet.reaction = None; self.pet.waking_until = 0.0; }
        let available = allowed && !self.morph.open && !self.drag_hot;
        let shown = available;
        self.pet_a = anim::approach(self.pet_a, if shown {1.0}else{0.0}, 9.0, self.dt);
        let visible = available && self.pet_a > 0.7 && self.progress() < 0.1;
        self.pet.advance(self.dt, visible);
        let state = self.pet_state();
        let gaze = env.cursor.map(|p| [((p.x-cx)/28.0).clamp(-3.0,3.0), ((p.y-15.0)/40.0).clamp(-1.0,1.5)])
            .unwrap_or([(self.t as f32 * 0.35).sin()*0.8, 0.0]);
        self.pet.step(state, self.t, self.dt, gaze, visible);
    }

    // ---------------------------------------------------------------
    // 面板主体：页标题 + 页面（带切页过渡）+ 翻页箭头 + 页码豆
    // ---------------------------------------------------------------
    fn panel_body(&mut self, c: &Card, env: &Env, cmds: &mut Vec<Cmd>) {
        let settings=c.hit(380.0,4.0,28.0,28.0,"page-settings").on_hover_text(format!("打开{}设置窗口",PAGE_NAMES[self.nav.page]));
        let (hover,press)=c.hover_t(&settings);
        c.circle(394.0,18.0,13.0,Color32::from_white_alpha((hover*24.0) as u8));
        paint::gear(&c.scaled(394.0,18.0,0.8*(1.0-press*0.08)),394.0,18.0,paint::mix(theme::GRAY,theme::WHITE,hover));
        if settings.clicked(){self.settings_tab=[1,2,4,3][self.nav.page];self.open_settings();}
        if self.subpage==Some(self.nav.page) {
            c.text(15.0,25.0,"‹",19.0,W::Bold,theme::WHITE,H::Left);
            c.text(37.0,25.0,"专注设置",size::TITLE,W::Bold,theme::WHITE,H::Left);
            if c.hit(9.0,7.0,28.0,23.0,"subpage-back").clicked(){self.subpage=None;}
            self.page_settings(c);self.nav(c);return;
        }
        let e = self.page_e();
        let in_tx = e < 0.55 && self.nav.page_from != self.nav.page;
        if in_tx {
            // 旧页：原地向反方向轻移并快速淡出；新页：弹簧滑入 + 稍晚淡入
            let old_a = 1.0 - smoothstep(0.0, 0.16, e);
            let new_a = smoothstep(0.03, 0.28, e);
            let old_dx = -self.nav.page_dir * 18.0 * anim::ease_out_cubic(e / 0.3);
            if old_a > 0.0 {
                let oc = c.faded(old_a).ghost();
                self.page_title(self.nav.page_from, &oc);
                self.render_page(self.nav.page_from, &oc.shifted(old_dx, 0.0), env, &mut Vec::new());
            }
            let nc = c.faded(new_a);
            self.page_title(self.nav.page, &nc);
            let nc = nc.shifted(self.nav.slide.pos, 0.0);
            let nc = if e < 0.12 { nc.ghost() } else { nc };
            self.render_page(self.nav.page, &nc, env, cmds);
        } else {
            self.page_title(self.nav.page, c);
            self.render_page(self.nav.page, c, env, cmds);
        }
        self.nav(c);
    }

    fn page_title(&self, page: usize, c: &Card) {
        let base = 25.0;
        let (a, b, col) = match page {
            0 => return,
            1 => ("番茄", "专注", theme::RED_TOP),
            // 置物架的标题就是「置物架 | 剪贴板」分栏，由页面自己画。
            2 => return,
            3 => ("待办", "清单", theme::GREEN),
            _ => ("胶囊坞", "设置", theme::WHITE),
        };
        let wa = c.text(15.0, base, a, size::TITLE, W::Bold, theme::WHITE, H::Left);
        if !b.is_empty() {
            c.text(15.0 + wa + 1.5, base, b, size::TITLE, W::Bold, col, H::Left);
        }
    }

    fn nav(&mut self, c: &Card) {
        let pages: Vec<_>=self.settings.page_order.iter().copied().filter(|page|self.settings.visible_pages[*page]).collect();
        if pages.len()<2 { return; }
        let l = c.hit(8.0, 55.0, 26.0, 26.0, "nav-l");
        let r = c.hit(385.0, 55.0, 26.0, 26.0, "nav-r");
        let (lh, lp) = c.hover_t(&l);
        let (rh, rp) = c.hover_t(&r);
        paint::chevron(c, 21.0, 68.0, -1.0, lh, lp);
        paint::chevron(c, 398.0, 68.0, 1.0, rh, rp);
        if l.clicked() { self.navigate_page(false); }
        if r.clicked() { self.navigate_page(true); }
        // 页码豆：宽度由弹簧驱动，整行实时居中；当前页是白色长条
        let gap = 4.0;
        let dot_y=self.expanded_height()-7.2;
        let total: f32 = pages.iter().map(|page| self.nav.dots[*page].pos).sum::<f32>() + gap * (pages.len()-1) as f32;
        let mut x = PW / 2.0 - total / 2.0;
        let mut clicked = None;
        for i in pages {
            let w = self.nav.dots[i].pos.max(3.0);
            let on = anim::remap(w, 4.0, 16.0);
            let col = paint::mix(Color32::from_rgb(92, 92, 96), theme::WHITE, on);
            c.fill(x, dot_y - 2.0, w, 4.0, 2.0, col);
            let hr = c.hit(x - 2.0, dot_y - 6.0, w + 4.0, 12.0, ("dot", i));
            hr.clone().on_hover_text(PAGE_NAMES[i]);
            if hr.hovered() && i != self.nav.page {
                c.fill(x - 1.0, dot_y - 3.0, w + 2.0, 6.0, 3.0, Color32::from_white_alpha(28));
            }
            if hr.clicked() {
                clicked = Some(i);
            }
            x += w + gap;
        }
        if let Some(i) = clicked {
            let position=|page|self.settings.page_order.iter().position(|p|*p==page).unwrap();
            let dir = if position(i) > position(self.nav.page) { 1.0 } else { -1.0 };
            self.go(i, dir);
        }
    }

    fn render_page(&mut self, page: usize, c: &Card, env: &Env, cmds: &mut Vec<Cmd>) {
        match page {
            0 => self.page_music(c, env, cmds),
            1 => self.page_focus(c, env),
            2 => self.page_shelf(c, env, cmds),
            3 => self.page_todo(c, env, cmds),
            _ => self.page_settings(c),
        }
    }

    fn toast_layer(&mut self, c: &Card, base_y:f32)->Option<Rect> {
        let Some((msg, at)) = self.toast.clone() else { return None };
        let e = (self.t - at) as f32;
        if e > 2.4 {
            self.toast = None;
            return None;
        }
        let a = smoothstep(0.0, 0.16, e) * (1.0 - smoothstep(2.1, 2.4, e));
        let rise = (1.0 - anim::ease_out_cubic(e / 0.3)) * 6.0;
        let msg=c.fit(&msg,size::BODY,W::Regular,310.0);
        let w = c.text_w(&msg, size::BODY, W::Regular) + 26.0;
        let tc = c.faded(a);
        let y = base_y + rise;
        tc.fill(PW / 2.0 - w / 2.0, y, w, 22.0, 11.0, Color32::from_rgb(44, 44, 48));
        tc.text(PW / 2.0, y + 15.0, &msg, size::BODY, W::Regular, theme::WHITE, H::Center);
        Some(tc.rect(PW/2.0-w/2.0,y,w,22.0))
    }

    // ---------------------------------------------------------------
    // 每帧推进：番茄钟与跨天
    // ---------------------------------------------------------------
    fn tick(&mut self, env: &Env, dt: f32, elapsed: f32) {
        if self.toast.as_ref().is_some_and(|(_,at)|self.t-at>2.4){self.toast=None;}
        // 本机状态：每 2 秒采样一次 CPU / 内存（窗口更长，读数更稳），缓动量让圆环平滑跟上。
        self.sysmon.sample_if_ready(std::time::Duration::from_secs(2));
        self.cpu_shown = anim::approach(self.cpu_shown, self.sysmon.cpu, 4.0, dt);
        self.mem_shown = anim::approach(self.mem_shown, self.sysmon.mem, 4.0, dt);
        if env.today != self.today {
            self.today = env.today.clone();
        }
        if self.pomo.running {
            self.pomo.remaining -= elapsed;
            if self.pomo.remaining <= 0.0 {
                let next = match self.pomo.phase {
                    Phase::Focus => {
                        self.pet.react(PetState::Happy);
                        Phase::Rest
                    }
                    Phase::Rest => Phase::Focus,
                };
                self.pomo.phase = next;
                self.pomo.remaining = self.phase_secs(next);
                self.pomo.running = false;
                self.alert("专注计时", format!("该{}了", next.label()), 1, true);
            }
        }
        self.ring_shown.target = (self.pomo.remaining / self.phase_secs(self.pomo.phase)).clamp(0.0, 1.0);
        self.ring_shown.step(dt);
        self.reset_spin.step(dt);
        self.todo.tick(dt);

        let now = format!("{} {}", env.today, env.clock.get(..5).unwrap_or("00:00"));
        let due: Vec<_> = self.todos.items.iter().enumerate().filter(|(_, item)| item.reminder_due(&now)).map(|(i,item)| (i,item.text.clone())).collect();
        for (i,text) in due { self.todos.items[i].mark_reminded(&now); self.todos.save(); self.alert("待办到时间了", text, TODO_PAGE, true); }
        // 剪贴板记录开关：关闭时同时清掉未固定的记录。
        if self.clipboard.is_enabled() != self.settings.clipboard_history {
            self.clipboard.set_enabled(self.settings.clipboard_history);
        }
        self.shelf_view.tick(dt, self.t);
    }

    /// 媒体进度在两次 SMTC 刷新之间按时间外推，进度条才会连续走动。
    fn update_media(&mut self, env: &Env, ctx: &egui::Context) {
        let m = &env.media;
        if !m.is_music() {
            self.media=NowPlaying::default();
            self.media_anchor=(String::new(),0.0,0.0,false);
            self.music_artwork=None;
            self.ly=LyricsState::default();
            return;
        }
        let a = &self.media_anchor;
        if a.0 != m.title || (a.1 - m.position).abs() > 0.01 || a.3 != m.playing {
            self.media_anchor = (m.title.clone(), m.position, env.t, m.playing);
        }
        let mut np = m.clone();
        if m.playing {
            let (_, pos, _, _) = &self.media_anchor;
            np.position = if let Some(sampled)=m.sampled_at {m.position+sampled.elapsed().as_secs_f64()}
                else {*pos};
            if m.duration>0.0 {np.position=np.position.min(m.duration);}
        }
        // 网易云的 song_id 只来自本机日志，SMTC 那一路没有；None 与 Some 之间来回不是换歌，不能每次都清空歌词重查。
        let other_song=matches!((self.media.song_id,np.song_id),(Some(old),Some(new)) if old!=new);
        if self.media.source != np.source || other_song { self.ly=LyricsState::default(); }
        self.media = np;
        if let Some(image)=&self.media.artwork {
            if !self.music_artwork.as_ref().is_some_and(|(old,_)|std::sync::Arc::ptr_eq(old,image)) {
                self.music_artwork=Some((image.clone(),ctx.load_texture("music-artwork",image.clone(),egui::TextureOptions::LINEAR)));
            }
        } else {self.music_artwork=None;}
    }

    fn poll_lyrics(&mut self) {
        if !self.media.is_music(){self.ly=LyricsState::default();return;}
        if !self.show_lyrics && !self.settings.island_lyrics {self.ly=LyricsState::default();return;}
        let np = &self.media;
        if np.title.is_empty(){self.ly=LyricsState::default();return;}
        let key = format!("{}|{}|{:.0}", np.title, np.artist, np.duration);
        let id_arrived=key==self.ly.key && !self.ly.loading && self.ly.song_id.is_none() && np.song_id.is_some()
            && !self.ly.lyrics.as_ref().is_some_and(|lyrics|lyrics.synced);
        if key != self.ly.key || id_arrived || self.ly.retry_at.is_some_and(|at|self.t>=at) {
            // 歌曲信息抖动或快速切歌不能叠加网络线程；旧请求完成后只查询当前歌曲。
            if self.ly_worker.as_ref().is_some_and(|worker|!worker.is_finished()){return;}
            let Some(lease)=crate::resources::LYRICS.acquire()else{return;};
            let native_only=key==self.ly.key && np.source.to_ascii_lowercase().contains("kugou")
                && self.ly.error==Some(lyrics::FetchError::NotFound);
            let cached=if key==self.ly.key && np.source.to_ascii_lowercase().contains("kugou"){self.ly.lyrics.take()}else{None};
            let attempts=if key==self.ly.key {self.ly.attempts.saturating_add(1)}else{0};
            self.ly = LyricsState { key, loading: cached.is_none() && !native_only, lyrics:cached.clone(),
                error:native_only.then_some(lyrics::FetchError::NotFound), song_id:np.song_id, attempts, ..Default::default() };
            let (tx, rx) = std::sync::mpsc::channel();
            self.ly.rx = Some(rx);
            let (source, title, artist, dur) = (np.source.clone(), np.title.clone(), np.artist.clone(), np.duration);
            let song_id=np.song_id;
            if source.to_ascii_lowercase().contains("kugou") || lyrics::is_soda(&source) {
                let (tx,rx)=std::sync::mpsc::sync_channel(1);self.ly.native_rx=Some(rx);
                let (source,title,artist)=(source.clone(),title.clone(),artist.clone());
                crate::resources::NATIVE_LYRICS.spawn(move ||{
                    // 客户端通常在开播几秒内写好歌词：先密后疏（0.5s 起，最长 5s 一次），最多等 90 秒。
                    // 汽水的缓存文件可达 32MB，修改时间和大小都没变时不重复整份读取，但每 15 秒仍完整核对一次。
                    let started=std::time::Instant::now();
                    let mut wait=std::time::Duration::from_millis(500);
                    let mut last:Option<((std::time::SystemTime,u64),std::time::Instant)>=None;
                    loop {
                        let print=lyrics::native_fingerprint(&source);
                        let unchanged=print.is_some_and(|print|last.is_some_and(|(seen,at)|seen==print && at.elapsed()<std::time::Duration::from_secs(15)));
                        let found=if unchanged {None}else{
                            if let Some(print)=print {last=Some((print,std::time::Instant::now()));}
                            lyrics::read_native(&source,&title,&artist,dur)
                        };
                        let done=found.is_some();
                        match tx.try_send(found) {
                            Ok(())=>{if done{break;}}
                            Err(std::sync::mpsc::TrySendError::Disconnected(_))=>break,
                            Err(std::sync::mpsc::TrySendError::Full(_))=>{}
                        }
                        if started.elapsed()>=std::time::Duration::from_secs(90){break;}
                        std::thread::sleep(wait);
                        wait=(wait*2).min(std::time::Duration::from_secs(5));
                    }
                });
            }
            self.ly_worker=lease.spawn(move || {
                let result=if native_only {
                    lyrics::kugou_cache::read(&title,&artist,dur).map(|lyrics|(lyrics,false)).ok_or(lyrics::FetchError::NotFound)
                }else{lyrics::fetch_for_display(&source, &title, &artist, dur, song_id, cached)};
                let _ = tx.send(result);
            });
            if self.ly_worker.is_none(){self.ly.loading=false;self.ly.key.clear();}
        }
        if let Some(rx)=&self.ly.native_rx {
            let mut found=None;
            while let Ok(result)=rx.try_recv(){if result.is_some(){found=result;}}
            if let Some(lyrics)=found {
                self.ly.lyrics=Some(lyrics);self.ly.loading=false;self.ly.error=None;
                self.ly.retry_at=None;self.ly.rx=None;self.ly.native_rx=None;
            }
        }
        if let Some(rx) = &self.ly.rx {
            if let Ok(r) = rx.try_recv() {
                self.ly.loading = false;
                match r {
                    Ok((l,waiting_native)) => {
                        self.ly.error=None;
                        self.ly.retry_at=None;
                        if (waiting_native || lyrics::is_soda(&self.media.source) && !l.synced) && self.ly.attempts<NATIVE_RETRIES {self.ly.retry_at=Some(self.t+5.0);}
                        self.ly.lyrics = Some(l);
                    },
                    Err(error) => {
                        let native=lyrics::is_soda(&self.media.source) || self.media.source.to_ascii_lowercase().contains("kugou") && error==lyrics::FetchError::NotFound;
                        if native {if self.ly.attempts<NATIVE_RETRIES{self.ly.retry_at=Some(self.t+5.0);}}
                        else if matches!(error,lyrics::FetchError::Unavailable(_)|lyrics::FetchError::InvalidResponse(_)) && self.ly.attempts<NETWORK_RETRIES {self.ly.retry_at=Some(self.t+30.0);}
                        self.ly.error = Some(error);
                    },
                }
                self.ly.rx = None;
            }
        }
    }

    /// 外部直接提供歌词（录制演示用）。
    pub fn set_lyrics(&mut self, title: &str, artist: &str, duration: f64, l: lyrics::Lyrics) {
        self.ly = LyricsState { key: format!("{title}|{artist}|{duration:.0}"), lyrics: Some(l), ..Default::default() };
    }

    pub fn on_drop(&mut self, files: Vec<PathBuf>) {
        if !self.settings.visible_pages[2] || files.is_empty() { return; }
        self.drag_hot = false;
        self.drag_saved = None;
        if self.nav.page != 2 {
            self.go(2, 1.0);
        }
        let (mut added,mut duplicate,mut failed) = (0,0,0);
        for p in files {
            if !p.exists() {failed+=1;continue;}
            if self.shelf.items.contains(&p) {
                duplicate+=1;
                continue;
            }
            if self.shelf.add(&p).is_ok() {
                self.shelf_view.note_drop(p.clone(), self.t);
                added += 1;
            }else{failed+=1;}
        }
        let message=if added==0 && failed==0 {
            if duplicate==1{"该文件已在置物架中".into()}else{"这些文件已在置物架中".into()}
        }else if added==0 && duplicate==0 {"文件不存在或无法访问".into()}
        else {
            let mut parts=Vec::new();
            if added>0{parts.push(format!("已添加 {added} 个文件"));}
            if duplicate>0{parts.push(format!("{duplicate} 个已存在"));}
            if failed>0{parts.push(format!("{failed} 个无法添加"));}
            parts.join("，")
        };
        self.say(message);
        if added>0 {
            self.pet.react(PetState::Happy);
            self.shelf_view.shelf_first = self.shelf.items.len().saturating_sub(5);
        }
    }

    /// 待办输入框：真正的 egui 文本框，放在待办卡片顶部那一行。
    fn input_field(&mut self, ui: &mut egui::Ui, origin: Pos2, cmds: &mut Vec<Cmd>) {
        let rect = Rect::from_min_size(origin + Vec2::new(222.0, 37.0), Vec2::new(142.0, 18.0));
        let mut buf = std::mem::take(&mut self.todo.buf);
        let te = egui::TextEdit::singleline(&mut buf)
            .id(egui::Id::new("todo-input"))
            .frame(false)
            .margin(egui::Margin::ZERO)
            .text_color(theme::WHITE)
            .font(theme::font(size::BODY, W::Regular))
            .hint_text(egui::RichText::new("要做点什么？回车保存").color(theme::DIM).font(theme::font(size::BODY, W::Regular)))
            .desired_width(rect.width());
        let resp = ui.put(rect, te);
        let first_focus=!self.todo.todo_input_focused;
        self.todo.buf=buf;
        let lost_window=ui.input(|i|i.events.iter().any(|event|matches!(event,egui::Event::WindowFocused(false))));
        if !first_focus && self.todo.adding && self.todo.buf.trim().is_empty() && (resp.clicked_elsewhere() || lost_window) {
            resp.surrender_focus();
            self.cancel_edit();
            return;
        }
        if first_focus {
            self.todo.todo_input_focused=true;
            resp.request_focus();
            cmds.push(Cmd::Focus);
        }
        let submit = ui.input(|i| i.key_pressed(egui::Key::Enter));
        if submit {
            let text = self.todo.buf.trim().to_string();
            let mut added=false;
            if let Some(i) = self.todo.editing {
                if !text.is_empty() && i < self.todos.items.len() {
                    self.todos.items[i].text = text;
                    self.todos.save();
                }
            } else if !text.is_empty() {
                self.todo_first = 0;
                self.todos.items.insert(0, model::TodoItem::new(text));
                self.todos.save();
                self.todo.pop(6.0);
                added=true;
            }
            self.cancel_edit();
            if added {self.open_task_reminder(0);}
        }
    }
}
