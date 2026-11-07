//! 四个页面：音乐 → 专注 → 置物 → 待办。
//! 坐标全部是面板局部逻辑点（419×140），文字 y 是基线。

use crate::anim::{self, lerp, smoothstep, Spring};
use crate::app::{App, Cmd, Env, Phase};
use crate::paint::{self, capsule, mix, Card, H};
use crate::smtc::Transport;
use crate::theme::{self, size, W};
use eframe::egui::{self, Color32, Pos2, Rect};
use std::f32::consts::{FRAC_PI_2, TAU};

/// 剪贴板分栏一屏显示的条数。
pub(crate) const CLIP_ROWS: usize = 4;

fn mmss(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// 由标题生成稳定的封面渐变色。
fn cover_colors(title: &str) -> (Color32, Color32) {
    let mut h: u64 = 1469598103934665603;
    for b in title.bytes() {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    let palette = [
        (Color32::from_rgb(255, 94, 98), Color32::from_rgb(255, 153, 102)),
        (Color32::from_rgb(86, 204, 242), Color32::from_rgb(47, 128, 237)),
        (Color32::from_rgb(168, 85, 247), Color32::from_rgb(236, 72, 153)),
        (Color32::from_rgb(52, 211, 153), Color32::from_rgb(16, 120, 200)),
        (Color32::from_rgb(251, 191, 36), Color32::from_rgb(239, 68, 68)),
    ];
    palette[(h % palette.len() as u64) as usize]
}

impl App {
    // ================= 音乐 =================
    pub(crate) fn page_music(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        let np=self.media.clone();
        let has=!np.title.is_empty();
        let with_lyrics=has && self.show_lyrics;
        let ctx=c.ui.ctx().clone();
        let playing_t=ctx.animate_bool_with_time(egui::Id::new("np-playing"),np.playing,0.22);
        let art_size=if with_lyrics {44.0}else{62.0};
        let art_y=if with_lyrics {23.0}else{13.0};
        if let Some((image,texture))=&self.music_artwork {
            let [w,h]=image.size;
            let (uw,vh)=if w>h {(h as f32/w as f32,1.0)}else{(1.0,w as f32/h as f32)};
            let uv=Rect::from_center_size(Pos2::new(0.5,0.5),egui::vec2(uw,vh));
            c.painter.add(egui::epaint::RectShape::filled(c.rect(44.0,art_y,art_size,art_size),10.0*c.s,Color32::WHITE).with_texture(texture.id(),uv));
        } else if has {
            let (a,b)=cover_colors(&np.title);
            let cc=c.clipped(44.0,art_y,art_size,art_size);
            cc.vgrad(44.0,art_y,art_size,art_size,10.0,a,b);
            paint::note(&cc,44.0+art_size/2.0,art_y+art_size/2.0,art_size*0.39,Color32::from_white_alpha(235));
        } else {
            c.fill(44.0,13.0,62.0,62.0,13.0,Color32::from_rgb(25,25,27));
            paint::note(c,75.0,44.0,26.0,Color32::from_rgb(110,110,114));
        }

        if !has {
            c.text(121.0,47.0,"暂无音乐",14.0,W::Bold,theme::WHITE,H::Left);
        } else {
            let text_x=if with_lyrics {99.0}else{121.0};
            let text_width=if with_lyrics {123.0}else{220.0};
            let title_size=if with_lyrics {12.0}else{14.0};
            c.text(text_x,if with_lyrics {41.0}else{53.0},&c.fit(&np.title,title_size,W::Bold,text_width),title_size,W::Bold,theme::WHITE,H::Left);
            c.text(text_x,if with_lyrics {57.0}else{67.0},&c.fit(&np.artist,size::BODY,W::Regular,text_width),size::BODY,W::Regular,theme::GRAY,H::Left);
            if with_lyrics {self.draw_music_lyrics(c);}
        }

        let tc=if has {theme::WHITE}else{Color32::from_rgb(78,78,82)};
        let (bx,bw,by)=if with_lyrics {(44.0,176.0,82.0)}else{(96.0,221.0,92.6)};
        let bar=c.hit_drag(bx-4.0,by-7.0,bw+8.0,14.0,"m-seek");
        let seek_position=if has && np.duration>0.0 && (bar.is_pointer_button_down_on() || bar.clicked() || bar.drag_stopped()) {
            bar.interact_pointer_pos().or_else(||ctx.input(|i|i.pointer.interact_pos()))
                .map(|pointer|((c.to_local(pointer).x-bx)/bw).clamp(0.0,1.0) as f64*np.duration)
        }else{None};
        let position=seek_position.unwrap_or(np.position);
        let frac=if np.duration>0.0 {(position/np.duration).clamp(0.0,1.0) as f32}else{0.0};
        let (time_y,time_size,right)=if with_lyrics {(100.0,10.0,220.0)}else{(97.0,15.0,375.0)};
        c.text(44.0,time_y,&mmss(position),time_size,W::Regular,tc,H::Left);
        let remaining=if has && np.duration<=0.0{"--:--".into()}else{format!("-{}",mmss((np.duration-position).max(0.0)))};
        c.text(right,time_y,&remaining,time_size,W::Regular,tc,H::Right);
        let (bh,_)=c.hover_t(&bar);
        let bh=if has {bh}else{0.0};
        let th=lerp(3.5,5.0,bh);
        c.fill(bx,by-th/2.0,bw,th,th/2.0,if has {Color32::from_gray(90)}else{theme::TRACK});
        if self.settings.skin==crate::model::Skin::Fluid {
            let head=bx+bw*frac;
            let hot=if bar.is_pointer_button_down_on(){1.0}else if np.playing{0.62}else{0.45};
            if has && frac>0.0 {
                c.fill(bx,by-th/2.0,bw*frac,th,th/2.0,self.fluid.tint_color(0.78));
                paint::chase(c,bx,head,by,th,self.fluid.tint_color(0.30),hot);
            }
            if c.live {
                let pos=c.pos(head,by)-self.fluid_origin;
                self.fluid.set_seek((has && frac>0.0).then_some((pos.x,pos.y)),hot);
            }
        }else{
        if frac>0.0 {c.fill(bx,by-th/2.0,(bw*frac).max(th),th,th/2.0,theme::WHITE);}
        if bh>0.01 {c.faded(bh).circle(bx+bw*frac,by,4.5,theme::WHITE);}
        }
        if bar.clicked() || bar.drag_stopped() {
            if let Some(position)=seek_position {cmds.push(Cmd::Media(Transport::Seek(position)));}
        }

        let y=118.0;
        let ic=if has {theme::WHITE}else{Color32::from_rgb(88,88,92)};
        let btn=|x:f32,w:f32,id:&str| {
            let response=c.hit(x-w/2.0,y-12.0,w,24.0,id);
            let (hover,pressed)=c.hover_t(&response);
            if has && hover>0.01 {c.faded(hover).circle(x,y,12.5,Color32::from_white_alpha(20));}
            (response,c.scaled(x,y,1.0-0.1*pressed))
        };
        let (lyrics_button,lyrics_card)=btn(74.0,26.0,"m-lyr");
        lyrics_card.text(74.0,y+5.0,"词",14.0,W::Semibold,if !has {ic}else if self.show_lyrics {theme::WHITE}else{theme::GRAY},H::Center);
        let (previous,previous_card)=btn(162.0,30.0,"m-prev");
        paint::skip(&previous_card,162.0,y,-1.0,ic);
        let (play,play_card)=btn(210.5,30.0,"m-play");
        paint::play_pause(&play_card,211.0,y,15.0,playing_t,ic);
        let (next,next_card)=btn(258.0,30.0,"m-next");
        paint::skip(&next_card,258.0,y,1.0,ic);
        let (volume_button,volume_card)=btn(367.0,26.0,"m-volume");
        let (volume,muted)=self.volume_ui.unwrap_or((0,false));
        paint::speaker(&volume_card,367.0,y,muted,ic);
        if self.volume_ui.is_some() && volume_button.on_hover_text(if muted {"取消静音"}else{"静音"}).clicked() {
            self.volume_ui=Some((volume,!muted));
            self.volume_quiet_until=self.t+0.9;
            cmds.push(Cmd::ToggleMute);
        }
        if has {
            if previous.clicked() {cmds.push(Cmd::Media(Transport::Prev));}
            if play.clicked() {cmds.push(Cmd::Media(Transport::Toggle));}
            if next.clicked() {cmds.push(Cmd::Media(Transport::Next));}
        }
        if lyrics_button.on_hover_text(if self.show_lyrics {"隐藏歌词"}else{"显示歌词"}).clicked() {
            if !has {self.say("先放一首歌吧");}
            else {
                self.show_lyrics=!self.show_lyrics;
                self.settings.lyrics_enabled=self.show_lyrics;
                self.settings.save();
            }
        }
        // 坞上歌词是横向的，只在顶部停靠时提供。
        if has && !self.side() {
            let (button,button_card)=capsule(c,279.0,108.0,68.0,20.0,"m-island-lyrics",if self.settings.island_lyrics {Color32::from_gray(39)}else{Color32::from_gray(22)},Color32::from_gray(47));
            button_card.text(313.0,122.0,"坞上歌词",10.0,W::Semibold,if self.settings.island_lyrics {theme::WHITE}else{theme::GRAY},H::Center);
            if button.on_hover_text(if self.settings.island_lyrics {"关闭收起时的歌词"}else{"收起时显示当前歌词"}).clicked() {
                self.settings.island_lyrics=!self.settings.island_lyrics;
                self.settings.save();
            }
        }
    }

    fn draw_music_lyrics(&mut self,c:&Card) {
        let col=c.clipped(243.0,28.0,136.0,74.0);
        c.seg((233.0,32.0),(233.0,98.0),1.0,Color32::from_gray(35));
        if let Some(lyrics)=&self.ly.lyrics {
            let current=if lyrics.synced {lyrics.index_at(self.lyric_position())}else{Some(0)};
            let target=current.unwrap_or(0);
            let id=egui::Id::new("music-lyrics-scroll");
            if !self.ly.scroll_animation_started {
                c.ui.ctx().animate_value_with_time(id,target as f32,0.0);
                self.ly.scroll_animation_started=true;
            }
            let position=c.ui.ctx().animate_value_with_time(id,target as f32,0.3);
            let first=(position.floor() as usize).saturating_sub(2);
            for (index,line) in lyrics.lines.iter().enumerate().skip(first).take(6) {
                let distance=index as f32-position;
                if distance.abs()>2.2 {continue;}
                let active=current==Some(index);
                let ink=if active {theme::WHITE}else{theme::GRAY};
                let row=col.faded((1.0-distance.abs()*0.35).clamp(0.0,1.0));
                let mut job=egui::text::LayoutJob::simple(line.text.clone(),row.font(if active {12.0}else{10.0},if active {W::Semibold}else{W::Regular}),ink,132.0*c.s);
                job.wrap.max_rows=if active {3}else{1};
                job.wrap.break_anywhere=true;
                let galley=row.painter.layout_job(job);
                let y=64.0+distance*29.0-galley.size().y/(2.0*c.s);
                row.painter.galley(row.pos(246.0,y),galley,ink);
            }
            if !lyrics.synced {col.text(246.0,39.0,"文本歌词",8.0,W::Regular,theme::GRAY,H::Left);}
        } else {
            let status=if self.ly.loading {"正在查找歌词…"}else if let Some(error)=&self.ly.error {error.label()}else{"暂无歌词"};
            let galley=col.painter.layout(status.into(),col.font(10.0,W::Regular),theme::GRAY,130.0*c.s);
            col.painter.galley(col.pos(246.0,61.0-galley.size().y/(2.0*c.s)),galley,theme::GRAY);
            if self.ly.error.is_some() {
                col.text(246.0,87.0,"点击重试",9.0,W::Regular,theme::GRAY,H::Left);
                if c.hit(243.0,28.0,136.0,74.0,"lyrics-retry").on_hover_text("重新查询当前歌曲的歌词").clicked(){self.ly.key.clear();}
            }
        }
    }

    // ================= 专注（番茄钟） =================
    pub(crate) fn page_focus(&mut self, c: &Card, _env: &Env) {
        if c.hit(90.0,8.0,42.0,21.0,"focus-settings").on_hover_text("专注设置").clicked(){self.open_settings_category(1);}
        c.text(111.0,23.0,"设置",10.5,W::Regular,theme::GRAY,H::Center);
        let (rx, ry, rr) = (79.0, 74.0, 32.0);
        let frac = self.ring_shown.pos.clamp(0.0, 1.0);
        let run = c.ui.ctx().animate_bool_with_time(egui::Id::new("pomo-run"), self.pomo.running, 0.3);
        let resting=self.pomo.phase!=Phase::Focus;
        let (top,bottom)=if resting{(theme::SOFT_GREEN,mix(theme::SOFT_GREEN,Color32::BLACK,0.2))}else{(theme::RED_TOP,theme::RED)};
        // 外圈柔光；计时中缓慢呼吸
        let breathe = 0.5 + 0.5 * ((self.t * 2.2).sin() as f32);
        let glow = 0.30 + 0.25 * run * breathe;
        c.glow(rx, ry, 48.0, paint::alpha(bottom, glow * 0.55));
        c.ring(rx, ry, rr, 7.0, if resting{mix(theme::CARD,top,0.18)}else{Color32::from_rgb(58,22,26)});
        // 渐变进度环随当前阶段配色。
        if frac > 0.001 {
            let steps = (120.0 * frac).ceil().max(2.0) as usize;
            let a0 = -FRAC_PI_2;
            let mut prev = (rx + rr * a0.cos(), ry + rr * a0.sin());
            for i in 1..=steps {
                let a = a0 + TAU * frac * i as f32 / steps as f32;
                let next = (rx + rr * a.cos(), ry + rr * a.sin());
                let t = ((prev.1 + next.1) * 0.5 - (ry - rr)) / (2.0 * rr);
                c.seg(prev, next, 7.0, mix(top, bottom, t));
                prev = next;
            }
            // 圆头
            c.circle(rx, ry - rr, 3.5, top);
            c.circle(prev.0, prev.1, 3.5, mix(top, bottom, (prev.1 - (ry - rr)) / (2.0 * rr)));
        }
        let hand = if self.pomo.running { (self.t as f32 * 1.2) % TAU } else { 0.0 };
        if resting {
            let t=if self.pomo.running && crate::bar_layout::animations_enabled(){self.t as f32}else{0.0};
            crate::focus_bar::tea(&c.scaled(rx,ry,2.2),rx,ry,frac,t);
        }else{paint::stopwatch(c, rx, ry + 0.5, 15.5, hand * run, theme::WHITE);}

        let secs = self.pomo.remaining.max(0.0).ceil() as u32;
        c.text(264.0, 61.0, &format!("{:02}:{:02}", secs / 60, secs % 60), size::HERO + 1.0, W::Bold, if resting{theme::SOFT_GREEN}else{theme::WHITE}, H::Center);
        c.text(264.0, 79.0, self.pomo.phase.label(), size::SMALL, W::Regular, theme::GRAY, H::Center);

        // 重置：点一下图标转一圈
        let r = c.hit(317.0, 38.0, 24.0, 24.0, "f-reset");
        let (h, p) = c.hover_t(&r);
        let rc = c.scaled(329.0, 50.0, 1.0 - 0.1 * p);
        rc.circle(329.0, 50.0, 11.0, mix(Color32::from_rgb(28, 28, 30), Color32::from_rgb(50, 50, 54), h));
        paint::reset(&rc, 329.0, 50.0, -self.reset_spin.pos, Color32::from_gray(230));
        if r.clicked() {
            self.pomo.remaining = self.phase_secs(self.pomo.phase);
            self.pomo.running = false;
            self.reset_spin.target += TAU;
        }

        for (phase,label,x,width,id) in [(Phase::Focus,"专注",148.0,88.0,"f-focus"),(Phase::Rest,"休息",292.0,88.0,"f-rest")] {
            let selected=self.pomo.phase==phase;
            let accent=if phase==Phase::Focus{theme::RED_TOP}else{theme::SOFT_GREEN};
            let background=if selected{mix(theme::BTN,accent,0.22)}else{theme::BTN};
            let (response,button)=capsule(c,x,94.0,width,25.0,id,background,if selected{mix(theme::BTN_HOVER,accent,0.25)}else{theme::BTN_HOVER});
            button.text(x+width/2.0,110.0,label,size::BODY,W::Semibold,if selected{mix(accent,theme::WHITE,0.5)}else{theme::WHITE},H::Center);
            if response.clicked(){self.set_phase(phase);}
        }
        let (b2, c2) = capsule(c, 246.0, 94.0, 36.0, 25.0, "f-start", theme::RED, Color32::from_rgb(255, 82, 98));
        paint::play_pause(&c2, 264.0, 106.5, 11.0, run, theme::WHITE);
        let b2=b2.on_hover_text(if self.pomo.running{"暂停计时"}else{"开始计时"});
        if b2.clicked() {
            if self.pomo.remaining <= 0.0 {
                self.pomo.remaining = self.phase_secs(self.pomo.phase);
            }
            self.pomo.running = !self.pomo.running;
        }
    }

    fn set_phase(&mut self, ph: Phase) {
        if self.pomo.phase==ph {return;}
        self.pomo.phase = ph;
        self.pomo.remaining = self.phase_secs(ph);
        self.pomo.running = false;
    }

    // ================= 置物架 =================
    /// 标题旁的「文件 | 剪贴板」分栏，返回分栏右边缘。
    /// 页面标题处的「置物架 | 剪贴板」分栏，返回分栏右边缘。
    fn shelf_tabs(&mut self, c: &Card, x: f32) -> f32 {
        let labels = ["置物架", "剪贴板"];
        let font = 12.0;
        let widths: Vec<f32> = labels.iter().map(|l| c.text_w(l, font, W::Bold) + 14.0).collect();
        let total = widths.iter().sum::<f32>() + 4.0;
        c.fill(x, 9.5, total, 21.0, 10.5, theme::BTN);
        let mut left = x + 2.0;
        for (i, (label, w)) in labels.iter().zip(&widths).enumerate() {
            let selected = self.shelf_view.shelf_tab == i;
            let r = c.hit(left, 9.5, *w, 21.0, ("shelf-tab", i));
            let (h, _) = c.hover_t(&r);
            if selected {
                c.fill(left, 11.5, *w, 17.0, 8.5, Color32::from_gray(88));
            } else if h > 0.01 {
                c.fill(left, 11.5, *w, 17.0, 8.5, Color32::from_white_alpha((18.0 * h) as u8));
            }
            c.text(left + w / 2.0, 24.3, label, font, W::Bold, if selected { theme::WHITE } else { theme::GRAY }, H::Center);
            if r.clicked() && !selected {
                self.shelf_view.switch_tab(i);
            }
            left += w;
        }
        x + total
    }

    pub(crate) fn page_shelf(&mut self, c: &Card, env: &Env, cmds: &mut Vec<Cmd>) {
        let n = self.shelf.items.len();
        let drag = self.drag_a;
        // 拖文件进来时回到「文件」分栏。
        if drag > 0.002 { self.shelf_view.shelf_tab = 0; }
        self.shelf_tabs(c, 13.0);
        if self.shelf_view.shelf_tab == 1 {
            self.page_clipboard(c, env, cmds);
            return;
        }
        c.text(344.0,23.0,"添加",size::SMALL,W::Regular,theme::GRAY,H::Right);
        if c.hit(311.0,8.0,38.0,22.0,"shelf-add").on_hover_text("从电脑选择文件").clicked(){cmds.push(Cmd::AddFiles);}
        if n>5 {c.text(209.5,121.0,&format!("{}–{} / {n} · 滚轮查看",self.shelf_view.shelf_first+1,(self.shelf_view.shelf_first+5).min(n)),10.5,W::Regular,theme::DIM,H::Center);}

        if n > 0 {
            // 「添加」左边的「清空」：第一次点变红，3 秒内再点才真正清空。
            // 「确认」与「清空」同宽，不会挤到顶栏时间。
            let confirming = self.shelf_view.confirm_clear.is_some();
            let label = if confirming { "确认" } else { "清空" };
            let lw = c.text_w("清空", size::SMALL, W::Regular) + 16.0;
            let base = if confirming { Color32::from_rgb(120, 28, 36) } else { theme::BTN };
            let hover = if confirming { Color32::from_rgb(150, 34, 44) } else { theme::BTN_HOVER };
            // 放在右下角：顶栏中间是日期时间，宽度随 12 小时制等设置变化。
            let (x, y) = (377.0 - lw, 109.0);
            let (r, bc) = capsule(&c.faded(1.0 - drag), x, y, lw, 17.0, "shelf-clear", base, hover);
            let r = if confirming { r.on_hover_text("再点一次清空置物架（文件本身不动）") } else { r.on_hover_text("清空置物架") };
            bc.text(x + lw / 2.0, y + 12.0, label, size::SMALL, W::Regular, theme::WHITE, H::Center);
            if r.clicked() {
                if confirming {
                    self.shelf.clear();
                    self.shelf_view.reset_after_clear();
                    self.say("置物架已清空（文件本身没动）");
                } else {
                    self.shelf_view.confirm_clear = Some(self.t);
                }
            }
        }

        let body = c.faded(lerp(1.0, 0.14, drag));
        if n == 0 {
            let a = 1.0 - drag;
            if a > 0.0 {
                let ec = body.faded(a);
                ec.dashed(42.0, 32.0, 335.0, 91.0, 12.0, 1.2, Color32::from_gray(70), 0.0);
                paint::tray(&ec, 209.5, 60.0, 28.0, 0.0, Color32::from_gray(200), Color32::from_gray(80));
                ec.text(209.5, 93.0, "把文件拖到这儿", 12.0, W::Semibold, theme::WHITE, H::Center);
                ec.text(209.5, 108.0, "只记位置，不复制文件 · 点一下打开", size::TINY, W::Regular, theme::GRAY, H::Center);
            }
        } else {
            self.shelf_row(&body, cmds);
        }
        if drag > 0.002 {
            self.drag_overlay(&c.faded(drag));
        }
    }

    fn shelf_row(&mut self, c: &Card, cmds: &mut Vec<Cmd>) {
        let n = self.shelf.items.len();
        let slot = 68.0;
        let center = 209.5;
        let visible = n.min(5) as f32;
        let scroll = self.shelf_view.shelf_scroll.pos;
        let row = c.clipped(36.0, 28.0, 347.0, 100.0);
        let dt = self.dt;
        let items = self.shelf.items.clone();
        self.shelf_view.icons.retain(&items);
        self.shelf_view.shelf_x.retain(|p, _| items.contains(p));
        for (i, p) in items.iter().enumerate() {
            let target = center + (i as f32 - scroll - (visible - 1.0) / 2.0) * slot;
            let born = self.shelf_view.shelf_born.get(p).copied();
            let s = self.shelf_view.shelf_x.entry(p.clone()).or_insert_with(|| {
                // 新放入的从右侧滑进来；启动时已有的直接就位
                let start = if born.is_some() { target + slot * 0.9 } else { target };
                Spring::new(start, 230.0, 0.9)
            });
            s.target = target;
            s.step(dt);
            let x = s.pos;
            if x < 0.0 || x > 420.0 {
                continue;
            }
            let appear = born.map(|b| smoothstep(0.0, 0.35, (self.t - b) as f32)).unwrap_or(1.0);
            let edge = smoothstep(26.0, 60.0, x) * smoothstep(392.0, 358.0, x);
            let a = appear * edge;
            if a <= 0.01 {
                continue;
            }
            let r = row.hit_drag(x - 31.0, 30.0, 62.0, 78.0, ("shelf", p));
            let show_hint=self.shelf_view.shelf_menu_rect.is_none() && !r.context_menu_opened() && !r.secondary_clicked();
            if show_hint {r.clone().on_hover_text(p.display().to_string());}
            let (h, pr) = row.hover_t(&r);
            let cc = row.faded(a).scaled(x, 66.0, 1.0 - 0.04 * pr);
            if h > 0.01 {
                cc.fill(x - 31.0, 30.0, 62.0, 78.0, 10.0, Color32::from_white_alpha((16.0 * h) as u8));
            }
            let lift = -2.0 * h + (1.0 - appear) * 8.0;
            self.file_icon_at(&cc, p, x, 52.0 + lift, 36.0);
            let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let name = cc.fit(&name, size::SMALL, W::Semibold, 60.0);
            cc.text(x, 86.0, &name, size::SMALL, W::Semibold, theme::WHITE, H::Center);
            let meta=self.shelf_view.icons.get(c.ui.ctx(),p).map(|preview|preview.metadata).unwrap_or_default();
            cc.text(x, 99.0, &meta, size::TINY, W::Regular, theme::GRAY, H::Center);
            if r.drag_started() && p.exists() {cmds.push(Cmd::DragFile(p.clone()));}
            let remove=row.hit(x+15.0,30.0,17.0,18.0,("shelf-remove",p));
            let remove_card=row.faded(a);
            remove_card.circle(x+23.5,39.0,7.5,Color32::from_rgb(55,40,46));
            paint::close(&remove_card,x+23.5,39.0,2.4,theme::WHITE);
            let mut delete=remove.clicked();
            if show_hint {remove.on_hover_text("从置物架移除，保留原文件");}
            r.context_menu(|ui|{
                if ui.button("打开").clicked(){if p.exists(){cmds.push(Cmd::Open(p.clone()));}ui.close_menu();}
                if ui.button("在文件夹中显示").clicked(){cmds.push(Cmd::Reveal(p.clone()));ui.close_menu();}
                if ui.button("复制完整路径").clicked(){cmds.push(Cmd::Copy(p.display().to_string()));ui.close_menu();}
                if ui.button("从置物架移除").clicked(){delete=true;ui.close_menu();}
                self.shelf_view.shelf_menu_rect=Some(egui::Frame::menu(ui.style()).widget_rect(ui.min_rect()));
            });
            if delete { self.shelf.remove(p); self.shelf_view.remove_path(p, self.shelf.items.len()); }
            else if r.clicked() { if p.exists(){cmds.push(Cmd::Open(p.clone()));} else {self.say("文件已移动或删除，可从置物架移除");} }
        }
    }

    // ================= 剪贴板（置物架第二个分栏） =================
    fn page_clipboard(&mut self, c: &Card, _env: &Env, _cmds: &mut Vec<Cmd>) {
        if !self.settings.clipboard_history {
            c.dashed(42.0, 32.0, 335.0, 91.0, 12.0, 1.2, Color32::from_gray(70), 0.0);
            c.text(209.5, 66.0, "剪贴板记录已关闭", 12.0, W::Semibold, theme::WHITE, H::Center);
            c.text(209.5, 82.0, "开启后，最近复制的文字会出现在这里", size::TINY, W::Regular, theme::GRAY, H::Center);
            let (r, bc) = capsule(c, 179.5, 92.0, 60.0, 20.0, "clip-enable", theme::BTN, theme::BTN_HOVER);
            bc.centered_text(209.5, 102.0, "开启", size::SMALL, W::Semibold, theme::WHITE, H::Center);
            if r.clicked() { self.settings.clipboard_history = true; self.clipboard.set_enabled(true); self.settings.save(); }
            return;
        }
        let items = self.clipboard.lock().items.clone();
        let n = items.len();
        let recent = items.iter().filter(|clip| !clip.pinned).count();
        self.shelf_view.clip_first = self.shelf_view.clip_first.min(n.saturating_sub(CLIP_ROWS));

        // 右上：条数与「清空」（只清未固定的）。
        if recent > 0 {
            let confirming = self.shelf_view.clip_confirm_clear.is_some_and(|at| self.t - at < 3.0);
            if !confirming { self.shelf_view.clip_confirm_clear = None; }
            let label = if confirming { "确认" } else { "清空" };
            let lw = c.text_w("清空", size::SMALL, W::Regular) + 16.0;
            let base = if confirming { Color32::from_rgb(120, 28, 36) } else { theme::BTN };
            let hover = if confirming { Color32::from_rgb(150, 34, 44) } else { theme::BTN_HOVER };
            let x = 351.0 - lw;
            let (r, bc) = capsule(c, x, 11.5, lw, 17.0, "clip-clear", base, hover);
            bc.text(x + lw / 2.0, 23.5, label, size::SMALL, W::Regular, theme::WHITE, H::Center);
            let r = r.on_hover_text(if confirming { "再点一次清空未固定的记录" } else { "清空未固定的记录" });
            if r.clicked() {
                if confirming {
                    self.clipboard.lock().clear_recent();
                    self.shelf_view.clip_first = 0;
                    self.shelf_view.clip_confirm_clear = None;
                    self.say("剪贴板记录已清空（固定的保留）");
                } else {
                    self.shelf_view.clip_confirm_clear = Some(self.t);
                }
            }
        }

        if n == 0 {
            c.dashed(42.0, 32.0, 335.0, 91.0, 12.0, 1.2, Color32::from_gray(70), 0.0);
            paint::clipboard_glyph(c, 209.5, 58.0, 22.0, Color32::from_gray(200), Color32::from_gray(80));
            c.text(209.5, 93.0, "复制的文字会出现在这里", 12.0, W::Semibold, theme::WHITE, H::Center);
            c.text(209.5, 108.0, "记录保存在本机，重新打开仍可查看", size::TINY, W::Regular, theme::GRAY, H::Center);
            return;
        }

        c.fill(42.0, 31.0, 335.0, 93.0, 12.0, theme::CARD);
        // 超过一屏时右侧细滚动条，滚轮查看。
        if n > CLIP_ROWS {
            let (top, track) = (38.0, 79.0);
            let thumb = (track * CLIP_ROWS as f32 / n as f32).max(12.0);
            let y = top + (track - thumb) * self.shelf_view.clip_first as f32 / (n - CLIP_ROWS) as f32;
            c.fill(371.0, top, 2.5, track, 1.25, Color32::from_white_alpha(14));
            c.fill(371.0, y, 2.5, thumb, 1.25, Color32::from_white_alpha(70));
        }
        let mut action: Option<(usize, u8)> = None; // 0 复制 1 固定 2 删除
        for (row, (index, clip)) in items.iter().enumerate().skip(self.shelf_view.clip_first).take(CLIP_ROWS).enumerate() {
            let y = 44.0 + row as f32 * 22.5;
            let r = c.hit(48.0, y - 10.0, 323.0, 20.0, ("clip", index, clip.text.len(), clip.at.as_str()));
            let (h, _) = c.hover_t(&r);
            if h > 0.01 {
                c.fill(48.0, y - 10.0, 323.0, 20.0, 7.0, Color32::from_white_alpha((12.0 * h) as u8));
            }
            if clip.pinned {
                paint::pin(c, 58.0, y, 9.0, theme::BLUE);
            } else {
                c.circle(58.0, y, 2.2, Color32::from_gray(110));
            }
            let text = c.fit(&crate::clipboard::preview(&clip.text), size::BODY, W::Regular, 210.0);
            c.text(68.0, y + 3.8, &text, size::BODY, W::Regular, Color32::from_gray(235), H::Left);
            c.text(318.0, y + 3.5, &clip.at, size::TINY, W::Regular, theme::DIM, H::Right);
            let tip: String = clip.text.chars().take(400).collect();
            let r = r.on_hover_text(format!("{tip}{}\n点击复制", if clip.text.chars().count() > 400 { "…" } else { "" }));
            let pin = c.hit(323.0, y - 9.0, 18.0, 18.0, ("clip-pin", index)).on_hover_text(if clip.pinned { "取消固定" } else { "固定（清空时保留）" });
            paint::pin(c, 332.0, y, 9.0, if clip.pinned || pin.hovered() { theme::BLUE } else { theme::GRAY });
            let delete = c.hit(345.0, y - 9.0, 17.0, 18.0, ("clip-delete", index)).on_hover_text("删除这条记录");
            paint::close(c, 353.5, y, 2.4, theme::GRAY);
            if delete.clicked() { action = Some((index, 2)); }
            else if pin.clicked() { action = Some((index, 1)); }
            else if r.clicked() { action = Some((index, 0)); }
        }
        match action {
            Some((index, 0)) => {
                #[cfg(windows)]
                self.say(if crate::clipboard::copy_history(&items[index].text) { "已复制" } else { "复制失败，请重试" });
                #[cfg(not(windows))]
                _cmds.push(Cmd::Copy(items[index].text.clone()));
            }
            Some((index, 1)) => {
                if !self.clipboard.lock().toggle_pin(index) { self.say(format!("最多固定 {} 条", crate::clipboard::MAX_PINNED)); }
            }
            Some((index, _)) => { self.clipboard.lock().remove(index); }
            None => {}
        }
    }

    fn file_icon_at(&mut self, c: &Card, p: &std::path::Path, cx: f32, cy: f32, sz: f32) {
        let tex=self.shelf_view.icons.get(c.ui.ctx(),p).and_then(|preview|preview.image);
        if let Some(tex) = tex {
            let s = tex.size_vec2();
            let k = sz / s.x.max(s.y);
            let r = Rect::from_center_size(c.pos(cx, cy), s * k * c.s);
            c.painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        } else {
            paint::file_glyph(c, paint::file_kind(p), cx, cy, sz);
        }
    }

    /// 拖入遮罩：流动虚线框 + 跟随光标的柔光 + 上下轻跳的托盘图标。
    fn drag_overlay(&self, c: &Card) {
        let (x, y, w, h) = (42.0, 31.0, 335.0, 93.0);
        c.fill(x, y, w, h, 12.0, Color32::from_black_alpha(110));
        if let Some(p) = self.cursor_local {
            let px = p.x.clamp(x + 20.0, x + w - 20.0);
            let py = p.y.clamp(y + 10.0, y + h - 10.0);
            c.clipped(x, y, w, h).glow(px, py, 90.0, Color32::from_rgba_unmultiplied(120, 170, 255, 34));
        }
        c.dashed(x, y, w, h, 12.0, 1.4, Color32::from_white_alpha(230), self.t as f32 * 14.0);
        let bob = ((self.t * 5.0).sin() as f32 * 0.5 + 0.5) * 3.0;
        paint::tray(c, 209.5, 64.0, 30.0, bob, theme::WHITE, Color32::from_gray(90));
        c.text(209.5, 98.0, "松手放进置物架", 12.0, W::Semibold, theme::WHITE, H::Center);
    }

    // ================= 待办 =================
    pub(crate) fn page_todo(&mut self, c: &Card, env: &Env, _cmds: &mut Vec<Cmd>) {
        // 今天要做的在前；不在今天的每周重复待办排在后面，变暗显示、不能勾选。
        let (done, total) = self.todos.progress(&env.today);
        let order = self.todos.display_order(&env.today);
        let count = order.len();
        let pop = self.todo.todo_pop.pos.abs().min(0.4);
        let hc = c.scaled(60.0, 50.0, 1.0 + pop * 0.25);
        let w = hc.text(42.0, 61.0, &format!("{done}"), size::HERO, W::Bold, if done > 0 && done == total { theme::GREEN } else { theme::WHITE }, H::Left);
        hc.text(42.0 + w, 61.0, &format!("/{total}"), size::HERO, W::Bold, theme::WHITE, H::Left);
        c.text(42.0, 78.0, "已完成", size::SMALL, W::Regular, theme::GRAY, H::Left);

        let (add, ac) = capsule(c, 41.0, 96.0, 74.0, 26.0, "t-add", theme::BTN, theme::BTN_HOVER);
        paint::plus(&ac, 56.0, 109.0, 6.2, theme::WHITE, Color32::BLACK);
        ac.text(67.0, 112.5, "加一个", size::BODY, W::Regular, theme::WHITE, H::Left);
        let can_edit = self.todos.items.iter().any(|t| !t.done_for(&env.today));
        let ec = if can_edit { c.clone() } else { c.faded(0.45) };
        let (edit, edc) = capsule(&ec, 121.0, 96.0, 68.0, 26.0, "t-edit", theme::BTN, if can_edit { theme::BTN_HOVER } else { theme::BTN });
        paint::pencil(&edc, 135.0, 109.0, 11.0, theme::WHITE);
        edc.text(145.0, 112.5, "改一下", size::BODY, W::Regular, theme::WHITE, H::Left);

        c.fill(196.0, 31.0, 181.0, 93.0, 12.0, theme::CARD);
        let editing = self.todo.adding || self.todo.editing.is_some();
        if count == 0 && !editing {
            paint::checklist(c, 286.5, 64.0, 32.0, Color32::from_gray(150), Color32::from_rgb(52, 54, 58));
            c.text(286.5, 99.0, "今天还没有安排", size::SMALL, W::Regular, theme::GRAY, H::Center);
        } else {
            let mut row = 0;
            let row_y = |r: usize| 46.0 + r as f32 * 22.0;
            if editing {
                // 输入行（文本框本身由 app::input_field 放置）
                let y = row_y(0);
                c.fill(203.0, y - 10.0, 167.0, 20.0, 7.0, Color32::from_rgb(34, 34, 38));
                c.stroke(203.0, y - 10.0, 167.0, 20.0, 7.0, 1.0, paint::alpha(theme::BLUE, 0.7));
                if self.todo.editing.is_some() {
                    paint::pencil(c, 212.0, y, 9.0, theme::BLUE);
                } else {
                    paint::plus(c, 212.0, y, 5.0, theme::BLUE, Color32::BLACK);
                }
                row = 1;
            }
            let ctx = c.ui.ctx().clone();
            let cap = if editing { 2 } else { 3 };
            let mut shown = 0;
            for &i in order.iter().skip(self.todo_first.min(count.saturating_sub(1))) {
                if shown >= cap {
                    break;
                }
                if Some(i) == self.todo.editing {
                    continue;
                }
                let item = self.todos.items[i].clone();
                let today_item = item.scheduled_on(&env.today);
                let y = row_y(row);
                let r = c.hit(202.0, y - 10.0, 169.0, 20.0, ("todo", i, item.text.as_str()));
                let (h, _) = c.hover_t(&r);
                if h > 0.01 {
                    c.fill(202.0, y - 10.0, 169.0, 20.0, 7.0, Color32::from_white_alpha((12.0 * h) as u8));
                }
                let checked = item.done_for(&env.today);
                let ct = ctx.animate_bool_with_time(egui::Id::new(("todo-check", item.text.as_str())), checked, 0.28);
                let rc = if today_item { c.clone() } else { c.faded(0.45) };
                rc.ring(212.0, y, 5.2, 1.3, paint::alpha(Color32::from_gray(120), 1.0 - ct));
                if ct > 0.0 {
                    c.circle(212.0, y, 6.0 * anim::ease_out_cubic(ct.min(1.0) / 0.6).min(1.0), theme::GREEN);
                    paint::check(c, 212.0, y, 6.0, smoothstep(0.3, 1.0, ct), 1.4, Color32::BLACK);
                }
                // 每周重复：文字右侧小标签（周一三五 / 工作日 / 每天）。
                let tag = crate::model::repeat_tag(item.repeat);
                let tag_w = if tag.is_empty() { 0.0 } else { c.text_w(&tag, size::TINY, W::Regular) + 6.0 };
                let text = c.fit(&item.text, size::BODY, W::Regular, 104.0 - tag_w);
                let repeat_hint = if item.is_repeating() { format!("\n{}{}", crate::model::repeat_label(item.repeat), if today_item { "" } else { " · 今天不用做" }) } else { String::new() };
                r.clone().on_hover_text(format!("{}{}{}",item.text,repeat_hint,item.reminder_text().map(|d|format!("\n提醒：{d}")).unwrap_or_default()));
                let tcol = mix(Color32::from_gray(235), Color32::from_gray(110), ct);
                let tw = rc.text(223.0, y + 3.8, &text, size::BODY, W::Regular, tcol, H::Left);
                if !tag.is_empty() {
                    rc.text(223.0 + tw + 5.0, y + 3.2, &tag, size::TINY, W::Regular, if today_item { theme::BLUE } else { theme::GRAY }, H::Left);
                }
                if ct > 0.0 {
                    c.fill(223.0, y + 0.2, tw * smoothstep(0.2, 1.0, ct), 1.0, 0.5, Color32::from_gray(110));
                }
                let delete=c.hit(353.0,y-9.0,17.0,18.0,("todo-delete",i)).on_hover_text("删除待办").clicked();
                paint::close(c,361.5,y,2.4,theme::GRAY);
                let reminder=c.hit(333.0,y-9.0,18.0,18.0,("todo-reminder",i)).on_hover_text(item.reminder_text().map(|due|format!("提醒：{due} · 点击修改")).unwrap_or_else(||"设置提醒与每周重复（可选）".into()));
                let reminder_clicked=reminder.clicked();
                let ink=if item.due.is_some()||reminder.hovered(){theme::BLUE}else{theme::GRAY};
                c.ring(342.0,y,4.7,1.1,ink);
                c.path(&[(342.0,y-2.5),(342.0,y),(344.3,y+1.0)],1.1,ink);
                if reminder_clicked {self.open_task_reminder(i);}
                let mut edit_this=false;
                r.context_menu(|ui|{
                    if ui.button("编辑与提醒…").clicked(){edit_this=true;ui.close_menu();}
                    if ui.button("取消提醒").clicked(){self.todos.items[i].due=None;self.todos.save();ui.close_menu();}
                });
                if edit_this {self.open_task_dialog(i);}
                if delete {self.todos.items.remove(i);self.todos.save();self.todo_first=self.todo_first.min(self.todos.items.len().saturating_sub(3));break;}
                if r.clicked() && !edit_this && !reminder_clicked && !today_item {
                    self.say(format!("{}的待办，今天不用做", crate::model::repeat_label(item.repeat)));
                } else if r.clicked() && !edit_this && !reminder_clicked {
                    let now = self.todos.items[i].done_for(&env.today);
                    self.todos.items[i].toggle(&env.today);
                    self.todos.save();
                    if !now {
                        self.todo.pop(5.0);
                        self.pet.react(crate::extras::PetState::Happy);
                    }
                }
                row += 1;
                shown += 1;
            }
            let rest = count.saturating_sub(shown + self.todo.editing.map(|_|1).unwrap_or(0));
            if rest > 0 {
                c.text(370.0, 120.0, &format!("共 {count} 项 · 滚轮查看"), size::TINY, W::Regular, theme::DIM, H::Right);
            }
        }

        if add.clicked() {
            self.todo_first=0;
            self.todo.begin_add();
        }
        if edit.clicked() && can_edit {
            if let Some(i) = self.todos.items.iter().position(|t| !t.done_for(&env.today)) {
                self.todo.begin_edit(i, &self.todos.items[i].text);
            }
        }
    }

}
