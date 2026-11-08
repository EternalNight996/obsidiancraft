//! Overlays: command palette, quick switcher, settings, about, rename, create.

use crate::icons::{self, Icon};
use crate::theme::{self, Palette};
use crate::{CreateKind, PaletteKind, ViewMode, VERSION};
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, Id, Rect, RichText, Sense, Stroke, Ui,
};

impl crate::App {
    pub(crate) fn overlays(&mut self, ui: &mut Ui, ctx: &egui::Context, full: Rect, now: f64) {
        if !self.settings_open
            && !self.show_about
            && self.palette.is_none()
            && self.rename.is_none()
            && self.create.is_none()
        {
            return;
        }
        // scrim swallows clicks aimed at what is behind
        ui.painter()
            .rect_filled(full, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 90));
        let scrim = ui.interact(full, Id::new("scrim-block"), Sense::click_and_drag());
        let clicked_outside = scrim.clicked();
        if self.settings_open {
            self.settings_modal(ui, ctx, full, now);
        }
        if self.show_about {
            self.about_modal(ui, full);
        }
        if self.palette.is_some() {
            self.palette_modal(ui, ctx, full, now);
        }
        if self.rename.is_some() {
            self.rename_modal(ui, full, now);
        }
        if self.create.is_some() {
            self.create_modal(ui, full, now);
        }
        if clicked_outside {
            self.palette = None;
        }
    }

    fn settings_modal(&mut self, ui: &mut Ui, ctx: &egui::Context, full: Rect, _now: f64) {
        let pal = self.pal;
        let rect = Rect::from_center_size(
            full.center(),
            vec2(760.0, 540.0).min(full.size() - vec2(40.0, 40.0)),
        );
        modal_frame(ui, rect, pal, "settings", |ui| {
            let header = Rect::from_min_size(rect.min, vec2(rect.width(), 52.0));
            ui.painter().rect_filled(header, 0.0, pal.bg_secondary);
            ui.painter().line_segment(
                [header.left_bottom(), header.right_bottom()],
                Stroke::new(1.0, pal.border),
            );
            ui.painter().text(
                pos2(header.left() + 20.0, header.center().y),
                Align2::LEFT_CENTER,
                "设置",
                theme::font_bold(16.0),
                pal.text,
            );
            let close = Rect::from_center_size(
                pos2(header.right() - 26.0, header.center().y),
                vec2(26.0, 26.0),
            );
            let resp = ui.interact(close, Id::new("settings-close"), Sense::click());
            if resp.clicked() {
                self.settings_open = false;
            }
            icons::draw(
                &ui.painter().clone(),
                Rect::from_center_size(close.center(), vec2(14.0, 14.0)),
                Icon::X,
                pal.text_muted,
            );

            let nav = Rect::from_min_max(
                pos2(rect.left(), header.bottom()),
                pos2(rect.left() + 200.0, rect.bottom()),
            );
            ui.painter().rect_filled(nav, 0.0, pal.bg_secondary_alt);
            let items = ["外观", "编辑器", "窗口", "关于"];
            for (i, item) in items.iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(nav.left() + 8.0, nav.top() + 12.0 + i as f32 * 34.0),
                    vec2(nav.width() - 16.0, 30.0),
                );
                let active = self.settings_section == i;
                let resp = ui.interact(r, Id::new(("setnav", i)), Sense::click());
                if active {
                    ui.painter().rect_filled(r, 5.0, pal.accent_soft);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 5.0, pal.bg_hover);
                }
                ui.painter().text(
                    pos2(r.left() + 12.0, r.center().y),
                    Align2::LEFT_CENTER,
                    *item,
                    theme::font(13.0),
                    if active { pal.text } else { pal.text_muted },
                );
                if resp.clicked() {
                    self.settings_section = i;
                }
            }

            let body = Rect::from_min_max(
                pos2(nav.right() + 24.0, header.bottom() + 20.0),
                pos2(rect.right() - 24.0, rect.bottom() - 16.0),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(body)
                    .layout(egui::Layout::top_down(Align::LEFT)),
            );
            child.set_width(body.width());
            match self.settings_section {
                0 => {
                    child.label(
                        RichText::new("主题")
                            .font(theme::font_bold(14.0))
                            .color(pal.text),
                    );
                    child.add_space(8.0);
                    let dark = self.dark;
                    let y = child.cursor().top();
                    for (i, (label, value)) in [("深色", true), ("浅色", false)].iter().enumerate() {
                        let r = Rect::from_min_size(
                            pos2(body.left() + i as f32 * 110.0, y),
                            vec2(100.0, 56.0),
                        );
                        let resp = child.interact(r, Id::new(("theme", label)), Sense::click());
                        let selected = dark == *value;
                        ui.painter().rect_filled(
                            r,
                            6.0,
                            if *value {
                                Color32::from_rgb(0x1C, 0x1C, 0x1C)
                            } else {
                                Color32::from_rgb(0xF6, 0xF6, 0xF6)
                            },
                        );
                        ui.painter().rect_stroke(
                            r,
                            6.0,
                            Stroke::new(
                                if selected { 2.0 } else { 1.0 },
                                if selected { pal.accent } else { pal.border },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            *label,
                            theme::font(13.0),
                            if *value {
                                Color32::from_rgb(0xDA, 0xDA, 0xDA)
                            } else {
                                Color32::from_rgb(0x22, 0x22, 0x22)
                            },
                        );
                        if resp.clicked() {
                            self.set_theme(*value, ctx);
                        }
                    }
                    child.add_space(66.0);
                    child.label(
                        RichText::new("强调色")
                            .font(theme::font_bold(14.0))
                            .color(pal.text),
                    );
                    child.add_space(8.0);
                    let hues = [258.0f32, 220.0, 160.0, 30.0, 340.0, 0.0];
                    let y = child.cursor().top();
                    for (i, h) in hues.iter().enumerate() {
                        let r = Rect::from_min_size(
                            pos2(body.left() + i as f32 * 40.0, y),
                            vec2(30.0, 30.0),
                        );
                        let c = theme::hsl(*h, 0.88, 0.66);
                        ui.painter().circle_filled(r.center(), 13.0, c);
                        if (*h - self.accent_h).abs() < 0.5 {
                            ui.painter()
                                .circle_stroke(r.center(), 15.0, Stroke::new(2.0, pal.text));
                        }
                        let resp = child.interact(r, Id::new(("hue", i)), Sense::click());
                        if resp.clicked() {
                            self.accent_h = *h;
                            self.pal = Palette::for_dark(self.dark).with_accent_hue(*h);
                            theme::apply_style(ctx, &self.pal);
                            self.save_settings();
                        }
                    }
                    child.add_space(42.0);
                    child.label(
                        RichText::new("正文字号")
                            .font(theme::font_bold(14.0))
                            .color(pal.text),
                    );
                    child.add_space(6.0);
                    let mut size = self.base_size;
                    let resp = child.add(
                        egui::Slider::new(&mut size, 12.0..=24.0)
                            .suffix(" px")
                            .step_by(1.0),
                    );
                    if resp.changed() {
                        self.base_size = size;
                        self.save_settings();
                    }
                }
                1 => {
                    child.label(
                        RichText::new("编辑器")
                            .font(theme::font_bold(14.0))
                            .color(pal.text),
                    );
                    child.add_space(8.0);
                    for line in [
                        "编辑模式使用 Markdown 源码，阅读视图会渲染格式。",
                        "笔记在停止输入后自动保存。",
                        "快捷键：Ctrl+N 新建，Ctrl+O 快速切换，Ctrl+P 命令面板，Ctrl+E 切换视图。",
                    ] {
                        child.label(
                            RichText::new(line)
                                .font(theme::font(13.0))
                                .color(pal.text_muted),
                        );
                        child.add_space(8.0);
                    }
                }
                2 => {
                    child.label(
                        RichText::new("窗口边框")
                            .font(theme::font_bold(14.0))
                            .color(pal.text),
                    );
                    child.add_space(8.0);
                    let native = self.native_frame;
                    let y = child.cursor().top();
                    for (i, (label, value)) in
                        [("Obsidian 自绘", false), ("系统标题栏", true)].iter().enumerate()
                    {
                        let r = Rect::from_min_size(
                            pos2(body.left() + i as f32 * 130.0, y),
                            vec2(120.0, 32.0),
                        );
                        let resp = child.interact(r, Id::new(("frame", label)), Sense::click());
                        let selected = native == *value;
                        ui.painter().rect_filled(
                            r,
                            5.0,
                            if selected {
                                pal.accent_soft
                            } else if resp.hovered() {
                                pal.bg_hover
                            } else {
                                pal.bg_secondary_alt
                            },
                        );
                        if selected {
                            ui.painter().rect_stroke(
                                r,
                                5.0,
                                Stroke::new(1.0, pal.accent),
                                egui::StrokeKind::Inside,
                            );
                        }
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            *label,
                            theme::font(13.0),
                            if selected { pal.text } else { pal.text_muted },
                        );
                        if resp.clicked() && !selected {
                            self.toggle_frame(ctx);
                        }
                    }
                    child.add_space(46.0);
                    child.label(
                        RichText::new("显示缩放")
                            .font(theme::font_bold(14.0))
                            .color(pal.text),
                    );
                    child.add_space(8.0);
                    let y = child.cursor().top();
                    for (i, (label, delta)) in
                        [("缩小", -0.5f32), ("实际大小", 0.0), ("放大", 0.5)].iter().enumerate()
                    {
                        let r = Rect::from_min_size(
                            pos2(body.left() + i as f32 * 92.0, y),
                            vec2(84.0, 30.0),
                        );
                        let resp = child.interact(r, Id::new(("zoombtn", i)), Sense::click());
                        ui.painter().rect_filled(
                            r,
                            5.0,
                            if resp.hovered() {
                                pal.bg_hover
                            } else {
                                pal.bg_secondary_alt
                            },
                        );
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            *label,
                            theme::font(13.0),
                            pal.text_muted,
                        );
                        if resp.clicked() {
                            if *delta == 0.0 {
                                self.set_zoom(ctx, 0.0);
                            } else {
                                self.change_zoom(ctx, *delta);
                            }
                        }
                    }
                    child.add_space(40.0);
                    let pct = (1.2f32.powf(self.zoom_level) * 100.0).round() as i32;
                    child.label(
                        RichText::new(format!("当前缩放：{} 级 · {}%", self.zoom_level, pct))
                            .font(theme::font(12.0))
                            .color(pal.text_faint),
                    );
                }
                _ => {
                    child.label(
                        RichText::new("Obsidian 复刻版")
                            .font(theme::font_bold(16.0))
                            .color(pal.text),
                    );
                    child.add_space(8.0);
                    child.label(
                        RichText::new(format!("版本 {VERSION} · Rust + egui"))
                            .font(theme::font(13.0))
                            .color(pal.text_muted),
                    );
                    child.add_space(12.0);
                    child.label(
                        RichText::new(
                            "本地 Markdown 笔记库，支持双链、反向链接、关系图谱、搜索与命令面板。",
                        )
                        .font(theme::font(13.0))
                        .color(pal.text_muted),
                    );
                }
            }
        });
    }

    fn about_modal(&mut self, ui: &mut Ui, full: Rect) {
        let pal = self.pal;
        let rect = Rect::from_center_size(full.center(), vec2(520.0, 320.0));
        modal_frame(ui, rect, pal, "about", |ui| {
            ui.painter().text(
                pos2(rect.center().x, rect.top() + 48.0),
                Align2::CENTER_CENTER,
                "Obsidian",
                theme::font_bold(24.0),
                pal.text,
            );
            ui.painter().text(
                pos2(rect.center().x, rect.top() + 80.0),
                Align2::CENTER_CENTER,
                format!("复刻版 {VERSION}"),
                theme::font(13.0),
                pal.text_muted,
            );
            let lines = [
                "一个本地优先的 Markdown 笔记库。",
                "双向链接 · 关系图谱 · 全文搜索 · 命令面板",
                "你的笔记以纯文本文件保存在自己的文件夹里。",
            ];
            for (i, line) in lines.iter().enumerate() {
                ui.painter().text(
                    pos2(rect.center().x, rect.top() + 130.0 + i as f32 * 24.0),
                    Align2::CENTER_CENTER,
                    *line,
                    theme::font(13.0),
                    pal.text_muted,
                );
            }
            let btn =
                Rect::from_center_size(pos2(rect.center().x, rect.bottom() - 50.0), vec2(110.0, 32.0));
            let resp = ui.interact(btn, Id::new("about-ok"), Sense::click());
            ui.painter().rect_filled(
                btn,
                6.0,
                if resp.hovered() {
                    pal.accent_hover
                } else {
                    pal.accent
                },
            );
            ui.painter().text(
                btn.center(),
                Align2::CENTER_CENTER,
                "好的",
                theme::font(13.0),
                Color32::WHITE,
            );
            if resp.clicked() {
                self.show_about = false;
            }
        });
    }

    fn palette_modal(&mut self, ui: &mut Ui, ctx: &egui::Context, full: Rect, now: f64) {
        let pal = self.pal;
        let width = 640.0f32.min(full.width() - 40.0);
        let top = full.top() + full.height() * 0.16;
        let items: Vec<(String, String, Option<usize>)> =
            match self.palette.as_ref().map(|p| p.kind) {
                Some(PaletteKind::Files) => self
                    .vault
                    .notes
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (n.title.clone(), n.rel.clone(), Some(i)))
                    .collect(),
                _ => self
                    .commands()
                    .into_iter()
                    .map(|(n, s)| (n.to_string(), s.to_string(), None))
                    .collect(),
            };
        let query = self
            .palette
            .as_ref()
            .map(|p| p.query.to_lowercase())
            .unwrap_or_default();
        let filtered: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, (name, path, _))| {
                if query.is_empty() {
                    return true;
                }
                name.to_lowercase().contains(&query) || path.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        let max_rows = 8usize;
        let list_h = (filtered.len().min(max_rows) as f32 * 34.0).max(0.0);
        let rect = Rect::from_min_size(
            pos2(full.center().x - width * 0.5, top),
            vec2(width, 52.0 + list_h + 12.0),
        );
        let mut run: Option<(String, Option<usize>)> = None;
        let mut close = false;
        let mut new_selection: Option<usize> = None;

        modal_frame(ui, rect, pal, "palette", |ui| {
            let field = Rect::from_min_size(rect.min, vec2(rect.width(), 52.0));
            ui.painter().rect_filled(field, 0.0, pal.bg_secondary_alt);
            ui.painter().line_segment(
                [field.left_bottom(), field.right_bottom()],
                Stroke::new(1.0, pal.border),
            );
            if let Some(state) = &mut self.palette {
                let edit = egui::TextEdit::singleline(&mut state.query)
                    .id(Id::new("palette-input"))
                    .hint_text(if state.kind == PaletteKind::Files {
                        "输入笔记名称…"
                    } else {
                        "输入命令…"
                    })
                    .font(theme::font(15.0))
                    .background_color(Color32::TRANSPARENT)
                    .desired_width(field.width() - 32.0)
                    .margin(egui::Margin::ZERO)
                    .text_color(pal.text);
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(field.shrink2(vec2(16.0, 14.0)))
                        .layout(egui::Layout::left_to_right(Align::Center)),
                );
                child.add(edit);
                if !state.focus {
                    child.memory_mut(|m| m.request_focus(Id::new("palette-input")));
                    state.focus = true;
                }
            }
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_max(
                        pos2(rect.left() + 6.0, field.bottom() + 6.0),
                        pos2(rect.right() - 6.0, rect.bottom() - 6.0),
                    ))
                    .layout(egui::Layout::top_down(Align::LEFT)),
            );
            child.set_clip_rect(rect);
            let selected = self.palette.as_ref().map(|p| p.selected).unwrap_or(0);
            for (row, item_idx) in filtered.iter().take(max_rows).enumerate() {
                let (name, sub, note_idx) = &items[*item_idx];
                let r = Rect::from_min_size(
                    pos2(rect.left() + 8.0, field.bottom() + 6.0 + row as f32 * 34.0),
                    vec2(rect.width() - 16.0, 32.0),
                );
                let resp = child.interact(r, Id::new(("prow", row)), Sense::click());
                if row == selected {
                    child.painter().rect_filled(r, 5.0, pal.accent_soft);
                } else if resp.hovered() {
                    child.painter().rect_filled(r, 5.0, pal.bg_hover);
                }
                let icon = if note_idx.is_some() {
                    Icon::FileText
                } else {
                    Icon::Terminal
                };
                icons::draw(
                    &child.painter().clone(),
                    Rect::from_center_size(pos2(r.left() + 16.0, r.center().y), vec2(14.0, 14.0)),
                    icon,
                    pal.text_muted,
                );
                child.painter().text(
                    pos2(r.left() + 32.0, r.center().y),
                    Align2::LEFT_CENTER,
                    theme::truncate(name, 40),
                    theme::font(13.0),
                    pal.text,
                );
                if !sub.is_empty() {
                    child.painter().text(
                        pos2(r.right() - 10.0, r.center().y),
                        Align2::RIGHT_CENTER,
                        theme::truncate(sub, 30),
                        theme::font(11.0),
                        pal.text_faint,
                    );
                }
                if resp.clicked() {
                    run = Some((name.clone(), *note_idx));
                    close = true;
                }
                if resp.hovered() {
                    new_selection = Some(row);
                }
            }
        });

        if let Some(sel) = new_selection {
            if let Some(p) = &mut self.palette {
                p.selected = sel;
            }
        }
        let mut move_by: i32 = 0;
        let mut enter = false;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowDown) {
                move_by = 1;
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                move_by = -1;
            }
            if i.key_pressed(egui::Key::Enter) {
                enter = true;
            }
            if i.key_pressed(egui::Key::Escape) {
                close = true;
                run = None;
            }
        });
        if move_by != 0 {
            if let Some(p) = &mut self.palette {
                let max = filtered.len().saturating_sub(1);
                p.selected = (p.selected as i32 + move_by).clamp(0, max as i32) as usize;
            }
        }
        if enter {
            let sel = self.palette.as_ref().map(|p| p.selected).unwrap_or(0);
            if let Some(item_idx) = filtered.get(sel) {
                let (name, _, note_idx) = &items[*item_idx];
                run = Some((name.clone(), *note_idx));
                close = true;
            }
        }
        if let Some((name, note_idx)) = run {
            match note_idx {
                Some(idx) => self.open_note(idx, false),
                None => self.run_command(&name, ctx, now),
            }
            close = true;
        }
        if close {
            self.palette = None;
        }
    }

    fn rename_modal(&mut self, ui: &mut Ui, full: Rect, now: f64) {
        let pal = self.pal;
        let rect = Rect::from_center_size(full.center(), vec2(460.0, 170.0));
        let mut submit = false;
        let mut cancel = false;
        modal_frame(ui, rect, pal, "rename", |ui| {
            ui.painter().text(
                pos2(rect.left() + 20.0, rect.top() + 26.0),
                Align2::LEFT_CENTER,
                "重命名文件",
                theme::font_bold(15.0),
                pal.text,
            );
            if let Some(state) = &mut self.rename {
                let field = Rect::from_min_size(
                    pos2(rect.left() + 20.0, rect.top() + 52.0),
                    vec2(rect.width() - 40.0, 34.0),
                );
                ui.painter().rect_filled(field, 5.0, pal.bg_secondary_alt);
                ui.painter().rect_stroke(
                    field,
                    5.0,
                    Stroke::new(1.0, pal.border_strong),
                    egui::StrokeKind::Inside,
                );
                let edit = egui::TextEdit::singleline(&mut state.value)
                    .id(Id::new("rename-input"))
                    .font(theme::font(14.0))
                    .background_color(Color32::TRANSPARENT)
                    .desired_width(field.width() - 20.0)
                    .margin(egui::Margin::ZERO)
                    .text_color(pal.text);
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(field.shrink2(vec2(10.0, 8.0)))
                        .layout(egui::Layout::left_to_right(Align::Center)),
                );
                let resp = child.add(edit);
                if !state.focus {
                    child.memory_mut(|m| m.request_focus(Id::new("rename-input")));
                    state.focus = true;
                }
                if resp.lost_focus() && child.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submit = true;
                }
            }
            let ok = Rect::from_min_size(
                pos2(rect.right() - 190.0, rect.bottom() - 48.0),
                vec2(80.0, 30.0),
            );
            let cancel_r = Rect::from_min_size(
                pos2(rect.right() - 100.0, rect.bottom() - 48.0),
                vec2(80.0, 30.0),
            );
            for (r, label, primary) in [(cancel_r, "取消", false), (ok, "确定", true)] {
                let resp = ui.interact(r, Id::new(("rbtn", label)), Sense::click());
                let bg = if primary {
                    if resp.hovered() {
                        pal.accent_hover
                    } else {
                        pal.accent
                    }
                } else if resp.hovered() {
                    pal.bg_hover
                } else {
                    pal.bg_secondary_alt
                };
                ui.painter().rect_filled(r, 5.0, bg);
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    label,
                    theme::font(13.0),
                    if primary {
                        Color32::WHITE
                    } else {
                        pal.text_muted
                    },
                );
                if resp.clicked() {
                    if primary {
                        submit = true;
                    } else {
                        cancel = true;
                    }
                }
            }
        });
        if submit {
            if let Some(state) = self.rename.take() {
                let value = state.value.trim().to_string();
                if !value.is_empty() {
                    let old_rel = self
                        .vault
                        .notes
                        .get(state.idx)
                        .map(|n| n.rel.clone())
                        .unwrap_or_default();
                    match self.vault.rename_note(state.idx, &value) {
                        Ok(new_rel) => {
                            for pane in &mut self.panes {
                                for tab in &mut pane.tabs {
                                    if tab.path() == Some(old_rel.as_str()) {
                                        tab.kind = crate::TabKind::Note(new_rel.clone());
                                        tab.edit = None;
                                    }
                                }
                            }
                            self.title_dirty = true;
                            self.toast("已重命名", now);
                        }
                        Err(e) => self.toast(format!("重命名失败：{e}"), now),
                    }
                }
            }
        } else if cancel {
            self.rename = None;
        }
    }

    fn create_modal(&mut self, ui: &mut Ui, full: Rect, now: f64) {
        let pal = self.pal;
        let rect = Rect::from_center_size(full.center(), vec2(460.0, 170.0));
        let mut submit = false;
        let mut cancel = false;
        modal_frame(ui, rect, pal, "create", |ui| {
            let kind = self
                .create
                .as_ref()
                .map(|c| c.kind)
                .unwrap_or(CreateKind::Note);
            ui.painter().text(
                pos2(rect.left() + 20.0, rect.top() + 26.0),
                Align2::LEFT_CENTER,
                if kind == CreateKind::Note {
                    "新建笔记"
                } else {
                    "新建文件夹"
                },
                theme::font_bold(15.0),
                pal.text,
            );
            if let Some(state) = &mut self.create {
                let field = Rect::from_min_size(
                    pos2(rect.left() + 20.0, rect.top() + 52.0),
                    vec2(rect.width() - 40.0, 34.0),
                );
                ui.painter().rect_filled(field, 5.0, pal.bg_secondary_alt);
                ui.painter().rect_stroke(
                    field,
                    5.0,
                    Stroke::new(1.0, pal.border_strong),
                    egui::StrokeKind::Inside,
                );
                let edit = egui::TextEdit::singleline(&mut state.value)
                    .id(Id::new("create-input"))
                    .hint_text(if kind == CreateKind::Note {
                        "笔记名称"
                    } else {
                        "文件夹名称"
                    })
                    .font(theme::font(14.0))
                    .background_color(Color32::TRANSPARENT)
                    .desired_width(field.width() - 20.0)
                    .margin(egui::Margin::ZERO)
                    .text_color(pal.text);
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(field.shrink2(vec2(10.0, 8.0)))
                        .layout(egui::Layout::left_to_right(Align::Center)),
                );
                let resp = child.add(edit);
                if !state.focus {
                    child.memory_mut(|m| m.request_focus(Id::new("create-input")));
                    state.focus = true;
                }
                if resp.lost_focus() && child.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submit = true;
                }
            }
            let ok = Rect::from_min_size(
                pos2(rect.right() - 190.0, rect.bottom() - 48.0),
                vec2(80.0, 30.0),
            );
            let cancel_r = Rect::from_min_size(
                pos2(rect.right() - 100.0, rect.bottom() - 48.0),
                vec2(80.0, 30.0),
            );
            for (r, label, primary) in [(cancel_r, "取消", false), (ok, "确定", true)] {
                let resp = ui.interact(r, Id::new(("cbtn", label)), Sense::click());
                let bg = if primary {
                    if resp.hovered() {
                        pal.accent_hover
                    } else {
                        pal.accent
                    }
                } else if resp.hovered() {
                    pal.bg_hover
                } else {
                    pal.bg_secondary_alt
                };
                ui.painter().rect_filled(r, 5.0, bg);
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    label,
                    theme::font(13.0),
                    if primary {
                        Color32::WHITE
                    } else {
                        pal.text_muted
                    },
                );
                if resp.clicked() {
                    if primary {
                        submit = true;
                    } else {
                        cancel = true;
                    }
                }
            }
        });
        if submit {
            if let Some(state) = self.create.take() {
                let name = state.value.trim().to_string();
                if !name.is_empty() {
                    match state.kind {
                        CreateKind::Note => match self.vault.create_note(&state.folder, &name) {
                            Ok(rel) => {
                                self.vault.refresh();
                                self.open_rel_in_mode(&rel, ViewMode::Edit);
                                self.toast("已创建笔记", now);
                            }
                            Err(e) => self.toast(format!("创建失败：{e}"), now),
                        },
                        CreateKind::Folder => match self.vault.create_folder(&state.folder, &name) {
                            Ok(_) => self.toast("已创建文件夹", now),
                            Err(e) => self.toast(format!("创建失败：{e}"), now),
                        },
                    }
                }
            }
        } else if cancel {
            self.create = None;
        }
    }
}

fn modal_frame(
    ui: &mut Ui,
    rect: Rect,
    pal: Palette,
    name: &str,
    contents: impl FnOnce(&mut Ui),
) {
    let _ = name;
    ui.painter().rect_filled(rect, 10.0, pal.bg_secondary);
    ui.painter().rect_stroke(
        rect,
        10.0,
        Stroke::new(1.0, pal.border_strong),
        egui::StrokeKind::Inside,
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(Align::LEFT)),
    );
    child.set_clip_rect(rect);
    contents(&mut child);
}
