//! 无窗口交互测试：直接向 egui 注入输入事件，验证悬停展开、切页、按钮。
use crate::app::{App, Env, Options, PH, PW, WIN_H, WIN_W};
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn opaque_surface_gets_a_contour_with_the_classic_skin(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.skin=crate::model::Skin::Classic;
    for opaque_surface in [false,true] {
        let mut outlined=false;
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::app::CANVAS_W,WIN_H))),..Default::default()};
        let _=h.ctx.run(raw,|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));
                let out=h.app.frame(ui,win,&Env{opaque_surface,..Default::default()});
                outlined=out.glass.is_some_and(|shapes|!shapes.is_empty());
            });
        });
        assert_eq!(outlined,opaque_surface);
    }
}

#[test]
fn changing_qq_metadata_cannot_spawn_lyrics_workers_while_one_is_running(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let (release,wait)=std::sync::mpsc::channel();
    let worker=std::thread::spawn(move||{let _=wait.recv();});
    let id=worker.thread().id();h.app.ly_worker=Some(worker);
    h.app.opts.fetch_lyrics=true;h.app.settings.island_lyrics=true;
    for step in 0..1440 {
        h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:format!("歌曲{}",step%2),artist:"歌手".into(),duration:120.0,playing:true,..Default::default()};
        h.t=step as f64*0.25;h.frame(vec![]);
        assert_eq!(h.app.ly_worker.as_ref().unwrap().thread().id(),id,"旧请求未结束时不得另开取词线程");
        assert!(h.app.ly.rx.is_none(),"不能为抖动的歌曲堆积请求");
    }
    release.send(()).unwrap();h.app.ly_worker.take().unwrap().join().unwrap();
}

#[test]
fn v0649_settings_survive_upgrade(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let _h=H::new();
    let directory=crate::model::config_dir();
    std::fs::create_dir_all(&directory).unwrap();
    let legacy=concat!("settings_theme=dark\npet=0\nminimal=1\nbrowser_minimal=1\n",
        "visible_pages=10111111\npage_order=5,2,0,1,3,4,6,7\nshow_battery=0\n",
        "system_hud=0\nnotifications=0\nsound=1\n",
        "lyrics_enabled=0\nisland_lyrics=1\nlyric_delay_ms=-650\nhide_fullscreen=0\n",
        "focus_minutes=45\nrest_minutes=10\n");
    std::fs::write(directory.join("settings.txt"),legacy).unwrap();
    let upgraded=crate::model::Settings::load();
    assert!(upgraded.display_id.is_empty() && upgraded.avoid_apps.is_empty());
    assert!(upgraded.avoid_browsers && upgraded.bluetooth_notifications);
    assert!(!upgraded.clock_12h);
    assert_eq!(upgraded.clock_period,0);
    // 旧页面布局升级：饮水(2)、邮箱(5)、AI 额度(6)、日程(7) 都已下线，只保留 音乐／专注／置物／待办。
    assert_eq!(upgraded.visible_pages,[true,false,true,true]);
    assert_eq!(upgraded.page_order,[0,1,2,3]);
    for _ in 0..2 {
        crate::model::Settings::load().save();
        let saved=std::fs::read_to_string(directory.join("settings.txt")).unwrap();
        for line in legacy.lines().filter(|line|!line.starts_with("visible_pages")&&!line.starts_with("page_order")){
            assert!(saved.lines().any(|value|value==line),"升级保存不能改变旧设置：{line}");
        }
    }
}

#[test]
fn switching_music_source_discards_previous_lyrics() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
    h.frame(vec![]);
    for source in ["QQMusic.exe","cloudmusic.exe","汽水音乐","KuGou.exe","QQMusic.exe"] {
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:01]旧来源歌词"));
        h.media.source=source.into();
        h.frame(vec![]);
        assert!(h.app.ly.lyrics.is_none(),"切换到 {source} 后必须重新选择歌词来源");
    }
}

#[test]
fn closing_a_player_then_starting_native_netease_restores_music_and_lyrics(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{source:"SodaMusic.exe".into(),title:"旧歌曲".into(),artist:"歌手".into(),playing:true,duration:120.0,..Default::default()};h.frame(vec![]);
    h.app.set_lyrics("旧歌曲","歌手",120.0,crate::lyrics::parse_lrc("[00:01]旧歌词"));
    h.media=Default::default();h.frame(vec![]);assert!(h.app.media.title.is_empty());assert!(h.app.ly.lyrics.is_none());
    h.media=crate::smtc::NowPlaying{source:"cloudmusic.exe".into(),song_id:Some(123),title:"新歌曲".into(),artist:"歌手".into(),playing:true,position:7.0,duration:120.0,..Default::default()};h.frame(vec![]);
    h.app.set_lyrics("新歌曲","歌手",120.0,crate::lyrics::parse_lrc("[00:05]新歌词\n[00:10]下一句"));h.frame(vec![]);
    assert_eq!(h.app.media.title,"新歌曲");assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("新歌词"));
    h.media.song_id=Some(124);h.frame(vec![]);assert!(h.app.ly.lyrics.is_none(),"歌名相同但歌曲 ID 不同仍需重新取词");
}

#[test]
fn settings_keep_dark_buttons_when_system_theme_changes() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let h=H::new();
    for theme in [egui::Theme::Light,egui::Theme::Dark,egui::Theme::Light] {
        let _=h.ctx.run(egui::RawInput{system_theme:Some(theme),..Default::default()},|ctx| {
            assert!(ctx.style().visuals.widgets.inactive.weak_bg_fill.r()<100,"系统浅色模式不能把深色设置里的按钮变成白底");
        });
    }
}

#[test]
fn expanded_settings_routes_preserve_inline_controls(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (page,tab) in [1,2,4,3].into_iter().enumerate(){
        let mut h=H::new();h.app.show_page(page);h.hover(H::local(200.0,40.0),1.0);
        h.click(H::local(394.0,18.0));
        assert!(h.app.settings_open,"页面 {page} 应打开设置窗口");
        assert_eq!(h.app.settings_tab,tab);assert_eq!(h.app.nav.page,page);
        assert!(h.app.subpage.is_none());
    }
    for (page,point) in [(1,H::local(111.0,18.0))]{
        let mut h=H::new();h.app.show_page(page);h.hover(H::local(200.0,40.0),1.0);
        h.click(point);assert_eq!(h.app.subpage,Some(page));assert!(!h.app.settings_open);
        h.click(H::local(394.0,18.0));assert!(h.app.settings_open);
        assert_eq!(h.app.subpage,Some(page),"打开完整设置后应保留坞内调整页");
    }
    let mut h=H::new();h.app.show_page(2);h.hover(H::local(200.0,40.0),1.0);
    h.click(H::local(330.0,18.0));assert!(h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::AddFiles)));
    assert!(!h.app.settings_open);
}

#[test]
fn quality_distinct_reminders_are_queued_instead_of_overwritten() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.alert("待办到时间了","任务一",3,true);h.app.alert("待办到时间了","任务二",3,true);
    assert_eq!(h.app.notices.alerts.iter().filter(|a|a.title=="待办到时间了").count(),2,"同时到期的两个待办都需要提醒");
    h.app.alert("系统音量","10%",0,false);h.app.alert("系统音量","20%",0,false);
    assert_eq!(h.app.notices.alerts.iter().filter(|a|a.title=="系统音量").count(),1,"系统提示类更新应合并");
}

struct H {
    ctx: egui::Context,
    app: App,
    t: f64,
    cursor: Option<Pos2>,
    cmds:Vec<crate::app::Cmd>,
    region:Rect,
    hit:Rect,
    animating:bool,
    shapes:Vec<egui::epaint::ClippedShape>,
    bt: Vec<crate::bt::Device>,
    media: crate::smtc::NowPlaying,
    power: (Option<u8>, bool),
    network: Option<bool>,
    system_idle: Option<f64>,
    browser_at_top: Option<bool>,
    file_drop_zone:Option<Rect>,
    hovered_files:Vec<egui::HoveredFile>,
    dropped_files:Vec<egui::DroppedFile>,
}

impl H {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("capsuledock-test-{}-{}", std::process::id(), rand_id()));
        std::env::set_var("CAPSULEDOCK_DATA_DIR", &dir);
        let ctx = egui::Context::default();
        let app = App::new(&ctx, Options { fetch_lyrics: false, pet_idle: 1e9 });
        Self { ctx, app, t: 0.0, cursor: None,cmds:vec![],region:Rect::NOTHING,hit:Rect::NOTHING,animating:false,shapes:Vec::new(), bt:Vec::new(),media:Default::default(),power:(None,false),network:None,system_idle:None,browser_at_top:None,file_drop_zone:None,hovered_files:vec![],dropped_files:vec![] }
    }
    fn frame(&mut self, events: Vec<egui::Event>) {
        self.t += 1.0 / 60.0;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(crate::app::CANVAS_W, WIN_H))),
            time: Some(self.t),
            events,
            hovered_files:self.hovered_files.clone(),
            dropped_files:std::mem::take(&mut self.dropped_files),
            focused: true,
            ..Default::default()
        };
        let env = Env {
            t: self.t,
            clock: "09:41:00".into(),
            today: "2026-10-28".into(),
            date: "10月28日 周三".into(),
            cursor: self.cursor,
            bt: self.bt.clone(),
            bt_ready: true,
            media:self.media.clone(),
            battery:self.power.0,
            charging:self.power.1,
            network:self.network,
            idle_seconds:self.system_idle,
            browser_at_top:self.browser_at_top,
            ..Default::default()
        };
        let cmds=&mut self.cmds;
        let region=&mut self.region;
        let hit=&mut self.hit;
        let animating=&mut self.animating;
        let app = &mut self.app;
        let file_drop_zone=&mut self.file_drop_zone;
        let output = self.ctx.run(raw, |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx, |ui| {
                let win = Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));
                let frame=app.frame(ui, win, &env);*region=frame.region;*hit=frame.hit;*animating=frame.animating;*file_drop_zone=frame.file_drop_zone;cmds.extend(frame.cmds);
            });
        });
        self.shapes=output.shapes;
    }
    fn hover(&mut self, p: Pos2, secs: f64) {
        self.cursor = Some(p);
        let n = (secs * 60.0) as usize;
        for _ in 0..n {
            self.frame(vec![egui::Event::PointerMoved(p)]);
        }
    }
    fn click(&mut self, p: Pos2) {
        self.cursor = Some(p);
        self.frame(vec![egui::Event::PointerMoved(p)]);
        self.frame(vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }]);
        self.frame(vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }]);
    }
    fn click_text(&mut self,label:&str){
        if !self.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==label))&&self.app.notices.notice.is_some(){self.click_notice_badge();}
        let point=self.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label&&t.opacity_factor>0.9&&s.clip_rect.contains(t.visual_bounding_rect().center())=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap_or_else(||panic!("未找到可点击文字：{label}"));
        self.click(point);
    }
    fn click_notice_badge(&mut self){if self.app.compact_notices(){self.click(Pos2::new(WIN_W/2.0+(self.app.current_width()-44.0)/2.0+26.0,15.0));}}
    fn wheel(&mut self,delta:f32) {
        self.frame(vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,delta),modifiers:Default::default()}]);
    }
    fn picker_frame(&mut self,events:Vec<egui::Event>){
        self.frame(events);
    }
    fn picker_click(&mut self,label:&str){
        for _ in 0..60 {self.picker_frame(vec![]);if (self.app.progress()-1.0).abs()<0.01 {break;}}
        self.picker_frame(vec![]);
        let find=|h:&H|h.shapes.iter().find_map(|shape|match &shape.shape{
            egui::Shape::Text(text) if text.galley.text()==label && shape.clip_rect.contains(text.visual_bounding_rect().center())=>Some(text.visual_bounding_rect().center()),_=>None,
        });
        let point=find(self).unwrap_or_else(||panic!("未找到可见选项：{label}"));
        assert!(self.hit.contains(point),"选项 {label} 超出坞体命中区域");
        self.picker_frame(vec![egui::Event::PointerMoved(point)]);
        for pressed in [true,false]{self.picker_frame(vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
    }
    fn local(x: f32, y: f32) -> Pos2 {
        Pos2::new((WIN_W - PW) / 2.0 + x, y)
    }
}

fn rand_id() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64
}
#[test]
fn shelf_drop_zone_leaves_normal_hover_and_text_selection_unchanged(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for _ in 0..90{h.frame(vec![]);}let zone=h.file_drop_zone.unwrap();
    assert_eq!(zone.width(),PW);assert_eq!(zone.height(),96.0);
    let point=zone.right_bottom()-Vec2::splat(12.0);assert!(!h.hit.contains(point));
    h.hover(point,0.8);assert!(!h.app.morph.open);
    h.frame(vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    h.hover(point,0.8);assert!(!h.app.morph.open,"按住鼠标拖选文字不应打开置物架");
    h.frame(vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    h.app.settings.visible_pages[2]=false;h.frame(vec![]);assert!(h.file_drop_zone.is_none());
}

#[test]
fn shelf_file_drag_opens_below_the_island_and_cancellation_restores_the_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.go(1,1.0);for _ in 0..90{h.frame(vec![]);}
    let point=h.file_drop_zone.unwrap().right_bottom()-Vec2::splat(12.0);
    assert!(!h.hit.contains(point));h.cursor=Some(point);
    h.hovered_files.push(egui::HoveredFile{path:Some(crate::model::config_dir().join("拖入文件.txt")),..Default::default()});
    h.frame(vec![]);assert!(h.app.morph.open && h.app.nav.page==2 && h.app.drag_hot,"文件进入感应区应立即展开置物架");
    for _ in 0..60{h.frame(vec![]);}assert!(h.app.morph.open);
    h.hovered_files.clear();h.cursor=None;for _ in 0..60{h.frame(vec![]);}
    assert!(!h.app.drag_hot && !h.app.morph.open);assert_eq!(h.app.nav.page,1);
    assert!(h.app.shelf.items.is_empty(),"取消拖入不应保存文件");
}

#[test]
fn shelf_quick_drop_in_the_capture_zone_shows_the_added_file(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    let path=crate::model::config_dir().join("快速拖入.txt");std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,"test").unwrap();
    h.cursor=Some(h.file_drop_zone.unwrap().right_bottom()-Vec2::splat(12.0));
    h.dropped_files.push(egui::DroppedFile{path:Some(path.clone()),..Default::default()});
    for _ in 0..90{h.frame(vec![]);}
    assert!(h.app.morph.open && h.app.nav.page==2);assert_eq!(h.app.shelf.items,vec![path.clone()]);assert!(path.exists());
    let mut h=H::new();for _ in 0..90{h.frame(vec![]);}
    h.cursor=Some(h.file_drop_zone.unwrap().right_bottom()-Vec2::splat(12.0));h.app.settings.visible_pages[2]=false;
    h.dropped_files.push(egui::DroppedFile{path:Some(path),..Default::default()});h.frame(vec![]);
    assert!(!h.app.morph.open && !h.app.settings_open,"关闭置物架后应忽略拖入");
}

#[test]
fn settings_opening_restores_focus_once_without_becoming_topmost() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.settings_theme=crate::settings_theme::SettingsTheme::Dark;
    h.ctx.set_embed_viewports(false);
    let frame=|h:&mut H,focused:bool| {
        egui::Context::set_immediate_viewport_renderer(move |ctx,mut viewport| {
            let id=viewport.ids.this;
            let mut raw=egui::RawInput{viewport_id:id,focused,system_theme:Some(egui::Theme::Light),..Default::default()};
            raw.viewports.insert(id,egui::ViewportInfo{focused:Some(focused),minimized:Some(!focused),..Default::default()});
            let _=ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx));
        });
        let id=egui::ViewportId::from_hash_of("capsuledock-settings");
        let mut raw=egui::RawInput::default();
        raw.viewports.insert(id,egui::ViewportInfo{focused:Some(focused),..Default::default()});
        let mut output=h.ctx.run(raw,|ctx|h.app.settings_ui(ctx,false));
        let viewport=output.viewport_output.remove(&id).unwrap();
        assert!(viewport.commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark))),"浅色系统下窗口标题栏和边框也应使用深色");
        (viewport.builder,viewport.commands)
    };
    h.app.open_settings();
    let (builder,commands)=frame(&mut h,false);
    assert_eq!(builder.visible,Some(false),"设置窗口应隐藏创建，避免首帧绘制前露出白色背景");
    assert_eq!(builder.active,Some(false),"隐藏窗口不能提前激活");
    let show=commands.iter().position(|cmd|matches!(cmd,egui::ViewportCommand::Visible(true))).expect("首帧绘制后应显示设置窗口");
    let focus=commands.iter().position(|cmd|matches!(cmd,egui::ViewportCommand::Focus)).unwrap();
    let theme=commands.iter().position(|cmd|matches!(cmd,egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark))).unwrap();
    assert!(theme<show,"显示窗口前应设置深色边框，避免白边闪现");
    assert!(show<focus,"应先显示已绘制的窗口，再获取焦点");
    assert_eq!(builder.window_level,Some(egui::WindowLevel::Normal),"打开设置时应保持普通窗口层级");
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Minimized(false))),"应恢复已最小化的设置窗口：{commands:?}");
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Focus)),"应将焦点交给设置窗口");
    let (builder,commands)=frame(&mut h,true);
    assert!(!commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Visible(_))),"正常重绘不应重复显示窗口，避免打断最小化");
    assert_eq!(builder.window_level,Some(egui::WindowLevel::Normal),"操作设置时也不能切换到置顶层");
    assert!(!commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Focus)),"正常绘制不应反复抢焦点");
    let (builder,commands)=frame(&mut h,false);
    assert_eq!(builder.window_level,Some(egui::WindowLevel::Normal),"切回其他程序后应解除设置置顶");
    assert!(!commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Focus)));
    h.app.open_settings();
    let (builder,commands)=frame(&mut h,false);
    assert_eq!(builder.window_level,Some(egui::WindowLevel::Normal),"再次唤起设置仍须保持普通窗口层级");
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Minimized(false))));
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Focus)),"设置已打开时仍应响应再次唤起");
    h.app.settings_open=false;
    let _=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));
    h.app.open_settings();
    let (builder,commands)=frame(&mut h,false);
    assert_eq!(builder.visible,Some(false),"关闭后重新创建也应隐藏到首帧完成");
    assert!(commands.iter().any(|cmd|matches!(cmd,egui::ViewportCommand::Visible(true))),"重开设置应正常显示");
}

#[test]
fn web_video_is_hidden_without_changing_music_preferences(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;
    h.app.ly.key="Previous Song|Artist|120".into();
    h.app.ly.lyrics=Some(crate::lyrics::parse_lrc("[00:01.00]previous song"));
    h.app.ly.error=Some(crate::lyrics::FetchError::Unavailable("503".into()));
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    h.media=crate::smtc::NowPlaying{title:"小唐的Token的抖音 - 抖音".into(),playing:true,duration:17.0,..Default::default()};
    h.frame(vec![]);
    assert!(h.app.media.title.is_empty() && !h.app.media.playing,"视频标题和播放状态不应进入坞内音乐页面");
    assert!(h.app.ly.key.is_empty(),"视频不应建立歌曲取词任务");
    assert!(h.app.ly.lyrics.is_none() && h.app.ly.error.is_none(),"切到视频应清除上一首歌词和歌词错误");
    assert!(tx.send(Ok((crate::lyrics::parse_lrc("[00:01.00]late song"),false))).is_err(),"视频不能接收旧歌曲迟到的查询结果");
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.click(H::local(74.0,118.0));
    assert!(h.app.show_lyrics,"视频页面的标签不应切换用户的歌曲歌词偏好");
    h.click(H::local(210.5,118.0));
    assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Media(_))),"音乐页不应控制被隐藏的视频");
    assert!(!matches!(h.app.pet_state(),crate::extras::PetState::Music),"视频不应触发听歌表情");
    h.app.opts.fetch_lyrics=false;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
    h.hover(Pos2::new(WIN_W/2.0,12.0),0.5);
    h.click(H::local(74.0,118.0));
    assert!(!h.app.show_lyrics,"回到歌曲时应恢复原来的词按钮");
}

#[test]
fn web_video_does_not_override_the_minimal_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.minimal=true;
    h.media=crate::smtc::NowPlaying{title:"小唐的Token的抖音 - 抖音".into(),playing:true,duration:17.0,..Default::default()};
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"视频不能触发歌曲专用的收起态");
}

#[test]
fn minimal_mode_stays_a_line_with_music_and_after_media_changes(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.minimal=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"播放音乐时仍应遵守极简开关");
    assert!(h.hit.height()<=crate::app::BAR_H_MIN+0.1,"鼠标命中范围也应收回细线");
    h.media.title="Another Track".into();h.media.playing=false;
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"暂停或切歌不能覆盖极简开关");
    h.media=Default::default();
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"没有媒体时恢复用户选择的极简状态");
}

#[test]
fn toggling_minimal_changes_the_island_while_media_is_present(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    for _ in 0..90 {h.frame(vec![]);}assert!(h.region.height()>30.0);
    h.app.settings.minimal=true;for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()<20.0,"开启极简后应收成细线");
    h.app.settings.minimal=false;for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()>30.0,"关闭极简后应恢复普通坞体");
}

#[test]
fn minimal_mode_keeps_focus_running_and_returns_after_notifications(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.minimal=true;h.app.pomo.running=true;h.app.pomo.remaining=120.0;
    for _ in 0..90 {h.frame(vec![]);}assert!(h.region.height()<20.0,"专注不能覆盖极简开关");
    h.app.alert("系统音量","音量 50%",5,false);
    for _ in 0..60 {h.frame(vec![]);}assert!(h.region.height()>30.0,"临时提醒仍应可见");
    for _ in 0..240 {h.frame(vec![]);}assert!(h.region.height()<20.0,"提醒结束后应回到细线");
    assert!(h.app.pomo.running&&h.app.pomo.remaining<115.0,"极简仅改变显示，计时继续运行");
}

#[test]
fn delayed_media_samples_keep_the_live_position_and_follow_pause_and_seek(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:9.2,duration:120.0,playing:true,sampled_at:Some(std::time::Instant::now()-std::time::Duration::from_millis(600)),..Default::default()};
    h.frame(vec![]);
    assert!(h.app.media.position>=9.79,"媒体位置必须包含送到界面之前经过的时间");
    h.media.playing=false;h.media.position=9.4;h.frame(vec![]);
    assert_eq!(h.app.media.position,9.4,"暂停不能继续外推");
    h.media.playing=true;h.media.position=4.0;h.media.sampled_at=Some(std::time::Instant::now());h.frame(vec![]);
    assert!((4.0..4.2).contains(&h.app.media.position),"跳转后应使用新的播放位置");
    h.media.position=119.9;h.media.sampled_at=Some(std::time::Instant::now()-std::time::Duration::from_secs(1));h.frame(vec![]);
    assert_eq!(h.app.media.position,120.0,"外推不能超过歌曲结尾");
}

#[test]
fn kugou_external_seek_updates_lyrics_in_both_directions(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{source:"KuGou.exe".into(),title:"Track".into(),artist:"Artist".into(),duration:120.0,playing:true,..Default::default()};
    h.frame(vec![]);
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]开头\n[00:30]中段\n[01:00]后段"));
    for (position,line) in [(65.0,"后段"),(10.0,"开头"),(35.0,"中段")] {
        h.media.position=position;h.media.sampled_at=Some(std::time::Instant::now());h.frame(vec![]);
        assert_eq!(h.app.island_lyric().map(|(_,text)|text),Some(line),"酷狗跳转后应立即切换到对应歌词");
    }
}

#[test]
fn music_clients_keep_lyrics_across_two_repeat_cycles(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for source in ["kugou","QQMusic.exe","cloudmusic.exe","汽水音乐","Spotify.exe","kwmusic.exe"] {
        let mut h=H::new();h.app.settings.island_lyrics=true;
        h.media=crate::smtc::NowPlaying{source:source.into(),title:"循环测试".into(),artist:"歌手".into(),duration:120.0,playing:true,..Default::default()};
        h.frame(vec![]);h.app.set_lyrics("循环测试","歌手",120.0,crate::lyrics::parse_lrc("[00:00]开头\n[00:05]第一段\n[01:50]尾声"));
        for (position,line) in [(119.0,"尾声"),(0.0,"开头"),(8.0,"第一段"),(119.0,"尾声"),(0.0,"开头"),(8.0,"第一段")] {
            h.media.position=position;h.media.sampled_at=Some(std::time::Instant::now());h.frame(vec![]);
            assert_eq!(h.app.island_lyric().map(|(_,text)|text),Some(line),"{source} 第二、三遍也必须显示相应歌词");
        }
    }
}

#[test]
fn music_without_a_timeline_does_not_invent_elapsed_time(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"QQMusic.exe".into(),title:"歌曲".into(),playing:true,playback_type:crate::smtc::MediaKind::Music,..Default::default()};
    for _ in 0..240{h.frame(vec![]);}
    assert_eq!(h.app.media.title,"歌曲");assert_eq!(h.app.media.position,0.0);assert_eq!(h.app.media.duration,0.0);
}

#[test]
fn kugou_position_advances_without_duration_and_respects_pause_and_seek(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"kugou".into(),title:"Track".into(),artist:"Artist".into(),position:42.0,duration:0.0,playing:true,sampled_at:Some(std::time::Instant::now()-std::time::Duration::from_millis(600)),..Default::default()};
    h.frame(vec![]);
    assert!(h.app.media.position>=42.59,"未知总时长不能阻止真实进度外推");
    h.media.playing=false;h.frame(vec![]);assert_eq!(h.app.media.position,42.0);
    h.media.playing=true;h.media.position=12.0;h.media.sampled_at=Some(std::time::Instant::now());h.frame(vec![]);
    assert!((12.0..12.2).contains(&h.app.media.position),"拖动后必须跟随新的实际位置");
}

#[test]
fn music_speaker_button_toggles_mute(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
    h.app.volume_ui=Some((45,false));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.click(H::local(367.0,118.0));
    assert!(h.cmds.contains(&crate::app::Cmd::ToggleMute));
    assert_eq!(h.app.volume_ui,Some((45,true)));
}

#[test]
fn music_uses_system_artwork_and_clears_it_when_the_next_song_has_none(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    let image=std::sync::Arc::new(egui::ColorImage::new([32,32],egui::Color32::BLUE));
    h.media=crate::smtc::NowPlaying{title:"First".into(),artist:"Artist".into(),artwork:Some(image.clone()),..Default::default()};
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let (cached,texture)=h.app.music_artwork.as_ref().expect("播放页应使用系统封面");
    assert!(std::sync::Arc::ptr_eq(cached,&image));
    let texture_id=texture.id();
    h.frame(vec![]);
    assert_eq!(h.app.music_artwork.as_ref().unwrap().1.id(),texture_id,"进度更新不应重复上传纹理");
    h.media=crate::smtc::NowPlaying{title:"Second".into(),artist:"Artist".into(),..Default::default()};
    h.frame(vec![]);
    assert!(h.app.music_artwork.is_none(),"缺少封面的下一首歌应恢复占位图");
}

#[test]
fn failed_lyrics_can_be_retried_from_the_music_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
    h.app.ly.key="Track|Artist|120".into();
    h.app.ly.error=Some(crate::lyrics::FetchError::Unavailable("HTTP 502".into()));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="暂未获取歌词，请重试")),"展开内应保留歌词错误提示");
    h.click(H::local(310.0,68.0));
    assert!(h.app.ly.key.is_empty(),"点击错误提示后应允许重新请求当前歌曲");
}

#[test]
fn transient_lyric_failures_retry_but_missing_songs_do_not_loop(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (error,retry) in [(crate::lyrics::FetchError::Unavailable("503".into()),true),(crate::lyrics::FetchError::NotFound,false),(crate::lyrics::FetchError::IncompleteMetadata,false)] {
        let mut h=H::new();
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),duration:120.0,..Default::default()};
        h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;
        h.app.ly.key="Track|Artist|120".into();
        let (tx,rx)=std::sync::mpsc::channel();
        h.app.ly.rx=Some(rx);h.app.ly.loading=true;
        tx.send(Err(error.clone())).unwrap();
        h.frame(vec![]);
        assert_eq!(h.app.ly.error,Some(error));
        assert_eq!(h.app.ly.retry_at.is_some(),retry);
        assert!(!h.app.ly.loading);
        if let Some(at)=h.app.ly.retry_at {assert!((at-h.t-30.0).abs()<0.001,"网络恢复重试应有间隔");}
    }
}

#[test]
fn music_lyrics_show_in_a_separate_right_column() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:01]上一句\n[00:05]当前歌词\n[00:10]下一句"));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let current=h.shapes.iter().find_map(|shape|match &shape.shape {
        egui::Shape::Text(text) if text.galley.text()=="当前歌词"=>Some(text.pos),_=>None,
    }).expect("展开页应显示同步歌词");
    assert!(current.x>=H::local(243.0,0.0).x,"歌词必须位于播放器右侧");
    let title=h.shapes.iter().find_map(|shape|match &shape.shape {
        egui::Shape::Text(text) if text.galley.text()=="Track"=>Some(text.pos),_=>None,
    }).unwrap();
    assert!(title.x<H::local(233.0,0.0).x);
    h.click(H::local(74.0,118.0));
    assert!(!h.app.show_lyrics);
    h.frame(vec![]);
    assert!(!h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()=="当前歌词")));
}

#[test]
fn disabling_lyrics_disconnects_pending_workers(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"KuGou.exe".into(),title:"测试歌曲".into(),artist:"歌手".into(),..Default::default()};
    h.frame(vec![]);h.app.opts.fetch_lyrics=true;
    h.app.show_lyrics=false;h.app.settings.island_lyrics=false;
    h.app.ly.key="测试歌曲|歌手|0".into();h.app.ly.loading=true;
    let (network,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    let (native,rx)=std::sync::mpsc::channel();h.app.ly.native_rx=Some(rx);
    h.frame(vec![]);
    assert!(native.send(None).is_err(),"关闭歌词后应停止本机轮询");
    assert!(network.send(Err(crate::lyrics::FetchError::NotFound)).is_err());
    assert!(h.app.ly.key.is_empty(),"再次开启时应能重新加载歌词");
}

#[test]
fn late_native_lyrics_bypass_pending_network_for_both_clients(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for source in ["KuGou.exe","SodaMusic.exe"] {
        let mut h=H::new();
        h.media=crate::smtc::NowPlaying{source:source.into(),title:"测试歌曲".into(),artist:"歌手".into(),..Default::default()};
        h.frame(vec![]);h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;
        h.app.ly.key="测试歌曲|歌手|0".into();h.app.ly.loading=true;
        let (network,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
        let (native,rx)=std::sync::mpsc::channel();h.app.ly.native_rx=Some(rx);
        native.send(Some(crate::lyrics::parse_lrc("[00:01.00]本机歌词"))).unwrap();h.frame(vec![]);
        assert!(!h.app.ly.loading,"本机歌词不能等待在线请求结束");
        assert_eq!(h.app.ly.lyrics.as_ref().unwrap().lines[0].text,"本机歌词");
        assert!(network.send(Err(crate::lyrics::FetchError::NotFound)).is_err(),"迟到的网络结果必须丢弃");
    }
}

#[test]
fn kugou_fallback_waits_for_late_native_lyrics(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"kugou".into(),title:"测试歌曲".into(),artist:"歌手".into(),position:22.8,..Default::default()};
    h.frame(vec![]);h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;h.app.ly.key="测试歌曲|歌手|0".into();
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    tx.send(Ok((crate::lyrics::parse_lrc("[00:23.930]测试歌词"),true))).unwrap();h.frame(vec![]);
    assert!(h.app.ly.retry_at.is_some(),"临时通用歌词必须继续等待迟到的酷狗原生歌词");
    h.t=h.app.ly.retry_at.unwrap()+0.1;h.frame(vec![]);
    assert_eq!(h.app.ly.lyrics.as_ref().unwrap().lines[0].t,23.930,"接回原生歌词期间不能清空现有歌词");assert!(!h.app.ly.loading);
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    tx.send(Ok((crate::lyrics::parse_lrc("[00:22.991]测试歌词"),false))).unwrap();h.frame(vec![]);
    assert!((h.app.ly.lyrics.as_ref().unwrap().lines[0].t-22.991).abs()<0.001);assert!(h.app.ly.retry_at.is_none(),"接回原生歌词后应停止重查");
}

#[test]
fn kugou_missing_lyrics_rechecks_late_cache_without_flashing_loading() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{source:"kugou".into(),title:"不存在的测试歌曲".into(),artist:"测试歌手".into(),duration:120.0,..Default::default()};
    h.frame(vec![]);h.app.opts.fetch_lyrics=true;h.app.show_lyrics=true;
    h.app.ly.key="不存在的测试歌曲|测试歌手|120".into();
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    tx.send(Err(crate::lyrics::FetchError::NotFound)).unwrap();h.frame(vec![]);
    let retry=h.app.ly.retry_at.expect("酷狗尚未写入缓存时仍需自动检查");
    assert!((retry-h.t-5.0).abs()<0.001);
    h.t=retry+0.1;h.frame(vec![]);
    assert!(!h.app.ly.loading,"检查本机缓存不能反复闪出加载提示");
    assert_eq!(h.app.ly.error,Some(crate::lyrics::FetchError::NotFound));
    let (tx,rx)=std::sync::mpsc::channel();h.app.ly.rx=Some(rx);
    tx.send(Ok((crate::lyrics::parse_lrc("[00:01.00]迟到的原生歌词"),false))).unwrap();h.frame(vec![]);
    assert!(h.app.ly.lyrics.is_some());assert!(h.app.ly.error.is_none());assert!(h.app.ly.retry_at.is_none());
}

#[test]
fn music_lyrics_island_toggle_is_independent_and_persistent() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.show_lyrics=false;h.app.settings.lyrics_enabled=false;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句"));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.click(H::local(313.0,118.0));
    assert!(h.app.settings.island_lyrics);
    assert!(!h.app.show_lyrics && !h.app.settings.lyrics_enabled);
    let saved=crate::model::Settings::load();
    assert!(saved.island_lyrics && !saved.lyrics_enabled);
    h.hover(Pos2::new(WIN_W/2.0,PH+30.0),1.0);
    assert!(!h.app.morph.open);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("当前歌词"));
    let pet_width=if h.app.pet_enabled(){48.0}else{0.0};
    assert!((h.app.current_width()-crate::app::LYRIC_BAR_W-pet_width).abs()<2.0);
}

#[test]
fn music_lyric_timing_follows_saved_delay_and_preserves_playback_position(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:9.8,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]第一句\n[00:05]当前歌词\n[00:10]下一句"));
    h.frame(vec![]);
    assert_eq!(h.app.settings.lyric_delay_ms,-350,"未设置时应保留原有提前 0.35 秒的预设");
        assert!((h.app.lyric_position()-10.15).abs()<0.001,"默认应提前 0.35 秒显示，且只应用一次");
    std::fs::create_dir_all(crate::model::config_dir()).unwrap();
    for (delay,position,expected) in [(500,10.2,Some("当前歌词")),(-500,9.8,Some("下一句")),(0,9.8,Some("当前歌词")),(500,0.2,None),(500,0.6,Some("第一句"))]{
        std::fs::write(crate::model::config_dir().join("settings.txt"),format!("lyric_delay_ms={delay}\nisland_lyrics=1\n")).unwrap();
        h.app.settings=crate::model::Settings::load();
        h.media.position=position;h.frame(vec![]);
        assert_eq!(h.app.island_lyric().map(|(_,line)|line),expected,"延时 {delay} ms，播放位置 {position}");
        assert!((h.app.media.position-position).abs()<0.001,"调整歌词不应改变播放进度");
        h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
        for text in ["第一句","当前歌词","下一句"]{
            let active=h.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()==text && t.galley.job.sections[0].format.font_id.size==12.0));
            assert_eq!(active,expected==Some(text),"展开歌词也应使用相同的显示时间");
        }
        h.app.settings.save();
        assert_eq!(crate::model::Settings::load().lyric_delay_ms,delay,"重启后应保留时间调整");
    }
    for (input,expected) in [("invalid",-350),("999999",5000),("-999999",-5000),("",-350)]{
        std::fs::write(crate::model::config_dir().join("settings.txt"),format!("lyric_delay_ms={input}\n")).unwrap();
        assert_eq!(crate::model::Settings::load().lyric_delay_ms,expected);
    }
}

#[test]
fn music_lyric_timing_controls_match_focus_save_and_fit_the_small_settings_window(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings_tab=1;h.app.open_settings();h.ctx.set_embed_viewports(false);
    fn frame(h:&mut H,events:Vec<egui::Event>)->Vec<egui::epaint::ClippedShape>{
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,450.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.25),focused:Some(true),..Default::default()})).collect(),
                focused:true,events:events.clone(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let raw=egui::RawInput{viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(1.25),..Default::default()})).collect(),..Default::default()};
        let _=h.ctx.run(raw,|ctx|h.app.settings_ui(ctx,false));
        let child=output.lock().unwrap().take().unwrap();child.shapes
    }
    let text_rect=|shapes:&[egui::epaint::ClippedShape],text:&str|shapes.iter().find_map(|shape|match &shape.shape{
        egui::Shape::Text(t)if t.galley.text()==text=>Some(t.visual_bounding_rect()),_=>None,
    }).unwrap_or_else(||panic!("设置中缺少 {text}"));
    for _ in 0..3{frame(&mut h,vec![]);}
    let music=frame(&mut h,vec![]);
    text_rect(&music,"0.35 秒");
    assert_eq!(h.app.settings.lyric_delay_ms,-350,"仅打开设置不能将 0.35 秒舍入为 0.4 秒");
    for button in ["−","+"]{text_rect(&music,button);}
    h.app.settings_tab=2;
    for _ in 0..3{frame(&mut h,vec![]);}
    let focus=frame(&mut h,vec![]);
    for button in ["−","+"]{
        let a=text_rect(&music,button);let b=text_rect(&focus,button);
        assert!((a.left()-b.left()).abs()<0.1 && (a.width()-b.width()).abs()<0.1 && (a.height()-b.height()).abs()<0.1,"歌词和专注应共用同样的数值控件及右侧对齐");
    }
    h.app.settings_tab=1;
    for _ in 0..3{frame(&mut h,vec![]);}
    for (button,expected) in [("+",-450),("−",-350),("−",-250),("+",-350)]{
        let shapes=frame(&mut h,vec![]);
        let rect=text_rect(&shapes,button);
        assert!(Rect::from_min_max(egui::pos2(180.0,0.0),egui::pos2(600.0,450.0)).contains_rect(rect),"时间调整按钮应完整显示");
        let pos=rect.center();
        frame(&mut h,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        frame(&mut h,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        assert_eq!(h.app.settings.lyric_delay_ms,expected);
        assert_eq!(crate::model::Settings::load().lyric_delay_ms,expected,"按钮调整后应自动保存");
        text_rect(&frame(&mut h,vec![]),&format!("{:.2} 秒",-expected as f64/1000.0));
    }
}

#[test]
fn island_lyrics_arrive_just_before_vocals_and_finish_fading_before_the_timestamp(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:9.4,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句"));
    for _ in 0..60 {h.frame(vec![]);}
    h.media.playing=true;
    for step in 1..=30 {
        h.media.position=9.4+step as f64/60.0;
        h.frame(vec![]);
        if step==6 {assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("当前歌词"),"不能提前半秒就跳到下一句");}
    }
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("下一句"),"下一句应在开唱前提前出现");
    let text=h.shapes.iter().find_map(|shape|match &shape.shape {
        egui::Shape::Text(text) if text.galley.text()=="下一句"=>Some(text),_=>None,
    }).expect("开唱前应能看到下一句");
    assert!(text.opacity_factor>0.95,"开唱前应完成淡入，不能等到开唱后才清晰");
    h.media.playing=false;h.frame(vec![]);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("下一句"),"暂停时不应跳回上一句");
    h.media.position=4.5;h.frame(vec![]);
    assert!(h.app.island_lyric().is_none(),"前奏不能过早显示第一句");
    h.media.position=4.75;h.frame(vec![]);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("当前歌词"),"第一句也应提前出现");
}

#[test]
fn expanded_and_segmented_lyrics_share_the_early_display_time(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:9.8,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句"));
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let next=h.shapes.iter().find_map(|shape|match &shape.shape{egui::Shape::Text(text)if text.galley.text()=="下一句"=>Some(text),_=>None}).unwrap();
    assert_eq!(next.galley.job.sections[0].format.font_id.size,12.0,"展开歌词也应在开唱前高亮下一句");
    h.hover(Pos2::new(WIN_W/2.0,PH+30.0),1.0);
    let line="愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光";
    h.media.position=6.0;h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(&format!("[00:05]{line}\n[00:15]下一句")));
    for _ in 0..60{h.frame(vec![]);}
    let shown=|h:&H|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if line.contains(t.galley.text())&&!t.galley.text().is_empty()=>Some(t.galley.text().to_owned()),_=>None}).unwrap();
    let first=shown(&h);
    let boundary=5.0+10.0*first.chars().count()as f64/line.chars().count()as f64;
    h.media.position=boundary-0.2;for _ in 0..30{h.frame(vec![]);}
    assert_ne!(shown(&h),first,"长句的后半段也应使用相同的提前量");
}

#[test]
fn lyric_short_gap_and_line_switch_keep_text_visible(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:2.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",0.0,crate::lyrics::parse_lrc("[00:01]上一句\n[00:05]下一句"));
    for _ in 0..60{h.frame(vec![]);}
    for (position,expected) in [(3.0,"上一句"),(4.64,"上一句"),(4.7,"下一句"),(4.72,"下一句"),(4.9,"下一句")]{
        h.media.position=position;h.frame(vec![]);
        let text=h.shapes.iter().find_map(|shape|match &shape.shape{
            egui::Shape::Text(text)if text.galley.text()==expected=>Some(text),_=>None,
        }).expect("短停顿及换句时必须持续显示歌词");
        assert!(text.opacity_factor>0.95,"换句不能淡出歌词，位置 {position}，透明度 {}",text.opacity_factor);
    }
}

#[test]
fn island_lyrics_center_segments_in_a_fixed_height(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
    for line in ["晚风","愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光","晚风"] {
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(&format!("[00:05]{line}\n[00:15]下一句")));
        for _ in 0..90 {
            h.frame(vec![]);
            assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="09:41")),"切换显示段时应保持歌词布局");
        }
        let text=h.shapes.iter().find_map(|shape|match &shape.shape {
            egui::Shape::Text(text) if !text.galley.text().is_empty() && line.contains(text.galley.text())=>Some(text),_=>None,
        }).expect("坞上应显示当前歌词的一段");
        let center=text.pos.x+text.galley.rect.center().x;
        let offset=if h.app.pet_enabled(){24.0}else{0.0};
        assert!((center-WIN_W/2.0-offset).abs()<1.5,"歌词应在内容区居中");
        assert!(!text.galley.elided,"显示段应完整显示");
        assert_eq!(text.galley.rows.len(),1,"每次只显示一段歌词");
        assert!((h.hit.height()-(crate::app::BAR_H+4.0)).abs()<0.1,"坞应保持固定高度");
        assert!(text.visual_bounding_rect().bottom()<=h.hit.bottom(),"文字应完整显示在坞内");
    }
}

#[test]
fn island_lyric_segments_follow_playback_pause_and_seek(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.island_lyrics=true;
    let line="愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光";
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:6.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(&format!("[00:05]{line}\n[00:15]下一句")));
    let shown=|h:&H|h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t)=>Some(t.galley.text().to_owned()),_=>None}).unwrap();
    for _ in 0..90 {h.frame(vec![]);}let first=shown(&h);
    h.media.position=14.0;for _ in 0..30 {h.frame(vec![]);}let last=shown(&h);
    assert_ne!(first,last,"播放到后半句时应切换显示段");
    assert_eq!(format!("{first}{last}"),line,"前后显示段应覆盖完整歌词");
    for _ in 0..120 {h.frame(vec![]);}assert_eq!(shown(&h),last,"暂停时显示段应保持不动");
    h.media.position=6.0;for _ in 0..30 {h.frame(vec![]);}assert_eq!(shown(&h),first,"拖回前半句应恢复对应显示段");
    h.media.position=16.0;for _ in 0..30 {h.frame(vec![]);}assert_eq!(shown(&h),"下一句");
}

#[test]
fn island_lyrics_fall_back_without_synced_lyrics_and_keep_intro_and_gaps(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.settings.minimal=true;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:1.0,duration:120.0,..Default::default()};
    for state in 0..6 {
        h.app.ly=Default::default();
        h.app.ly.key="Track|Artist|120".into();
        h.media.position=if state==1 {7.0}else{1.0};
        match state {
            0=>h.app.ly.lyrics=Some(crate::lyrics::parse_lrc("[00:05]第一句")),
            1=>h.app.ly.lyrics=Some(crate::lyrics::parse_lrc("[00:01]第一句\n[00:05]\n[00:10]下一句")),
            2=>h.app.ly.loading=true,
            3=>h.app.ly.error=Some(crate::lyrics::FetchError::NotFound),
            4=>h.app.ly.error=Some(crate::lyrics::FetchError::Unavailable("503".into())),
            _=>h.app.ly.lyrics=Some(crate::lyrics::from_plain("没有时间戳")),
        }
        for _ in 0..90 {h.frame(vec![]);}
        if let Some(error)=&h.app.ly.error {
            assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==error.label())),"歌词错误提示只应在展开内显示");
        }
        if state<2 {
            assert!((h.app.current_width()-crate::app::LYRIC_BAR_W).abs()<2.0,"前奏和间奏应保持歌词坞宽度");
            assert!(h.region.height()>30.0);
        } else {
            let fallback_width=h.app.current_width();
            assert!(!h.app.compact_notices(),"无同步歌词时应恢复普通通知布局");
            h.app.settings.island_lyrics=false;
            for _ in 0..90 {h.frame(vec![]);}
            assert!((fallback_width-h.app.current_width()).abs()<2.0,"状态 {state} 应恢复关闭坞上歌词时的宽度");
            h.app.settings.island_lyrics=true;
        }
    }
}

#[test]
#[ignore="按需导出坞上歌词的实际渲染画面"]
fn island_lyrics_layout_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_LYRIC_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    for (name,raw,position) in [("short",Some("[00:05]晚风"),7.0),("long",Some("[00:05]愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光\n[00:15]下一句"),7.0),("long-later",Some("[00:05]愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光\n[00:15]下一句"),14.0),("intro",Some("[00:20]晚风"),7.0),("missing",None,7.0)] {
        let mut h=H::new();h.app.settings.island_lyrics=true;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position,duration:120.0,..Default::default()};
        if let Some(raw)=raw {h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(raw));}
        else {h.app.ly.key="Track|Artist|120".into();h.app.ly.error=Some(crate::lyrics::FetchError::NotFound);}
        let mut raster=crate::preview_raster::Raster::new(1280,380);
        for frame in 0..90 {
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),today:"2026-10-31".into(),date:"10月31日 周六".into(),media:h.media.clone(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
                viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});
            });
            raster.update_textures(&output.textures_delta);
            if frame==89 {
                raster.buf.fill([0.95,0.96,0.98,1.0]);
                raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                let image=image::RgbaImage::from_raw(1280,380,rgba).unwrap();
                image::imageops::crop_imm(&image,300,0,680,170).to_image().save(dest.join(format!("{name}.png"))).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
}

#[test]
fn music_lyrics_keep_syncing_when_only_the_collapsed_lyrics_are_enabled() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.opts.fetch_lyrics=true;h.app.show_lyrics=false;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
    let (tx,rx)=std::sync::mpsc::channel();
    h.app.ly.key="Track|Artist|120".into();h.app.ly.loading=true;h.app.ly.rx=Some(rx);
    tx.send(Ok((crate::lyrics::parse_lrc("[00:05]当前歌词\n[00:10]下一句"),false))).unwrap();
    h.frame(vec![]);
    assert!(!h.app.ly.loading);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("当前歌词"));
    h.media.position=12.0;h.frame(vec![]);
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("下一句"));
}

#[test]
fn music_lyrics_respect_the_switch_and_do_not_show_stale_or_unsynced_lines() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.minimal=true;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词"));
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.region.height()>30.0,"主动开启坞上歌词后应可在收起状态阅读");
    h.app.settings.island_lyrics=false;
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.island_lyric().is_none() && h.region.height()<20.0);
    h.app.settings.island_lyrics=true;
    h.media.title="Another track".into();h.frame(vec![]);
    assert!(h.app.island_lyric().is_none(),"换歌后不能显示上一首的歌词");
    h.app.set_lyrics("Another track","Artist",120.0,crate::lyrics::from_plain("没有时间戳的歌词"));h.frame(vec![]);
    assert!(h.app.island_lyric().is_none(),"不能将纯文本歌词伪装成当前句");
}

#[test]
fn hover_expands_with_overshoot_and_leave_collapses() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 0.1);
    assert!(h.app.progress() < 0.1, "意图缓冲期内不应展开");
    let mut peak: f32 = 0.0;
    for _ in 0..60 {
        h.hover(Pos2::new(WIN_W / 2.0, 12.0), 1.0 / 60.0);
        peak = peak.max(h.app.progress());
    }
    assert!(peak > 1.01 && peak < 1.12, "展开应有轻微过冲, peak={peak}");
    assert!((h.app.progress() - 1.0).abs() < 0.02);
    h.cursor = Some(Pos2::new(WIN_W / 2.0, PH + 30.0));
    for _ in 0..40 {
        h.frame(vec![]);
    }
    assert!(h.app.progress() < 0.02, "移开后应收起");
}

#[test]
fn browser_minimal_preserves_preferences_and_restores_the_previous_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=true;h.app.settings.minimal=false;h.browser_at_top=Some(true);
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.hit.height()>20.0,"开关默认关闭时不应自动改变模式");
    h.app.settings.browser_minimal=true;h.app.settings.save();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]当前歌词"));
    for _ in 0..90 {h.frame(vec![]);}
    assert!(h.hit.height()<10.0,"浏览器顶部受遮挡时应收为细刘海，歌词和吉祥物不能撑大它");
    assert!(h.app.settings.pet && !h.app.settings.minimal,"自动极简不能修改手动模式");
    let saved=crate::model::Settings::load();
    assert!(saved.browser_minimal && saved.pet && !saved.minimal,"应只持久化开关，保留原模式");
    h.browser_at_top=None;for _ in 0..30 {h.frame(vec![]);}
    assert!(h.hit.height()<10.0,"打开胶囊坞设置或托盘时应保留浏览器避让状态");
    h.browser_at_top=Some(false);for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.pet_enabled() && h.hit.height()>20.0,"离开浏览器或还原窗口后应恢复");
    h.browser_at_top=Some(true);for _ in 0..90 {h.frame(vec![]);}
    h.app.settings.browser_minimal=false;for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.pet_enabled() && h.hit.height()>20.0,"关闭开关后应立即恢复");
    h.app.settings.island_lyrics=false;h.app.set_minimal(true);h.browser_at_top=Some(false);for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.settings.minimal && h.hit.height()<10.0,"离开浏览器不能撤销手动极简");
}

#[test]
fn browser_minimal_closes_the_open_island_but_still_allows_deliberate_hover(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.browser_minimal=true;
    h.app.show_page(0);h.hover(H::local(300.0,60.0),0.8);
    assert!(h.app.morph.open);
    h.browser_at_top=Some(true);h.frame(vec![]);
    assert!(!h.app.morph.open,"切到浏览器时不能继续保留展开状态遮挡标签栏");
    assert!(!h.hit.contains(Pos2::new(WIN_W/2.0,12.0)),"收起动画期间也应让标签栏接收点击");
    h.hover(Pos2::new(WIN_W/2.0,2.0),0.25);
    assert!(!h.app.morph.open,"短暂碰到刘海不能误触");
    h.hover(Pos2::new(WIN_W/2.0,2.0),0.6);
    assert!(h.app.morph.open,"仍应能主动悬停打开坞");
}

#[test]
fn minimal_hover_excludes_the_surrounding_browser_tabs() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.minimal=true;
    for _ in 0..90 {h.frame(vec![]);}
    for cursor in [
        Pos2::new(WIN_W/2.0,crate::app::BAR_H_MIN+1.0),
        Pos2::new(WIN_W/2.0+crate::app::BAR_W_MIN/2.0+2.0,2.0),
        Pos2::new(WIN_W/2.0-crate::app::BAR_W_MIN/2.0-2.0,2.0),
    ] {
        assert!(!h.hit.contains(cursor),"刘海外的标签栏应接收鼠标操作");
        h.hover(cursor,1.0);
        assert!(!h.app.morph.open,"鼠标未触碰极简刘海时不得展开");
    }
}

#[test]
fn minimal_hover_requires_continuous_deliberate_contact() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.minimal=true;
    for _ in 0..90 {h.frame(vec![]);}
    let inside=Pos2::new(WIN_W/2.0,2.0);
    h.hover(inside,0.25);
    assert!(!h.app.morph.open,"短暂经过极简刘海不得展开");
    h.hover(Pos2::new(WIN_W/2.0,8.0),0.1);
    h.hover(inside,0.25);
    assert!(!h.app.morph.open,"移出刘海应重新计算停留时间");
    h.hover(inside,0.7);
    assert!(h.app.morph.open,"在刘海内持续停留仍应展开");
    h.click(H::local(398.0,68.0));
    assert_eq!(h.app.nav.page,1,"展开后应能正常操作面板");
}

#[test]
fn minimal_hover_does_not_catch_tabs_during_collapse() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.minimal=true;
    h.hover(Pos2::new(WIN_W/2.0,2.0),1.0);
    assert!(h.app.morph.open);
    h.cursor=Some(Pos2::new(WIN_W/2.0,PH+30.0));
    for _ in 0..30 {
        h.frame(vec![]);
        if !h.app.morph.open {break;}
    }
    assert!(!h.app.morph.open);
    assert!(h.app.progress()>0.04,"检查收起动画仍在进行时的命中范围");
    let tab=Pos2::new(WIN_W/2.0,12.0);
    assert!(!h.hit.contains(tab),"收起动画不得继续拦截标签栏");
    h.hover(tab,1.0);
    assert!(!h.app.morph.open);
}

#[test]
fn arrows_and_dots_switch_pages() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 1.0);
    h.click(H::local(398.0, 68.0));
    assert_eq!(h.app.nav.page, 1);
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(21.0, 68.0));
    assert_eq!(h.app.nav.page, 0);
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(398.0, 68.0));
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(398.0, 68.0));
    assert_eq!(h.app.nav.page, 2);
}

#[test]
fn wheel_navigation_reaches_every_page_in_both_directions(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.shelf.items.clear();h.app.todos.items.clear();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.hover(H::local(210.0,18.0),0.3);
    for (delta,pages) in [(-40.0,[1,2,3,0,1,2]),(40.0,[1,0,3,2,1,0])] {
        for page in pages {
            h.wheel(delta);
            assert_eq!(h.app.nav.page,page,"滚轮翻页不能被空的置物架或待办卡住");
            h.hover(H::local(210.0,18.0),0.25);
        }
    }
}

#[test]
fn wheel_scrolls_lists_only_inside_content_and_leaves_at_the_edge(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items=(0..4).map(|i|crate::model::TodoItem{text:format!("任务 {i}"),done_on:None,due:None,reminded:false,..Default::default()}).collect();
    h.app.shelf.items=(0..6).map(|i|std::path::PathBuf::from(format!("scroll-test-{i}.txt"))).collect();
    for page in [2,3] {
        let next=(page+1)%4;
        h.app.show_page(page);h.hover(H::local(300.0,60.0),1.0);
        h.wheel(-40.0);
        assert_eq!(h.app.nav.page,page,"有更多内容时应先滚动列表");
        assert_eq!(if page==2 {h.app.shelf_view.shelf_first}else{h.app.todo_first},1);
        h.hover(H::local(300.0,60.0),0.25);h.wheel(-40.0);
        assert_eq!(h.app.nav.page,next,"列表到底后应能继续翻页");
        h.app.show_page(page);h.hover(H::local(210.0,18.0),0.3);h.wheel(-40.0);
        assert_eq!(h.app.nav.page,next,"顶栏滚轮不应被内容列表拦截");
    }
}

#[test]
fn wheel_clipboard_stays_at_both_edges() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.show_page(2);
    h.app.shelf_view.shelf_tab=1;
    for count in [0, 1, crate::pages::CLIP_ROWS+1] {
        h.app.clipboard.lock().items.clear();
        for i in 0..count { h.app.clipboard.lock().push(&i.to_string(), "09:00"); }
        let last=count.saturating_sub(crate::pages::CLIP_ROWS);
        for (delta, first) in [(40.0,0),(-40.0,last),(-40.0,last),(40.0,0),(40.0,0)] {
            h.hover(H::local(300.0,60.0),0.4);
            h.wheel(delta);
            assert_eq!(h.app.nav.page,2);
            assert_eq!(h.app.shelf_view.clip_first,first);
        }
    }
}
#[test]
fn wheel_handles_small_deltas_without_skipping_pages_on_bursts(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.hover(H::local(210.0,18.0),0.3);
    for _ in 0..16 {h.wheel(-0.5);}
    assert_eq!(h.app.nav.page,1,"高精度滚轮的小幅输入应能累积成一次翻页");
    h.hover(H::local(210.0,18.0),0.3);
    for _ in 0..5 {h.wheel(-40.0);}
    assert_eq!(h.app.nav.page,2,"一次连续滚动不能瞬间跳过多个页面");
}

#[test]
fn navigation_wraps_between_six_pages_and_notification_settings_remain_accessible() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.click(H::local(21.0,68.0));assert_eq!(h.app.nav.page,3);
    h.hover(H::local(300.0,60.0),0.8);h.click(H::local(398.0,68.0));assert_eq!(h.app.nav.page,0);
    h.app.open_settings_category(3);assert!(h.app.settings_open);assert_eq!(h.app.settings_tab,5);
}

#[test]
#[ignore="按需导出常驻组件排列预览"]
fn resident_layout_gallery(){
    fn capture(h:&mut H,raster:&mut crate::preview_raster::Raster,t:f64,draw:bool)->Vec<u8>{
        let env=Env{t,clock:"09:41:00".into(),today:"2026-10-31".into(),date:"10月31日 周六".into(),battery:h.power.0,charging:h.power.1,media:h.media.clone(),bt_ready:true,..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});});
        raster.update_textures(&output.textures_delta);let mut rgba=Vec::new();
        if draw {raster.buf.fill([0.09,0.11,0.18,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);raster.to_rgba8(&mut rgba);}
        raster.free_textures(&output.textures_delta);rgba
    }
    fn demo(battery:bool,music:bool,focus:bool,pet:bool)->H{
        let mut h=H::new();h.app.settings.show_battery=battery;h.app.settings.pet=pet;h.power=(Some(80),true);
        if music {h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};}
        if focus {h.app.pomo.running=true;h.app.pomo.remaining=1283.0;}
        if pet {h.app.pet.react(crate::extras::PetState::Happy);}
        h
    }
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_RESIDENT_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let scenes=[(true,false,false,false),(false,false,false,false),(true,false,true,false),(false,true,false,false),
        (true,true,false,false),(false,true,true,false),(false,false,false,true),(true,false,false,true)];
    for (i,(battery,music,focus,pet)) in scenes.into_iter().enumerate(){
        let mut h=demo(battery,music,focus,pet);let mut raster=crate::preview_raster::Raster::new(1280,380);
        for frame in 0..90 {
            let rgba=capture(&mut h,&mut raster,frame as f64/60.0,frame==89);
            if frame==89 {image::save_buffer(dest.join(format!("layout-{i}.png")),&rgba,1280,380,image::ColorType::Rgba8).unwrap();}
        }
    }
    let mut h=demo(true,true,false,false);let mut raster=crate::preview_raster::Raster::new(1280,380);
    for frame in 0..240 {
        if frame==60 {h.app.settings.show_battery=false;}if frame==180 {h.app.settings.show_battery=true;}
        let rgba=capture(&mut h,&mut raster,frame as f64/60.0,frame%3==0);
        if frame%3==0 {
            let image=image::RgbaImage::from_raw(1280,380,rgba).unwrap();
            image::imageops::crop_imm(&image,200,0,880,80).to_image().save(dest.join(format!("frame-{frame:03}.png"))).unwrap();
        }
    }
    eprintln!("常驻组件预览：{}",dest.display());
}

#[test]
fn page_visibility_is_saved_and_sets_the_starting_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    assert_eq!(h.app.settings.visible_pages,[true;4]);
    assert!(h.app.settings.show_battery);
    h.app.settings.show_battery=false;
    h.app.set_page_visible(0,false);
    h.app.set_page_visible(2,false);
    h.app.settings.save();
    let settings=crate::model::Settings::load();
    assert!(!settings.show_battery);
    assert_eq!(settings.visible_pages,[false,true,false,true]);
    let restarted=App::new(&h.ctx,Options{fetch_lyrics:false,pet_idle:1e9});
    assert_eq!(restarted.page(),1);
    assert_eq!(restarted.settings.visible_pages,settings.visible_pages);
    assert!(!restarted.settings.show_battery);
    std::fs::write(crate::model::config_dir().join("settings.txt"),"visible_pages=0000\n").unwrap();
    assert_eq!(crate::model::Settings::load().visible_pages,[true,false,false,false],"全部关闭时保留第一页");
    // 旧版八位开关升级：邮箱(5)、AI 额度(6)、日程(7) 已下线，只保留 音乐／专注／置物／待办。
    std::fs::write(crate::model::config_dir().join("settings.txt"),"visible_pages=10111111\n").unwrap();
    assert_eq!(crate::model::Settings::load().visible_pages,[true,false,true,true],"旧版八位开关应压缩为四页");
    std::fs::write(crate::model::config_dir().join("settings.txt"),"visible_pages=invalid\n").unwrap();
    assert_eq!(crate::model::Settings::load().visible_pages,[true;4]);
}

#[test]
fn battery_module_hides_power_in_collapsed_and_expanded_headers() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for expanded in [false,true] {
        let mut h=H::new();
        h.app.settings.pet=false;
        h.app.settings.show_battery=false;
        if expanded {h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);} else {h.frame(vec![]);}
        let baseline=h.shapes.clone();
        h.power=(Some(80),true);
        h.t-=1.0/60.0;h.frame(vec![]);
        assert!(h.shapes==baseline,"关闭电量后，本机电池状态不应影响界面布局");
        h.app.settings.show_battery=true;
        h.t-=1.0/60.0;h.frame(vec![]);
        assert!(h.shapes!=baseline,"开启后应显示电量");
        h.power=(None,true);
        h.t-=1.0/60.0;h.frame(vec![]);
        assert!(h.shapes==baseline,"没有电池的设备应自动隐藏电量并释放占位");
    }
}

#[test]
fn battery_module_can_disable_only_the_power_warning() {
    use crate::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.show_battery=false;
    h.power=(Some(10),false);h.frame(vec![]);
    assert_eq!(h.app.pet_state(),PetState::Awake);
    h.network=Some(false);h.frame(vec![]);
    assert_eq!(h.app.pet_state(),PetState::Worried);
}

#[test]
fn page_visibility_skips_hidden_pages_in_every_navigation_control() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    // 日程页的滚轮用于浏览当月列表，会让本页的滚轮翻页断言失真，所以这里让日程保持隐藏。
    for page in [1] {h.app.set_page_visible(page,false);}
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    for expected in [2,3,0] {
        h.click(H::local(398.0,68.0));
        assert_eq!(h.app.nav.page,expected);
        h.hover(H::local(300.0,60.0),0.7);
    }
    h.click(H::local(PW/2.0+6.0,132.8));
    assert_eq!(h.app.nav.page,2,"底部圆点应仅包含启用的页面");
    h.hover(H::local(300.0,60.0),0.7);
    h.frame(vec![egui::Event::Key{key:egui::Key::ArrowRight,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert_eq!(h.app.nav.page,3);
    h.frame(vec![egui::Event::Key{key:egui::Key::ArrowRight,physical_key:None,pressed:false,repeat:false,modifiers:Default::default()}]);
    h.hover(H::local(300.0,60.0),0.7);
    h.frame(vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,40.0),modifiers:Default::default()}]);
    assert_eq!(h.app.nav.page,2,"滚轮应跳过隐藏页面");
    h.click(H::local(21.0,68.0));
    assert_eq!(h.app.nav.page,0);
}

#[test]
fn page_visibility_moves_away_from_hidden_page_and_keeps_one_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.go(3,1.0);
    h.app.todo.adding=true;
    h.app.set_page_visible(3,false);
    assert_eq!(h.app.nav.page,0);
    assert!(!h.app.is_editing());
    for page in [0,1] {h.app.set_page_visible(page,false);}
    assert_eq!(h.app.nav.page,2);
    h.app.set_page_visible(2,false);
    assert!(h.app.settings.visible_pages[2],"不能关闭最后一个页面");
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.click(H::local(398.0,68.0));
    h.click(H::local(21.0,68.0));
    assert_eq!(h.app.nav.page,2);
    h.app.set_page_visible(3,true);
    h.hover(H::local(300.0,60.0),0.7);
    h.click(H::local(398.0,68.0));
    assert_eq!(h.app.nav.page,3,"重新开启的页面应恢复导航");
}

#[test]
fn page_visibility_does_not_reopen_hidden_pages_through_shortcuts_or_drops() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.set_page_visible(2,false);
    h.app.show_page(2);
    assert_eq!(h.app.nav.page,0);
    assert!(h.app.settings_open);
    assert_eq!(h.app.settings_tab,4,"隐藏页面的入口应进入对应设置");
    h.app.set_page_visible(2,false);
    let file=crate::model::config_dir().join("shelf-hidden.txt");
    std::fs::create_dir_all(crate::model::config_dir()).unwrap();
    std::fs::write(&file,"test").unwrap();
    h.app.on_drop(vec![file]);
    assert_eq!(h.app.nav.page,0);
    assert!(h.app.shelf.items.is_empty());
}

#[test]
fn empty_todo_draft_can_be_cancelled_by_clicking_elsewhere(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for text in ["","   "]{
        let mut h=H::new();h.app.todos.items=vec![crate::model::TodoItem{text:"保留任务".into(),due:None,done_on:None,reminded:false,..Default::default()}];h.app.todos.save();
        h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
        assert!(h.app.todo.adding,"点击添加后应保留空白输入框");
        if !text.is_empty(){h.frame(vec![egui::Event::Text(text.into())]);}
        h.cmds.clear();h.click(H::local(65.0,60.0));
        assert!(!h.app.is_editing(),"空白草稿点击坞内其他位置应退出");
        assert!(h.app.todo.buf.is_empty());assert_eq!(h.app.todos.items.len(),1);assert_eq!(h.app.todos.items[0].text,"保留任务");
        assert_eq!(crate::model::TodoList::load().items.len(),1,"不能保存空白待办");
        h.hover(H::local(65.0,60.0),0.3);
        assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)),"取消后不能反复激活窗口");
    }
}

#[test]
fn blank_todo_draft_closes_when_window_focus_is_lost(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
    h.cmds.clear();h.frame(vec![egui::Event::WindowFocused(false)]);
    assert!(!h.app.is_editing(),"点击其他软件后空白草稿应自动退出");
    assert!(h.app.todos.items.is_empty());
    assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)),"失去焦点时不能把窗口再次激活");
}

#[test]
fn todo_input_requests_window_focus_once_and_preserves_typed_drafts(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);h.cmds.clear();h.click(H::local(70.0,109.0));
    assert_eq!(h.cmds.iter().filter(|cmd|matches!(cmd,crate::app::Cmd::Focus)).count(),1,"新建时只应请求一次窗口焦点");
    h.click(H::local(280.0,46.0));assert!(h.app.todo.adding,"点击输入框不能取消草稿");
    h.frame(vec![egui::Event::Text("尚未保存的内容".into())]);h.cmds.clear();
    h.click(H::local(65.0,60.0));h.frame(vec![egui::Event::WindowFocused(false)]);
    for _ in 0..20{h.frame(vec![]);}
    assert!(h.app.todo.adding);assert_eq!(h.app.todo.buf,"尚未保存的内容");assert!(h.app.todos.items.is_empty());
    assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)),"已有内容失焦后应保留草稿，不能抢回焦点");
}

#[test]
fn switching_apps_cancels_only_empty_new_todos(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for text in ["","   ","保留未提交的内容"]{
        let mut h=H::new();h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(70.0,109.0));
        if !text.is_empty(){h.frame(vec![egui::Event::Text(text.into())]);}
        h.cmds.clear();h.app.foreground_changed();h.frame(vec![]);
        assert_eq!(h.app.todo.adding,!text.trim().is_empty());
        assert_eq!(h.app.todo.buf,if text.trim().is_empty(){""}else{text});
        assert!(h.app.todos.items.is_empty());
        assert!(!h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::Focus)));
    }
}

#[test]
fn todo_can_be_added_from_the_todo_page() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.hover(Pos2::new(WIN_W / 2.0, 12.0), 1.0);
    h.app.go(3, 1.0);
    h.hover(H::local(300.0, 60.0), 0.8);
    h.click(H::local(70.0, 109.0));
    assert!(h.app.todo.adding);
    h.hover(H::local(300.0, 60.0), 0.4);
    h.frame(vec![egui::Event::Text("写周报".into())]);
    h.frame(vec![egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]);
    h.hover(H::local(300.0, 60.0), 0.1);
    assert_eq!(h.app.todos.items.first().map(|t| t.text.as_str()), Some("写周报"));
    assert!(!h.app.todo.adding);
}

#[test]
fn timer_counts_elapsed_time_after_sleep() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = H::new();
    h.frame(vec![]);
    h.app.pomo.running = true;
    h.app.pomo.remaining = 120.0;
    h.t += 60.0;
    h.frame(vec![]);
    assert!(h.app.pomo.remaining < 61.0, "挂起恢复后必须补算实际流逝时间");
}

#[test]
fn shelf_remove_keeps_original_file() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let file=crate::model::config_dir().join("保留文件.txt");std::fs::create_dir_all(file.parent().unwrap()).unwrap();std::fs::write(&file,"内容").unwrap();
    h.app.shelf.add(&file).unwrap();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(2,1.0);h.hover(H::local(300.0,60.0),0.8);
    h.click(H::local(233.0,39.0));assert!(h.app.shelf.items.is_empty());assert!(file.exists());assert!(crate::model::Shelf::load().items.is_empty());
}

#[test]
fn shelf_duplicate_drop_reports_existing_files_and_preserves_the_current_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let dir=crate::model::config_dir();std::fs::create_dir_all(&dir).unwrap();
    let files:Vec<_>=(0..7).map(|i|{let path=dir.join(format!("文件-{i}.txt"));std::fs::write(&path,"内容").unwrap();path}).collect();
    h.app.on_drop(files.clone());h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.shelf_view.shelf_first=0;
    h.app.on_drop(vec![files[0].clone()]);h.hover(H::local(300.0,60.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="该文件已在置物架中")));
    h.app.on_drop(vec![files[0].clone(),files[1].clone()]);h.hover(H::local(300.0,60.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="这些文件已在置物架中")));
    assert_eq!(h.app.shelf_view.shelf_first,0,"重复添加不应将列表跳到最后");
    assert_eq!(h.app.shelf.items,files);assert_eq!(crate::model::Shelf::load().items,files);
}

#[test]
fn shelf_drop_distinguishes_new_duplicate_and_missing_files() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let dir=crate::model::config_dir();std::fs::create_dir_all(&dir).unwrap();
    let old=dir.join("已添加.txt");let new=dir.join("新文件.txt");let missing=dir.join("已移动.txt");
    for file in [&old,&new]{std::fs::write(file,"内容").unwrap();}
    h.app.on_drop(vec![old.clone()]);h.app.on_drop(vec![old.clone(),new.clone(),new.clone(),missing.clone()]);
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="已添加 1 个文件，2 个已存在，1 个无法添加")));
    assert_eq!(h.app.shelf.items,vec![old.clone(),new]);
    std::fs::remove_file(&old).unwrap();h.app.on_drop(vec![old]);h.hover(H::local(300.0,60.0),0.3);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="文件不存在或无法访问")));
}
#[test]
fn mascot_rest_and_focus_have_distinct_states(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.opts.pet_idle=45.0;h.frame(vec![]);
    assert_eq!(h.app.pet_state(),crate::extras::PetState::Awake);
    h.t+=50.0;h.frame(vec![]);assert_eq!(h.app.pet_state(),crate::extras::PetState::Dozing);
    h.t+=140.0;h.frame(vec![]);assert_eq!(h.app.pet_state(),crate::extras::PetState::Sleeping);
    h.app.pomo.running=true;assert_eq!(h.app.pet_state(),crate::extras::PetState::Focused);
}

#[test]
fn mascot_stays_beside_the_clock_and_all_resident_components(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;h.power=(Some(80),true);
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    h.app.pomo.running=true;h.app.pomo.remaining=1200.0;
    for _ in 0..120 {h.frame(vec![]);}let width=h.app.current_width();
    h.app.settings.pet=true;for _ in 0..120 {h.frame(vec![]);}
    assert!((h.app.current_width()-width-44.0).abs()<0.1,"胶囊坞应有独立常驻位置");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="09:41")),"胶囊坞与时钟共存");
    assert!(h.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Path(p) if p.points.len()==13 && (p.stroke.width-3.6).abs()<0.01)).count()>=2,"没有闲置或悬停也应显示眼睛");
}

#[test]
fn mascot_and_album_share_the_single_line_lyric_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=true;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,
        artwork:Some(std::sync::Arc::new(egui::ColorImage::new([32,32],egui::Color32::from_rgb(70,120,220)))),..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]晚风吹过了山岗\n[00:15]下一句"));
    for _ in 0..120 {h.frame(vec![]);}
    assert!((h.app.current_width()-348.0).abs()<0.1,"专辑、胶囊坞与原有歌词宽度一起保留");
    let texture=h.app.music_artwork.as_ref().unwrap().1.id();
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.brush.as_ref().is_some_and(|b|b.fill_texture_id==texture))),"专辑封面不能被胶囊坞替代");
    let text=h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t) if t.galley.text()=="晚风吹过了山岗"=>Some(t),_=>None}).unwrap();
    assert_eq!(text.galley.rows.len(),1,"保留单行歌词");
    assert!((text.pos.x+text.galley.rect.center().x-WIN_W/2.0-24.0).abs()<1.5,"歌词在封面和胶囊坞右侧居中");
    assert!(h.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Path(p) if p.points.len()==13 && (2.79..=3.61).contains(&p.stroke.width))).count()>=2);
}

#[test]
fn singing_microphone_moves_relative_to_the_face_and_disappears_on_pause(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
    for _ in 0..120 {h.frame(vec![]);}
    let head=|shapes:&[egui::epaint::ClippedShape]|shapes.iter().find_map(|s|match &s.shape {
        egui::Shape::Circle(circle) if (circle.radius-2.6).abs()<0.01=>Some(circle.center),_=>None,
    });
    let mut offsets=Vec::new();
    for _ in 0..120 {
        h.frame(vec![]);
        let head=head(&h.shapes).expect("演唱应显示球头麦克风");
        let eyes:Vec<_>=h.shapes.iter().filter_map(|s|match &s.shape {
            egui::Shape::Path(p) if p.points.len()==13 && (p.stroke.width-2.8).abs()<0.01=>Some(p.points.iter().map(|p|p.y).sum::<f32>()/13.0),_=>None,
        }).collect();
        assert_eq!(eyes.len(),2);offsets.push(head.y-(eyes[0]+eyes[1])/2.0);
    }
    if crate::bar_layout::animations_enabled(){
        let range=offsets.iter().copied().fold(f32::NEG_INFINITY,f32::max)-offsets.iter().copied().fold(f32::INFINITY,f32::min);
        assert!(range>0.4,"麦克风需要有独立动态");
    }
    h.media.playing=false;for _ in 0..90 {h.frame(vec![]);}
    assert!(head(&h.shapes).is_none(),"暂停后应收起麦克风");
}

#[test]
fn music_mascot_puts_away_the_microphone_during_notices_and_their_fade_out(){
    use crate::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=true;h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]晚风吹过了山岗\n[00:15]下一句"));
    let microphone=|h:&H|h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Circle(c)if(c.radius-2.6).abs()<0.01));
    for _ in 0..90{h.frame(vec![]);}
    assert_eq!(h.app.pet_state(),PetState::Music);assert!(microphone(&h));
    for (title,body) in [("大写锁定","已开启"),("大写锁定","已关闭"),("键盘输入语言","中文"),("系统音量","音量 60%"),("网络连接","Wi-Fi · 已连接"),("电源已连接","当前电量 80%"),("蓝牙设备","耳机")]{
        h.app.alert(title,body,5,false);
        for _ in 0..60{h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Awake,"{title} 出现时应使用普通表情");assert!(!microphone(&h),"{title} 期间不能带着演唱道具");}
        h.t+=2.0;h.frame(vec![]);
        assert!(h.app.notices.notice.is_none());
        assert_eq!(h.app.pet_state(),PetState::Awake,"提示淡出期间也不能提前开始演唱");assert!(!microphone(&h));
        for _ in 0..90{h.frame(vec![]);}
        assert_eq!(h.app.pet_state(),PetState::Music);assert!(microphone(&h),"提示完全结束后应恢复听歌动作");
        assert!(h.media.playing);assert!(h.app.island_lyric().is_some());
    }
    h.app.pet.react(PetState::Happy);h.app.alert("大写锁定","已开启",3,false);h.frame(vec![]);
    assert_eq!(h.app.pet_state(),PetState::Happy,"已有的吉祥物反馈应保留优先级");
}

#[test]
fn selecting_permanent_mascot_or_minimal_applies_the_selected_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.set_minimal(true);for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.settings.minimal && !h.app.settings.pet && h.region.height()<20.0);
    h.app.set_pet(true);for _ in 0..90 {h.frame(vec![]);}
    assert!(h.app.settings.pet && !h.app.settings.minimal && h.region.height()>30.0);
    let saved=crate::model::Settings::load();assert!(saved.pet && !saved.minimal);
}
#[test]
fn mascot_wakes_before_returning_to_idle(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.opts.pet_idle=45.0;
    h.frame(vec![]);h.t+=190.0;for _ in 0..60 {h.frame(vec![]);}
    assert_eq!(format!("{:?}",h.app.pet_state()),"Sleeping");
    h.cursor=Some(Pos2::new(550.0,100.0));h.frame(vec![egui::Event::PointerMoved(h.cursor.unwrap())]);
    assert_eq!(format!("{:?}",h.app.pet_state()),"Waking");
    for _ in 0..180 {h.frame(vec![]);}
    assert_eq!(format!("{:?}",h.app.pet_state()),"Awake");
}
#[test]
fn mascot_reacts_to_music_and_respects_focus(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};h.frame(vec![]);
    assert_eq!(format!("{:?}",h.app.pet_state()),"Music");
    h.app.pomo.running=true;h.frame(vec![]);assert_eq!(format!("{:?}",h.app.pet_state()),"Focused");
    h.app.pomo.running=false;h.media.playing=false;h.frame(vec![]);assert_eq!(format!("{:?}",h.app.pet_state()),"Awake");
}
#[test]
fn mascot_keeps_task_celebration_until_panel_closes(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.todos.items.push(crate::model::TodoItem{text:"完成测试任务".into(),done_on:None,due:None,reminded:false,..Default::default()});
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(3,1.0);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(250.0,47.0));
    assert!(h.app.todos.items[0].done_on.is_some());
    h.hover(H::local(300.0,60.0),4.0);assert_eq!(format!("{:?}",h.app.pet_state()),"Happy");
    h.cursor=None;for _ in 0..240 {h.frame(vec![]);}
    assert_eq!(format!("{:?}",h.app.pet_state()),"Awake");
}
#[test]
fn mascot_focus_completion_celebrates(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.pomo.running=true;h.app.pomo.remaining=0.01;h.frame(vec![]);
    assert_eq!(format!("{:?}",h.app.pet_state()),"Happy");
}
#[test]
fn mascot_power_and_network_recover_without_stale_warning(){
    use crate::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.power=(Some(15),false);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Worried);
    h.power.1=true;h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Awake);
    h.network=Some(false);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Worried);
    h.network=Some(true);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Awake);
    h.power=(None,false);h.network=None;h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Awake);
}
#[test]
fn mascot_system_idle_wakes_once_then_follows_pointer(){
    use crate::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.opts.pet_idle=45.0;
    h.system_idle=Some(200.0);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Sleeping);
    h.system_idle=Some(0.0);h.frame(vec![]);assert_eq!(h.app.pet_state(),PetState::Waking);
    for _ in 0..180 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Awake);
    h.cursor=Some(Pos2::new(WIN_W/2.0,50.0));
    for _ in 0..60 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Curious);assert!(!h.app.morph.open);
}
#[test]
fn mascot_feedback_plays_beside_notice_and_respects_switches(){
    use crate::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.pet.react(PetState::Happy);h.app.alert("待办到时间了","测试提醒",3,false);
    for _ in 0..60 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Happy);
    assert!(h.app.notices.notice.is_some());
    assert!(h.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Path(p) if p.points.len()==13 && (p.stroke.width-3.6).abs()<0.01)).count()>=2,"通知期间胶囊坞仍应可见");
    for _ in 0..240 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Awake);
    h.app.notices.notice=None;for _ in 0..240 {h.frame(vec![]);}assert_eq!(h.app.pet_state(),PetState::Awake);
    h.app.pet.react(PetState::Happy);h.app.settings.minimal=true;h.frame(vec![]);assert!(h.app.pet.reaction.is_none());
    h.app.settings.minimal=false;h.app.settings.pet=false;h.app.pet.react(PetState::Happy);h.frame(vec![]);assert!(h.app.pet.reaction.is_none());
}

#[test]
#[ignore = "按需导出吉祥物的实际渲染画面"]
fn mascot_expression_gallery(){
    use crate::extras::PetState;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::env::var_os("CAPSULEDOCK_PET_PREVIEW_DIR").map(std::path::PathBuf::from)
        .unwrap_or_else(||std::env::temp_dir().join(format!("capsuledock-pet-preview-{}",std::process::id())));
    std::fs::create_dir_all(&dest).unwrap();
    let states=[("awake",PetState::Awake),("curious",PetState::Curious),("waking",PetState::Waking),
        ("happy",PetState::Happy),("music",PetState::Music),
        ("worried",PetState::Worried),("focused",PetState::Focused),("dozing",PetState::Dozing),("sleeping",PetState::Sleeping)];
    for (name,state) in states {
        let mut h=H::new();h.app.pet.react(state);
        let mut raster=crate::preview_raster::Raster::new(1280,380);
        let last=if state==PetState::Waking{42}else{79};
        for frame in 0..=last {
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),today:"2026-10-31".into(),date:"10月31日 周六".into(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H))),time:Some(t),
                viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{
                    let win=ui.max_rect();h.app.frame(ui,win,&env);
                });
            });
            raster.update_textures(&output.textures_delta);
            if frame==last {
                raster.buf.fill([0.09,0.11,0.18,1.0]);
                raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                assert!(rgba.chunks_exact(4).any(|p|p[0]>230&&p[1]>230&&p[2]>230),"表情画面不能空白：{name}");
                image::save_buffer(dest.join(format!("{name}.png")),&rgba,1280,380,image::ColorType::Rgba8).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
    eprintln!("表情预览：{}",dest.display());
}
#[test]
fn todo_due_roundtrip_and_fires_only_once(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items.push(crate::model::TodoItem{text:"会议 | 准备".into(),done_on:None,due:Some("2026-10-28 09:40".into()),reminded:false,..Default::default()});h.app.todos.save();
    let loaded=crate::model::TodoList::load();assert_eq!(loaded.items[0].text,"会议 | 准备");assert_eq!(loaded.items[0].due.as_deref(),Some("2026-10-28 09:40"));
    h.frame(vec![]);assert!(h.app.todos.items[0].reminded);let n=h.cmds.len();h.frame(vec![]);assert_eq!(n,h.cmds.len());assert!(crate::model::TodoList::load().items[0].reminded);
}
#[test]
fn repeated_system_changes_merge_without_losing_history(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.alert("系统音量","音量 20%",5,false);h.frame(vec![]);
    h.app.alert("系统音量","音量 30%",5,false);h.frame(vec![]);
    assert_eq!(h.app.notices.notice.as_ref().unwrap().body,"音量 30%");assert_eq!(h.app.notices.history.len(),2);assert!(h.app.notices.alerts.is_empty());
}
#[test]
fn minimal_takes_precedence_over_mascot(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.pet=true;h.app.settings.minimal=true;
    for _ in 0..120 {h.frame(vec![]);}assert!(h.region.height()<20.0,"启用极简和吉祥物时必须显示细线");
    h.hover(Pos2::new(WIN_W/2.0,2.0),1.0);assert!(h.app.progress()>0.9,"极简线仍要能悬停展开");
}
#[test]
fn focus_completion_offers_starting_the_break(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);h.app.pomo.running=true;h.app.pomo.remaining=1.0;h.t+=2.0;h.frame(vec![]);
    assert!(h.app.pomo.phase==crate::app::Phase::Rest);assert!(!h.app.pomo.running);assert!(h.app.notices.notice.is_some());
    for _ in 0..60 {h.frame(vec![]);}h.click_text("开始");assert!(h.app.pomo.running);
}
#[test]
fn task_editor_saves_and_cancels_inside_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    h.app.todos.items.push(crate::model::TodoItem{text:"准备会议".into(),done_on:None,due:None,reminded:false,..Default::default()});h.app.go(3,1.0);h.hover(H::local(300.0,60.0),0.8);h.app.open_task_dialog(0);
    h.hover(H::local(300.0,60.0),0.2);assert!(h.app.is_editing());h.click(H::local(326.0,110.0));assert!(!h.app.is_editing());assert_eq!(h.app.todos.items[0].text,"准备会议");
    h.app.open_task_dialog(0);h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);assert!(!h.app.is_editing());
}
#[test]
fn focus_timer_gets_room_in_the_collapsed_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();for _ in 0..90 {h.frame(vec![]);}
    let before=h.app.current_width();h.app.pomo.running=true;
    for _ in 0..100 {h.frame(vec![]);}assert!(h.app.current_width()>before+20.0);assert!(h.app.pomo.remaining<1500.0);
}

#[test]
fn switching_to_rest_waits_for_the_start_button(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(1,1.0);h.hover(H::local(260.0,70.0),1.0);
    h.click(H::local(296.0,106.5));
    assert!(h.app.pomo.phase==crate::app::Phase::Rest);
    assert!(!h.app.pomo.running,"切换阶段应等待用户点击开始");
}

#[test]
fn starting_timer_keeps_lyrics_album_and_mascot_with_a_compact_countdown(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.island_lyrics=true;h.app.settings.pet=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,
        artwork:Some(std::sync::Arc::new(egui::ColorImage::new([32,32],egui::Color32::from_rgb(70,120,220)))),..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:05]晚风吹过了山岗\n[00:15]下一句"));
    for _ in 0..120 {h.frame(vec![]);}
    for phase in [crate::app::Phase::Focus,crate::app::Phase::Rest] {
        h.app.pomo.phase=phase;h.app.pomo.remaining=h.app.phase_secs(phase);h.app.pomo.running=true;
        for _ in 0..120 {
            h.frame(vec![]);
            assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("晚风吹过了山岗"),"计时不能关闭坞上歌词");
            assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="晚风吹过了山岗" && t.galley.rows.len()==1)));
            assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="09:41")));
        }
        let texture=h.app.music_artwork.as_ref().unwrap().1.id();
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.brush.as_ref().is_some_and(|b|b.fill_texture_id==texture))));
        let secs=h.app.pomo.remaining.ceil() as u32;let countdown=format!("{:02}:{:02}",secs/60,secs%60);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==countdown)),"歌词模式中也应显示计时");
        assert!(h.app.current_width()<=PW+0.1);assert!((h.hit.height()-34.0).abs()<0.1);
    }
    h.app.pomo.running=false;for _ in 0..90 {h.frame(vec![]);}
    assert_eq!(h.app.island_lyric().map(|(_,line)|line),Some("晚风吹过了山岗"));
}

#[test]
fn timer_keeps_the_existing_lyric_segments_and_content_bounds(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let line="愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光";
    for pet in [true,false] {
        let mut h=H::new();h.app.settings.island_lyrics=true;h.app.settings.pet=pet;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:6.0,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc(&format!("[00:05]{line}\n[00:15]下一句")));
        let shown=|h:&H|h.shapes.iter().find_map(|s|match &s.shape {
            egui::Shape::Text(t) if !t.galley.text().is_empty() && line.contains(t.galley.text())=>Some((t.galley.text().to_owned(),t.visual_bounding_rect(),t.galley.rows.len(),t.galley.elided)),_=>None,
        }).unwrap();
        for position in [6.0,14.0] {
            h.media.position=position;h.app.pomo.running=false;
            for _ in 0..90 {h.frame(vec![]);}let expected=shown(&h).0;
            h.app.pomo.running=true;for _ in 0..120 {h.frame(vec![]);}
            let (text,bounds,rows,elided)=shown(&h);assert_eq!(text,expected,"计时不应改变歌词分段");
            assert_eq!(rows,1);assert!(!elided);
            let secs=h.app.pomo.remaining.ceil() as u32;let countdown=format!("{:02}:{:02}",secs/60,secs%60);
            let timer=h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t) if t.galley.text()==countdown=>Some(t.visual_bounding_rect()),_=>None}).unwrap();
            assert!(bounds.right()+20.0<timer.left(),"歌词与倒计时之间应留出波形和图标的位置");
            assert!(timer.right()<h.hit.right() && bounds.top()>=h.hit.top() && bounds.bottom()<h.hit.bottom());
            assert!((h.app.current_width()-if pet{418.0}else{370.0}).abs()<0.1);
        }
    }
}

#[test]
fn focus_page_selects_stopped_phases_and_starts_only_from_play(){
    use crate::app::Phase;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.focus_minutes=30;h.app.settings.rest_minutes=7;
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(1,1.0);h.hover(H::local(260.0,70.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="休息")));
    h.click(H::local(336.0,106.5));
    assert!(h.app.pomo.phase==Phase::Rest && !h.app.pomo.running);
    assert_eq!(h.app.pomo.remaining,420.0,"休息采用设置中的时长");
    h.click(H::local(264.0,106.5));assert!(h.app.pomo.running);
    h.click(H::local(264.0,106.5));assert!(!h.app.pomo.running);
    let paused=h.app.pomo.remaining;for _ in 0..30 {h.frame(vec![]);}assert_eq!(h.app.pomo.remaining,paused);
    h.click(H::local(264.0,106.5));assert!(h.app.pomo.running && h.app.pomo.phase==Phase::Rest);
    h.click(H::local(336.0,106.5));assert!(h.app.pomo.running,"点击当前阶段不重置计时");
    h.click(H::local(192.0,106.5));assert!(h.app.pomo.phase==Phase::Focus && !h.app.pomo.running);
    assert_eq!(h.app.pomo.remaining,1800.0);
    h.click(H::local(264.0,106.5));assert!(h.app.pomo.running);
    h.click(H::local(336.0,106.5));assert!(h.app.pomo.phase==Phase::Rest && !h.app.pomo.running);
    h.click(H::local(264.0,106.5));assert!(h.app.pomo.running);
    h.click(H::local(329.0,50.0));assert!(!h.app.pomo.running && h.app.pomo.phase==Phase::Rest);
    assert_eq!(h.app.pomo.remaining,420.0,"重置应保留休息阶段");
}

#[test]
fn collapsed_focus_uses_tomato_and_rest_uses_a_white_cup(){
    use crate::app::Phase;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    for phase in [Phase::Focus,Phase::Rest] {
        h.app.pomo.phase=phase;h.app.pomo.remaining=h.app.phase_secs(phase);h.app.pomo.running=true;
        for _ in 0..120 {h.frame(vec![]);}
        let secs=h.app.pomo.remaining.ceil() as u32;let text=format!("{:02}:{:02}",secs/60,secs%60);
        let color=if phase==Phase::Focus{crate::theme::RED_TOP}else{crate::theme::SOFT_GREEN};
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==text && t.fallback_color==color)),"常驻倒计时颜色应跟随阶段");
        let tomato=h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Mesh(m) if m.vertices.iter().any(|v|v.color==crate::theme::RED_TOP)));
        if phase==Phase::Focus {assert!(tomato,"专注时应显示红色番茄水位");}
        else {
            assert!(!tomato,"休息时应替换番茄");
            assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.fill==egui::Color32::from_rgb(245,242,234) && (7.0..13.0).contains(&r.rect.width()) && (5.0..10.0).contains(&r.rect.height()))),"休息时应显示白瓷茶杯");
        }
        assert!((h.hit.height()-34.0).abs()<0.1,"图标不能撑高常驻坞");
    }
}

#[test]
fn focus_and_rest_completion_prepare_the_configured_next_phase_without_starting(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.settings.rest_minutes=7;h.app.settings.focus_minutes=30;
    for _ in 0..5 {
        h.app.pomo.phase=crate::app::Phase::Focus;h.app.pomo.running=true;h.app.pomo.remaining=0.01;h.frame(vec![]);
        assert!(h.app.pomo.phase==crate::app::Phase::Rest && !h.app.pomo.running);
        assert_eq!(h.app.pomo.remaining,420.0);assert!(h.app.notices.notice.is_some());
        h.app.pomo.running=true;h.app.pomo.remaining=0.01;h.frame(vec![]);
        assert!(h.app.pomo.phase==crate::app::Phase::Focus && !h.app.pomo.running);
        assert_eq!(h.app.pomo.remaining,1800.0);
    }
}

#[test]
fn rest_settings_keep_the_existing_duration_and_save_the_new_key(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let _h=H::new();
    assert_eq!(crate::model::Settings::default().rest_minutes,5);
    let path=crate::model::config_dir().join("settings.txt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,"focus_minutes=30\nshort_minutes=7\nlong_minutes=17\n").unwrap();
    let settings=crate::model::Settings::load();assert_eq!(settings.rest_minutes,7);
    settings.save();let saved=std::fs::read_to_string(&path).unwrap();
    assert!(saved.lines().any(|line|line=="rest_minutes=7"));
    assert!(!saved.contains("short_minutes") && !saved.contains("long_minutes"));
    let reloaded=crate::model::Settings::load();assert_eq!(reloaded.rest_minutes,7);assert_eq!(reloaded.focus_minutes,30);
}

#[test]
fn collapsed_focus_animates_without_music_or_mascot_and_stops_in_minimal_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;h.app.pomo.running=true;
    for _ in 0..120 {h.frame(vec![]);}
    if crate::bar_layout::animations_enabled(){assert!(h.animating,"关闭吉祥物后，常驻番茄的水面仍应轻微波动");}
    h.app.settings.minimal=true;for _ in 0..120 {h.frame(vec![]);}
    assert!(!h.animating,"极简细线隐藏计时图标后，应停止其动画刷新");
}
#[test]
fn fluid_music_seek_draws_solid_played_segment(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for lyrics in [false,true] {
        let mut h=H::new();
        h.app.settings.skin=crate::model::Skin::Fluid;
        h.app.show_lyrics=lyrics;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:30.0,duration:120.0,..Default::default()};
        h.hover(Pos2::new(WIN_W/2.0,12.0),1.2);
        let (x,w,y)=if lyrics{(44.0,176.0,82.0)}else{(96.0,221.0,92.6)};
        let start=H::local(x,y);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r)
            if (r.rect.left()-start.x).abs()<1.0 && (r.rect.center().y-start.y).abs()<1.0
            && (r.rect.width()-w*0.25).abs()<1.0 && r.fill.a()==255 && r.fill.r()>160)));
    }
}

#[test]
fn music_seek_drag_previews_and_commits_on_release(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for lyrics in [false,true] {
        let mut h=H::new();h.app.show_lyrics=lyrics;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),position:12.0,duration:120.0,..Default::default()};
        h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
        let (x,w,y)=if lyrics{(44.0,176.0,82.0)}else{(96.0,221.0,92.6)};
        let press=H::local(x+w*0.1,y);h.cursor=Some(press);
        h.frame(vec![egui::Event::PointerMoved(press)]);
        h.frame(vec![egui::Event::PointerButton{pos:press,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        let to=H::local(x+w*0.75,y);h.cursor=Some(to);
        h.frame(vec![egui::Event::PointerMoved(to)]);
        assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="01:30")),"拖动中应显示目标时间");
        assert!(!h.cmds.iter().any(|c|matches!(c,crate::app::Cmd::Media(crate::smtc::Transport::Seek(_)))),"拖动中不应反复发送跳转");
        h.frame(vec![egui::Event::PointerButton{pos:to,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        assert!(h.cmds.iter().any(|c|matches!(c,crate::app::Cmd::Media(crate::smtc::Transport::Seek(t)) if (*t-90.0).abs()<0.1)),"释放应提交目标进度");
    }
}

#[test]
fn volume_hud_drag_maps_pointer_to_percentage(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.frame(vec![]);
    h.app.alert("系统音量","音量 0%",5,false);
    for _ in 0..60 {h.frame(vec![]);}
    assert!(!h.app.morph.open);
    h.click_notice_badge();
    // 缩短后的系统音量条仍支持点击和拖动。
    let (x0,y)=if h.app.compact_notices(){(WIN_W/2.0+(h.app.current_width()-44.0)/2.0+8.0+crate::app::HUD_TRACK_X,55.0)}else{(WIN_W/2.0-h.app.current_width()/2.0+h.app.pet_space()+crate::app::HUD_TRACK_X,19.0)};
    let press=Pos2::new(x0+52.0,y);
    h.cursor=Some(press);h.frame(vec![egui::Event::PointerMoved(press)]);
    h.frame(vec![egui::Event::PointerButton{pos:press,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    let to=Pos2::new(x0+105.0,y);
    h.cursor=Some(to);h.frame(vec![egui::Event::PointerMoved(to)]);
    h.frame(vec![egui::Event::PointerButton{pos:to,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    let last=h.cmds.iter().rev().find_map(|c|match c{crate::app::Cmd::SetVolume(v)=>Some(*v),_=>None});
    assert_eq!(last,Some(70));
}

#[test]
fn bluetooth_connection_soon_after_initial_scan_is_not_suppressed(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.frame(vec![]);
    h.t=2.0;
    h.bt=vec![crate::bt::Device{id:"shokz".into(),name:"Shokz".into(),battery:Some(100),kind:crate::bt::DeviceKind::Bone}];
    h.frame(vec![]);
    assert_eq!(h.app.notices.notice.as_ref().map(|a|a.title.as_str()),Some("蓝牙设备"));
}

#[test]
fn bluetooth_audio_connection_takes_precedence_over_volume_notice(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for volume_already_visible in [false,true] {
        let mut h=H::new();h.frame(vec![]);
        h.app.alert("系统音量","音量 80%",5,false);
        if volume_already_visible {h.frame(vec![]);}
        h.bt=vec![crate::bt::Device{id:"audio".into(),name:"Shokz".into(),battery:Some(80),kind:crate::bt::DeviceKind::Bone}];
        h.frame(vec![]);
        assert_eq!(h.app.notices.notice.as_ref().map(|a|a.title.as_str()),Some("蓝牙设备"),"耳机连接应优先于音量提示");
        assert!(!h.app.notices.alerts.iter().any(|a|a.title=="系统音量"),"切换耳机产生的音量提示不应延后补弹");
        h.t+=3.0;h.frame(vec![]);
        h.app.alert("系统音量","音量 60%",5,false);h.frame(vec![]);
        assert_eq!(h.app.notices.notice.as_ref().map(|a|a.body.as_str()),Some("音量 60%"),"连接后手动调节音量应正常提示");
    }
}

#[test]
fn bluetooth_keyboard_connection_keeps_unrelated_volume_notice(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.frame(vec![]);
    h.app.alert("系统音量","音量 60%",5,false);h.frame(vec![]);
    h.bt=vec![crate::bt::Device{id:"keyboard".into(),name:"Keyboard".into(),battery:None,kind:crate::bt::DeviceKind::Keyboard}];h.frame(vec![]);
    assert_eq!(h.app.notices.notice.as_ref().map(|a|a.title.as_str()),Some("系统音量"));
    assert!(h.app.notices.alerts.iter().any(|a|a.title=="蓝牙设备"));
}

#[test]
fn bluetooth_notices_track_multiple_devices_independently(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    let audio=crate::bt::Device{id:"audio".into(),name:"Shokz".into(),battery:None,kind:crate::bt::DeviceKind::Bone};
    let keyboard=crate::bt::Device{id:"keyboard".into(),name:"QingLong 87".into(),battery:None,kind:crate::bt::DeviceKind::Keyboard};
    h.bt=vec![audio.clone()];h.frame(vec![]);
    h.bt.push(keyboard.clone());h.t=3.0;h.frame(vec![]);
    let notice=h.app.notices.notice.as_ref().unwrap();
    assert_eq!(notice.body,"QingLong 87");
    assert_eq!(notice.bluetooth,Some((crate::bt::DeviceKind::Keyboard,true)));
    h.t=7.0;h.bt=vec![keyboard];h.frame(vec![]);
    let notice=h.app.notices.notice.as_ref().unwrap();
    assert_eq!(notice.body,"Shokz · 已断开");
    assert_eq!(notice.bluetooth,Some((crate::bt::DeviceKind::Bone,false)));
}

#[test]
fn display_choice_saves_and_keeps_disconnected_selection(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings_open=true;h.ctx.set_embed_viewports(false);
    let bounds=crate::display_layout::Bounds{left:0,top:0,right:1920,bottom:1080};
    h.app.displays=vec![
        crate::display_layout::Display{id:"internal".into(),label:"显示器 1 · 主屏".into(),bounds,primary:true},
        crate::display_layout::Display{id:r"\\?\DISPLAY#external".into(),label:"显示器 2 · 外接屏".into(),bounds,primary:false},
    ];
    let mut raster=crate::preview_raster::Raster::new(690,560);
    fn frame(h:&mut H,raster:&mut crate::preview_raster::Raster,events:Vec<egui::Event>){
        h.t+=1.0/60.0;let time=h.t;
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            *capture.lock().unwrap()=Some(ctx.run(egui::RawInput{time:Some(time),viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,560.0))),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()},|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let root=h.ctx.run(egui::RawInput{time:Some(time),..Default::default()},|ctx|h.app.settings_ui(ctx,false));let child=output.lock().unwrap().take().unwrap();
        raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);h.shapes=child.shapes;
    }
    // 设置页内容超过一屏时先向下滚动，直到目标完整出现在窗口内。
    fn reveal(h:&mut H,raster:&mut crate::preview_raster::Raster,label:&str)->Option<Rect>{
        let find=|h:&H|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None}).filter(|r|r.bottom()<550.0);
        for _ in 0..20 {
            if let Some(r)=find(h){return Some(r);}
            frame(h,raster,vec![egui::Event::PointerMoved(Pos2::new(450.0,300.0)),egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,-60.0),modifiers:Default::default()}]);
            for _ in 0..6{frame(h,raster,vec![]);}
        }
        find(h)
    }
    fn click(h:&mut H,raster:&mut crate::preview_raster::Raster,label:&str){
        let pos=reveal(h,raster,label).map(|r|r.center()).unwrap_or_else(||panic!("未找到：{label}"));
        for pressed in [true,false]{frame(h,raster,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
        frame(h,raster,vec![]);
    }
    for _ in 0..3{frame(&mut h,&mut raster,vec![]);}
    assert!(h.app.settings.display_id.is_empty());click(&mut h,&mut raster,"跟随主屏");
    for _ in 0..20{frame(&mut h,&mut raster,vec![]);}
    if let Some(path)=std::env::var_os("CAPSULEDOCK_DISPLAY_PREVIEW"){
        raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(h.shapes.clone(),1.0),1.0);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(path,&rgba,690,560,image::ColorType::Rgba8).unwrap();
    }
    click(&mut h,&mut raster,"显示器 2 · 外接屏");assert_eq!(h.app.settings.display_id,r"\\?\DISPLAY#external");
    assert_eq!(crate::model::Settings::load().display_id,h.app.settings.display_id);
    h.app.displays.pop();frame(&mut h,&mut raster,vec![]);
    reveal(&mut h,&mut raster,"所选屏幕未连接");
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="所选屏幕未连接")));
    assert_eq!(crate::display_layout::selected(&h.app.displays,&h.app.settings.display_id).unwrap().id,"internal");
    click(&mut h,&mut raster,"所选屏幕未连接");click(&mut h,&mut raster,"跟随主屏");
    assert!(crate::model::Settings::load().display_id.is_empty());
}

#[test]
fn full_notifications_return_when_island_lyrics_are_not_visible(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (lyrics,music) in [(false,false),(false,true),(true,false)]{
        let mut h=H::new();h.app.settings.pet=true;h.app.settings.island_lyrics=lyrics;
        if music {h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};}
        for _ in 0..90{h.frame(vec![]);}h.app.alert("大写锁定","已开启",5,false);for _ in 0..60{h.frame(vec![]);}
        for label in ["大写锁定","已开启"]{assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==label&&t.opacity_factor>0.9&&s.clip_rect.contains(t.visual_bounding_rect().center()))),"非坞上歌词状态应直接显示通知文案：{label}");}
        assert!((h.app.current_width()-crate::app::HUD_W-44.0).abs()<1.0,"完整提示保留吉祥物与文案的宽度");
    }
}

#[test]
fn notification_style_follows_the_current_lyric_mode_without_a_new_notice(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗"));
    h.app.alert("大写锁定","已开启",5,false);
    for lyrics in [false,true,false]{
        h.app.settings.island_lyrics=lyrics;for _ in 0..45{h.frame(vec![]);}
        assert_eq!(h.app.compact_notices(),lyrics);
        let has_title=h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="大写锁定"&&t.opacity_factor>0.9));
        assert_eq!(has_title,!lyrics,"当前通知应跟随坞上歌词的显示状态切换样式");
    }
}

#[test]
#[ignore="按需导出普通模式与坞上歌词模式的通知画面"]
fn notice_mode_layout_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_NOTICE_MODES_PREVIEW").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    for lyrics in [false,true]{
        let mut h=H::new();h.app.settings.pet=true;h.app.settings.island_lyrics=lyrics;h.app.pomo.running=lyrics;
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗"));
        let mut raster=crate::preview_raster::Raster::new(1920,405);
        for frame in 0..120{
            let t=frame as f64/60.0;if frame==30{h.app.alert("大写锁定","已开启",5,false);}
            let env=Env{t,clock:"09:41:00".into(),today:"2026-11-01".into(),date:"11月1日 周日".into(),media:h.media.clone(),..Default::default()};
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::app::CANVAS_W,WIN_H))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(1.5),..Default::default()})).collect(),..Default::default()};
            let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});});
            raster.update_textures(&output.textures_delta);
            if frame==119{
                raster.buf.fill([0.94,0.95,0.97,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);let image=image::RgbaImage::from_raw(1920,405,rgba).unwrap();
                image::imageops::crop_imm(&image,105,0,960,140).to_image().save(dest.join(if lyrics{"坞上歌词模式.png"}else{"普通模式.png"})).unwrap();
            }
            raster.free_textures(&output.textures_delta);
        }
    }
}

#[test]
fn lyric_notifications_extend_right_and_keep_the_lyrics_in_place(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let text=|h:&H,label:&str|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==label&&t.opacity_factor>0.9&&s.clip_rect.contains(t.visual_bounding_rect().center())=>Some(t.visual_bounding_rect()),_=>None}).unwrap_or_else(||panic!("应保留可见内容：{label}"));
        let mut h=H::new();h.app.settings.pet=true;h.app.settings.island_lyrics=true;h.power=(Some(75),false);
        h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
        for _ in 0..90{h.frame(vec![]);}
        let label="晚风吹过了山岗";let original=text(&h,label);let right=WIN_W/2.0+h.app.current_width()/2.0;
        h.app.alert("大写锁定","已开启",5,false);
        for _ in 0..60{h.frame(vec![]);assert!(text(&h,label).center().distance(original.center())<0.2,"提示不能挤动或替换原内容");}
        assert!((h.app.current_width()-(right-WIN_W/2.0)*2.0-44.0).abs()<1.0,"通知图标占用 44 点宽度");
        assert!(h.region.right()<right+64.0);assert!(!h.app.morph.open);
        h.t+=3.0;for _ in 0..100{h.frame(vec![]);}
        assert!(text(&h,label).center().distance(original.center())<0.2);
}

#[test]
fn an_open_page_and_lyrics_can_coexist_with_notifications_and_the_bell(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.app.settings.island_lyrics=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
    for _ in 0..90{h.frame(vec![]);}
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.hover(H::local(300.0,60.0),0.5);assert!(h.app.morph.open);
    h.app.alert("键盘输入语言","中文",5,false);for _ in 0..60{h.frame(vec![]);}
    assert!(h.app.morph.open);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="Track")),"展开页面保留歌曲信息");
    assert!(!h.app.compact_notices(),"展开页面时不使用坞上歌词的小图标提示");
}

#[test]
#[ignore="按需录制右侧通知与原坞内容同时显示的实际画面"]
fn right_notice_preview(){
    use std::io::Write;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_RIGHT_NOTICE_PREVIEW").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    h.app.settings.pet=true;h.app.settings.island_lyrics=true;h.app.pomo.running=true;
    h.media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,position:7.0,duration:120.0,..Default::default()};
    h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
    let mut video=std::process::Command::new(std::env::var_os("CAPSULEDOCK_FFMPEG").unwrap_or_else(||"ffmpeg".into())).args(["-y","-loglevel","error","-f","rawvideo","-pix_fmt","rgba","-s","1920x405","-r","30","-i","-","-vf","crop=1440:210:105:0","-c:v","libx264","-crf","19","-pix_fmt","yuv420p"]).arg(dest.join("右侧通知演示.mp4")).stdin(std::process::Stdio::piped()).spawn().unwrap();
    let mut pipe=video.stdin.take().unwrap();let mut raster=crate::preview_raster::Raster::new(1920,405);
    for frame in 0..420{
        let t=frame as f64/30.0;
        if frame==90{h.app.alert("大写锁定","已开启",5,false);}
        if frame==240{h.app.alert("蓝牙设备","OpenMove by Shokz · 电量 80%",5,false);}
        let env=Env{t,clock:"09:41:00".into(),today:"2026-11-01".into(),date:"11月1日 周日".into(),media:h.media.clone(),..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::app::CANVAS_W,WIN_H))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(1.5),..Default::default()})).collect(),..Default::default()};
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});});
        raster.update_textures(&output.textures_delta);raster.buf.fill([0.94,0.95,0.97,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);pipe.write_all(&rgba).unwrap();
        if let Some(name)=match frame{60=>Some("原坞"),125=>Some("大小写提示"),270=>Some("蓝牙提示"),390=>Some("恢复原坞"),_=>None}{let image=image::RgbaImage::from_raw(1920,405,rgba).unwrap();image::imageops::crop_imm(&image,105,0,1440,210).to_image().save(dest.join(format!("{name}.png"))).unwrap();}
        raster.free_textures(&output.textures_delta);
    }
    drop(pipe);assert!(video.wait().unwrap().success());
}

#[test]
fn hud_expands_smoothly_from_collapsed_width(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    for _ in 0..60 { h.frame(vec![]); }
    let before=h.app.current_width();
    h.app.alert("大写锁定","已开启",5,false);
    h.frame(vec![]);
    assert!(h.app.current_width()-before < 30.0, "提示条不能在第一帧突然跳到目标宽度");
}

#[test]
fn bluetooth_notification_keeps_its_full_message_outside_lyric_mode(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();for _ in 0..90 {h.frame(vec![]);}
    h.app.alert("蓝牙设备","OpenMove by Shokz · 电量 80%",5,false);
    for _ in 0..60 {h.frame(vec![]);}
    assert!((h.app.current_width()-crate::app::HUD_W-h.app.pet_space()).abs()<1.0);
    assert!(h.region.height()>60.0,"两行蓝牙提示保持完整高度");
    h.click_notice_badge();
    for label in ["OpenMove by Shokz","已连接","80%"]{assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()==label)),"点击后可查看设备信息：{label}");}
}

#[test]
fn hud_repeated_volume_changes_do_not_restart_entrance(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.frame(vec![]);
    h.app.alert("系统音量","音量 20%",5,false);
    for _ in 0..45 {h.frame(vec![]);}
    let before=h.app.current_width();
    h.app.alert("系统音量","音量 30%",5,false);
    h.frame(vec![]);
    assert!((h.app.current_width()-before).abs()<5.0);
    assert_eq!(h.app.notices.notice.as_ref().unwrap().body,"音量 30%");
}

#[test]
fn hud_shrinks_smoothly_after_expiration(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    for _ in 0..90 {h.frame(vec![]);}let collapsed=h.app.current_width();
    h.app.alert("大写锁定","已开启",5,false);
    for _ in 0..60 {h.frame(vec![]);}
    let before=h.app.current_width();
    h.t+=2.0;
    h.frame(vec![]);
    assert!(h.app.notices.notice.is_none());
    assert!(h.app.current_width()>collapsed+5.0);
    assert!(h.app.current_width()<before);
}

#[test]
fn bluetooth_badge_follows_connect_and_disconnect(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    // 首次扫描已连接的设备，记录但不作为新连接提醒。
    h.bt=vec![crate::bt::Device{id:"buds".into(),name:"Buds".into(),battery:Some(80),kind:crate::bt::DeviceKind::Earbuds}];h.frame(vec![]);
    assert!(h.app.notices.notice.is_none());
    assert_eq!(h.app.bt_shown.first().map(|d|d.name.as_str()),Some("Buds"));
    // 之后断开、再连上，各弹一条 HUD 通知。
    h.t=20.0;h.bt.clear();h.frame(vec![]);
    assert_eq!(h.app.notices.notice.as_ref().map(|a|a.body.as_str()),Some("Buds · 已断开"));
    assert!(h.app.bt_shown.is_empty());
    h.t=30.0;h.bt=vec![crate::bt::Device{id:"buds".into(),name:"Buds".into(),battery:Some(80),kind:crate::bt::DeviceKind::Earbuds}];h.frame(vec![]);
    assert_eq!(h.app.notices.notice.as_ref().map(|a|a.body.as_str()),Some("Buds · 电量 80%"));
    // 没有设备时不留残留状态。
    h.t=40.0;h.bt.clear();h.frame(vec![]);
    assert!(h.app.bt_shown.is_empty()&&h.app.notices.notice.as_ref().is_none_or(|a|a.title!="蓝牙设备"||a.body=="Buds · 已断开"));
}

#[test]
fn focus_settings_are_inside_focus_page(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);h.app.go(1,1.0);h.hover(H::local(300.0,60.0),0.8);h.click(H::local(110.0,18.0));assert_eq!(h.app.nav.page,1);assert_eq!(h.app.subpage,Some(1));
}

#[test]
fn language_notice_text_is_vertically_centered() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.frame(vec![]);
    h.app.alert("键盘输入语言","英语",5,false);
    for _ in 0..60 {h.frame(vec![]);}
    h.click_notice_badge();
    fn find(shapes:&[egui::epaint::ClippedShape])->Option<Rect>{
        shapes.iter().find_map(|s|match &s.shape {
            egui::Shape::Text(text) if text.galley.text()=="英语"=>Some(text.visual_bounding_rect()),
            _=>None,
        })
    }
    let text=find(&h.shapes).expect("输入语言状态应可见");
    assert!((text.center().y-19.0).abs()<1.0,"文字应与图标垂直居中：{text:?}");
}

#[test]
fn todo_notice_view_locates_the_reminded_task() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.todos.items=(0..7).map(|i|crate::model::TodoItem{text:format!("任务 {i}"),done_on:None,due:if i==6{Some("2026-10-28 09:40".into())}else{None},reminded:false,..Default::default()}).collect();
    for _ in 0..60 {h.frame(vec![]);}
    h.click_text("查看");
    assert_eq!(h.app.nav.page,3);
    assert!(h.app.todo_first>=4,"打开待办页时应能看到收到提醒的任务");
}

#[test]
fn notification_bell_sways_twice_then_returns_to_rest() {
    for (i,expected_positive) in [true,false,true,false].into_iter().enumerate(){
        let angle=crate::paint::bell_swing(0.95*(i as f64*0.25+0.125));
        assert_eq!(angle>0.0,expected_positive);
    }
    assert_eq!(crate::paint::bell_swing(1.0),0.0);
}

#[test]
fn hud_titles_and_states_share_the_icon_center_without_overlapping() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for (title,body,left,right) in [
        ("网络连接","Wi-Fi · 网络已连接","Wi-Fi","网络已连接"),
        ("网络连接","以太网 · 网络已断开","以太网","网络已断开"),
        ("大写锁定","已开启","大写锁定","已开启"),
        ("键盘输入语言","中文","输入语言","中文"),
        ("电源已连接","当前电量 76%","电源已连接","当前电量 76%"),
        ("已切换电池供电","当前电量 76%","电池供电","当前电量 76%"),
    ] {
        let mut h=H::new();h.frame(vec![]);h.app.alert(title,body,5,false);
        for _ in 0..60{h.frame(vec![]);}
        h.click_notice_badge();
        let find=|needle:&str|h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==needle=>Some(t.visual_bounding_rect()),_=>None}).unwrap_or_else(||panic!("未显示状态文字：{needle}"));
        let l=find(left);let r=find(right);
        assert!((l.center().y-19.0).abs()<1.0 && (r.center().y-19.0).abs()<1.0,"{title} 应垂直居中");
        assert!(l.right()+8.0<=r.left(),"{title} 的标题和状态应留出间距");
    }
}

#[test]
fn low_power_notice_dismisses_without_opening_any_page() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.frame(vec![]);h.app.alert("电量偏低","剩余 18% · 请连接电源",5,false);
    for _ in 0..60{h.frame(vec![]);}
    h.click_text("知道了");
    assert!(h.app.notices.notice.is_none());
    assert_eq!(h.app.nav.page,0,"低电量提示只需确认关闭");
}

#[test]
fn adding_a_todo_saves_content_then_offers_an_optional_reminder() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for remind in [false,true] {
        let mut h=H::new();h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);
        h.click(H::local(70.0,109.0));h.frame(vec![egui::Event::Text("提交产品方案".into())]);
        h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
        assert_eq!(h.app.todos.items[0].text,"提交产品方案");
        assert!(h.app.todos.items[0].due.is_none());
        assert!(h.app.todo.task_dialog.is_some(),"保存内容后应进入可选提醒步骤");
        assert_eq!(crate::model::TodoList::load().items[0].text,"提交产品方案");
        h.frame(vec![egui::Event::Key{key:egui::Key::Enter,physical_key:None,pressed:false,repeat:false,modifiers:Default::default()}]);
        if remind {
            let initial=chrono::NaiveDateTime::parse_from_str(&h.app.todo.task_dialog.as_ref().unwrap().picker.due(),"%Y-%m-%d %H:%M").unwrap();
            h.hover(H::local(235.0,77.0),0.4);h.wheel(-40.0);h.picker_click("完成");
            let expected=(initial+chrono::Duration::days(1)).format("%Y-%m-%d %H:%M").to_string();
            assert_eq!(h.app.todos.items[0].due.as_deref(),Some(expected.as_str()));
        }else{h.picker_click("暂不提醒");assert!(h.app.todos.items[0].due.is_none());}
        assert!(h.app.todo.task_dialog.is_none());
        assert!(h.app.todos.items[0].done_on.is_none());
        assert_eq!(crate::model::TodoList::load().items[0].due,h.app.todos.items[0].due);
    }
}

#[test]
fn todo_clock_button_sets_a_reminder_without_completing_the_task() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items=vec![crate::model::TodoItem{text:"已有任务".into(),done_on:None,due:None,reminded:false,..Default::default()}];
    h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);h.click(H::local(343.0,46.0));
    assert!(h.app.todo.task_dialog.is_some(),"每项待办应有可见的提醒入口");
    assert!(h.app.todos.items[0].done_on.is_none(),"时钟入口不能同时勾选任务");
    h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert!(h.app.todo.task_dialog.is_none());assert_eq!(h.app.todos.items[0].text,"已有任务");
}

#[test]
fn todo_reminder_picker_is_embedded_in_the_island(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.todos.items=vec![crate::model::TodoItem{text:"准备会议".into(),done_on:None,due:None,reminded:false,..Default::default()}];
    h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);
    h.ctx.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(|_,_|panic!("待办提醒必须在坞内选择，不能创建独立窗口"));
    h.click(H::local(343.0,46.0));
    for _ in 0..90 {h.frame(vec![]);}
    assert_eq!(h.app.expanded_height(),PH,"右侧滚轮应保持待办页原来的高度");
    assert!(h.hit.height()<=PH+2.0,"选择时间不能撑大坞体");
    for label in ["准备会议","已完成","取消","完成","暂不提醒"] {
        let (rect,clip)=h.shapes.iter().find_map(|s|match &s.shape {
            egui::Shape::Text(t) if t.galley.text()==label=>Some((t.visual_bounding_rect(),s.clip_rect)),_=>None,
        }).unwrap_or_else(||panic!("坞内缺少选项：{label}"));
        assert!(h.hit.contains_rect(rect),"选项 {label} 应在坞体命中区域内");
        assert!(h.region.contains_rect(rect) && clip.contains_rect(rect),"选项 {label} 不能被窗口或坞体裁剪");
    }
    h.frame(vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert!(h.app.todo.task_dialog.is_none());assert!(h.app.todos.items[0].due.is_none());
}

#[test]
fn reminder_wheels_select_date_hour_and_minute_without_typing(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::Local::now().date_naive()+chrono::Duration::days(2);let original=format!("{day} 09:00");
    h.app.todos.items=vec![crate::model::TodoItem{text:"准备方案".into(),due:Some(original.clone()),done_on:None,reminded:false,..Default::default()}];
    h.app.open_task_reminder(0);
    for x in [235.0,290.0,346.0] {h.hover(H::local(x,77.0),0.4);h.wheel(-40.0);}
    assert_eq!(h.app.todos.items[0].due,Some(original),"调整时不能提前写入提醒");
    h.picker_click("完成");
    let next=day+chrono::Duration::days(1);
    assert_eq!(h.app.todos.items[0].due,Some(format!("{next} 10:01")));
    assert!(h.app.todo.task_dialog.is_none());
}

#[test]
fn reminder_wheels_support_clicks_and_dragging(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::Local::now().date_naive()+chrono::Duration::days(2);
    h.app.todos.items=vec![crate::model::TodoItem{text:"确认方案".into(),due:Some(format!("{day} 10:30")),done_on:None,reminded:false,..Default::default()}];
    h.app.open_task_reminder(0);h.hover(H::local(300.0,60.0),1.0);
    h.click(H::local(346.0,96.0));h.click(H::local(290.0,58.0));
    let start=H::local(235.0,77.0);h.cursor=Some(start);
    h.frame(vec![egui::Event::PointerMoved(start),egui::Event::PointerButton{pos:start,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
    for y in [59.0,41.0]{let point=H::local(235.0,y);h.cursor=Some(point);h.frame(vec![egui::Event::PointerMoved(point)]);}
    let end=H::local(235.0,41.0);
    h.frame(vec![egui::Event::PointerButton{pos:end,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    h.picker_click("完成");
    assert_eq!(h.app.todos.items[0].due,Some(format!("{} 09:31",day+chrono::Duration::days(2))));
    assert!(h.app.todos.items[0].done_on.is_none());
}

#[test]
fn reminder_wheels_accumulate_small_trackpad_movements(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let day=chrono::Local::now().date_naive()+chrono::Duration::days(2);
    h.app.todos.items=vec![crate::model::TodoItem{text:"检查进度".into(),due:Some(format!("{day} 10:30")),done_on:None,reminded:false,..Default::default()}];
    h.app.open_task_reminder(0);h.hover(H::local(346.0,77.0),1.0);
    for _ in 0..80{h.wheel(-0.5);}
    h.picker_click("完成");
    assert_eq!(h.app.todos.items[0].due,Some(format!("{day} 10:31")));
    assert_eq!(h.app.page(),3,"操作时间滚轮不能触发翻页");
}

#[test]
fn reminder_picker_rejects_past_time_and_cancel_preserves_the_task(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    let old=format!("{} 09:00",chrono::Local::now().date_naive()-chrono::Duration::days(1));
    h.app.todos.items=vec![crate::model::TodoItem{text:"保留任务".into(),due:Some(old.clone()),done_on:None,reminded:true,..Default::default()}];
    h.app.open_task_reminder(0);h.picker_click("完成");h.picker_frame(vec![]);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t)if t.galley.text()=="请选择晚于当前的时间")));
    assert_eq!(h.app.todos.items[0].due,Some(old.clone()));
    h.picker_click("取消");assert!(h.app.todo.task_dialog.is_none());assert_eq!(h.app.todos.items[0].due,Some(old));
}

#[test]
#[ignore="按需导出提醒选择器的实际画面"]
fn reminder_picker_layout_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_READER_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    {
        let picker=true;
        let mut h=H::new();h.ctx.set_embed_viewports(true);
        h.app.todos.items=vec![crate::model::TodoItem{text:"提交产品方案".into(),due:None,done_on:None,reminded:false,..Default::default()}];h.app.open_task_reminder(0);
        let (w,height)=if picker{(WIN_W as u32,WIN_H as u32)}else{(700,620)};let ppp=1.5;
        let mut raster=crate::preview_raster::Raster::new((w as f32*ppp)as usize,(height as f32*ppp)as usize);
        let last=89;
        for f in 0..=last{
            let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(w as f32,height as f32))),time:Some(f as f64/60.0),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
            let out=h.ctx.run(raw,|ctx|{
                let env=Env{t:f as f64/60.0,clock:"09:41:00".into(),today:chrono::Local::now().date_naive().to_string(),date:"周日".into(),..Default::default()};
                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});
            });raster.update_textures(&out.textures_delta);
            if f==last{raster.buf.fill([0.07,0.09,0.13,1.0]);raster.draw(&h.ctx.tessellate(out.shapes,out.pixels_per_point),out.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join("reminder-picker.png"),&rgba,(w as f32*ppp)as u32,(height as f32*ppp)as u32,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&out.textures_delta);
        }
    }
}

#[test]
fn remove_icons_are_centered_in_their_buttons_at_each_scale(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for scale in [1.0,1.25,1.5,2.0]{
        let mut h=H::new();h.ctx.set_pixels_per_point(scale);
        h.app.todos.items=["第一项","第二项","第三项"].into_iter().map(|text|crate::model::TodoItem{text:text.into(),due:None,done_on:None,reminded:false,..Default::default()}).collect();
        h.app.show_page(3);h.hover(H::local(300.0,60.0),1.0);
        for i in 0..3usize{
            let button=h.ctx.read_response(egui::Id::new(("card",true,("todo-delete",i)))).unwrap().rect;
            let ink=h.shapes.iter().filter_map(|s|match &s.shape{
                egui::Shape::Text(t) if t.galley.text()=="×" && button.contains(t.visual_bounding_rect().center())=>Some(t.visual_bounding_rect()),
                egui::Shape::LineSegment{points,..} if points.iter().all(|p|button.shrink(2.0).contains(*p)) && (points[0].x-points[1].x).abs()>2.0 && (points[0].y-points[1].y).abs()>2.0=>Some(Rect::from_two_pos(points[0],points[1])),
                _=>None,
            }).fold(Rect::NOTHING,|a,b|a.union(b));
            assert!(ink.is_positive(),"删除按钮应有可见图案");
            assert!(ink.center().distance(button.center())<0.4,"缩放 {scale}：删除图案中心 {:?} 偏离按钮中心 {:?}",ink.center(),button.center());
        }
        h.click(H::local(361.5,68.0));
        assert_eq!(h.app.todos.items.iter().map(|t|t.text.as_str()).collect::<Vec<_>>(),["第一项","第三项"]);
        assert!(h.app.todos.items.iter().all(|t|t.done_on.is_none()),"删除入口不能同时勾选其他待办");
    }
}

#[test]
#[ignore="按需检查全部页面与设置的实际排版"]
fn ui_alignment_layout_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_UI_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    fn fixture()->H{
        let mut h=H::new();h.app.settings.pet=false;h.app.settings.island_lyrics=false;h.power=(Some(76),false);
        h.media=crate::smtc::NowPlaying{title:"愿每一个认真生活的人，都能在平凡的日子里找到属于自己的光".into(),artist:"The Lanterns · Live Session".into(),duration:240.0,position:65.0,playing:true,..Default::default()};
        h.app.todos.items=["整理会议资料","Review design details","确认项目需求与下一阶段开发排期"].into_iter().map(|text|crate::model::TodoItem{text:text.into(),due:None,done_on:None,reminded:false,..Default::default()}).collect();
        h.app.shelf.items=["项目设计方案与交付说明.pdf","design-preview.png","本周工作安排.docx","project-files.zip","notes.md"].into_iter().map(std::path::PathBuf::from).collect();
        h
    }
    let collapsed=["collapsed","volume","caps","language","power","network","bt-headphones","bt-keyboard","bt-mouse","bt-gamepad","bt-phone","bt-other","bt-speaker"];
    let scenes=["page-0","page-1","page-2","page-3","lyrics","todo-empty","shelf-empty","reminder","focus-running"];
    for ppp in [1.25,2.0]{for name in scenes.into_iter().chain(collapsed){
        let mut h=fixture();let page=name.strip_prefix("page-").and_then(|p|p.parse::<usize>().ok()).unwrap_or(match name{"lyrics"=>0,"shelf-empty"=>2,_=>3});
        if name=="todo-empty"{h.app.todos.items.clear();}if name=="shelf-empty"{h.app.shelf.items.clear();}
        if !collapsed.contains(&name){h.app.show_page(page);}
        if name=="focus-running"{h.app.show_page(1);h.app.pomo.running=true;h.app.pomo.remaining=1283.0;}
        if let Some(kind)=name.strip_prefix("bt-"){
            let kind=match kind{"headphones"=>crate::bt::DeviceKind::Headphones,"keyboard"=>crate::bt::DeviceKind::Keyboard,"mouse"=>crate::bt::DeviceKind::Mouse,"gamepad"=>crate::bt::DeviceKind::Gamepad,"phone"=>crate::bt::DeviceKind::Phone,"speaker"=>crate::bt::DeviceKind::Speaker,_=>crate::bt::DeviceKind::Other};
            h.app.bluetooth_alert(&crate::bt::Device{id:"preview".into(),name:"我的蓝牙设备 · 超长设备名称与连接状态".into(),battery:Some(100),kind},true);
        }
        match name{"volume"=>{h.app.volume_ui=Some((64,false));h.app.alert("系统音量","64%",0,false);},"caps"=>h.app.alert("大写锁定","已开启",0,false),"language"=>h.app.alert("键盘输入语言","中文（简体，中国）",0,false),"network"=>h.app.alert("网络连接","Wi-Fi · 网络已连接",0,false),"power"=>h.app.alert("电源已连接","当前电量 76%",0,false),_=>{}}
        if name=="lyrics"{h.app.show_lyrics=true;h.app.set_lyrics(&h.media.title,&h.media.artist,240.0,crate::lyrics::parse_lrc("[00:00]慢慢走过这段时光\n[01:00]愿每个认真生活的人都能找到属于自己的光\n[01:10]愿风带你去远方"));}
        if name=="reminder"{h.app.open_task_reminder(0);}
        let (width,height)=((WIN_W*ppp)as u32,(WIN_H*ppp)as u32);let mut raster=crate::preview_raster::Raster::new(width as usize,height as usize);
        for f in 0..90{
            let t=f as f64/60.0;let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(WIN_W,WIN_H))),time:Some(t),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
            let env=Env{t,clock:"09:41:00".into(),today:"2026-11-01".into(),date:"11月1日 周日".into(),cursor:if collapsed.contains(&name){None}else{Some(H::local(150.0,40.0))},media:h.media.clone(),battery:h.power.0,bt_ready:true,..Default::default()};
            let out=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{let win=ui.max_rect();h.app.frame(ui,win,&env);});});raster.update_textures(&out.textures_delta);
            if f==89{raster.buf.fill([0.07,0.09,0.13,1.0]);raster.draw(&h.ctx.tessellate(out.shapes,out.pixels_per_point),out.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("{name}-{}.png",(ppp*100.0)as u32)),&rgba,width,height,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&out.textures_delta);
        }
    }}
    for width in [600.0,690.0]{for tab in 0..8{
        let mut h=fixture();h.app.settings_tab=tab;h.app.open_settings();h.ctx.set_embed_viewports(false);
        let size=egui::vec2(width,if width==600.0{450.0}else{560.0});let ppp=1.25;
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{let id=viewport.ids.this;let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,size)),viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(ppp),focused:Some(true),..Default::default()})).collect(),focused:true,..Default::default()};*capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));});
        let (w,height)=((size.x*ppp)as u32,(size.y*ppp)as u32);let mut raster=crate::preview_raster::Raster::new(w as usize,height as usize);
        for f in 0..5{let root=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));raster.update_textures(&root.textures_delta);let child=output.lock().unwrap().take().unwrap();raster.update_textures(&child.textures_delta);
            if f==4{raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(dest.join(format!("settings-{tab}-{}.png",width as u32)),&rgba,w,height,image::ColorType::Rgba8).unwrap();}
            raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
        }
    }}
}

#[test]
fn shelf_remove_icon_stays_centered_on_hover_and_preserves_the_file(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for ppp in [1.0,1.25,1.5,2.0]{
        let mut h=H::new();h.ctx.set_pixels_per_point(ppp);
        let path=crate::model::config_dir().join("设计说明.txt");std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,"保留原文件").unwrap();
        h.app.shelf.items=vec![path.clone()];h.app.show_page(2);
        for pointer in [H::local(300.0,60.0),H::local(233.0,39.0)]{
            h.hover(pointer,1.0);
            let button=h.ctx.read_response(egui::Id::new(("card",true,("shelf-remove",&path)))).unwrap().rect;
            let ink=h.shapes.iter().filter_map(|s|match &s.shape{
                egui::Shape::LineSegment{points,..} if points.iter().all(|p|button.shrink(2.0).contains(*p)) && (points[0].x-points[1].x).abs()>2.0 && (points[0].y-points[1].y).abs()>2.0=>Some(Rect::from_two_pos(points[0],points[1])),_=>None,
            }).fold(Rect::NOTHING,|a,b|a.union(b));
            assert!(ink.is_positive() && ink.center().distance(button.center())<0.4,"置物架移除图案应与按钮同心");
        }
        h.click(H::local(233.0,39.0));assert!(h.app.shelf.items.is_empty());assert!(path.exists());
    }
}

#[test]
fn resident_components_can_be_dragged_saved_and_reset(){
    use crate::bar_layout::{CLOCK,BATTERY};
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    fn draw(h:&mut H,events:Vec<egui::Event>)->[Pos2;2]{
        let mut points=[Pos2::ZERO;2];
        h.t+=0.02;
        let out=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,300.0))),time:Some(h.t),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.resident_order_settings(ui);for (index,slot) in [CLOCK,BATTERY].into_iter().enumerate(){points[index]=ui.ctx().read_response(ui.id().with(("resident-icon",slot))).unwrap().rect.center();}});});
        h.shapes=out.shapes;points
    }
    fn point(h:&H,label:&str)->Pos2{h.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap()}
    draw(&mut h,vec![]);let [from,to]=draw(&mut h,vec![]);
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    draw(&mut h,vec![egui::Event::PointerMoved(from),button(from,true)]);
    draw(&mut h,vec![egui::Event::PointerMoved(from+egui::vec2(0.0,10.0))]);
    draw(&mut h,vec![egui::Event::PointerMoved(to)]);
    for _ in 0..3{draw(&mut h,vec![]);}
    draw(&mut h,vec![button(to,false)]);
    assert_eq!(h.app.settings.resident_order.first(),Some(&CLOCK));
    assert!(h.app.settings.resident_order.contains(&BATTERY));
    h.app.settings.save();assert_eq!(crate::model::Settings::load().resident_order,h.app.settings.resident_order);
    let reset=point(&h,"恢复默认");draw(&mut h,vec![egui::Event::PointerMoved(reset),button(reset,true)]);draw(&mut h,vec![button(reset,false)]);
    assert!(h.app.settings.resident_order.is_empty());
}

#[test]
fn hidden_components_do_not_keep_static_pages_animating(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;
    h.app.show_page(3);h.hover(H::local(300.0,60.0),2.0);
    assert!(!h.animating);
    let mut moving=crate::anim::Spring::new(0.0,230.0,0.9);moving.target=200.0;
    h.app.shelf_view.shelf_x.insert("hidden.txt".into(),moving);
    h.frame(vec![]);
    assert!(!h.animating,"离开置物页后，未完成的文件动画不能让其他页面持续刷新");
    h.app.shelf_view.shelf_x.clear();h.app.pomo.running=true;
    h.hover(H::local(300.0,60.0),1.5);
    assert!(!h.animating,"待办页没有显示专注动画，计时继续但不应强制连续刷新");
}

#[test]
fn minimal_music_stops_hidden_playback_animations(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.set_minimal(true);
    h.media=crate::smtc::NowPlaying{title:"歌曲".into(),artist:"歌手".into(),playing:true,..Default::default()};
    for _ in 0..180{h.frame(vec![]);}
    assert!(!h.app.morph.open);
    assert!(!h.animating,"极简模式隐藏媒体组件后，不应继续为播放动画刷新");
}

#[test]
fn native_app_icon_uses_bounded_resolution(){
    let icon=crate::theme::app_icon();
    assert!(icon.width<=128 && icon.height<=128,"原生窗口图标不需要常驻完整的 1024 像素品牌原图");
    assert_eq!(icon.rgba.len(),(icon.width*icon.height*4)as usize);
    assert!(std::sync::Arc::ptr_eq(&icon,&crate::theme::app_icon()));
}

#[test]
fn page_order_persists_and_controls_navigation() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.settings.page_order=[2,3,0,1];
    h.app.settings.save();
    let settings=crate::model::Settings::load();
    assert_eq!(settings.page_order,[2,3,0,1]);
    let restarted=App::new(&h.ctx,Options{fetch_lyrics:false,pet_idle:1e9});
    assert_eq!(restarted.page(),2);
    for page in [1] {h.app.set_page_visible(page,false);}
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    for expected in [2,3,0] {
        h.click(H::local(398.0,68.0));
        assert_eq!(h.app.nav.page,expected);
        h.hover(H::local(300.0,60.0),0.7);
    }
    h.click(H::local(21.0,68.0));
    assert_eq!(h.app.nav.page,3);
    h.app.set_page_visible(3,false);
    assert_eq!(h.app.nav.page,2);
    for invalid in ["0,0,1,2","4,1,2,3","0,1","invalid"] {
        std::fs::write(crate::model::config_dir().join("settings.txt"),format!("page_order={invalid}\n")).unwrap();
        assert_eq!(crate::model::Settings::load().page_order,[0,1,2,3]);
    }
}

#[test]
fn page_order_drag_applies_on_release_and_saves() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings_tab=7;h.app.open_settings();h.ctx.set_embed_viewports(false);
    fn draw(h:&mut H,events:Vec<egui::Event>){
        let output=std::sync::Arc::new(Mutex::new(None));let capture=output.clone();
        h.t+=0.02;let time=h.t;
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1000.0))),time:Some(time),events:events.clone(),viewports:std::iter::once((id,egui::ViewportInfo::default())).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let _=h.ctx.run(egui::RawInput::default(),|ctx|h.app.settings_ui(ctx,false));
        let child=output.lock().unwrap().take().unwrap();h.shapes=child.shapes;
    }
    for _ in 0..3 {draw(&mut h,vec![]);}
    let point=|h:&H,label:&str|h.shapes.iter().find_map(|s|match &s.shape {egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().left_center()),_=>None}).unwrap();
    let from=point(&h,"播放控制与同步歌词")+egui::vec2(-18.0,-6.0);
    let to=point(&h,"临时收纳文件位置");
    let button=|pos,pressed|egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()};
    draw(&mut h,vec![egui::Event::PointerMoved(from),button(from,true)]);
    draw(&mut h,vec![egui::Event::PointerMoved(from+egui::vec2(0.0,10.0))]);
    draw(&mut h,vec![egui::Event::PointerMoved(to)]);
    assert_eq!(h.app.settings.page_order,[0,1,2,3]);
    draw(&mut h,vec![button(to,false)]);
    assert_eq!(h.app.settings.page_order,[1,2,0,3]);
    assert_eq!(crate::model::Settings::load().page_order,h.app.settings.page_order);
}

#[test]
fn settings_theme_follows_system_and_preserves_island_style() {
    use crate::settings_theme::SettingsTheme;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(false);
    assert_eq!(h.app.settings.settings_theme,SettingsTheme::System);
    assert_eq!(SettingsTheme::parse("unknown"),SettingsTheme::System);
    let mut raster=crate::preview_raster::Raster::new(690,1100);
    for (mode,system,dark) in [(SettingsTheme::System,false,false),(SettingsTheme::System,true,true),
        (SettingsTheme::System,false,false),(SettingsTheme::Dark,false,true),(SettingsTheme::Light,true,false)] {
        h.app.settings.settings_theme=mode;
        h.app.settings.save();assert_eq!(crate::model::Settings::load().settings_theme,mode);
        h.ctx.data_mut(|d|d.insert_temp(egui::Id::new("settings-system-theme"),(std::time::Instant::now(),system)));
        let output=std::sync::Arc::new(std::sync::Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,1100.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(1.0),..Default::default()})).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        for tab in 0..8 {
            h.app.settings_tab=tab;
            for frame in 0..3 {
                h.ctx.data_mut(|d|d.insert_temp(egui::Id::new("settings-system-theme"),(std::time::Instant::now(),system)));
                let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
                assert!(h.ctx.style().visuals.dark_mode,"设置页主题不能泄漏到坞体");
                let commands=&root.viewport_output[&egui::ViewportId::from_hash_of("capsuledock-settings")].commands;
                assert!(commands.iter().any(|c|matches!(c,egui::ViewportCommand::SetTheme(t) if *t==if dark{egui::SystemTheme::Dark}else{egui::SystemTheme::Light})),"原生边框必须与页面一致");
                let child=output.lock().unwrap().take().unwrap();
                if frame==2 {
                    let pairs:&[(&str,&str)]=match tab {
                        1=>&[("时间偏移","0.35 秒")],
                        2=>&[("专注","25 分钟"),("休息","5 分钟")],
                        _=>&[],
                    };
                    for (label,value) in pairs {
                        let centers=|needle:&str|child.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)if t.galley.text()==needle=>Some(t.visual_bounding_rect().center().y),_=>None}).collect::<Vec<_>>();
                        let distance=centers(label).iter().flat_map(|a|centers(value).into_iter().map(move|b|(a-b).abs())).fold(f32::INFINITY,f32::min);
                        assert!(distance<=1.0,"设置页 {tab} 的 {label} 与 {value} 未居中：{distance}");
                    }
                }
                if tab==0 && frame==2 {
                    let center=|label:&str|child.shapes.iter().find_map(|s|match &s.shape {
                        egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().center().y),_=>None,
                    }).unwrap();
                    for (label,choices) in [("显示屏",vec!["跟随主屏"]),("时间制式",vec!["24 小时","12 小时"]),("时段文字",vec!["不显示","上午 / 下午","详细时段"])] {
                        for choice in choices {assert!((center(label)-center(choice)).abs()<1.0,"{label}与{choice}应垂直居中：{} / {}",center(label),center(choice));}
                    }
                }
                // 设置窗口现在是一层没有模糊兜底的半透明玻璃，用底色亮度区分深/浅主题。
                let glass=child.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Rect(r) if r.rect.width()>400.0 && r.fill.a()<255 =>Some(r.fill),_=>None})
                    .expect("设置窗口应铺一层半透明玻璃底板");
                let brightness=glass.r() as u32+glass.g() as u32+glass.b() as u32;
                assert!(if dark{brightness<220}else{brightness>400},"玻璃底色应随主题变化：{glass:?}");
                raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);
                if frame==2 {
                    if let Some(dest)=std::env::var_os("CAPSULEDOCK_THEME_PREVIEW_DIR") {
                        let dest=std::path::PathBuf::from(dest);std::fs::create_dir_all(&dest).unwrap();
                        raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);
                        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                        image::save_buffer(dest.join(format!("settings-{}-{tab}.png",if dark{"dark"}else{"light"})),&rgba,690,1100,image::ColorType::Rgba8).unwrap();
                    }
                }
                raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
            }
        }
    }
}

#[test]
fn hover_tooltips_fit_native_region_and_release_it_after_leaving(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    for shelf in [false,true] {
        let mut h=H::new();
        let label;
        let pointer;
        if shelf {
            let path=crate::model::config_dir().join("开发项目源代码与当前维护的项目文件夹");
            std::fs::create_dir_all(&path).unwrap();
            label=path.display().to_string();h.app.shelf.add(&path).unwrap();
            h.app.show_page(2);h.hover(H::local(300.0,60.0),1.0);
            pointer=H::local(210.0,66.0);
        } else {
            h.app.settings.island_lyrics=true;
            h.media.title="测试歌曲".into();h.media.artist="歌手".into();h.media.duration=240.0;h.media.playing=true;
            h.app.set_lyrics("测试歌曲","歌手",240.0,crate::lyrics::parse_lrc("[00:00]测试歌词"));
            h.app.alert("邮件验证码","收到邮件，验证码 123456，请在十分钟内使用",5,false);
            h.hover(Pos2::new(600.0,220.0),1.0);
            label="邮件验证码\n收到邮件，验证码 123456，请在十分钟内使用".to_string();
            pointer=Pos2::new(WIN_W/2.0+(h.app.current_width()-44.0)/2.0+26.0,15.0);
        }
        h.hover(pointer,0.4);
        let hit_before_tooltip=h.hit;
        h.hover(pointer,1.1);
        let bounds=h.shapes.iter().find_map(|s|match &s.shape {
            egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None,
        }).expect("悬停后应显示完整提示");
        assert!(h.region.contains_rect(bounds),"悬停提示不能被原生窗口裁掉：{bounds:?} / {:?}",h.region);
        assert_eq!(h.hit,hit_before_tooltip,"提示不应扩大坞体交互区域");
        h.hover(Pos2::new(900.0,250.0),3.0);
        assert!(h.region.bottom()<100.0,"提示消失后应恢复收起区域");
    }
}

#[test]
fn clock_12h_formats_boundaries_and_persists() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    assert!(!h.app.settings.clock_12h);
    assert_eq!(h.app.settings.clock_text("23:59:00"),"23:59");
    assert_eq!(h.app.settings.clock_period,0);
    h.app.settings.clock_12h=true;
    assert_eq!(h.app.settings.clock_text("00:00"),"12:00");
    assert_eq!(h.app.settings.clock_text("23:59"),"11:59");
    h.app.settings.clock_period=1;
    for (input,expected) in [("00:00:00","上午 12:00"),("11:59:00","上午 11:59"),("12:00:00","下午 12:00"),("13:05:00","下午 1:05"),("23:59:00","下午 11:59")] {
        assert_eq!(h.app.settings.clock_text(input),expected);
    }
    assert_eq!(h.app.settings.clock_text("13:05"),"下午 1:05");
    h.app.settings.save();
    assert!(crate::model::Settings::load().clock_12h);
    h.app.settings.pet=false;
    for _ in 0..90 { h.frame(vec![]); }
    h.hover(H::local(200.0,40.0),1.0);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="9:41")));
    h.app.settings.clock_period=0;
    h.app.settings.clock_12h=false;
    h.app.settings.save();
    assert!(!crate::model::Settings::load().clock_12h);
    h.frame(vec![]);
    assert!(h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="09:41")));
}

#[test]
fn clock_period_boundaries_and_both_formats() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    for twelve in [false,true] {
        h.app.settings.clock_12h=twelve;
        h.app.settings.clock_period=2;
        for (time,label) in [("00:00","凌晨"),("05:59","凌晨"),("06:00","上午"),("10:59","上午"),("11:00","中午"),("12:59","中午"),("13:00","下午"),("16:59","下午"),("17:00","傍晚"),("18:59","傍晚"),("19:00","晚上"),("23:59","晚上")] {
            assert!(h.app.settings.clock_text(time).starts_with(&format!("{label} ")));
        }
        assert_eq!(h.app.settings.clock_text("19:30"),if twelve {"晚上 7:30"}else{"晚上 19:30"});
        for mode in 0..=2 {
            h.app.settings.clock_period=mode;
            h.app.settings.save();
            let loaded=crate::model::Settings::load();
            assert_eq!(loaded.clock_period,mode);
            assert_eq!(loaded.clock_12h,twelve);
        }
    }
}

#[test]
fn clock_period_visually_centers_with_digits() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let h=H::new();
    let mut raster=crate::preview_raster::Raster::new(420,230);
    for scale in [1.0,1.25,1.5,2.0] {
        let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(420.0,230.0))),..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                let mut c=crate::paint::Card::new(ui,Pos2::ZERO,ui.max_rect());c.s=scale;
                for (i,period) in ["凌晨","上午","中午","下午","傍晚","晚上"].iter().enumerate() {
                    for (j,size) in [12.5,15.5].iter().enumerate() {
                        crate::paint::clock(&c,10.0+j as f32*100.0,18.0+i as f32*18.0,&format!("{period} 12:59"),*size);
                    }
                }
            });
        });
        let texts:Vec<_>=output.shapes.iter().filter_map(|s|match &s.shape{egui::Shape::Text(t)=>Some(t),_=>None}).collect();
        for pair in texts.chunks_exact(2) {
            assert!((pair[0].visual_bounding_rect().center().y-pair[1].visual_bounding_rect().center().y).abs()<0.1,"时段文字与数字应按可见字形垂直居中，缩放 {scale}");
        }
        raster.update_textures(&output.textures_delta);
        if scale==2.0 {
            if let Some(dest)=std::env::var_os("CAPSULEDOCK_CLOCK_ALIGNMENT_PREVIEW") {
                raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                image::save_buffer(dest,&rgba,420,230,image::ColorType::Rgba8).unwrap();
            }
        }
    }
}

/// 坞体大小由视口层（`set_root_viewport_scale`）统一缩放，绘制阶段一律是逻辑单位。
/// 系统状态圆环若再把 `island_scale()` 乘进 `draw_sysmon` 的本地系数 `k`，
/// 就会被缩放两次。这里锁住「绘制半径不随 island_size 变化」这个不变量。
#[test]
fn sysmon_ring_radius_is_independent_of_island_size() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    // draw_sysmon 里 r = 8.2 * k，k 固定 1.0；线宽 2.6 用来把圆环和普通圆点区分开。
    const RING_RADIUS: f32 = 8.2;
    const RING_STROKE: f32 = 2.6;
    let mut measured: Vec<(u32, Vec<f32>)> = Vec::new();
    for island_size in [80u32, 100, 140] {
        let mut h=H::new();
        h.app.settings.show_sysmon=true;
        h.app.settings.island_size=island_size;
        // 复现 main.rs 每帧做的事：把坞体倍率交给视口层，而不是绘制层。
        eframe::set_root_viewport_scale(h.app.settings.island_scale());
        h.hover(H::local(209.5, 30.0), 2.0);
        let radii: Vec<f32> = h.shapes.iter().filter_map(|s| match &s.shape {
            egui::Shape::Circle(c) if (c.stroke.width - RING_STROKE).abs() < 0.01 => Some(c.radius),
            _ => None,
        }).collect();
        assert_eq!(radii.len(), 2, "island_size={island_size}% 应画出 CPU / MEM 两个底环，实际 {radii:?}");
        measured.push((island_size, radii));
    }
    eframe::set_root_viewport_scale(1.0);
    for (island_size, radii) in &measured {
        for radius in radii {
            assert!(
                (radius - RING_RADIUS).abs() < 0.01,
                "island_size={island_size}% 时圆环半径应为逻辑单位 {RING_RADIUS}（视口层负责缩放），\
                 实际 {radius}；若这里变成了 {}，说明 island_scale 被重复乘进了 draw_sysmon",
                RING_RADIUS * (*island_size as f32 / 100.0),
            );
        }
    }
}

#[test]
fn avoidance_picker_add_remove_and_save() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.ctx.set_visuals(egui::Visuals::dark());
    assert!(h.app.settings.avoid_browsers);
    assert!(h.app.settings.avoid_apps.is_empty());
    h.app.settings.browser_minimal=true;
    h.app.avoidance.entries=vec![
        crate::avoidance::Candidate{path:r"D:\Quark\quark.exe".into(),name:"夸克浏览器".into(),title:"新标签页".into(),browser:true},
        crate::avoidance::Candidate{path:r"C:\Windows\System32\notepad.exe".into(),name:"记事本".into(),title:"未命名 - 记事本".into(),browser:false},
    ];
    fn draw(h:&mut H,events:Vec<egui::Event>)->egui::FullOutput {
        h.t+=1.0/60.0;
        h.ctx.run(egui::RawInput{time:Some(h.t),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(600.0,560.0))),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{
                ui.heading("常规");
                h.app.avoidance.settings(ui,&mut h.app.settings);
            });
            h.app.avoidance.dialog(ctx,&mut h.app.settings);
        })
    }
    fn click(h:&mut H,point:Pos2){
        draw(h,vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        draw(h,vec![egui::Event::PointerMoved(point),egui::Event::PointerButton{pos:point,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
    }
    fn point(out:&egui::FullOutput,label:&str)->Pos2 {
        out.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect().center()),_=>None}).unwrap_or_else(||panic!("未找到 {label}"))
    }
    h.app.avoidance.open=true;
    let mut raster=crate::preview_raster::Raster::new(600,560);
    let mut out=draw(&mut h,vec![]);raster.update_textures(&out.textures_delta);
    for _ in 0..40 {std::thread::sleep(std::time::Duration::from_millis(10));out=draw(&mut h,vec![]);raster.update_textures(&out.textures_delta);}
    assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="浏览器规则已覆盖")));
    if let Some(dest)=std::env::var_os("CAPSULEDOCK_AVOID_PREVIEW_DIR") {
        std::fs::create_dir_all(&dest).unwrap();raster.buf.fill([0.0,0.0,0.0,1.0]);
        raster.draw(&h.ctx.tessellate(out.shapes.clone(),out.pixels_per_point),out.pixels_per_point);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(std::path::Path::new(&dest).join("添加应用.png"),&rgba,600,560,image::ColorType::Rgba8).unwrap();
    }
    click(&mut h,point(&out,"记事本"));
    let out=draw(&mut h,vec![]);
    click(&mut h,point(&out,"添加"));
    assert!(!h.app.avoidance.open);
    assert_eq!(h.app.settings.avoid_apps.len(),1);
    h.app.settings.avoid_browsers=false;h.app.settings.save();
    let loaded=crate::model::Settings::load();
    assert!(!loaded.avoid_browsers);assert!(loaded.browser_minimal);
    assert_eq!(loaded.avoid_apps,h.app.settings.avoid_apps);
    let mut out=draw(&mut h,vec![]);raster.update_textures(&out.textures_delta);
    for _ in 0..40{std::thread::sleep(std::time::Duration::from_millis(20));out=draw(&mut h,vec![]);raster.update_textures(&out.textures_delta);}
    assert!(!out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="记事本")),"图标区不应常驻显示应用名称");
    if let Some(dest)=std::env::var_os("CAPSULEDOCK_AVOID_PREVIEW_DIR") {
        raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(out.shapes.clone(),out.pixels_per_point),out.pixels_per_point);
        let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);image::save_buffer(std::path::Path::new(&dest).join("图标区域.png"),&rgba,600,560,image::ColorType::Rgba8).unwrap();
    }
    click(&mut h,point(&out,"×"));assert!(h.app.settings.avoid_apps.is_empty());
    h.app.settings.save();assert!(crate::model::Settings::load().avoid_apps.is_empty());
}

#[test]
fn six_minutes_without_interaction_do_not_restart_static_animation_or_grow_textures(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.app.settings.pet=false;
    for _ in 0..120{h.frame(vec![]);}
    let textures=h.ctx.tex_manager().read().num_allocated();
    for second in 3..=363 {
        h.t=second as f64;h.system_idle=Some(second as f64);h.frame(vec![]);
        assert!(!h.animating,"空闲到第 {second} 秒不应重新进入连续动画");
        assert_eq!(h.ctx.tex_manager().read().num_allocated(),textures,"空闲纹理数不能持续增长");
    }
}

#[test]
fn avoidance_icon_area_stays_one_row_and_scrolls() {
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.ctx.set_visuals(egui::Visuals::dark());h.app.settings.browser_minimal=true;
    fn draw(h:&mut H,events:Vec<egui::Event>)->egui::FullOutput {
        h.t+=1.0/60.0;
        h.ctx.run(egui::RawInput{time:Some(h.t),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(420.0,560.0))),events,..Default::default()},|ctx|{
            egui::CentralPanel::default().show(ctx,|ui|{h.app.avoidance.settings(ui,&mut h.app.settings);});
        })
    }
    fn text(out:&egui::FullOutput,label:&str)->Rect {
        out.shapes.iter().find_map(|s|match &s.shape{egui::Shape::Text(t) if t.galley.text()==label=>Some(t.visual_bounding_rect()),_=>None}).unwrap()
    }
    let footer="无需全屏或最大化；窗口移开或切换应用后恢复。";
    h.app.settings.avoid_apps=vec![("D:/sample/A.exe".into(),"A应用".into())];
    draw(&mut h,vec![]);let one=draw(&mut h,vec![]);let baseline=text(&one,footer).top();
    let cross=text(&one,"×").center();
    assert!(one.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Circle(c) if c.radius<=7.1 && c.center.distance(cross)<3.0)),"移除角标应为小圆点，不能被按钮最小尺寸撑大");
    h.app.settings.avoid_apps=(0..30).map(|i|(format!("D:/sample/{i}.exe"),format!("{}应用",char::from_u32(65+i).unwrap()))).collect();
    draw(&mut h,vec![]);let many=draw(&mut h,vec![]);
    assert!((text(&many,footer).top()-baseline).abs()<1.0,"应用增多时始终保持单行高度");
    let first=text(&many,"A");let pointer=first.center();
    draw(&mut h,vec![egui::Event::PointerMoved(pointer),egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:egui::vec2(0.0,-150.0),modifiers:Default::default()}]);
    let mut scrolled=draw(&mut h,vec![]);for _ in 0..30{scrolled=draw(&mut h,vec![]);}
    assert!((text(&scrolled,footer).top()-text(&many,footer).top()).abs()<1.0,"滚动图标不能移动区域下方内容");
    assert!(scrolled.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="A" && t.visual_bounding_rect().left()<first.left()-10.0)) || !scrolled.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="A")),"应能滚动查看后续应用");
}

#[test]
fn compact_canvas_preserves_panel_and_notice_pixels(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(width:f32,page:Option<usize>,notice:bool)->Vec<u8>{
        let mut h=H::new();h.app.settings.island_lyrics=true;h.app.settings.show_sysmon=false;
        let media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
        if let Some(page)=page{h.app.show_page(page);}
        let mut raster=crate::preview_raster::Raster::new(1280,270);
        let mut rgba=Vec::new();
        for frame in 0..180 {
            if notice && frame==60{h.app.alert("收到通知","会议提醒及验证码 834921",5,true);}
            let click=(notice && (frame==90 || frame==91)).then(||Pos2::new(WIN_W/2.0+(h.app.current_width()-44.0)/2.0+26.0,15.0));
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),today:"2026-10-28".into(),date:"10月28日 周三".into(),media:media.clone(),cursor:click.or_else(||page.map(|_|H::local(300.0,40.0))),..Default::default()};
            let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(width,WIN_H))),time:Some(t),events:click.map(|pos|vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:frame==90,modifiers:Default::default()}]).unwrap_or_default(),..Default::default()},|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});
            });
            raster.update_textures(&output.textures_delta);
            if [30,65,95,179].contains(&frame){
                raster.buf.fill([0.0;4]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut pixels=Vec::new();raster.to_rgba8(&mut pixels);
                if page.is_none() && notice && frame==95 {
                    if let Some(dir)=std::env::var_os("CAPSULEDOCK_GRAPHICS_PREVIEW") {let dir=std::path::PathBuf::from(dir);std::fs::create_dir_all(&dir).unwrap();image::save_buffer(dir.join(format!("canvas-{width}.png")),&pixels,1280,270,image::ColorType::Rgba8).unwrap();}
                }
                rgba.extend(pixels);
            }
        }
        rgba
    }
    for page in std::iter::once(None).chain((0..4).map(Some)) {
        for notice in [false,true] {
            assert_eq!(draw(1280.0,page,notice),draw(960.0,page,notice),"缩减透明余量不能改变页面 {page:?} 和通知 {notice} 的像素");
        }
    }
}

#[test]
fn compact_font_atlas_preserves_panel_and_notice_pixels(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(texture_side:usize,page:Option<usize>,notice:bool)->Vec<u8>{
        let mut h=H::new();h.app.settings.island_lyrics=true;h.app.settings.show_sysmon=false;
        let media=crate::smtc::NowPlaying{title:"Track".into(),artist:"Artist".into(),playing:true,duration:120.0,..Default::default()};
        h.app.set_lyrics("Track","Artist",120.0,crate::lyrics::parse_lrc("[00:00]晚风吹过了山岗\n[01:00]下一句"));
        if let Some(page)=page{h.app.show_page(page);}
        let mut raster=crate::preview_raster::Raster::new(1280,270);
        let mut rgba=Vec::new();
        for frame in 0..180 {
            if notice && frame==60{h.app.alert("收到通知","会议提醒及验证码 834921",5,true);}
            let click=(notice && (frame==90 || frame==91)).then(||Pos2::new(WIN_W/2.0+(h.app.current_width()-44.0)/2.0+26.0,15.0));
            let t=frame as f64/60.0;
            let env=Env{t,clock:"09:41:00".into(),today:"2026-10-28".into(),date:"10月28日 周三".into(),media:media.clone(),cursor:click.or_else(||page.map(|_|H::local(300.0,40.0))),..Default::default()};
            let output=h.ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(960.0,WIN_H))),max_texture_side:Some(texture_side),time:Some(t),events:click.map(|pos|vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed:frame==90,modifiers:Default::default()}]).unwrap_or_default(),..Default::default()},|ctx|{
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx,|ui|{h.app.frame(ui,Rect::from_min_size(Pos2::ZERO,Vec2::new(WIN_W,WIN_H)),&env);});
            });
            raster.update_textures(&output.textures_delta);
            if [30,65,95,179].contains(&frame){
                raster.buf.fill([0.0;4]);raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                let mut pixels=Vec::new();raster.to_rgba8(&mut pixels);
                if page.is_none() && notice && frame==95 {
                    if let Some(dir)=std::env::var_os("CAPSULEDOCK_GRAPHICS_PREVIEW") {let dir=std::path::PathBuf::from(dir);std::fs::create_dir_all(&dir).unwrap();image::save_buffer(dir.join(format!("atlas-{texture_side}.png")),&pixels,1280,270,image::ColorType::Rgba8).unwrap();}
                }
                rgba.extend(pixels);
            }
        }
        rgba
    }
    for page in std::iter::once(None).chain((0..4).map(Some)) {
        for notice in [false,true] {
            let reference=draw(16384,page,notice);
            for limit in [4096,2048] {let other=draw(limit,page,notice);
                let max=reference.iter().zip(&other).map(|(a,b)|a.abs_diff(*b)).max().unwrap_or(0);let count=reference.iter().zip(&other).filter(|(a,b)|a!=b).count();
                // 图集坐标改变只允许少量通道有 1/255 的浮点采样舍入差异（实测最多约 0.005% 的通道），肉眼不可见。
                assert!(max<=1 && count<reference.len()/1000,"图集上限 {limit} 改变了页面 {page:?}、通知 {notice} 的像素：{max}/{count}");}
        }
    }
}
#[test]
fn compact_font_atlas_preserves_settings_pixels_at_multiple_scales(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    fn draw(limit:usize,scale:f32)->Vec<Vec<u8>>{
        let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(false);
        let output=std::sync::Arc::new(std::sync::Mutex::new(None));let capture=output.clone();
        egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
            let id=viewport.ids.this;
            let raw=egui::RawInput{viewport_id:id,max_texture_side:Some(limit),screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(690.0,560.0))),
                viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()};
            *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
        });
        let mut raster=crate::preview_raster::Raster::new((690.0*scale)as usize,(560.0*scale)as usize);let mut result=Vec::new();
        for tab in 0..8 {h.app.settings_tab=tab;
            for frame in 0..3 {
                let root=h.ctx.run(egui::RawInput{max_texture_side:Some(limit),viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()},|ctx|h.app.settings_ui(ctx,false));
                let mut child=output.lock().unwrap().take().unwrap();
                raster.update_textures(&root.textures_delta);raster.update_textures(&child.textures_delta);
                if frame==2 {
                    // 常驻预览读取真实时钟，排除跨分钟变化；固定时钟已由主界面对照覆盖。
                    if tab==9{child.shapes.retain(|s|!matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().len()==5 && t.galley.text().as_bytes()[2]==b':'));}
                    raster.buf.fill([0.0,0.0,0.0,1.0]);raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);result.push(rgba);
                }
                raster.free_textures(&root.textures_delta);raster.free_textures(&child.textures_delta);
            }
        }result
    }
    for scale in [1.0,1.5,2.0] {
        let before=draw(16384,scale);
        for limit in [4096,2048] {let after=draw(limit,scale);
        for (tab,(before,after)) in before.iter().zip(&after).enumerate(){
            // 图集坐标改变只允许少量通道有 1/255 的浮点采样舍入差异。
            let max=before.iter().zip(after).map(|(a,b)|a.abs_diff(*b)).max().unwrap_or(0);
            let count=before.iter().zip(after).filter(|(a,b)|a!=b).count();assert!(max<=1 && count<before.len()/1000,"设置页 {tab} 在图集上限 {limit}、缩放 {scale} 下出现可见差异：{max}/{count}");
        }}
    }
}

#[test]
fn bluetooth_switch_clears_notices_and_restarts_with_fresh_baseline(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=H::new();
    h.hover(Pos2::new(WIN_W/2.0,12.0),1.0);
    let device=crate::bt::Device{id:"keyboard".into(),name:"Keyboard".into(),battery:None,kind:crate::bt::DeviceKind::Keyboard};
    h.app.bluetooth_alert(&device,true);h.frame(vec![]);
    h.app.bluetooth_alert(&device,false);
    h.app.settings.bluetooth_notifications=false;h.frame(vec![]);
    assert!(h.app.notices.notice.as_ref().is_none_or(|a|a.title!="蓝牙设备"));
    assert!(!h.app.notices.alerts.iter().any(|a|a.title=="蓝牙设备"));
    h.app.bluetooth_alert(&device,true);
    assert!(!h.app.notices.alerts.iter().any(|a|a.title=="蓝牙设备"));
    h.app.settings.save();assert!(!crate::model::Settings::load().bluetooth_notifications);
    h.bt=vec![device];h.app.settings.bluetooth_notifications=true;h.frame(vec![]);
    assert!(h.app.notices.notice.as_ref().is_none_or(|a|a.title!="蓝牙设备"));
    h.bt.clear();h.frame(vec![]);
    assert!(h.app.notices.notice.as_ref().is_some_and(|a|a.title=="蓝牙设备") || h.app.notices.alerts.iter().any(|a|a.title=="蓝牙设备"));
}

#[test]
#[ignore = "按需导出五套皮肤的预览图：CAPSULEDOCK_SKIN_PREVIEW_DIR=目录 cargo test skin_gallery -- --ignored"]
fn skin_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_SKIN_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    let (w,hgt)=((crate::app::CANVAS_W*2.0) as usize,(WIN_H*2.0) as usize);
    // 日落渐变 + 几条城市剪影，用来观察半透明皮肤的通透感。
    let backdrop=|raster:&mut crate::preview_raster::Raster|{
        for y in 0..hgt {for x in 0..w {
            let t=y as f32/hgt as f32;let s=x as f32/w as f32;
            let mut c=[0.98-0.25*t+0.05*s,0.62+0.12*t-0.1*s,0.45+0.35*t,1.0];
            if (y>hgt*7/10) && ((x/37)%3!=0) && y>hgt*7/10+((x*7919)%97) {c=[0.30,0.33,0.45,1.0];}
            let cloud=((x as f32/90.0).sin()*(y as f32/40.0).cos()).max(0.0);if y<hgt/2 {for k in 0..3{c[k]=c[k]+(1.0-c[k])*cloud*0.6;}}
            if x%160<12 && y<hgt*6/10 {c=[0.18,0.20,0.30,1.0];}
            raster.buf[y*w+x]=c;
        }}
    };
    for (skin,shape) in crate::model::Skin::ALL.into_iter().flat_map(|s|crate::model::IslandShape::ALL.map(|p|(s,p))) {
        for open in [false,true] {
            let mut h=H::new();h.app.settings.skin=skin;h.app.settings.island_shape=shape;h.app.settings.pet=true;h.app.opts.pet_idle=0.0;
            h.media=crate::smtc::NowPlaying{title:"Paper Boats".into(),artist:"The Lanterns".into(),position:64.0,duration:201.0,playing:true,..Default::default()};
            let mut raster=crate::preview_raster::Raster::new(w,hgt);
            for frame in 0..120 {
                let t=frame as f64/60.0;
                let cursor=open.then_some(Pos2::new(WIN_W/2.0,14.0));
                let env=Env{t,clock:"11:13:00".into(),today:"2026-10-26".into(),date:"10月26日 周一".into(),media:h.media.clone(),cursor,backdrop:Some([0.85,0.62,0.6]),glass_blur:true,..Default::default()};
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::app::CANVAS_W,WIN_H))),time:Some(t),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let mut glass=None;
                let output=h.ctx.run(raw,|ctx|{
                    egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));glass=h.app.frame(ui,win,&env).glass;});
                });
                raster.update_textures(&output.textures_delta);
                if open && frame==100 {h.app.show_page(1);}
                if open && frame==104 && shape==crate::model::IslandShape::Notch {
                    backdrop(&mut raster);
                    raster.draw(&h.ctx.tessellate(output.shapes.clone(),output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw(w as u32,hgt as u32,rgba).unwrap();
                    image::imageops::crop_imm(&image,120,0,1040,400).to_image().save(dest.join(format!("{}-switch.png",skin.key()))).unwrap();
                }
                if frame==119 {
                    backdrop(&mut raster);
                    // 模拟系统实时模糊：只有坞体轮廓内的背景被模糊（与实机的合成层一致）。
                    if let Some(shapes)=glass {
                        let pieces=crate::app::blur_pieces(&shapes,2.0);
                        let mut blurred=raster.buf.clone();
                        for _ in 0..3 {
                            let src=blurred.clone();let r=18i32;
                            for y in 0..hgt as i32 {for x in 0..w as i32 {let mut s=[0.0f32;4];let mut n=0.0;for dx in (-r..=r).step_by(3){let xx=(x+dx).clamp(0,w as i32-1);let c=src[(y as usize)*w+xx as usize];for k in 0..4{s[k]+=c[k];}n+=1.0;}blurred[(y as usize)*w+x as usize]=s.map(|v|v/n);}}
                            let src=blurred.clone();
                            for y in 0..hgt as i32 {for x in 0..w as i32 {let mut s=[0.0f32;4];let mut n=0.0;for dy in (-r..=r).step_by(3){let yy=(y+dy).clamp(0,hgt as i32-1);let c=src[(yy as usize)*w+x as usize];for k in 0..4{s[k]+=c[k];}n+=1.0;}blurred[(y as usize)*w+x as usize]=s.map(|v|v/n);}}
                        }
                        for y in 0..hgt {for x in 0..w {
                            let (px,py)=(x as f32+0.5,y as f32+0.5);
                            let inside=pieces.iter().any(|p|{px>=p.x&&px<p.x+p.w&&py>=p.y&&py<p.y+p.h&&{let cx=px.clamp(p.x+p.radius,p.x+p.w-p.radius);let cy=py.clamp(p.y+p.radius,p.y+p.h-p.radius);(px-cx).powi(2)+(py-cy).powi(2)<=p.radius*p.radius}});
                            if inside {raster.buf[y*w+x]=blurred[y*w+x];}
                        }}
                    }
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw(w as u32,hgt as u32,rgba).unwrap();
                    let crop=if open{image::imageops::crop_imm(&image,120,0,1040,400)}else{image::imageops::crop_imm(&image,300,0,680,110)};
                    crop.to_image().save(dest.join(format!("{}-{}-{}.png",skin.key(),shape.key(),if open{"open"}else{"closed"}))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
        }
    }
}

#[test]
#[ignore = "按需导出：坞体收起态与设置页常驻排列预览"]
fn skin_gallery_resident(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_SKIN_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    for skin in crate::model::Skin::ALL {
        for shape in crate::model::IslandShape::ALL {
            let mut h=H::new();h.app.settings.skin=skin;h.app.settings.island_shape=shape;h.app.opts.pet_idle=0.0;
            // 坞体收起态
            let (w,hgt)=((crate::app::CANVAS_W*2.0) as usize,(WIN_H*2.0) as usize);
            let mut raster=crate::preview_raster::Raster::new(w,hgt);
            for frame in 0..90 {
                let t=frame as f64/60.0;
                let env=Env{t,clock:"21:54:00".into(),today:"2026-11-04".into(),date:"11月4日 周三".into(),backdrop:Some([0.55,0.68,0.75]),glass_blur:true,..Default::default()};
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::app::CANVAS_W,WIN_H))),time:Some(t),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));h.app.frame(ui,win,&env);});});
                raster.update_textures(&output.textures_delta);
                if frame==89 {
                    raster.buf.fill([0.62,0.72,0.78,1.0]);
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw(w as u32,hgt as u32,rgba).unwrap();
                    image::imageops::crop_imm(&image,300,0,680,90).to_image().save(dest.join(format!("bar-{}-{}.png",skin.key(),shape.key()))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
            // 设置页常驻排列预览
            let mut raster=crate::preview_raster::Raster::new(1200,300);
            for frame in 0..3 {
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(600.0,150.0))),time:Some(frame as f64),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{h.app.resident_order_settings(ui);});});
                raster.update_textures(&output.textures_delta);
                if frame==2 {
                    raster.buf.fill([0.13,0.13,0.15,1.0]);
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    image::RgbaImage::from_raw(1200,300,rgba).unwrap().save(dest.join(format!("resident-{}-{}.png",skin.key(),shape.key()))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
        }
    }
}

#[test]
#[ignore = "按需导出：浅色皮肤下每个页面的展开态，用于排查配色"]
fn skin_gallery_pages(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_SKIN_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    let skins=std::env::var("CAPSULEDOCK_SKINS").unwrap_or_else(|_|"glass_light,cream,classic".into());
    for skin in skins.split(',').map(crate::model::Skin::parse) {
        for page in 0..4usize {
            let mut h=H::new();h.app.settings.skin=skin;h.app.settings.island_shape=crate::model::IslandShape::Pill;
            h.app.todos.items.push(crate::model::TodoItem{text:"整理胶囊坞皮肤反馈".into(),done_on:None,due:Some("2026-11-05 10:00".into()),reminded:false,..Default::default()});
            h.app.todos.items.push(crate::model::TodoItem{text:"已完成的事项".into(),done_on:Some("2026-11-04".into()),due:None,reminded:false,..Default::default()});
            h.media=crate::smtc::NowPlaying{title:"Paper Boats".into(),artist:"The Lanterns".into(),position:64.0,duration:201.0,playing:true,..Default::default()};
            let (w,hgt)=((crate::app::CANVAS_W*2.0) as usize,(WIN_H*2.0) as usize);
            let mut raster=crate::preview_raster::Raster::new(w,hgt);
            for frame in 0..140 {
                let t=frame as f64/60.0;
                if frame==40 {h.app.show_page(page);}
                let env=Env{t,clock:"22:46:00".into(),today:"2026-11-04".into(),date:"11月4日 周三".into(),media:h.media.clone(),cursor:Some(Pos2::new(WIN_W/2.0,20.0)),backdrop:Some([0.8,0.84,0.88]),glass_blur:true,..Default::default()};
                let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::app::CANVAS_W,WIN_H))),time:Some(t),
                    viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(2.0),..Default::default()})).collect(),..Default::default()};
                let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));h.app.frame(ui,win,&env);});});
                raster.update_textures(&output.textures_delta);
                if frame==139 {
                    raster.buf.fill([0.80,0.84,0.88,1.0]);
                    raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw(w as u32,hgt as u32,rgba).unwrap();
                    image::imageops::crop_imm(&image,160,0,960,500).to_image().save(dest.join(format!("page-{}-{page}.png",skin.key()))).unwrap();
                }
                raster.free_textures(&output.textures_delta);
            }
        }
    }
}

/// 新功能画面：按需导出，检查排版。
fn feature_raster()->crate::preview_raster::Raster{crate::preview_raster::Raster::new((crate::app::CANVAS_W*2.0) as usize,(WIN_H*2.0) as usize)}
fn shoot_feature(h:&mut H,raster:&mut crate::preview_raster::Raster,dest:&std::path::Path,name:&str,frames:usize,cursor:Option<Pos2>,today:&str){
    let ppp=2.0;let (w,hgt)=(crate::app::CANVAS_W,WIN_H);
    for f in 0..frames {
        h.t+=1.0/60.0;
        let env=Env{t:h.t,clock:"09:41:00".into(),today:today.into(),date:"11月4日 周三".into(),cursor,bt_ready:true,..Default::default()};
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
            events:cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default(),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
        let app=&mut h.app;
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H));app.frame(ui,win,&env);});});
        raster.update_textures(&output.textures_delta);
        if f+1==frames {
            raster.buf.fill([0.20,0.24,0.32,1.0]);
            raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
            let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            let image=image::RgbaImage::from_raw((w*ppp) as u32,(hgt*ppp) as u32,rgba).unwrap();
            image::imageops::crop_imm(&image,200,0,880,330).to_image().save(dest.join(format!("{name}.png"))).unwrap();
        }
        raster.free_textures(&output.textures_delta);
    }
}

#[test]
#[ignore="按需导出每周待办与剪贴板的实际画面"]
fn feature_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_FEATURE_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let today="2026-11-04";// 周三
    let todos=||{
        let mut a=crate::model::TodoItem::new("写周报");a.repeat=0b001_0000;
        let mut b=crate::model::TodoItem::new("站会");b.repeat=crate::model::REPEAT_WORKDAYS;b.due=Some("2026-11-04 09:30".into());b.done_on=Some("2026-11-04".into());
        let mut c=crate::model::TodoItem::new("健身");c.repeat=0b000_0101;
        let d=crate::model::TodoItem::new("回复设计评审意见");
        vec![a,b,c,d]
    };
    let mut h=H::new();let mut r=feature_raster();h.app.todos.items=todos();h.app.show_page(3);
    shoot_feature(&mut h,&mut r,&dest,"todo-page",90,Some(H::local(120.0,60.0)),today);
    let mut h=H::new();let mut r=feature_raster();h.app.todos.items=todos();h.app.show_page(3);
    shoot_feature(&mut h,&mut r,&dest,"todo-editor-0",30,Some(H::local(120.0,60.0)),today);
    h.app.open_task_dialog(2);
    shoot_feature(&mut h,&mut r,&dest,"todo-editor",60,Some(H::local(120.0,60.0)),today);
    let mut h=H::new();let mut r=feature_raster();h.app.todos.items=todos();h.app.show_page(3);
    shoot_feature(&mut h,&mut r,&dest,"todo-picker-0",30,Some(H::local(120.0,60.0)),today);
    h.app.open_task_reminder(3);
    shoot_feature(&mut h,&mut r,&dest,"todo-picker",60,Some(H::local(120.0,60.0)),today);
    // 剪贴板
    let mut h=H::new();let mut r=feature_raster();h.app.show_page(2);h.app.shelf_view.shelf_tab=1;
    shoot_feature(&mut h,&mut r,&dest,"clip-empty",60,Some(H::local(120.0,60.0)),today);
    {let mut hist=h.app.clipboard.lock();hist.items.clear();
        for (i,t) in ["https://github.com/qbbyte/capsuleDock/releases","会议纪要：\n1. 周三前确认胶囊坞竖版方案\n2. 剪贴板记录上线","13800138000","SELECT * FROM todo WHERE repeat > 0 ORDER BY due;","验证码 482913"].iter().enumerate(){hist.push(t,&format!("09:{:02}",30+i));}
        hist.items[3].pinned=true;}
    shoot_feature(&mut h,&mut r,&dest,"clip-list",30,Some(H::local(200.0,66.0)),today);
    h.app.shelf_view.shelf_tab=0;
    let file=std::env::temp_dir().join("胶囊坞周报.docx");std::fs::write(&file,b"x").unwrap();h.app.shelf.items=vec![file];
    shoot_feature(&mut h,&mut r,&dest,"shelf-files",30,Some(H::local(120.0,60.0)),today);
    h.app.shelf_view.confirm_clear=Some(h.app.t);
    shoot_feature(&mut h,&mut r,&dest,"shelf-confirm",5,Some(H::local(120.0,60.0)),today);
    h.app.settings.clipboard_history=false;h.app.shelf_view.shelf_tab=1;
    shoot_feature(&mut h,&mut r,&dest,"clip-off",30,Some(H::local(120.0,60.0)),today);
}

static SIDE_MUSIC:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
/// 侧边停靠的画面：画布为 SIDE_W × SIDE_H。
fn shoot_side(h:&mut H,raster:&mut crate::preview_raster::Raster,dest:&std::path::Path,name:&str,frames:usize,cursor:Option<Pos2>){
    let ppp=2.0;let (w,hgt)=(crate::side_dock::SIDE_W,crate::side_dock::SIDE_H);
    for f in 0..frames {
        h.t+=1.0/60.0;
        let mut env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-11-04".into(),date:"11月4日 周三".into(),cursor,bt_ready:true,battery:Some(76),..Default::default()};
        if SIDE_MUSIC.load(std::sync::atomic::Ordering::Relaxed) {env.media.title="晴天".into();env.media.artist="周杰伦".into();env.media.playing=true;env.media.source="cloudmusic.exe".into();env.media.duration=269.0;}
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
            events:cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default(),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
        let app=&mut h.app;let mut out=None;
        let output=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let win=Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt));out=Some(app.frame(ui,win,&env));});});
        raster.update_textures(&output.textures_delta);
        if f+1==frames {
            raster.buf.fill([0.20,0.24,0.32,1.0]);
            raster.draw(&h.ctx.tessellate(output.shapes,output.pixels_per_point),output.pixels_per_point);
            let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
            let mut image=image::RgbaImage::from_raw((w*ppp) as u32,(hgt*ppp) as u32,rgba).unwrap();
            // 画出窗口区域（红）与命中区域（绿）的边框，方便检查。
            if let Some(out)=out {
                for (r,color) in [(out.region,image::Rgba([255,80,80,255])),(out.hit,image::Rgba([80,255,120,255]))] {
                    let (x0,y0,x1,y1)=((r.left()*ppp).max(0.0) as u32,(r.top()*ppp).max(0.0) as u32,((r.right()*ppp) as u32).min(image.width()-1),((r.bottom()*ppp) as u32).min(image.height()-1));
                    for x in x0..=x1 {image.put_pixel(x,y0,color);image.put_pixel(x,y1,color);}
                    for y in y0..=y1 {image.put_pixel(x0,y,color);image.put_pixel(x1,y,color);}
                }
            }
            image.save(dest.join(format!("{name}.png"))).unwrap();
        }
        raster.free_textures(&output.textures_delta);
    }
}

#[test]
#[ignore="按需导出左右停靠的实际画面"]
fn side_dock_gallery(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_FEATURE_PREVIEW_DIR").expect("需指定预览目录"));std::fs::create_dir_all(&dest).unwrap();
    let (w,hgt)=(crate::side_dock::SIDE_W,crate::side_dock::SIDE_H);
    for (dock,shape,skin) in [(crate::model::Dock::Left,crate::model::IslandShape::Notch,crate::model::Skin::Classic),(crate::model::Dock::Right,crate::model::IslandShape::Pill,crate::model::Skin::Cream)] {
        let tag=format!("{}-{}",dock.key(),shape.key());
        let mut h=H::new();let mut r=feature_raster_sized(w,hgt);
        h.app.settings.dock=dock;h.app.settings.island_shape=shape;h.app.settings.skin=skin;h.app.settings.pet=false;
        h.app.media.title="晴天".into();h.app.media.artist="周杰伦".into();h.app.media.playing=true;h.app.media.source="cloudmusic.exe".into();
        let away=Some(Pos2::new(w/2.0,hgt-10.0));
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-collapsed"),60,away);
        let edge_x=if dock==crate::model::Dock::Left{8.0}else{w-8.0};
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-hover"),8,Some(Pos2::new(edge_x,hgt/2.0)));
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-opening"),12,Some(Pos2::new(edge_x,hgt/2.0)));
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-open"),70,Some(Pos2::new(edge_x,hgt/2.0)));
        h.app.show_page(3);
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-todo"),50,Some(Pos2::new(edge_x,hgt/2.0)));
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-closed"),320,away);
        h.app.alert("待办到时间了","回复设计评审意见",4,false);
        shoot_side(&mut h,&mut r,&dest,&format!("side-{tag}-notice"),40,away);
    }
    // 播放音乐 + 专注中 + 毛玻璃；极简模式；侧边提示。
    SIDE_MUSIC.store(true,std::sync::atomic::Ordering::Relaxed);
    let mut h=H::new();let mut r=feature_raster_sized(w,hgt);
    h.app.settings.dock=crate::model::Dock::Left;h.app.settings.skin=crate::model::Skin::GlassDark;
    h.app.pomo.running=true;h.app.pomo.remaining=1430.0;
    let away=Some(Pos2::new(w/2.0,hgt-10.0));
    shoot_side(&mut h,&mut r,&dest,"side-left-glass-busy",60,away);
    shoot_side(&mut h,&mut r,&dest,"side-left-glass-open",70,Some(Pos2::new(8.0,hgt/2.0)));
    shoot_side(&mut h,&mut r,&dest,"side-left-glass-closed",120,away);
    SIDE_MUSIC.store(false,std::sync::atomic::Ordering::Relaxed);
    let mut h=H::new();let mut r=feature_raster_sized(w,hgt);
    h.app.settings.dock=crate::model::Dock::Right;h.app.settings.minimal=true;
    shoot_side(&mut h,&mut r,&dest,"side-right-minimal",60,away);
}
fn feature_raster_sized(w:f32,h:f32)->crate::preview_raster::Raster{crate::preview_raster::Raster::new((w*2.0) as usize,(h*2.0) as usize)}

#[test]
fn side_dock_auto_minimal_follows_avoidance(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let (w,hgt)=(crate::side_dock::SIDE_W,crate::side_dock::SIDE_H);
    let mut h=H::new();h.app.settings.dock=crate::model::Dock::Right;h.app.settings.browser_minimal=true;
    let away=Some(Pos2::new(w/2.0,hgt-5.0));
    for _ in 0..60 {side_frame(&mut h,away);}
    let normal=side_frame(&mut h,away).hit;
    h.browser_at_top=Some(true);
    let mut out=side_frame_with(&mut h,away);
    for _ in 0..60 {out=side_frame_with(&mut h,away);}
    assert!(out.hit.width()<normal.width() && out.hit.height()<normal.height(),"避让时收成贴边细线：{:?} → {:?}",normal,out.hit);
    h.browser_at_top=Some(false);
    for _ in 0..60 {out=side_frame_with(&mut h,away);}
    assert!((out.hit.height()-normal.height()).abs()<1.0,"离开后恢复：{:?}",out.hit);
}

fn side_frame_with(h:&mut H,cursor:Option<Pos2>)->crate::app::FrameOut{
    let at_top=h.browser_at_top;
    let (w,hgt)=(crate::side_dock::SIDE_W,crate::side_dock::SIDE_H);
    h.t+=1.0/60.0;
    let env=Env{t:h.t,clock:"09:41:00".into(),cursor,battery:Some(76),browser_at_top:at_top,..Default::default()};
    let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,..Default::default()};
    let app=&mut h.app;let mut out=None;
    let _=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{out=Some(app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt)),&env));});});
    out.unwrap()
}

fn side_frame(h:&mut H,cursor:Option<Pos2>)->crate::app::FrameOut{
    let (w,hgt)=(crate::side_dock::SIDE_W,crate::side_dock::SIDE_H);
    h.t+=1.0/60.0;
    let env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-11-04".into(),date:"11月4日 周三".into(),cursor,bt_ready:true,battery:Some(76),..Default::default()};
    let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,
        events:cursor.map(|p|vec![egui::Event::PointerMoved(p)]).unwrap_or_default(),..Default::default()};
    let app=&mut h.app;let mut out=None;
    let _=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{out=Some(app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt)),&env));});});
    out.unwrap()
}

#[test]
fn side_dock_opens_from_the_edge_and_collapses_back(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let (w,hgt)=(crate::side_dock::SIDE_W,crate::side_dock::SIDE_H);
    for (dock,shape) in [(crate::model::Dock::Left,crate::model::IslandShape::Notch),(crate::model::Dock::Right,crate::model::IslandShape::Pill)] {
        let mut h=H::new();h.app.settings.dock=dock;h.app.settings.island_shape=shape;
        let away=Some(Pos2::new(w/2.0,hgt-5.0));
        let mut out=side_frame(&mut h,away);
        for _ in 0..60 {out=side_frame(&mut h,away);}
        assert!(!h.app.morph.open);
        // 收起：竖向胶囊贴边、竖直居中。
        assert!(out.hit.height()>out.hit.width(),"{dock:?} 收起时是竖向：{:?}",out.hit);
        assert!((out.hit.center().y-hgt/2.0).abs()<1.0);
        let edge=if dock==crate::model::Dock::Left{0.5}else{w-0.5};
        assert!(out.hit.contains(Pos2::new(edge,hgt/2.0)),"{dock:?} 光标推到屏幕边缘也能命中：{:?}",out.hit);
        assert!(out.region.contains_rect(out.hit.shrink(0.5)));
        // 推到边缘停留 → 展开，面板从边缘滑出且完整留在画布内。
        for _ in 0..60 {out=side_frame(&mut h,Some(Pos2::new(edge,hgt/2.0)));}
        assert!(h.app.morph.open && h.app.progress()>0.95,"{dock:?} 应展开");
        assert!(out.hit.width()>crate::app::PW-1.0 && out.hit.height()<200.0,"{:?}",out.hit);
        assert!(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt)).contains_rect(out.region),"区域不能超出画布：{:?}",out.region);
        if dock==crate::model::Dock::Left {assert!(out.hit.left()<=0.5)} else {assert!(out.hit.right()>=w-0.5)}
        // 离开 → 收起。
        for _ in 0..90 {out=side_frame(&mut h,away);}
        assert!(!h.app.morph.open && h.app.progress()<0.05);
        assert!(out.hit.width()<60.0);
    }
}

/// 内存实测：模拟长时间听歌（不断出现新汉字）并浏览各页面，输出进程内存、字形图集和贴图占用。
/// 运行：CAPSULEDOCK_ATLAS=4096 CAPSULEDOCK_PPP=1.5 cargo test --lib memory_profile -- --ignored --nocapture --test-threads=1
#[test]
#[ignore="内存实测，按需运行"]
fn memory_profile_long_listening_session(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let side:usize=std::env::var("CAPSULEDOCK_ATLAS").ok().and_then(|s|s.parse().ok()).unwrap_or(4096);
    let ppp:f32=std::env::var("CAPSULEDOCK_PPP").ok().and_then(|s|s.parse().ok()).unwrap_or(1.5);
    let songs:usize=std::env::var("CAPSULEDOCK_SONGS").ok().and_then(|s|s.parse().ok()).unwrap_or(120);
    // 匿名内存≈Windows 的私有内存；文件映射（程序代码、系统字体）可在进程间共享，不计入私有内存。
    fn rss()->String{
        let status=std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let kb=|key:&str|status.lines().find_map(|l|l.strip_prefix(key)?.trim().trim_end_matches(" kB").trim().parse::<f64>().ok()).unwrap_or(0.0)/1024.0;
        format!("匿名={:>6.1}MB 文件映射={:>6.1}MB",kb("RssAnon:"),kb("RssFile:"))
    }
    fn report(label:&str,h:&H){
        let [w,hgt]=h.ctx.fonts(|f|f.font_image_size());
        let tex:usize=h.ctx.tex_manager().read().allocated().map(|(_,m)|m.bytes_used()).sum();
        let galleys=h.ctx.fonts(|f|f.num_galleys_in_cache());
        // 图集在 CPU 端按 f32 存覆盖率，显卡端再按 RGBA 存一份。
        println!("{label:<16} {} 图集={w}x{hgt}（CPU≈{:.1}MB）贴图合计={:.1}MB 文字缓存={galleys}",rss(),(w*hgt*4) as f64/1048576.0,tex as f64/1048576.0);
    }
    println!("{:<16} {}","测试进程基线",rss());
    let mut h=H::new();h.app.settings.island_lyrics=true;h.app.show_lyrics=true;
    println!("{:<16} {}","创建界面后",rss());
    let mut seed=12345u64;let mut next=||{seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);(seed>>33) as u32};
    let run=|h:&mut H,media:&crate::smtc::NowPlaying|{
        h.t+=1.0/60.0;
        let raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(crate::app::CANVAS_W,WIN_H))),time:Some(h.t),max_texture_side:Some(side),
            viewports:std::iter::once((egui::ViewportId::ROOT,egui::ViewportInfo{native_pixels_per_point:Some(ppp),..Default::default()})).collect(),..Default::default()};
        let env=Env{t:h.t,clock:"09:41:00".into(),today:"2026-10-28".into(),date:"10月28日 周三".into(),media:media.clone(),cursor:h.cursor,bt_ready:true,..Default::default()};
        let app=&mut h.app;
        let out=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{let _=app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(WIN_W,WIN_H)),&env);});});
        h.shapes=out.shapes;
    };
    let idle=crate::smtc::NowPlaying::default();
    for _ in 0..30 {run(&mut h,&idle);}
    report("启动后",&h);
    for page in 0..4 {h.app.show_page(page);for _ in 0..20 {run(&mut h,&idle);}}
    report("浏览8个页面后",&h);
    for song in 0..songs {
        // 常用字约 3000 个：每首歌 40 句、每句 8–12 字，从中随机取，接近真实歌词的用字分布。
        let lines:Vec<String>=(0..40).map(|_|(0..8+next()%5).map(|_|char::from_u32(0x4E00+next()%3000).unwrap()).collect()).collect();
        let lrc=lines.iter().enumerate().map(|(i,l)|format!("[{:02}:{:02}.00]{l}\n",i*3/60,i*3%60)).collect::<String>();
        let title=format!("测试歌曲{song}");
        h.app.set_lyrics(&title,"歌手",200.0,crate::lyrics::parse_lrc(&lrc));
        if song==songs/2 {h.app.show_page(0);}
        for i in 0..40 {
            let media=crate::smtc::NowPlaying{title:title.clone(),artist:"歌手".into(),duration:200.0,position:i as f64*3.0+0.5,playing:false,..Default::default()};
            for _ in 0..2 {run(&mut h,&media);}
        }
        if (song+1)%20==0 {report(&format!("听完{}首",song+1),&h);}
    }
}

#[test]
#[ignore = "按需导出设置页预览：CAPSULEDOCK_SETTINGS_PREVIEW_DIR=目录 cargo test settings_gallery -- --ignored"]
fn settings_gallery(){
    use crate::settings_theme::SettingsTheme;
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let dest=std::path::PathBuf::from(std::env::var_os("CAPSULEDOCK_SETTINGS_PREVIEW_DIR").expect("需指定预览目录"));
    std::fs::create_dir_all(&dest).unwrap();
    let width:f32=std::env::var("CAPSULEDOCK_SETTINGS_WIDTH").ok().and_then(|v|v.parse().ok()).unwrap_or(690.0);
    let (w,hgt)=(width,2400.0f32);
    let scale=1.5f32;
    for (mode,dark) in [(SettingsTheme::Dark,true),(SettingsTheme::Light,false)] {
        for tab in [0usize,9,1,2,4,6,7,8] {
            let mut h=H::new();h.app.open_settings();h.ctx.set_embed_viewports(false);
            h.app.settings.settings_theme=mode;h.app.settings_tab=tab;
            let output=std::sync::Arc::new(std::sync::Mutex::new(None));let capture=output.clone();
            egui::Context::set_immediate_viewport_renderer(move|ctx,mut viewport|{
                let id=viewport.ids.this;
                let raw=egui::RawInput{viewport_id:id,screen_rect:Some(Rect::from_min_size(Pos2::ZERO,egui::vec2(w,hgt))),
                    viewports:std::iter::once((id,egui::ViewportInfo{native_pixels_per_point:Some(scale),..Default::default()})).collect(),..Default::default()};
                *capture.lock().unwrap()=Some(ctx.run(raw,|ctx|(viewport.viewport_ui_cb)(ctx)));
            });
            let mut raster=crate::preview_raster::Raster::new((w*scale) as usize,(hgt*scale) as usize);
            for frame in 0..4 {
                let root=h.ctx.run(Default::default(),|ctx|h.app.settings_ui(ctx,false));
                raster.update_textures(&root.textures_delta);
                let child=output.lock().unwrap().take().unwrap();
                raster.update_textures(&child.textures_delta);
                if frame==3 {
                    raster.buf.fill(if dark{[0.08,0.08,0.09,1.0]}else{[0.96,0.96,0.97,1.0]});
                    raster.draw(&h.ctx.tessellate(child.shapes,child.pixels_per_point),child.pixels_per_point);
                    let mut rgba=Vec::new();raster.to_rgba8(&mut rgba);
                    let image=image::RgbaImage::from_raw((w*scale) as u32,(hgt*scale) as u32,rgba).unwrap();
                    // 去掉页面下方的空白
                    let bg=image.get_pixel(((w-20.0)*scale) as u32,((hgt-5.0)*scale) as u32).0;
                    let mut bottom=image.height()-1;
                    'scan: while bottom>200 { for x in (270*scale as u32)..image.width()-10 { if image.get_pixel(x,bottom).0!=bg {break 'scan;} } bottom-=1; }
                    image::imageops::crop_imm(&image,0,0,image.width(),(bottom+20).min(image.height())).to_image()
                        .save(dest.join(format!("settings-{}-{tab:02}.png",if dark{"dark"}else{"light"}))).unwrap();
                }
                raster.free_textures(&child.textures_delta);
            }
        }
    }
}

#[test]
fn side_dock_file_drag_opens_the_shelf_and_drop_saves_it(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let (w,hgt)=(crate::side_dock::SIDE_W,crate::side_dock::SIDE_H);
    let path=crate::model::config_dir().join("侧边拖入.txt");std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,"x").unwrap();
    for dock in [crate::model::Dock::Left,crate::model::Dock::Right] {
        let mut h=H::new();h.app.settings.dock=dock;
        let edge=if dock==crate::model::Dock::Left{10.0}else{w-10.0};
        let at=Pos2::new(edge,hgt/2.0);
        let frame=|h:&mut H,hovered:bool,dropped:bool|{
            h.t+=1.0/60.0;
            let env=Env{t:h.t,clock:"09:41:00".into(),cursor:Some(at),..Default::default()};
            let mut raw=egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(w,hgt))),time:Some(h.t),focused:true,..Default::default()};
            if hovered {raw.hovered_files.push(egui::HoveredFile{path:Some(path.clone()),..Default::default()});}
            if dropped {raw.dropped_files.push(egui::DroppedFile{path:Some(path.clone()),..Default::default()});}
            let app=&mut h.app;let mut out=None;
            let _=h.ctx.run(raw,|ctx|{egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx,|ui|{out=Some(app.frame(ui,Rect::from_min_size(ui.max_rect().min,Vec2::new(w,hgt)),&env));});});
            out.unwrap()
        };
        let out=frame(&mut h,false,false);
        let zone=out.file_drop_zone.expect("置物架开启时侧边也有拖放感应区");
        assert!(zone.contains(at),"{dock:?} 感应区应覆盖贴边的胶囊：{zone:?}");
        frame(&mut h,true,false);
        assert!(h.app.drag_hot && h.app.nav.page==2,"{dock:?} 文件拖到胶囊上应切到置物架");
        for _ in 0..40 {frame(&mut h,true,false);}
        assert!(h.app.morph.open);
        frame(&mut h,false,true);
        assert!(h.app.shelf.items.contains(&path),"{dock:?} 松手后应放进置物架");
        h.app.shelf.items.clear();h.app.shelf.save();
    }
}

#[test]
fn shelf_menu_last_action_fits_inside_window_and_works(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());
    let mut h=H::new();
    h.app.shelf.items=vec![r"folder\menu-test.zip".into()];
    h.app.show_page(2);
    let point=H::local(209.5,101.0);
    h.hover(point,1.2);
    for pressed in [true,false]{
        h.frame(vec![egui::Event::PointerButton{pos:point,button:egui::PointerButton::Secondary,pressed,modifiers:Default::default()}]);
    }
    h.hover(point,1.5);
    assert!(!h.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==r"folder\menu-test.zip")));
    let label=h.shapes.iter().find_map(|s|match &s.shape{
        egui::Shape::Text(t) if t.galley.text()=="从置物架移除"=>Some(Rect::from_min_size(t.pos,t.galley.size())),
        _=>None,
    }).expect("右键菜单应显示移除操作");
    assert!(h.region.contains_rect(h.app.shelf_view.shelf_menu_rect.unwrap() + h.ctx.style().visuals.popup_shadow.margin()));
    assert!(h.app.shelf_view.shelf_menu_rect.unwrap().contains_rect(label));
    h.hover(label.center(),1.0);
    assert!(h.app.morph.open);
    h.click(label.center());
    assert!(h.app.shelf.items.is_empty());
}
