//! 设置、通知队列与胶囊坞表情。
use crate::settings_theme::color as settings_color;
use crate::app::{App, Cmd, Phase, HUD_TRACK_W, HUD_TRACK_X, HUD_W, PAGE_NAMES};
use crate::paint::{Card, H};
use crate::theme::{self, size, W};
use eframe::egui::{self, Color32, Rect};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AlertSource { Internal }

#[derive(Clone, Debug)]
pub(crate) struct Alert {
    pub title: String,
    pub source: AlertSource,
    pub body: String,
    pub page: usize,
    pub at: f64,
    pub duration: f64,
    pub bluetooth: Option<(crate::bt::DeviceKind, bool)>,
}
impl Alert {
    pub(crate) fn is_hud(&self)->bool {matches!(self.title.as_str(),"系统音量"|"大写锁定"|"键盘输入语言"|"网络连接"|"电源已连接"|"已切换电池供电"|"蓝牙设备")}
}
fn bluetooth_notice_content(body:&str)->(&str,&str,Option<u8>){
    if let Some(name)=body.strip_suffix(" · 已断开") {return (name,"已断开",None);}
    if body=="已断开连接" {return ("蓝牙设备","已断开",None);}
    if let Some((name,level))=body.rsplit_once(" · 电量 ") {
        if let Some(level)=level.strip_suffix('%').and_then(|s|s.parse::<u8>().ok()).filter(|v|*v<=100){
            return (name,"已连接",Some(level));
        }
    }
    (body,"已连接",None)
}

pub(crate) fn device_icon(c: &Card, kind: crate::bt::DeviceKind, x: f32, y: f32, size: f32) {
    let icon=match kind {
        crate::bt::DeviceKind::Bone | crate::bt::DeviceKind::Earbuds | crate::bt::DeviceKind::Headphones=>egui::include_image!("../assets/device-icons/headphones-round-bold.svg"),
        crate::bt::DeviceKind::Speaker=>egui::include_image!("../assets/device-icons/speaker-bold.svg"),
        crate::bt::DeviceKind::Keyboard=>egui::include_image!("../assets/device-icons/keyboard-bold.svg"),
        crate::bt::DeviceKind::Mouse=>egui::include_image!("../assets/device-icons/mouse-bold.svg"),
        crate::bt::DeviceKind::Gamepad=>egui::include_image!("../assets/device-icons/gamepad-bold.svg"),
        crate::bt::DeviceKind::Phone=>egui::include_image!("../assets/device-icons/smartphone-bold.svg"),
        crate::bt::DeviceKind::Other=>egui::include_image!("../assets/device-icons/bluetooth-linear.svg"),
    };
    let rect=c.rect(x,y,size,size);
    let egui::ImageSource::Bytes{uri,bytes}=icon else{return};
    let pixels=(rect.width()*c.ui.ctx().pixels_per_point()).ceil() as u32;
    let icon=egui::Image::from_bytes(format!("{uri}/{pixels}.svg"),bytes);
    if let Ok(egui::load::TexturePoll::Ready { texture })=icon.fit_to_exact_size(rect.size()).load_for_size(c.ui.ctx(),rect.size()) {
        c.painter.image(texture.id,rect,Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),Color32::WHITE);
    }
}

pub(crate) use crate::pet::PetState;

pub(crate) fn notice_badge(c:&Card,x:f32,y:f32,a:&Alert){
    if a.title=="蓝牙设备"{
        c.circle(x,y,11.0,if a.body.contains("已断开"){Color32::from_gray(95)}else{theme::BLUE});
        device_icon(c,a.bluetooth.map_or(crate::bt::DeviceKind::Other,|v|v.0),x-7.5,y-7.5,15.0);
    }else{notice_icon(&c.scaled(x,y,0.85),x,y,a);}
}
fn notice_icon(c:&Card,x:f32,y:f32,a:&Alert) {
    match a.title.as_str() {
        "系统音量"=>{
            c.poly(&[(x-9.0,y-3.0),(x-5.0,y-3.0),(x,y-7.0),(x,y+7.0),(x-5.0,y+3.0),(x-9.0,y+3.0)],theme::WHITE);
            c.arc(x,y,5.0,-0.8,0.8,1.5,theme::WHITE);c.arc(x,y,8.0,-0.7,0.7,1.3,theme::WHITE);
        },
        "大写锁定"=>{
            let ink=if a.body=="已开启"{theme::GREEN}else{theme::GRAY};
            c.poly(&[(x-8.0,y-1.0),(x,y-9.0),(x+8.0,y-1.0)],ink);
            c.fill(x-3.0,y-1.5,6.0,6.5,0.5,ink);
            c.fill(x-4.0,y+7.0,8.0,2.0,0.8,ink);
        },
        "键盘输入语言"=>{
            c.stroke(x-10.0,y-8.5,20.0,17.0,4.0,1.2,theme::GRAY);
            let label=match a.body.as_str(){"中文"=>"中","英语"=>"EN",_=>"Aa"};
            c.centered_text(x,y,label,if label=="EN"{9.0}else{11.0},W::Semibold,theme::WHITE,H::Center);
        },
        "网络连接"=>{
            let connected=a.body.contains("已连接");
            let ink=if connected{theme::GREEN}else{theme::GRAY};
            if a.body.starts_with("Wi-Fi") {
                for r in [4.0,8.0,12.0]{c.arc(x,y+6.0,r,-2.35,-0.79,1.7,ink);}
                c.circle(x,y+6.0,1.6,ink);
            }else if a.body.starts_with("以太网") {
                c.stroke(x-8.0,y-8.0,16.0,14.0,2.5,1.6,ink);
                for dx in [-4.0,-1.3,1.3,4.0]{c.seg((x+dx,y-5.0),(x+dx,y-1.0),1.2,ink);}
                c.path(&[(x-4.0,y+6.0),(x-4.0,y+9.0),(x+4.0,y+9.0),(x+4.0,y+6.0)],1.6,ink);
            }else{
                c.ring(x,y,8.0,1.4,ink);c.seg((x-8.0,y),(x+8.0,y),1.0,ink);
                c.stroke(x-3.5,y-8.0,7.0,16.0,3.5,1.0,ink);
            }
            if !connected{c.seg((x-8.0,y-8.0),(x+9.0,y+9.0),2.0,theme::WHITE);}
        },
        "电源已连接"|"已切换电池供电"|"电量偏低"=>{
            let low=a.title=="电量偏低";let ink=if low{theme::RED_TOP}else if a.title=="电源已连接"{theme::GREEN}else{theme::WHITE};
            c.stroke(x-11.0,y-6.0,20.0,12.0,3.0,1.5,ink);c.fill(x+10.0,y-2.5,2.0,5.0,0.8,ink);
            if low{c.fill(x-8.0,y-3.5,3.5,7.0,1.0,ink);}
            else if a.title=="电源已连接"{
                c.poly(&[(x,y-4.5),(x-4.0,y+0.8),(x+0.5,y+0.8)],ink);
                c.poly(&[(x+0.5,y-0.8),(x+4.0,y-0.8),(x-1.0,y+4.5)],ink);
            }else{c.fill(x-8.0,y-3.5,12.0,7.0,1.0,ink);}
        },
        "待办到时间了"=>crate::paint::checklist(c,x,y,23.0,Color32::from_rgb(190,180,255),Color32::from_rgb(43,35,66)),
        "专注计时"=>crate::paint::stopwatch(&c.scaled(x,y,0.65),x,y,8.0,0.0,theme::RED_TOP),
        _=>crate::paint::bell(&c.scaled(x,y,1.7),x,y,theme::GRAY,0.0),
    }
}

impl App {
    pub(crate) fn phase_secs(&self, phase: Phase) -> f32 {
        (match phase {
            Phase::Focus => self.settings.focus_minutes,
            Phase::Rest => self.settings.rest_minutes,
        }) as f32
            * 60.0
    }
    pub(crate) fn pet_state(&self) -> PetState {
        if let Some((state, _)) = self.pet.reaction {
            state
        } else if self.pet.waking_until > self.t {
            PetState::Waking
        } else if self.pomo.running && self.pomo.phase == Phase::Focus {
            PetState::Focused
        } else if self.pet.worried {
            PetState::Worried
        } else if self.media.playing && self.media.is_music() {
            if self.notices.notice.is_some() || self.notices.hud_notice.is_some() {PetState::Awake}else{PetState::Music}
        } else if self.pet.hovered {
            PetState::Curious
        } else if self.idle_for() >= 180.0 {
            PetState::Sleeping
        } else if self.idle_for() >= self.opts.pet_idle {
            PetState::Dozing
        } else {
            PetState::Awake
        }
    }
    pub fn alert(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        page: usize,
        system: bool,
    ) {
        let title = title.into();
        let transient = matches!(
            title.as_str(),
            "系统音量" | "大写锁定" | "键盘输入语言" | "网络连接" | "电源已连接" | "已切换电池供电" | "蓝牙设备"
        );
        let a = Alert {
            source: AlertSource::Internal,
            title,
            body: body.into(),
            page,
            at: self.t,
            duration: if transient { 2.5 } else { 8.0 },
            bluetooth: None,
        };
        self.enqueue_alert(a, system);
    }
    pub(crate) fn bluetooth_alert(&mut self, device: &crate::bt::Device, connected: bool) {
        if !self.settings.bluetooth_notifications { return; }
        if connected && matches!(device.kind,crate::bt::DeviceKind::Bone|crate::bt::DeviceKind::Earbuds|crate::bt::DeviceKind::Headphones|crate::bt::DeviceKind::Speaker) {
            self.notices.alerts.retain(|a|a.title!="系统音量");
            if self.notices.notice.as_ref().is_some_and(|a|a.title=="系统音量") {self.notices.notice=None;}
        }
        let a = Alert {
            source: AlertSource::Internal,
            title: "蓝牙设备".into(),
            body: if connected { device.summary() } else { format!("{} · 已断开", device.name) },
            page: usize::MAX, at: self.t, duration: 2.5,
            bluetooth: Some((device.kind, connected)),
        };
        self.enqueue_alert(a, false);
    }
    fn enqueue_alert(&mut self, a: Alert, system: bool) {
        let settings = crate::notice::NoticeSettings { notifications_enabled: self.settings.notifications };
        match self.notices.enqueue(a, system, &settings) {
            crate::notice::EnqueueAction::Notify(title, body) => {
                self.pending.push(Cmd::Notify(title, body));
            }
            crate::notice::EnqueueAction::None => {}
        }
    }
    pub(crate) fn tick_alerts(&mut self) {
        // 告警在当前设置下是否允许展示：纯设置决策，留在 App 编排；队列/过滤/过期/出队交给 NoticeStack。
        let allowed=|_a:&Alert|true;
        self.notices.tick(self.t, allowed);
    }
    pub(crate) fn draw_notice(&mut self, c:&Card,panel:Rect,a:&Alert,_cmds:&mut Vec<Cmd>){
        let nc=c.clipped(0.0,0.0,panel.width(),panel.height());
        if a.is_hud(){
            if a.title=="蓝牙设备" {
                let (name,status,battery)=bluetooth_notice_content(&a.body);
                nc.circle(26.0,24.0,14.0,if status=="已连接"{Color32::from_rgb(10,132,255)}else{Color32::from_gray(95)});
                let kind=a.bluetooth.map(|(kind,_)|kind).unwrap_or(crate::bt::DeviceKind::Other);
                device_icon(&nc,kind,17.0,15.0,18.0);
                let right=HUD_W-14.0;
                let name_end=if let Some(level)=battery {
                    let pct=format!("{level}%");
                    let pct_width=nc.text_w(&pct,10.5,W::Semibold);
                    let x=right-pct_width-22.0;
                    nc.stroke(x,20.0,15.0,8.0,2.0,1.0,Color32::from_gray(175));
                    nc.fill(x+15.5,22.5,2.0,3.0,0.8,Color32::from_gray(175));
                    let fill=11.0*level as f32/100.0;
                    if fill>0.0 {nc.fill(x+2.0,22.0,fill,4.0,1.0,if level<=20{theme::RED}else{theme::GREEN});}
                    nc.centered_text(right,24.0,&pct,10.5,W::Semibold,theme::WHITE,H::Right);
                    x-8.0
                } else {right};
                nc.centered_text(49.0,16.0,&nc.fit(name,11.0,W::Semibold,(name_end-49.0).max(20.0)),11.0,W::Semibold,theme::WHITE,H::Left);
                nc.centered_text(49.0,32.0,status,10.5,W::Regular,theme::GRAY,H::Left);
                return;
            }
            match a.title.as_str(){
                "系统音量"=>{
                    nc.poly(&[(17.0,16.0),(21.0,16.0),(26.0,12.0),(26.0,26.0),(21.0,22.0),(17.0,22.0)],theme::WHITE);
                    if self.volume_ui.is_some_and(|(_,muted)|muted){nc.seg((29.0,16.0),(34.0,22.0),1.4,theme::WHITE);nc.seg((34.0,16.0),(29.0,22.0),1.4,theme::WHITE);}
                    else{nc.arc(26.0,19.0,5.0,-0.8,0.8,1.5,theme::WHITE);nc.arc(26.0,19.0,8.0,-0.7,0.7,1.3,theme::WHITE);}
                },
                _=>notice_icon(&nc,24.0,19.0,a),
            }
            if a.title=="系统音量" {
                self.draw_volume_hud(&nc);
            } else {
                let (title,body)=if a.title=="键盘输入语言"{("输入语言",a.body.as_str())}
                    else if a.title=="已切换电池供电"{("电池供电",a.body.as_str())}
                    else if a.title=="网络连接"{a.body.split_once(" · ").unwrap_or(("网络",a.body.as_str()))}
                    else{(a.title.as_str(),a.body.as_str())};
                nc.centered_text(48.0,19.0,&nc.fit(title,11.0,W::Semibold,90.0),11.0,W::Semibold,theme::WHITE,H::Left);
                nc.centered_text(HUD_W-16.0,19.0,&nc.fit(body,10.5,W::Regular,92.0),10.5,W::Regular,if a.title=="大写锁定"&&a.body=="已开启"||a.title=="网络连接"&&a.body.contains("已连接"){theme::GREEN}else{theme::GRAY},H::Right);
            }
        } else {
            let is_focus=a.page==1;let is_power=a.title=="电量偏低";
            notice_icon(&nc,22.0,22.0,a);
            nc.centered_text(43.0,14.0,&nc.fit(&a.title,11.0,W::Semibold,195.0),11.0,W::Semibold,theme::WHITE,H::Left);
            nc.centered_text(43.0,30.0,&nc.fit(&a.body,10.5,W::Regular,195.0),10.5,W::Regular,theme::GRAY,H::Left);
            let label=if is_focus{"开始"}else if is_power{"知道了"}else{"查看"};
            if notice_button(&nc,257.0,12.0,60.0,label,"notice-action"){
                self.notices.notice=None;if is_power{return;}if is_focus{self.pomo.running=true;}else if a.page<crate::app::PAGE_NAMES.len(){
                    self.show_page(a.page);
                    if a.page==crate::app::TODO_PAGE {
                        let order=self.todos.display_order(&self.today);
                        if let Some(index)=order.iter().position(|&i|self.todos.items[i].text==a.body && !self.todos.items[i].done_for(&self.today)){
                            self.todo_first=index.min(order.len().saturating_sub(3));
                        }
                    }
                }else{self.settings_tab=5;self.open_settings();}
            }
        }
    }
    /// v0.5.4：可交互音量条。样式参考 oyenza-studio（MIT）的 HUD 音量条：
    /// 圆角胶囊、填充缓动、拖动时加粗并出现手柄；拖动/滚轮改音量，点喇叭静音。
    pub(crate) fn draw_volume_hud(&mut self, nc: &Card) {
        let (v, muted) = self.volume_ui.unwrap_or((0, false));
        let v = v.min(100);
        // 喇叭：点击静音 / 恢复
        let icon = nc.hit(12.0, 9.0, 28.0, 22.0, "volume-mute");
        if icon.clicked() && nc.live {
            self.volume_ui = Some((v, !muted));
            self.volume_quiet_until = self.t + 0.9;
            self.pending.push(Cmd::ToggleMute);
        }
        // 滑条：拖动 / 点击定位；滚轮微调
        let track = nc.hit_drag(HUD_TRACK_X, 8.0, HUD_TRACK_W, 24.0, "volume-track");
        let mut want: Option<u32> = None;
        if track.dragged() || track.clicked() {
            if let Some(p) = track.interact_pointer_pos() {
                let x = nc.to_local(p).x;
                want = Some((((x - HUD_TRACK_X) / HUD_TRACK_W) * 100.0).clamp(0.0, 100.0).round() as u32);
            }
        } else if nc.live && track.hovered() {
            let dy = nc.ui.ctx().input(|i| i.raw_scroll_delta.y);
            if dy.abs() > 0.5 {
                want = Some(((v as f32 + (dy / 120.0) * 4.0).clamp(0.0, 100.0)).round() as u32);
            }
        }
        if let Some(nv) = want {
            if self.volume_ui.map(|(o, _)| o) != Some(nv) || muted {
                self.volume_ui = Some((nv, false));
                self.volume_quiet_until = self.t + 0.9;
                self.pending.push(Cmd::SetVolume(nv));
            }
        }
        let (v, muted) = self.volume_ui.unwrap_or((v, muted));
        let pressed = track.dragged() || track.is_pointer_button_down_on() || icon.is_pointer_button_down_on();
        let target = if muted { 0.0 } else { v as f32 / 100.0 };
        self.volume_a = crate::anim::approach(self.volume_a, target, if pressed { 34.0 } else { 13.0 }, self.dt);
        let h = if pressed { 7.0 } else { 5.0 };
        let y = 19.0 - h / 2.0;
        nc.fill(HUD_TRACK_X, y, HUD_TRACK_W, h, h / 2.0, Color32::from_white_alpha(if pressed { 46 } else { 34 }));
        if self.volume_a > 0.0 {
            let w = (HUD_TRACK_W * self.volume_a).max(h);
            let col = if muted { Color32::from_gray(150) } else { theme::WHITE };
            nc.fill(HUD_TRACK_X, y, w, h, h / 2.0, col);
            nc.fill(HUD_TRACK_X, y + 0.8, (w - 1.2).max(0.0), h * 0.4, h * 0.2, Color32::from_white_alpha(64));
            if pressed {
                nc.circle(HUD_TRACK_X + w - h / 2.0, y + h / 2.0, h * 0.95, col);
            }
        }
        let label = if muted { "静音".to_string() } else { format!("{v}%") };
        nc.centered_text(HUD_W - 26.0, 19.0, &label, 10.5, if pressed { W::Semibold } else { W::Regular }, if muted { theme::GRAY } else { theme::WHITE }, H::Center);
    }

}

/// 将本地提醒时间规范化，拒绝不存在的日期与超出范围的时分。
pub(crate) fn parse_due(value: &str, today: &str) -> Option<String> {
    let value = value.trim();
    let full = if value.len() == 5 {
        format!("{today} {value}")
    } else {
        value.to_string()
    };
    if !full.is_ascii() || full.len() != 16 {
        return None;
    }
    let b = full.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b' ' || b[13] != b':' {
        return None;
    }
    let year = full[0..4].parse::<u32>().ok()?;
    let month = full[5..7].parse::<usize>().ok()?;
    let day = full[8..10].parse::<u32>().ok()?;
    let hour = full[11..13].parse::<u32>().ok()?;
    let minute = full[14..16].parse::<u32>().ok()?;
    if !(2000..=9999).contains(&year) || !(1..=12).contains(&month) || hour > 23 || minute > 59 {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day == 0 || day > days[month - 1] {
        return None;
    }
    Some(full)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn island_size_drag_commits_only_on_release(){
        let ctx=egui::Context::default();let mut value=100;
        let pos=egui::pos2(30.0,30.0);let end=egui::pos2(110.0,30.0);
        let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
        let frames=[vec![],vec![egui::Event::PointerMoved(pos)],vec![button(pos,true)],vec![egui::Event::PointerMoved(end)],vec![button(end,false)]];
        for (i,events) in frames.into_iter().enumerate(){
            let _=ctx.run(egui::RawInput{events,screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(320.0,80.0))),..Default::default()},|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(20)).show(ctx,|ui|{ui.spacing_mut().slider_width=180.0;island_size_slider(ui,&mut value);});
            });
            if i<4{assert_eq!(value,100);}else{assert_ne!(value,100);}
        }
    }
    #[test]
    fn number_stepper_clicks_follow_steps_and_limits() {
        for (initial,step,x,expected) in [(5u32,1,127.0,6),(5,1,13.0,4),(0,1,13.0,0),(10,1,127.0,10),(9,3,127.0,10),(1,3,13.0,0)] {
            let ctx=egui::Context::default();
            let mut value=initial;
            let pos=egui::pos2(x+20.0,34.0);
            let events=[vec![],vec![egui::Event::PointerMoved(pos)],vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}],vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]];
            for events in events {
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(200.0,80.0))),events,..Default::default()};
                let _=ctx.run(raw,|ctx|{
                    egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(20)).show(ctx,|ui|{
                        ui.horizontal(|ui|{number_stepper(ui,&mut value,0,10,"",step,76.0);});
                    });
                });
            }
            assert_eq!(value,expected,"初值 {initial}，步长 {step}，点击位置 {x}");
        }
    }

    #[test]
    fn number_stepper_fits_with_settings_button_padding() {
        let ctx=egui::Context::default();
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(340.0,240.0))),..Default::default()};
        let _=ctx.run(raw,|ctx|{
            egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                ui.spacing_mut().button_padding=egui::vec2(10.0,6.0);
                for mut value in [0u32,25,180] {
                    let row=ui.horizontal(|ui|{number_stepper(ui,&mut value,0,180," 分钟",1,76.0);});
                    assert!(row.response.rect.width()<=140.1,"数值控件超出预留宽度：{}",row.response.rect.width());
                }
                for mut value in [25u32,5,15] {setting_number(ui,"计时时长",&mut value,1,180," 分钟",1.0);}
                assert!(ui.min_rect().right()<=340.1,"连续数值行不应撑宽卡片");
            });
        });
    }

    #[test]
    fn settings_groups_keep_every_page_reachable() {
        let groups = ["常规", "音乐与歌词", "专注", "待办与置物", "待办与置物", "通知", "关于", "页面管理"];
        for (tab, expected) in groups.iter().enumerate() {
            let matches: Vec<_> = SETTINGS_GROUPS.iter().filter(|(_,start,end)| (*start..=*end).contains(&tab)).collect();
            assert_eq!(matches.len(), 1, "页面 {tab} 必须且只能属于一个分组");
            assert_eq!(matches[0].0, *expected);
        }
    }

    #[test]
    fn bluetooth_notice_uses_real_connection_and_battery_data(){
        assert_eq!(bluetooth_notice_content("OpenMove by Shokz · 电量 80%"),("OpenMove by Shokz","已连接",Some(80)));
        assert_eq!(bluetooth_notice_content("耳机"),("耳机","已连接",None));
        assert_eq!(bluetooth_notice_content("已断开连接"),("蓝牙设备","已断开",None));
        assert_eq!(bluetooth_notice_content("青龙键盘 · 已断开"),("青龙键盘","已断开",None));
        assert_eq!(bluetooth_notice_content("耳机 · 电量 101%"),("耳机 · 电量 101%","已连接",None));
    }

    #[test]
    fn device_icons_render_as_visible_svg() {
        let icons: &[&[u8]] = &[
            include_bytes!("../assets/device-icons/headphones-round-bold.svg"),
            include_bytes!("../assets/device-icons/speaker-bold.svg"),
            include_bytes!("../assets/device-icons/keyboard-bold.svg"),
            include_bytes!("../assets/device-icons/mouse-bold.svg"),
            include_bytes!("../assets/device-icons/gamepad-bold.svg"),
            include_bytes!("../assets/device-icons/smartphone-bold.svg"),
            include_bytes!("../assets/device-icons/bluetooth-linear.svg"),
        ];
        for bytes in icons {
            let image=egui_extras::image::load_svg_bytes(bytes).unwrap();
            assert!(image.pixels.iter().any(|p|p.a()>0));
        }
    }

    #[test]
    fn due_time_checks_calendar_and_format() {
        assert_eq!(
            parse_due("09:30", "2026-10-29").as_deref(),
            Some("2026-10-29 09:30")
        );
        assert!(parse_due("2026-02-29 12:00", "").is_none());
        assert!(parse_due("2028-02-29 12:00", "").is_some());
        for value in [
            "24:00",
            "12:60",
            "明天中午",
            "2026-13-01 10:00",
            "2026-00-01 10:00",
        ] {
            assert!(parse_due(value, "2026-10-29").is_none());
        }
    }
}

impl App {
    pub(crate) fn open_task_dialog(&mut self,index:usize){
        if let Some(task)=self.todos.items.get(index).cloned(){
            self.show_page(crate::app::TODO_PAGE);
            self.todo.begin_dialog(index,&task.text,task.due.as_deref(),task.repeat);
        }
    }
    pub(crate) fn open_task_reminder(&mut self,index:usize){
        if let Some(task)=self.todos.items.get(index).cloned(){self.show_page(crate::app::TODO_PAGE);self.todo.begin_dialog_reminder(index,&task.text,task.due.as_deref(),task.repeat);}
    }
    pub(crate) fn draw_task_editor(&mut self,c:&Card){
        if self.todo.task_dialog.as_ref().is_some_and(|draft|draft.picker.open){self.draw_task_reminder(c);return;}
        let reminder_only=self.todo.task_dialog.as_ref().is_some_and(|d|d.reminder_only);
        c.text(15.0,25.0,if reminder_only{"设置提醒"}else{"编辑待办"},size::TITLE,W::Bold,theme::WHITE,H::Left);
        c.text(43.0,48.0,if reminder_only{"待办"}else{"内容"},10.5,W::Regular,theme::GRAY,H::Left);
        c.fill(91.0,33.0,275.0,20.0,6.0,theme::CARD);
        if let Some(d)=self.todo.task_dialog.as_ref().filter(|d|d.reminder_only){
            c.text(97.0,47.0,&c.fit(&d.text,11.5,W::Regular,260.0),11.5,W::Regular,theme::WHITE,H::Left);
        }
        c.text(43.0,72.0,"提醒",10.5,W::Regular,theme::GRAY,H::Left);
        let time=self.todo.task_dialog.as_ref().map(|d|if d.due.is_empty(){"点击选择时间".into()}
            else if d.repeat!=0{format!("{} {}",crate::model::repeat_label(d.repeat),d.due.get(11..16).unwrap_or(&d.due))}else{d.due.clone()}).unwrap_or_default();
        if setting_button(c,91.0,58.0,275.0,&time,false,"task-select-reminder"){if let Some(d)=self.todo.task_dialog.as_mut(){d.picker.show();}}
        // 每周重复：七个星期按钮，全不选即一次性待办。
        c.text(43.0,95.0,"重复",10.5,W::Regular,theme::GRAY,H::Left);
        self.task_repeat_chips(c,91.0,81.0,25.0,19.0,28.0);
        let repeat=self.todo.task_dialog.as_ref().map(|d|d.repeat).unwrap_or(0);
        let quick=if repeat==crate::model::REPEAT_ALL{"不重复"}else{"每天"};
        if setting_button(c,289.0,81.0,77.0,quick,false,"task-repeat-all"){
            if let Some(d)=self.todo.task_dialog.as_mut(){d.repeat=if d.repeat==crate::model::REPEAT_ALL{0}else{crate::model::REPEAT_ALL};}
        }
        if let Some(d)=&self.todo.task_dialog {
            let summary=if d.repeat!=0{crate::model::repeat_label(d.repeat)}else{"只做一次".into()};
            let hint=if d.error.is_empty(){summary.as_str()}else{&d.error};
            c.text(43.0,117.0,&c.fit(hint,10.5,W::Regular,if reminder_only{140.0}else{154.0}),10.5,if d.error.is_empty(){W::Regular}else{W::Semibold},if d.error.is_empty(){theme::GRAY}else{theme::RED},H::Left);
        }
        if reminder_only {
            let has_reminder=self.todo.task_dialog.as_ref().and_then(|d|self.todos.items.get(d.index)).is_some_and(|task|task.due.is_some());
            let skip=if has_reminder{"取消提醒"}else if repeat!=0{"只设重复"}else{"暂不提醒"};
            if setting_button(c,188.0,104.0,88.0,skip,false,"task-reminder-skip"){
                if let Some(d)=self.todo.task_dialog.as_mut(){d.due.clear();}
                self.save_task_editor();
            }
        }else if setting_button(c,202.0,104.0,74.0,"取消",false,"task-cancel"){self.todo.task_dialog=None;}
        if setting_button(c,286.0,104.0,80.0,if reminder_only{"选择时间"}else{"保存"},true,"task-save"){
            if reminder_only{if let Some(d)=self.todo.task_dialog.as_mut(){d.picker.show();}}else{self.save_task_editor();}
        }
    }
    pub(crate) fn task_editor_inputs(&mut self,ui:&mut egui::Ui,origin:egui::Pos2,cmds:&mut Vec<Cmd>){
        let Some(d)=self.todo.task_dialog.as_mut() else{return};
        if d.reminder_only || d.picker.open{return;}
        let name=ui.put(Rect::from_min_size(origin+egui::vec2(97.0,35.0),egui::vec2(260.0,17.0)),egui::TextEdit::singleline(&mut d.text).vertical_align(egui::Align::Center).id(egui::Id::new("task-name-edit")).frame(false).font(theme::font(11.5,W::Regular)));
        let first_focus=!d.focused;
        if first_focus{name.request_focus();d.focused=true;cmds.push(Cmd::Focus);}
        // 新建内容与下一步共用这一帧，避免同一次回车又提交了空提醒。
        if !first_focus && ui.input(|i|i.key_pressed(egui::Key::Enter)){self.save_task_editor();}
    }
    /// 七个星期按钮（一…日），点一下切换；全不选即一次性待办。
    fn task_repeat_chips(&mut self,c:&Card,x:f32,y:f32,w:f32,h:f32,step:f32){
        let repeat=self.todo.task_dialog.as_ref().map(|d|d.repeat).unwrap_or(0);
        for (day,name) in crate::model::WEEKDAY_NAMES.iter().enumerate() {
            let on=repeat&(1<<day)!=0;
            let cx=x+day as f32*step;
            let (r,b)=crate::paint::capsule(c,cx,y,w,h,&format!("task-repeat-{day}"),if on{Color32::from_rgb(39,94,150)}else{theme::BTN},theme::BTN_HOVER);
            b.centered_text(cx+w/2.0,y+h/2.0,name,if h<18.0{9.5}else{10.5},W::Semibold,if on{theme::WHITE}else{theme::GRAY},H::Center);
            if r.on_hover_text(format!("每周{name}")).clicked(){
                if let Some(d)=self.todo.task_dialog.as_mut(){d.repeat^=1<<day;d.error.clear();}
            }
        }
    }
    fn draw_task_reminder(&mut self,c:&Card){
        use crate::task_picker::PickerAction;
        let Some(d)=self.todo.task_dialog.as_mut().filter(|d|d.picker.open) else{return};
        let has_reminder=self.todos.items.get(d.index).is_some_and(|task|task.due.is_some());
        c.text(15.0,25.0,"待办",size::TITLE,W::Bold,theme::WHITE,H::Left);
        c.text(44.0,25.0,"清单",size::TITLE,W::Bold,theme::GREEN,H::Left);
        let (done,total)=self.todos.progress(&self.today);
        c.text(42.0,61.0,&format!("{done}/{total}"),29.0,W::Bold,theme::WHITE,H::Left);
        // 左侧：每周重复。选了星期几后，提醒只按时间、在这些日子到点提醒。
        d.picker.repeat_note=Some(if d.repeat!=0{format!("{} · 到点提醒",crate::model::repeat_label(d.repeat))}else{String::new()});
        let action=d.picker.draw(c,&d.text,has_reminder);
        let focus=std::mem::take(&mut d.picker.focus);
        self.task_repeat_chips(c,41.0,84.0,17.0,16.0,19.5);
        let Some(d)=self.todo.task_dialog.as_mut() else{return};
        if focus{self.pending.push(Cmd::Focus);}
        match action{
            Some(PickerAction::Save)=>{d.due=d.picker.due();d.picker.open=false;if d.reminder_only{self.save_task_editor();}},
            Some(PickerAction::Skip)=>{d.due.clear();d.picker.open=false;if d.reminder_only{self.save_task_editor();}},
            Some(PickerAction::Cancel)=>{d.picker.open=false;if d.reminder_only{self.todo.task_dialog=None;}},
            None=>{},
        }
    }
    fn save_task_editor(&mut self){
        let Some(mut d)=self.todo.task_dialog.take() else{return};
        let due_empty=d.due.trim().is_empty();
        let due_valid=due_empty || parse_due(&d.due,&self.today).is_some();
        let original_unchanged=self.todos.items.get(d.index).is_some_and(|t|t.text==d.original);
        match crate::todo::TodoEditor::validate_dialog(&mut d, original_unchanged, due_valid, due_empty) {
            crate::todo::SaveOutcome::Invalid => { self.todo.task_dialog=Some(d); }
            crate::todo::SaveOutcome::Valid { index, text, due, repeat } => {
                let item=&mut self.todos.items[index];
                if item.repeat!=repeat{item.done_on=None;}
                item.text=text;item.due=due;item.reminded=false;item.reminded_on=None;item.repeat=repeat;
                self.todos.save();self.say("待办已保存");
            }
        }
    }
}

fn notice_button(c:&Card,x:f32,y:f32,w:f32,text:&str,id:&str)->bool {
    let (r,b)=crate::paint::capsule(c,x,y,w,20.0,id,Color32::from_gray(35),theme::BTN_HOVER);
    b.centered_text(x+w/2.0,y+10.0,text,10.5,W::Semibold,theme::WHITE,H::Center);
    r.clicked()
}
fn setting_button(c:&Card,x:f32,y:f32,w:f32,text:&str,on:bool,id:&str)->bool {
    let (r,b)=crate::paint::capsule(c,x,y,w,19.0,id,if on{Color32::from_rgb(39,94,150)}else{theme::BTN},theme::BTN_HOVER);
    b.text(x+w/2.0,y+13.3,text,10.5,W::Semibold,theme::WHITE,H::Center);
    r.clicked()
}
impl App {
    pub(crate) fn page_settings(&mut self,c:&Card) {
        let help=match self.settings_category{0=>"外观与系统",1=>"调整每个阶段的时长",_=>"通知"};
        c.text(44.0,44.0,help,10.5,W::Regular,theme::GRAY,H::Left);
        let before=format!("{:?}",self.settings);
        match self.settings_category {
            0=>{
                let choices=[("吉祥物",self.settings.pet),("极简刘海",self.settings.minimal),("系统提示",self.settings.system_hud),("全屏隐藏",self.settings.hide_fullscreen)];
                for (i,(name,on)) in choices.iter().enumerate(){let x=42.0+(i%2) as f32*169.0;let y=57.0+(i/2) as f32*27.0;let text=format!("{}  {}",name,if *on{"开"}else{"关"});if setting_button(c,x,y,158.0,&text,*on,&format!("general-{i}")){match i{0=>self.set_pet(!self.settings.pet),1=>self.set_minimal(!self.settings.minimal),2=>self.settings.system_hud=!self.settings.system_hud,_=>self.settings.hide_fullscreen=!self.settings.hide_fullscreen}}}
                c.text(42.0,116.0,"胶囊坞收起时常驻；极简模式保留细线",10.5,W::Regular,theme::GRAY,H::Left);
            }
            1=>{
                let values=[self.settings.focus_minutes,self.settings.rest_minutes];
                for (i,name) in ["专注","休息"].iter().enumerate(){let y=53.0+i as f32*23.0;c.text(48.0,y+13.0,name,10.5,W::Regular,theme::GRAY,H::Left);c.text(206.0,y+13.0,&format!("{} 分钟",values[i]),11.0,W::Bold,theme::WHITE,H::Center);
                    if setting_button(c,131.0,y,27.0,"−",false,&format!("focus-minus-{i}")){if i==0{self.settings.focus_minutes=values[i].saturating_sub(1).max(1)}else{self.settings.rest_minutes=values[i].saturating_sub(1).max(1)}}
                    if setting_button(c,255.0,y,27.0,"+",false,&format!("focus-plus-{i}")){if i==0{self.settings.focus_minutes=(values[i]+1).min(180)}else{self.settings.rest_minutes=(values[i]+1).min(60)}}}
                c.text(315.0,93.0,"下次生效",10.5,W::Regular,theme::GRAY,H::Center);
            }
            _=>{
                if setting_button(c,43.0,56.0,152.0,if self.settings.notifications{"提醒 · 开"}else{"提醒 · 关"},self.settings.notifications,"set-notice"){self.settings.notifications=!self.settings.notifications;}
                if setting_button(c,212.0,56.0,153.0,if self.settings.sound{"声音 · 开"}else{"声音 · 关"},self.settings.sound,"set-sound"){self.settings.sound=!self.settings.sound;}
            }
        }
        if before!=format!("{:?}",self.settings){self.settings.save();}
    }
}

fn switch(ui:&mut egui::Ui,value:&mut bool)->bool {
    let ctx=ui.ctx().clone();
    // 热区 40×30、轨道 40×22 居中：比原来 20 高的轨道更容易点中。
    let (rect,response)=ui.allocate_exact_size(egui::vec2(40.0,30.0),egui::Sense::click());
    if response.clicked(){*value=!*value;}
    let track=egui::Rect::from_center_size(rect.center(),egui::vec2(40.0,22.0));
    let fill=if *value{Color32::from_rgb(48,184,92)}else{settings_color(&ctx,Color32::from_rgba_unmultiplied(255,255,255,90),Color32::from_rgba_unmultiplied(0,0,0,40))};
    ui.painter().rect_filled(track,11.0,fill);
    let x=if *value{track.right()-11.0}else{track.left()+11.0};
    ui.painter().circle_filled(egui::pos2(x,track.center().y+0.5),8.5,Color32::from_black_alpha(35));
    ui.painter().circle_filled(egui::pos2(x,track.center().y),8.0,Color32::WHITE);
    response.clicked()
}
/// 设置页版式：标签列宽、开关尺寸、控件与卡片右边缘之间的固定留白。
pub(crate) const SETTINGS_LABEL_W: f32 = 80.0;
const SETTINGS_SWITCH_W: f32 = 40.0;
const SETTINGS_ROW_GAP: f32 = 16.0;

/// 开关行：左侧标题 + 说明（宽度固定、自动换行），右侧开关，长说明也不会把开关挤出卡片。
pub(crate) fn setting_row(ui:&mut egui::Ui,title:&str,description:&str,value:&mut bool){
    let ctx=&ui.ctx().clone();
    ui.add_space(2.0);
    ui.horizontal(|ui|{
        let text_w=(ui.available_width()-SETTINGS_SWITCH_W-SETTINGS_ROW_GAP).max(120.0);
        ui.vertical(|ui|{
            ui.set_width(text_w);
            ui.spacing_mut().item_spacing.y=2.0;
            ui.add(egui::Label::new(egui::RichText::new(title).size(13.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41)))).wrap());
            if !description.is_empty(){ui.add(egui::Label::new(egui::RichText::new(description).size(11.5).color(settings_color(ctx,Color32::from_gray(175),Color32::from_gray(90)))).wrap());}
        });
        // add_space 之后还会再插入一次 item_spacing.x，这里一并扣掉，开关才不会每行往外挤一点。
        ui.add_space((ui.available_width()-SETTINGS_SWITCH_W-ui.spacing().item_spacing.x).max(0.0));
        switch(ui,value);
    });
    ui.add_space(2.0);
}

/// 设置页统一的分段选择器（灰色底框 + 选中块）。返回值为是否发生了切换。
pub(crate) fn settings_segmented<T:PartialEq+Copy>(ui:&mut egui::Ui,value:&mut T,options:&[(T,&str)],item_w:f32)->bool{
    let ctx=&ui.ctx().clone();
    let mut changed=false;
    egui::Frame::new().fill(settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,16),Color32::from_rgba_unmultiplied(0,0,0,12)))
        .corner_radius(8.0).inner_margin(3).show(ui,|ui|{
            ui.horizontal(|ui|{
                ui.spacing_mut().item_spacing.x=2.0;
                for (option,label) in options {
                    let selected=*value==*option;
                    let text=egui::RichText::new(*label).size(12.0).color(if selected{settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))}else{settings_color(ctx,Color32::from_gray(200),Color32::from_gray(70))});
                    let button=egui::Button::new(text)
                        .fill(if selected{settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,46),Color32::from_rgba_unmultiplied(0,0,0,26))}else{Color32::TRANSPARENT})
                        .stroke(egui::Stroke::NONE).corner_radius(6.0);
                    let width=item_w.max(ctx.fonts(|f|f.layout_no_wrap(label.to_string(),egui::FontId::proportional(12.0),Color32::WHITE).size().x)+20.0);
                    if ui.add_sized([width,26.0],button).clicked() && !selected {*value=*option;changed=true;}
                }
            });
        });
    changed
}

/// 选项行：标签列之后紧跟控件，同一卡片里所有控件左缘对齐成一条竖线。
pub(crate) fn settings_control_row_right(ui:&mut egui::Ui,title:&str,controls:impl FnOnce(&mut egui::Ui))->egui::Response{
    settings_control_row(ui,title,SETTINGS_LABEL_W,|ui|ui.horizontal(|ui|controls(ui)).response)
}

/// 设置项下方的说明文字。
fn island_size_slider(ui:&mut egui::Ui,value:&mut u32)->egui::Response {
    let id=ui.make_persistent_id("island-size-draft");
    let mut draft=ui.data(|d|d.get_temp::<u32>(id)).unwrap_or(*value);
    let response=ui.add(egui::Slider::new(&mut draft,crate::model::ISLAND_SIZE_MIN..=crate::model::ISLAND_SIZE_MAX)
        .step_by(crate::model::ISLAND_SIZE_STEP as f64).suffix("%"));
    if ui.input(|i|i.pointer.primary_down()) {
        if response.changed(){ui.data_mut(|d|d.insert_temp(id,draft));}
    }else{
        *value=crate::model::island_size_value(draft);
        ui.data_mut(|d|d.remove::<u32>(id));
    }
    response
}

pub(crate) fn settings_hint(ui:&mut egui::Ui,text:&str){
    let ctx=&ui.ctx().clone();
    ui.add(egui::Label::new(egui::RichText::new(text).size(11.5).color(settings_color(ctx,Color32::from_gray(160),Color32::from_gray(105)))).wrap());
}
pub(crate) fn settings_card(ui:&mut egui::Ui,add:impl FnOnce(&mut egui::Ui)) {
    let ctx=&ui.ctx().clone();
    // 半透明卡片：像 Big Sur 的分组面板；填充更实、描边退后，卡片边界靠明度差而不是硬线。
    egui::Frame::new().fill(settings_color(ctx,Color32::from_rgba_unmultiplied(44,45,49,196),Color32::from_rgba_unmultiplied(255,255,255,236)))
        .stroke(egui::Stroke::new(1.0,settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,12),Color32::from_black_alpha(8))))
        .corner_radius(10.0).inner_margin(egui::Margin::symmetric(16,12))
        .show(ui,|ui|{ui.set_min_width(ui.available_width());ui.spacing_mut().item_spacing.y=8.0;add(ui);});
}
pub(crate) fn settings_label(ui:&mut egui::Ui,text:&str){
    let ctx=&ui.ctx().clone();
    ui.add_space(8.0);
    ui.label(egui::RichText::new(text).size(11.5).color(settings_color(ctx,Color32::from_gray(185),Color32::from_gray(70))));
    ui.add_space(6.0);
}
/// 设置项之间的发丝线：从文字左缘起画、只留 4% 的痕迹，避免系统灰线压在玻璃上显得脏。
pub(crate) fn settings_separator(ui:&mut egui::Ui){
    let ctx=ui.ctx().clone();
    let spacing=ui.spacing().item_spacing.y;
    let (rect,_)=ui.allocate_exact_size(egui::vec2(ui.available_width(),spacing),egui::Sense::hover());
    ui.painter().line_segment([egui::pos2(rect.left(),rect.center().y),egui::pos2(rect.right(),rect.center().y)],
        egui::Stroke::new(1.0,settings_color(&ctx,Color32::from_white_alpha(12),Color32::from_black_alpha(10))));
}
pub(crate) fn settings_control_row(ui:&mut egui::Ui,title:&str,label_width:f32,controls:impl FnOnce(&mut egui::Ui)->egui::Response)->egui::Response{
    ui.horizontal(|ui|{
        // 同一张卡片里的控件左缘对齐：标签列至少 SETTINGS_LABEL_W。
        let (label_rect,_)=ui.allocate_exact_size(egui::vec2(label_width.max(SETTINGS_LABEL_W),30.0),egui::Sense::hover());
        let content=controls(ui);
        ui.painter().text(egui::pos2(label_rect.left(),content.rect.center().y),egui::Align2::LEFT_CENTER,title,egui::TextStyle::Body.resolve(ui.style()),ui.visuals().text_color());
        content
    }).inner
}
fn setting_number<N:egui::emath::Numeric>(ui:&mut egui::Ui,title:&str,value:&mut N,min:N,max:N,suffix:&str,speed:f64)->bool{
    let mut changed=false;
    settings_control_row(ui,title,80.0,|ui|{
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
            ui.allocate_ui_with_layout(egui::vec2(146.0,30.0),egui::Layout::left_to_right(egui::Align::Center),|ui|{
                changed=number_stepper(ui,value,min,max,suffix,N::from_f64(speed),76.0);
            });
        }).response
    });
    changed
}
fn number_stepper<N:egui::emath::Numeric>(ui:&mut egui::Ui,value:&mut N,min:N,max:N,suffix:&str,step:N,field_width:f32)->bool{
    let ctx=&ui.ctx().clone();
    let mut changed=false;
    ui.scope(|ui|{
        ui.spacing_mut().item_spacing.x=4.0;
        ui.spacing_mut().button_padding=egui::vec2(4.0,2.0);
        let button=|text|egui::Button::new(egui::RichText::new(text).size(13.0).color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))))
            .fill(settings_color(ctx,Color32::from_rgb(48,48,51),Color32::from_rgb(229,232,238))).stroke(egui::Stroke::NONE).corner_radius(8.0);
        let decrease=ui.add_enabled(*value>min,button("−").min_size(egui::vec2(28.0,28.0))).on_hover_text("减少");
        if decrease.clicked(){
            *value=N::from_f64((value.to_f64()-step.to_f64()).max(min.to_f64()));changed=true;
        }
        let field=ui.add_sized([field_width,28.0],egui::DragValue::new(value).range(min..=max).speed(step.to_f64()).suffix(suffix).fixed_decimals(if N::INTEGRAL{0}else{2}));
        changed|=field.changed();
        if ui.add_enabled(*value<max,button("+").min_size(egui::vec2(28.0,28.0))).on_hover_text("增加").clicked(){
            *value=N::from_f64((value.to_f64()+step.to_f64()).min(max.to_f64()));changed=true;
        }
    });
    changed
}
const SETTINGS_GROUPS: [(&str,usize,usize);7] = [
    ("常规",0,0), ("页面管理",7,7), ("音乐与歌词",1,1), ("专注",2,2),
    ("待办与置物",3,4), ("通知",5,5), ("关于",6,6),
];

/// 侧栏分组的图标：Lucide 24 网格线性图标，不铺底色。
/// 单色渲染，颜色跟随该行的文字前景色（浅色主题近黑、暗色主题浅灰，选中行用高对比色），
/// 类似 macOS 系统设置的单色侧栏；24 网格四周有留白，因此按行高的 80% 绘制，视觉尺寸约 14px。
fn paint_settings_group_icon(ui:&egui::Ui,center:egui::Pos2,size:f32,group:usize,ink:Color32){
    let icon=match group {
        0=>egui::include_image!("../assets/settings-icons/system/settings.svg"),
        7=>egui::include_image!("../assets/settings-icons/system/layout-grid.svg"),
        1=>egui::include_image!("../assets/settings-icons/system/music.svg"),
        2=>egui::include_image!("../assets/settings-icons/system/target.svg"),
        3|4=>egui::include_image!("../assets/settings-icons/system/list-todo.svg"),
        5=>egui::include_image!("../assets/settings-icons/system/bell.svg"),
        _=>egui::include_image!("../assets/settings-icons/system/info.svg"),
    };
    let glyph=egui::Rect::from_center_size(center,egui::vec2(size*0.80,size*0.80));
    let egui::ImageSource::Bytes{uri,bytes}=icon else{return;};
    // 按物理像素为每个 DPI 单独光栅化，缓存键带尺寸避免不同缩放互相串用。
    let pixels=(glyph.width()*ui.ctx().pixels_per_point()).ceil() as u32;
    let image=egui::Image::from_bytes(format!("{uri}/{pixels}.svg"),bytes).fit_to_exact_size(glyph.size());
    if let Ok(egui::load::TexturePoll::Ready{texture})=image.load_for_size(ui.ctx(),glyph.size()) {
        // 图标本身是白色描边，用 tint 相乘即可得到任意前景色。
        ui.painter().image(texture.id,glyph,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),ink);
    }
}

impl App {
    pub fn settings_ui(&mut self,ctx:&egui::Context,autostart:bool){
        self.updates.poll(ctx);
        self.updates.dialog_ui(ctx);
        if !self.settings_open{
            // 关闭后模糊底板窗口不会跟着消失，必须主动收起，否则桌面上会留下一块模糊方块。
            // 只隐藏、不销毁：Compositor/D3D 初始化很贵，复用后下次打开能立刻出玻璃。
            #[cfg(windows)]
            if let Some(backdrop)=&mut self.settings_backdrop{backdrop.hide();}
            return;
        }
        let id=egui::ViewportId::from_hash_of("capsuledock-settings");
        let focus_requested=std::mem::take(&mut self.settings_focus_requested);
        ctx.show_viewport_immediate(id,egui::ViewportBuilder::default().with_title("胶囊坞设置").with_icon(theme::app_icon()).with_inner_size([690.0,560.0]).with_min_inner_size([600.0,450.0]).with_visible(false).with_active(false).with_window_level(egui::WindowLevel::Normal).with_decorations(false).with_transparent(true),|ctx,_|{
            let dark=self.settings.settings_theme.dark(crate::settings_theme::system_dark(ctx));
            let original_style=ctx.style();
            // 克隆整个 Style 是每帧的大头，只在主题真正变化时才做，渲染完再恢复坞体样式。
            let theme_changed=original_style.visuals.dark_mode!=dark;
            if theme_changed {
                let mut style=(*original_style).clone();
                style.visuals=if dark{egui::Visuals::dark()}else{egui::Visuals::light()};
                ctx.set_style(style);
            }
            // 视图主题命令每帧照发：视图可能被重建或在最小化/失焦后被恢复，只在样式变化时发会漏掉这些场景。
            ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(if dark{egui::SystemTheme::Dark}else{egui::SystemTheme::Light}));
            if self.settings.settings_theme==crate::settings_theme::SettingsTheme::System {
                ctx.request_repaint_after(std::time::Duration::from_secs(1));
            }
            if focus_requested {
                // Glow 在首帧交换缓冲后执行窗口命令，避免先显示白色空窗口。
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            if ctx.input(|i|i.viewport().close_requested()){self.settings_open=false;}
            // 无边框窗口：裁圆角、铺玻璃底板，并自绘一条融入玻璃的标题栏。
            self.settings_window_chrome(ctx);
            let before=format!("{:?}",self.settings);
            // Big Sur 的侧边栏是一块更明显的 vibrancy 面板，用一层淡雾和内容区分开。
            egui::SidePanel::left("settings-sidebar").resizable(false).exact_width(180.0).frame(egui::Frame::new().fill(settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,18),Color32::from_rgba_unmultiplied(24,31,40,22))).inner_margin(egui::Margin{left:10,right:10,top:46,bottom:12})).show(ctx,|ui|{
                ui.add_space(5.0);
                ui.label(egui::RichText::new("设置").size(11.0).color(settings_color(ctx,Color32::from_gray(150),Color32::from_gray(105))));
                ui.add_space(7.0);
                let mut draw_row=|ui:&mut egui::Ui,rect:egui::Rect,response:&egui::Response,name:&str,start:usize,end:usize|{
                    let selected=(start..=end).contains(&self.settings_tab);
                    if response.clicked() && !selected{self.settings_tab=start;}
                    if selected || response.hovered(){
                        ui.painter().rect_filled(rect,8.0,if selected{settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,30),Color32::from_rgba_unmultiplied(0,0,0,16))}else{settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,14),Color32::from_rgba_unmultiplied(0,0,0,8))});
                    }
                    let ink=if selected{settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))}else{settings_color(ctx,Color32::from_gray(225),Color32::from_gray(55))};
                    paint_settings_group_icon(ui,egui::pos2(rect.left()+18.5,rect.center().y),21.0,start,ink);
                    ui.painter().text(egui::pos2(rect.left()+36.0,rect.center().y),egui::Align2::LEFT_CENTER,name,egui::FontId::proportional(12.5),ink);
                };
                for (name,start,end) in SETTINGS_GROUPS {
                    if start==6 {continue;} // “关于”固定贴底，不参与常规排列
                    let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),34.0),egui::Sense::click());
                    draw_row(ui,rect,&response,name,start,end);
                    ui.add_space(3.0);
                }
                if let Some((name,start,end))=SETTINGS_GROUPS.iter().find(|(_,start,_)|*start==6).copied(){
                    // 用内容区底部定位，避免窗口高度变化时这一行跑出可视区域。
                    let bottom=ui.max_rect().bottom();
                    let rect=egui::Rect::from_min_size(egui::pos2(ui.max_rect().left(),(bottom-37.0).max(ui.cursor().top())),egui::vec2(ui.available_width(),34.0));
                    let response=ui.interact(rect,ui.id().with("settings-about-row"),egui::Sense::click());
                    draw_row(ui,rect,&response,name,start,end);
                }
            });
            // 内容区完全透到玻璃上，只靠卡片承载信息层级。
            egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::TRANSPARENT).inner_margin(egui::Margin{left:22,right:22,top:46,bottom:16})).show(ctx,|ui|{
                ui.spacing_mut().item_spacing=egui::vec2(8.0,5.0);
                ui.spacing_mut().button_padding=egui::vec2(10.0,6.0);
                ui.style_mut().text_styles.insert(egui::TextStyle::Small,egui::FontId::proportional(12.0));
                ui.visuals_mut().override_text_color=Some(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41)));
                ui.visuals_mut().extreme_bg_color=settings_color(ctx,Color32::from_rgba_unmultiplied(18,20,24,150),Color32::from_rgba_unmultiplied(255,255,255,210));
                ui.visuals_mut().widgets.inactive.bg_fill=settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,26),Color32::from_rgba_unmultiplied(0,0,0,14));
                ui.visuals_mut().widgets.hovered.bg_fill=settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,42),Color32::from_rgba_unmultiplied(0,0,0,22));
                // 统一按钮：高度 28、圆角 7、内边距 12×5，无描边；默认按钮也套用这套样式。
                ui.spacing_mut().button_padding=egui::vec2(12.0,5.0);
                ui.spacing_mut().interact_size.y=28.0;
                {
                    let fill=settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,26),Color32::from_rgba_unmultiplied(0,0,0,14));
                    let hover=settings_color(ctx,Color32::from_rgba_unmultiplied(255,255,255,42),Color32::from_rgba_unmultiplied(0,0,0,22));
                    let widgets=&mut ui.visuals_mut().widgets;
                    for (w,f) in [(&mut widgets.inactive,fill),(&mut widgets.hovered,hover),(&mut widgets.active,hover)] {
                        w.corner_radius=egui::CornerRadius::same(7);
                        w.weak_bg_fill=f;
                        w.bg_stroke=egui::Stroke::NONE;
                    }
                }
                ui.visuals_mut().selection.bg_fill=settings_color(ctx,Color32::from_rgb(45,105,165),Color32::from_rgb(209,226,248));
                let titles=["常规","音乐与歌词","专注","待办","置物","通知","关于","页面管理"];
                ui.label(egui::RichText::new(titles[self.settings_tab.min(7)]).size(18.0).strong().color(settings_color(ctx,Color32::WHITE,Color32::from_rgb(30,34,41))));
                if let Some((_,start,end))=SETTINGS_GROUPS.iter().find(|(_,start,end)|(*start..=*end).contains(&self.settings_tab)) {
                    if start!=end {
                        ui.add_space(7.0);
                        // 复用统一分段器：宽度随文字自适应，固定 68px 会裁掉较长的中文标签。
                        let options:Vec<(usize,&str)>=(*start..=*end).map(|index|(index,titles[index])).collect();
                        settings_segmented(ui,&mut self.settings_tab,&options,68.0);
                    }
                }
                ui.add_space(8.0);
                egui::ScrollArea::vertical().id_salt(("settings-scroll",self.settings_tab)).auto_shrink([false,false]).show(ui,|ui|{
                    // 右侧给滚动条留出空隙，卡片和开关不会被它压住。
                    ui.set_width((ui.available_width()-12.0).max(240.0));
                    match self.settings_tab {
                    7=>{
                        ui.label(egui::RichText::new("选择需要的页面和状态组件。").size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                        settings_label(ui,"常驻排列");
                        settings_card(ui,|ui|self.resident_order_settings(ui));
                        settings_label(ui,"状态组件");
                        settings_card(ui,|ui|{
                            setting_row(ui,"电量","本机电量、充电状态与提醒；无电池设备自动隐藏",&mut self.settings.show_battery);
                            settings_separator(ui);
                            setting_row(ui,"电脑状态","坞内常驻显示 CPU 与内存占用圆环",&mut self.settings.show_sysmon);
                            settings_separator(ui);
                            setting_row(ui,"坞上歌词","收起时显示正在唱的那一句",&mut self.settings.island_lyrics);
                        });
                        settings_label(ui,"功能页面");
                        settings_card(ui,|ui|{
                            let descriptions=["播放控制与同步歌词","番茄钟与休息计时","临时收纳文件位置","任务清单与到期提醒"];
                            ui.label(egui::RichText::new("拖动左侧手柄调整顺序，松开后自动应用到坞。").size(11.5).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            #[derive(Clone)]
                            struct PageDrag(usize);
                            let mut movement=None;
                            let row_w=ui.available_width();
                            for (index,page) in self.settings.page_order.into_iter().enumerate() {
                                if index>0 {settings_separator(ui);}
                                let mut visible=self.settings.visible_pages[page];
                                let can_change=!visible || self.settings.visible_pages.iter().filter(|enabled|**enabled).count()>1;
                                let row=ui.push_id(("page-order",page),|ui|ui.horizontal(|ui|{
                                    ui.set_max_width(row_w);
                                    let (rect,handle)=ui.allocate_exact_size(egui::vec2(20.0,40.0),egui::Sense::drag());
                                    for x in [-3.0,3.0] {for y in [-6.0,0.0,6.0] {
                                        ui.painter().circle_filled(rect.center()+egui::vec2(x,y),1.4,settings_color(ctx,Color32::from_gray(155),Color32::from_gray(100)));
                                    }}
                                    handle.on_hover_cursor(egui::CursorIcon::Grab).dnd_set_drag_payload(PageDrag(page));
                                    ui.add_enabled_ui(can_change,|ui|{setting_row(ui,PAGE_NAMES[page],descriptions[page],&mut visible);});
                                })).inner.response;
                                if let Some(payload)=row.dnd_hover_payload::<PageDrag>() {
                                    if payload.0!=page {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                        let y=if self.settings.page_order.iter().position(|p|*p==payload.0).unwrap()<index {row.rect.bottom()}else{row.rect.top()};
                                        ui.painter().line_segment([egui::pos2(row.rect.left(),y),egui::pos2(row.rect.right(),y)],egui::Stroke::new(2.0,Color32::from_rgb(70,150,255)));
                                    }
                                }
                                if let Some(payload)=row.dnd_release_payload::<PageDrag>() {movement=Some((payload.0,index));}
                                if visible!=self.settings.visible_pages[page] {self.set_page_visible(page,visible);}
                            }
                            if let Some((page,to))=movement {
                                let from=self.settings.page_order.iter().position(|p|*p==page).unwrap();
                                if from<to {self.settings.page_order[from..=to].rotate_left(1);}
                                else {self.settings.page_order[to..=from].rotate_right(1);}
                            }
                            if self.settings.visible_pages.iter().filter(|enabled|**enabled).count()==1 {
                                ui.add_space(5.0);
                                ui.label(egui::RichText::new("至少保留一个页面。关闭仅隐藏页面，数据和提醒设置保留。").size(11.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            }
                        });
                    }
                    0=>{
                        settings_label(ui,"设置页外观");
                        settings_card(ui,|ui|{
                            use crate::settings_theme::SettingsTheme;
                            settings_control_row_right(ui,"主题",|ui|{
                                if settings_segmented(ui,&mut self.settings.settings_theme,&[(SettingsTheme::System,"跟随系统"),(SettingsTheme::Light,"浅色"),(SettingsTheme::Dark,"深色")],52.0) {ctx.request_repaint();}
                            });
                            settings_hint(ui,"仅影响设置页，坞的外观保持不变。");
                        });
                        settings_label(ui,"启动");
                        settings_card(ui,|ui|{
                            let mut a=autostart;
                            setting_row(ui,"开机启动","登录 Windows 时自动打开胶囊坞",&mut a);
                            if a!=autostart{self.pending.push(Cmd::ToggleAutostart);}
                        });
                        settings_label(ui,"坞体外观");
                        settings_card(ui,|ui|{
                            use crate::model::{self as model,Skin};
                            let skins:Vec<(Skin,&str)>=Skin::ALL.iter().map(|s|(*s,s.label())).collect();
                            settings_control_row_right(ui,"皮肤",|ui|{settings_segmented(ui,&mut self.settings.skin,&skins,64.0);});
                            settings_hint(ui,self.settings.skin.description());
                            settings_separator(ui);
                            let shapes:Vec<(model::IslandShape,&str)>=model::IslandShape::ALL.iter().map(|s|(*s,s.label())).collect();
                            settings_control_row_right(ui,"形状",|ui|{settings_segmented(ui,&mut self.settings.island_shape,&shapes,64.0);});
                            settings_hint(ui,self.settings.island_shape.description());
                            if self.settings.skin.is_glass() {
                                settings_separator(ui);
                                let mut blur=self.settings.glass_blur;
                                setting_row(ui,"背景模糊","使用 Windows 合成引擎的实时模糊，严格贴合坞体形状（需开启系统“透明效果”）；关闭后改为烟熏半透明",&mut blur);
                                self.settings.glass_blur=blur;
                            }
                            settings_separator(ui);
                            settings_control_row(ui,"大小",56.0,|ui|{
                                ui.horizontal(|ui|{
                                    ui.spacing_mut().slider_width=180.0;
                                    island_size_slider(ui,&mut self.settings.island_size);
                                    if ui.add_enabled(self.settings.island_size!=100,egui::Button::new("恢复默认")).clicked(){self.settings.island_size=100;}
                                }).response
                            });
                            settings_hint(ui,"文字、图标和吉祥物按实际像素重新绘制，放大后依旧清晰；只影响坞体，不影响设置窗口。");
                        });
                        settings_label(ui,"坞体显示");
                        settings_card(ui,|ui|{
                            let selected=self.displays.iter().find(|d|d.id==self.settings.display_id);
                            let label=if self.settings.display_id.is_empty(){"跟随主屏"}else{selected.map_or("所选屏幕未连接",|d|d.label.as_str())};
                            settings_control_row(ui,"显示屏",56.0,|ui|{
                                egui::ComboBox::from_id_salt("island-display").selected_text(label).width(ui.available_width().min(300.0)).truncate().show_ui(ui,|ui|{
                                    ui.selectable_value(&mut self.settings.display_id,String::new(),"跟随主屏");
                                    for display in &self.displays {ui.selectable_value(&mut self.settings.display_id,display.id.clone(),&display.label);}
                                }).response
                            });
                            let edge=self.settings.dock.label();
                            ui.label(egui::RichText::new(if !self.settings.display_id.is_empty()&&selected.is_none(){"暂时显示在主屏，重新连接所选屏幕后自动恢复。".to_string()}else{format!("显示在所选屏幕{edge}；断开时暂时回到主屏。")}).size(11.0).color(ui.visuals().weak_text_color()));
                            ui.add_space(4.0);
                            // 停靠边缘：顶部为横向；左右为竖向排列，展开面板从边缘滑出。
                            let docks:Vec<(crate::model::Dock,&str)>=crate::model::Dock::ALL.iter().map(|d|(*d,d.label())).collect();
                            settings_control_row_right(ui,"停靠",|ui|{
                                // 换边后偏移的含义变了（水平 ↔ 上下），回到居中。
                                if settings_segmented(ui,&mut self.settings.dock,&docks,64.0) {self.settings.island_offset=0;}
                            });
                            if self.settings.dock.is_side() {
                                ui.label(egui::RichText::new("收起时竖向排列时间、电池、播放、专注和提醒铃铛；展开面板从屏幕边缘滑出。吉祥物和坞上歌词只在顶部停靠时显示。").size(11.0).color(ui.visuals().weak_text_color()));
                            }
                            ui.add_space(4.0);
                            let side=self.settings.dock.is_side();
                            settings_control_row(ui,if side{"上下位置"}else{"水平位置"},56.0,|ui|{
                                ui.horizontal(|ui|{
                                    let limit=crate::model::ISLAND_OFFSET_MAX;
                                    let offset=&mut self.settings.island_offset;
                                    let (less,more,less_tip,more_tip)=if side{("↑","↓","向上微调","向下微调")}else{("←","→","向左微调","向右微调")};
                                    if ui.add_enabled(*offset>-limit,egui::Button::new(less).min_size(egui::vec2(26.0,22.0))).on_hover_text(less_tip).clicked(){*offset=(*offset-5).max(-limit);}
                                    ui.spacing_mut().slider_width=150.0;
                                    ui.add(egui::Slider::new(&mut *offset,-limit..=limit).show_value(false));
                                    if ui.add_enabled(*offset<limit,egui::Button::new(more).min_size(egui::vec2(26.0,22.0))).on_hover_text(more_tip).clicked(){*offset=(*offset+5).min(limit);}
                                    if ui.add_enabled(*offset!=0,egui::Button::new("恢复居中")).clicked(){*offset=0;}
                                }).response
                            });
                            let offset=self.settings.island_offset;
                            let (less,more)=if side{("偏上","偏下")}else{("偏左","偏右")};
                            let position=match offset {0=>"居中".to_string(),o if o<0=>format!("{less} {}%",-o),o=>format!("{more} {o}%")};
                            ui.label(egui::RichText::new(format!("当前：{position}。展开面板和通知始终留在屏幕内；换屏或调整大小后保持相对位置。")).size(11.0).color(ui.visuals().weak_text_color()));
                            settings_separator(ui);
                            settings_control_row_right(ui,"时间制式",|ui|{settings_segmented(ui,&mut self.settings.clock_12h,&[(false,"24 小时"),(true,"12 小时")],72.0);});
                            settings_control_row_right(ui,"时段文字",|ui|{settings_segmented(ui,&mut self.settings.clock_period,&[(0,"不显示"),(1,"上午 / 下午"),(2,"详细时段")],72.0);});
                            settings_hint(ui,"详细时段：凌晨、上午、中午、下午、傍晚、晚上");
                            settings_separator(ui);
                            let mut minimal=self.settings.minimal;
                            setting_row(ui,"极简刘海","收起时显示一条细线",&mut minimal);
                            if minimal!=self.settings.minimal {self.set_minimal(minimal);}
                            settings_separator(ui);

                            let mut pet=self.settings.pet;
                            setting_row(ui,"胶囊坞吉祥物","收起时常驻，播放音乐时一起唱歌",&mut pet);
                            if pet!=self.settings.pet {self.set_pet(pet);}
                        });
                        self.avoidance.settings(ui,&mut self.settings);
                        settings_label(ui,"声音与显示");
                        settings_card(ui,|ui|{
                            setting_row(ui,"提醒声音","待办和专注提醒",&mut self.settings.sound);
                            settings_separator(ui);
                            setting_row(ui,"全屏时隐藏","游戏、视频和演示全屏时隐藏",&mut self.settings.hide_fullscreen);
                        });
                    }
                    1=>{
                        settings_label(ui,"歌词");
                        settings_card(ui,|ui|{
                            setting_row(ui,"展开时显示歌词","在播放器右侧同步滚动显示歌词",&mut self.settings.lyrics_enabled);
                            self.show_lyrics=self.settings.lyrics_enabled;
                            settings_separator(ui);
                            setting_row(ui,"坞上歌词","收起时显示当前句；无同步歌词时恢复普通坞",&mut self.settings.island_lyrics);
                        });
                        settings_label(ui,"歌词显示时间");
                        settings_card(ui,|ui|{
                            let mut advance=-self.settings.lyric_delay_ms as f64/1000.0;
                            if setting_number(ui,"时间偏移",&mut advance,-5.0,5.0," 秒",0.1){self.settings.lyric_delay_ms=(-advance*1000.0).round()as i32;}
                            ui.add_space(3.0);
                            ui.small("正数提前，负数延后；默认提前 0.35 秒。");
                        });
                    }
                    2=>{
                        settings_label(ui,"计时时长");
                        settings_card(ui,|ui|{
                            setting_number(ui,"专注",&mut self.settings.focus_minutes,1,180," 分钟",1.0);
                            settings_separator(ui);
                            setting_number(ui,"休息",&mut self.settings.rest_minutes,1,60," 分钟",1.0);
                            ui.add_space(3.0);
                            ui.small("调整后从下一阶段开始生效。");
                        });
                    }
                    3=>{
                        settings_label(ui,"待办清单");
                        settings_card(ui,|ui|{
                            ui.label(egui::RichText::new("在待办页添加、编辑任务并设置提醒时间。").size(13.0));
                            ui.add_space(5.0);
                            if ui.add_sized([120.0,32.0],egui::Button::new("打开待办页")).clicked(){self.settings_open=false;self.show_page(crate::app::TODO_PAGE);}
                        });
                    }
                    4=>{
                        settings_label(ui,"置物架");
                        settings_card(ui,|ui|{
                            ui.label(egui::RichText::new("保存文件位置，移除条目不会删除原文件。").size(13.0));
                            ui.label(egui::RichText::new(format!("当前 {} 项",self.shelf.items.len())).size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            ui.add_space(5.0);
                            if ui.add_sized([120.0,32.0],egui::Button::new("打开置物架")).clicked(){self.settings_open=false;self.shelf_view.shelf_tab=0;self.show_page(2);}
                        });
                        settings_label(ui,"剪贴板");
                        settings_card(ui,|ui|{
                            setting_row(ui,"记录剪贴板","在置物架的「剪贴板」里查看最近复制的文字，点一下再次复制",&mut self.settings.clipboard_history);
                            ui.label(egui::RichText::new(format!("本机保存最近 {} 条，重新打开后可继续使用。关闭记录会暂停收集，手动清空会保留固定条目。密码管理器标记为敏感的内容不会记录。",crate::clipboard::MAX_RECENT)).size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            ui.add_space(5.0);
                            if ui.add_sized([120.0,32.0],egui::Button::new("打开剪贴板")).clicked(){self.settings_open=false;self.shelf_view.shelf_tab=1;self.show_page(2);}
                        });
                    }
                    5=>{
                        settings_label(ui,"系统提示");
                        settings_card(ui,|ui|{
                            setting_row(ui,"系统状态","音量、充电、网络和键盘状态",&mut self.settings.system_hud);
                            settings_separator(ui);
                            setting_row(ui,"蓝牙通知","设备连接与断开提醒；关闭后停止蓝牙监听",&mut self.settings.bluetooth_notifications);
                            settings_separator(ui);
                            setting_row(ui,"胶囊坞提醒","在坞上显示专注和待办提醒",&mut self.settings.notifications);
                        });
                    }
                    _=>{
                        settings_label(ui,"胶囊坞 CapsuleDock");
                        settings_card(ui,|ui|{
                            ui.horizontal(|ui|{
                                ui.label(egui::RichText::new(format!("版本 {}",env!("CARGO_PKG_VERSION"))).size(13.0));
                                ui.add_space(8.0);
                                ui.vertical(|ui|self.updates.ui(ui));
                            });
                            ui.label(egui::RichText::new("桌面顶部的音乐、专注与待办小助手。").size(12.0).color(settings_color(ctx,Color32::from_gray(180),Color32::from_gray(75))));
                            ui.horizontal_wrapped(|ui|{
                                if ui.button("打开本地数据目录").clicked(){self.pending.push(Cmd::Open(crate::model::config_dir()));}
                                ui.label(egui::RichText::new("设备图标：Solar Icons · 480 Design").size(11.0).color(settings_color(ctx,Color32::from_gray(150),Color32::from_gray(105))));
                                ui.hyperlink_to("CC BY 4.0", "https://creativecommons.org/licenses/by/4.0/");
                            });
                        });
                    }
                    }
                });
            });
            self.avoidance.dialog(ctx,&mut self.settings);
            if before!=format!("{:?}",self.settings){self.settings.save();}
            if theme_changed {ctx.set_style(original_style);}
        });
    }

    /// 设置窗口的无边框外观：圆角裁剪 + 玻璃底板 + 融入玻璃的极简标题栏。
    ///
    /// 标题栏不占独立区域，直接压在内容之上，只有一行标题和右侧的最小化/关闭按钮，
    /// 靠拖拽整条空白带移动窗口。
    fn settings_window_chrome(&mut self,ctx:&egui::Context){
        let dark=ctx.style().visuals.dark_mode;
        let window=ctx.input(|i|i.viewport().inner_rect).map_or_else(
            || egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(690.0,560.0)),
            |r|egui::Rect::from_min_size(egui::Pos2::ZERO,r.size()),
        );
        #[cfg(windows)]
        {
            let scale=ctx.pixels_per_point();
            // 句柄缓存：按标题枚举窗口每帧都要扫一遍线程窗口，窗口多时是固定的几百微秒；
            // 命中缓存只做一次 IsWindow + 标题比对，失效再重新找。
            let hwnd_key=egui::Id::new("capsuledock-settings-hwnd");
            let cached=ctx.data(|d|d.get_temp::<isize>(hwnd_key));
            let hwnd=match cached {
                Some(raw) if crate::native::window_is_titled(raw,"胶囊坞设置")=>Some(crate::native::window_from_raw(raw)),
                _=>{
                    if cached.is_some(){ctx.data_mut(|d|d.remove_temp::<isize>(hwnd_key));}
                    crate::frameless::find_thread_window("胶囊坞设置")
                }
            };
            if let Some(hwnd)=hwnd {ctx.data_mut(|d|d.insert_temp(hwnd_key,hwnd.0 as isize));}
            // 圆角直径 24 逻辑点，与坞体、更新弹窗的视觉语言一致。
            let clipped=match hwnd {Some(hwnd)=>crate::frameless::clip_window(hwnd,window,scale,24.0),None=>false};
            // 模糊不可用时退回不透明背景，避免整窗透明看不见内容。
            let glass=!self.settings_blur_unavailable && clipped;
            if glass && self.settings_backdrop.is_none() {
                match crate::composition::Backdrop::new() {
                    Ok(backdrop)=>{
                        crate::frameless::glass_diagnostic("设置窗口模糊底板已创建");
                        self.settings_backdrop=Some(backdrop);
                    }
                    Err(error)=>{
                        crate::frameless::glass_diagnostic(&format!("设置窗口模糊底板创建失败：{error:?}"));
                        self.settings_blur_unavailable=true;
                    }
                }
            }
            if let Some(hwnd)=hwnd {
                // 关键：DwmEnableBlurBehindWindow / DwmSetWindowAttribute 每帧调用会强制 DWM
                // 反复重建窗口合成，整个桌面都会跟着掉帧。只在窗口创建或从最小化恢复时应用一次。
                let raw=hwnd.0 as isize;
                let key=egui::Id::new("capsuledock-settings-native");
                let (last,was_minimized)=ctx.data(|d|d.get_temp::<(isize,bool)>(key)).unwrap_or((isize::MIN,false));
                let minimized=ctx.input(|i|i.viewport().minimized.unwrap_or(false));
                if raw!=last || (was_minimized && !minimized) {
                    let _=crate::native::enable_alpha_composition(hwnd);
                    // 去掉 Win11 的 1px 系统边框，否则玻璃顶部会有一条白线。
                    crate::native::disable_window_frame_border(hwnd);
                }
                ctx.data_mut(|d|d.insert_temp(key,(raw,minimized)));
            }
            if let Some(backdrop)=&mut self.settings_backdrop {
                let pieces=[crate::app::BlurPiece{x:0.0,y:0.0,w:window.width()*scale,h:window.height()*scale,radius:12.0*scale}];
                // 视图重建的头一帧可能还拿不到 HWND，跳过即可，不能因此永久禁用玻璃。
                if let Some(hwnd)=hwnd {
                    if let Err(error)=backdrop.show(hwnd,&pieces) {
                        crate::frameless::glass_diagnostic(&format!("设置窗口模糊层更新失败：{error:?}"));
                        self.settings_backdrop=None;
                        self.settings_blur_unavailable=true;
                    }
                }
            }
        }
        // 玻璃着色必须画进与侧边栏/内容区同一个背景层，且在本函数里先于面板提交，
        // 面板才会盖在着色之上；另起图层会整片盖住内容。
        let painter=ctx.layer_painter(egui::LayerId::background());
        #[cfg(windows)]
        let blurred=!self.settings_blur_unavailable;
        #[cfg(not(windows))]
        let blurred=false;
        // 底色：Big Sur 的窗口底是偏灰的白，透明度适中；亮壁纸下也不会整窗发白。
        let tint_alpha=if blurred{if dark{0.62}else{0.60}}else{1.0};
        let tint=settings_color(ctx,Color32::from_rgb(20,22,26),Color32::from_rgb(246,247,249));
        painter.rect_filled(window,12.0,crate::paint::alpha(tint,tint_alpha));
        // 不做顶部柔光：Big Sur 是均匀的 vibrancy，顶部额外提亮会形成一条突兀的白带。
        // 边缘：用一条很细的亮边收口，顶部镜面保持克制。
        let rim=settings_color(ctx,Color32::from_white_alpha((crate::glass::rim_alpha()*255.0*0.72).round() as u8),Color32::from_black_alpha(22));
        painter.rect_stroke(window,12.0,egui::Stroke::new(1.0,rim),egui::StrokeKind::Inside);
        painter.rect_stroke(window.shrink(1.0),11.0,egui::Stroke::new(1.0,settings_color(ctx,Color32::from_white_alpha(9),Color32::from_white_alpha(26))),egui::StrokeKind::Inside);
        let glint=settings_color(ctx,Color32::from_white_alpha(46),Color32::from_white_alpha(64));
        painter.line_segment([egui::pos2(window.left()+16.0,window.top()+0.7),egui::pos2(window.right()-16.0,window.top()+0.7)],egui::Stroke::new(1.0,glint));
        // 侧边栏与内容区之间的发丝线。
        painter.vline(180.0,egui::Rangef::new(window.top()+12.0,window.bottom()-12.0),egui::Stroke::new(1.0,settings_color(ctx,Color32::from_white_alpha(14),Color32::from_black_alpha(12))));
        // 标题栏：拖拽带 + 标题 + 窗口按钮。
        let title_h=38.0;
        let drag=egui::Rect::from_min_size(window.min,egui::vec2(window.width(),title_h));
        let close=egui::Rect::from_min_size(egui::pos2(window.right()-40.0,window.top()+7.0),egui::vec2(26.0,26.0));
        let minimize=egui::Rect::from_min_size(egui::pos2(close.left()-30.0,window.top()+7.0),egui::vec2(26.0,26.0));
        let layer=egui::LayerId::new(egui::Order::Middle,egui::Id::new("settings-chrome-ui"));
        let ui=egui::Ui::new(ctx.clone(),egui::Id::new("settings-chrome-ui"),egui::UiBuilder::new().layer_id(layer).max_rect(drag));
        let drag_zone=egui::Rect::from_min_max(drag.min,egui::pos2(minimize.left()-6.0,drag.max.y));
        if ui.interact(drag_zone,egui::Id::new("settings-drag"),egui::Sense::drag()).drag_started(){
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        let ink=settings_color(ctx,Color32::from_gray(225),Color32::from_gray(40));
        ui.painter().text(egui::pos2(window.left()+18.0,drag.center().y),egui::Align2::LEFT_CENTER,"胶囊坞设置",egui::FontId::proportional(12.5),ink);
        for (rect,kind) in [(minimize,0u8),(close,1u8)] {
            let response=ui.interact(rect,egui::Id::new(("settings-winbtn",kind)),egui::Sense::click());
            if response.hovered(){ui.painter().rect_filled(rect,7.0,settings_color(ctx,Color32::from_white_alpha(40),Color32::from_black_alpha(22)));}
            let center=rect.center();
            if kind==0 {
                ui.painter().line_segment([center+egui::vec2(-5.0,0.0),center+egui::vec2(5.0,0.0)],egui::Stroke::new(1.3,ink));
            } else {
                for direction in [-1.0,1.0] {
                    ui.painter().line_segment([center+egui::vec2(-4.0,-4.0*direction),center+egui::vec2(4.0,4.0*direction)],egui::Stroke::new(1.3,ink));
                }
            }
            if response.clicked() {
                if kind==0 {ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));}
                else {self.settings_open=false;}
            }
        }
    }
}
