//! Everything that draws the main window: header, ribbons, sidebars, panes.

use crate::icons::{self, Icon};
use crate::markdown;
use crate::theme::{self, HEADER_H, RIBBON_W, STATUS_H};
use crate::vault;
use crate::{
    Action, CreateKind, CreateState, EditBuf, LeftTab, Pane, RightTab, Tab, TabKind, ViewMode,
};
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CursorIcon, Id, Pos2, Rect, RichText, Sense, Stroke,
    Ui,
};

impl crate::App {
    // -------------------------------------------------------------- top bar

    pub(crate) fn top_bar(&mut self, ui: &mut Ui, full: Rect, now: f64) {
        let pal = self.pal;
        let bar = Rect::from_min_size(full.min, vec2(full.width(), HEADER_H));
        ui.painter().rect_filled(bar, 0.0, pal.bg_secondary);
        ui.painter().line_segment(
            [bar.left_bottom(), bar.right_bottom()],
            Stroke::new(1.0, pal.border),
        );

        // Dragging the empty header moves the window; registered first so buttons win.
        let drag = ui.interact(bar, Id::new("title-drag"), Sense::click_and_drag());
        if !self.native_frame {
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let max = ui.ctx().input(|i| i.viewport().maximized).unwrap_or(false);
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
        }

        // left sidebar tabs, starting after the ribbon
        let mut x = RIBBON_W;
        if self.left_open {
            let tab_area = Rect::from_min_size(
                pos2(RIBBON_W, bar.top()),
                vec2(self.left_width, HEADER_H),
            );
            self.sidebar_header(ui, tab_area, now);
            x = RIBBON_W + self.left_width;
        }
        let body_right =
            bar.right() - RIBBON_W - if self.right_open { self.right_width } else { 0.0 };
        if self.right_open {
            let area = Rect::from_min_max(
                pos2(body_right, bar.top()),
                pos2(bar.right() - RIBBON_W, bar.bottom()),
            );
            self.right_header(ui, area);
        }
        let main_area = Rect::from_min_max(pos2(x, bar.top()), pos2(body_right, bar.bottom()));
        self.pane_headers(ui, main_area, now);

        // window controls sit above the header content (hidden when the OS draws the frame)
        if !self.native_frame {
            let ctrl_w = 46.0;
            let close_r =
                Rect::from_min_size(pos2(bar.right() - ctrl_w, bar.top()), vec2(ctrl_w, HEADER_H));
            let max_r = Rect::from_min_size(
                pos2(bar.right() - ctrl_w * 2.0, bar.top()),
                vec2(ctrl_w, HEADER_H),
            );
            let min_r = Rect::from_min_size(
                pos2(bar.right() - ctrl_w * 3.0, bar.top()),
                vec2(ctrl_w, HEADER_H),
            );
            for (r, icon, id) in [
                (min_r, Icon::Minus, "win-min"),
                (max_r, Icon::Square, "win-max"),
                (close_r, Icon::X, "win-close"),
            ] {
                let resp = ui.interact(r, Id::new(id), Sense::click());
                let hovered = resp.hovered();
                if hovered {
                    let bg = if id == "win-close" {
                        Color32::from_rgb(0xC4, 0x2B, 0x1C)
                    } else {
                        pal.bg_hover
                    };
                    ui.painter().rect_filled(r, 0.0, bg);
                }
                let color = if hovered && id == "win-close" {
                    Color32::WHITE
                } else {
                    pal.text_muted
                };
                icons::draw(
                    &ui.painter().clone(),
                    Rect::from_center_size(r.center(), vec2(14.0, 14.0)),
                    icon,
                    color,
                );
                if resp.clicked() {
                    match id {
                        "win-min" => {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true))
                        }
                        "win-max" => {
                            let max = ui.ctx().input(|i| i.viewport().maximized).unwrap_or(false);
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                        }
                        _ => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                    }
                }
            }
        }
    }

    fn sidebar_header(&mut self, ui: &mut Ui, area: Rect, _now: f64) {
        let pal = self.pal;
        // The original hides sidebar tab titles (--sidebar-tab-text-display: none):
        // only the icon shows, the title is the tooltip.
        let tabs = [
            (LeftTab::Files, "文件列表"),
            (LeftTab::Search, "搜索"),
            (LeftTab::Bookmarks, "书签"),
        ];
        let tab_w = 40.0;
        for (i, (tab, title)) in tabs.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(area.left() + 4.0 + tab_w * i as f32, area.top()),
                vec2(tab_w, area.height()),
            );
            let active = *tab == self.left_tab;
            let resp = ui.interact(r, Id::new(("left-tab", i)), Sense::click());
            if active {
                ui.painter().rect_filled(
                    r,
                    egui::CornerRadius {
                        nw: 4,
                        ne: 4,
                        sw: 0,
                        se: 0,
                    },
                    pal.bg_primary,
                );
            } else if resp.hovered() {
                ui.painter().rect_filled(r, 4.0, pal.bg_hover);
            }
            icons::draw(
                &ui.painter().clone(),
                Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
                tab.icon(),
                if active { pal.text_muted } else { pal.text_faint },
            );
            let resp = resp.on_hover_text(*title);
            if resp.clicked() {
                self.left_tab = *tab;
                self.left_open = true;
                self.save_settings();
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
        }
    }

    fn right_header(&mut self, ui: &mut Ui, area: Rect) {
        let pal = self.pal;
        let tab_w = (area.width() / 6.0).min(48.0);
        for (i, tab) in RightTab::ALL.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(area.left() + tab_w * i as f32, area.top()),
                vec2(tab_w, area.height()),
            );
            let active = *tab == self.right_tab;
            let resp = ui.interact(r, Id::new(("rtab", i)), Sense::click());
            if active {
                ui.painter().rect_filled(
                    r,
                    egui::CornerRadius {
                        nw: 4,
                        ne: 4,
                        sw: 0,
                        se: 0,
                    },
                    pal.bg_primary,
                );
            } else if resp.hovered() {
                ui.painter().rect_filled(r, 0.0, pal.bg_hover);
            }
            icons::draw(
                &ui.painter().clone(),
                Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
                tab.icon(),
                if active { pal.text_muted } else { pal.text_faint },
            );
            if resp.on_hover_text(tab.title()).clicked() {
                self.right_tab = *tab;
                self.right_open = true;
                self.save_settings();
            }
        }
    }

    fn pane_headers(&mut self, ui: &mut Ui, area: Rect, now: f64) {
        let pal = self.pal;
        let n = self.panes.len().max(1);
        let pane_w = area.width() / n as f32;
        let mut action = None;
        let snapshot: Vec<(Vec<(String, bool)>, usize)> = self
            .panes
            .iter()
            .map(|p| {
                (
                    p.tabs
                        .iter()
                        .map(|t| (self.tab_title(t), t.kind == TabKind::Graph))
                        .collect(),
                    p.active,
                )
            })
            .collect();
        let vault_root = self.vault.root.clone();
        let vault_name = self.vault_name();
        for (pi, (tabs_snap, active_tab)) in snapshot.iter().enumerate() {
            let pane_rect =
                Rect::from_min_size(pos2(area.left() + pane_w * pi as f32, area.top()), vec2(pane_w, area.height()));
            if pi > 0 {
                ui.painter().line_segment(
                    [pane_rect.left_top(), pane_rect.left_bottom()],
                    Stroke::new(1.0, pal.border),
                );
            }
            let mut x = pane_rect.left();
            let max_x = pane_rect.right() - 36.0;
            for (ti, (title, is_graph)) in tabs_snap.iter().enumerate() {
                let galley = ui.ctx().fonts_mut(|f| {
                    f.layout_no_wrap(title.clone(), theme::font(13.0), pal.text_muted)
                });
                let tw = (galley.size().x + 46.0).clamp(104.0, 220.0);
                if x + tw > max_x && ti > 0 {
                    break;
                }
                let r = Rect::from_min_size(pos2(x, pane_rect.top()), vec2(tw, pane_rect.height()));
                let active = ti == *active_tab;
                let resp = ui.interact(r, Id::new(("tab", pi, ti)), Sense::click());
                if active {
                    ui.painter().rect_filled(
                        r,
                        egui::CornerRadius {
                            nw: 4,
                            ne: 4,
                            sw: 0,
                            se: 0,
                        },
                        pal.bg_primary,
                    );
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 0.0, pal.bg_hover);
                }
                if ti > 0 && !active {
                    ui.painter().line_segment(
                        [
                            pos2(r.left(), r.top() + 10.0),
                            pos2(r.left(), r.bottom() - 10.0),
                        ],
                        Stroke::new(1.0, pal.border),
                    );
                }
                let icon = if *is_graph {
                    Icon::Graph
                } else {
                    Icon::FileText
                };
                icons::draw(
                    &ui.painter().clone(),
                    Rect::from_center_size(pos2(r.left() + 16.0, r.center().y), vec2(14.0, 14.0)),
                    icon,
                    if active { pal.text_muted } else { pal.text_faint },
                );
                let clip = Rect::from_min_max(
                    pos2(r.left() + 28.0, r.top()),
                    pos2(r.right() - 26.0, r.bottom()),
                );
                let tab_text_color = if active {
                    if pi == self.active_pane {
                        pal.text
                    } else {
                        pal.text_muted
                    }
                } else {
                    pal.text_faint
                };
                let painter = ui.painter().with_clip_rect(clip);
                painter.text(
                    pos2(r.left() + 28.0, r.center().y),
                    Align2::LEFT_CENTER,
                    theme::truncate(title, 26),
                    theme::font(13.0),
                    tab_text_color,
                );
                // Register the close button on every pass. egui resolves hover and
                // clicks from the previous pass's widget rects, so a button that only
                // exists while its parent is hovered keeps stealing the hover from that
                // parent and then vanishing: the X flickers and its clicks are lost.
                let close =
                    Rect::from_center_size(pos2(r.right() - 16.0, r.center().y), vec2(18.0, 18.0));
                let cr = ui.interact(close, Id::new(("tab-close", pi, ti)), Sense::click());
                if resp.hovered() || cr.hovered() || active {
                    ui.painter().rect_filled(
                        close,
                        4.0,
                        if cr.hovered() {
                            pal.bg_hover
                        } else {
                            pal.bg_primary
                        },
                    );
                    icons::draw(
                        &ui.painter().clone(),
                        Rect::from_center_size(close.center(), vec2(12.0, 12.0)),
                        Icon::X,
                        if cr.hovered() { pal.text } else { pal.text_muted },
                    );
                }
                if cr.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if cr.clicked() {
                    action = Some(Action::CloseTab(pi, ti));
                }
                let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                let tab_title = title.clone();
                let tab_path = self
                    .panes
                    .get(pi)
                    .and_then(|p| p.tabs.get(ti))
                    .and_then(|t| t.path())
                    .map(|p| p.to_string());
                resp.context_menu(|ui| {
                    if ui.button("关闭").clicked() {
                        action = Some(Action::CloseTab(pi, ti));
                        ui.close();
                    }
                    if tabs_snap.len() > 1 && ui.button("关闭其他标签页").clicked() {
                        action = Some(Action::CloseOtherTabs(pi, ti));
                        ui.close();
                    }
                    if ti + 1 < tabs_snap.len() && ui.button("关闭右侧所有标签页").clicked() {
                        action = Some(Action::CloseTabsRight(pi, ti));
                        ui.close();
                    }
                    if tabs_snap.len() > 1 && ui.button("全部关闭").clicked() {
                        action = Some(Action::CloseAllTabs(pi));
                        ui.close();
                    }
                    if let Some(rel) = &tab_path {
                        ui.separator();
                        let mut copied: Vec<Action> = Vec::new();
                        copy_path_menu(
                            ui,
                            rel,
                            &vault_root,
                            &vault_name,
                            true,
                            &mut copied,
                        );
                        if let Some(a) = copied.pop() {
                            action = Some(a);
                        }
                        if ui.button("复制链接").clicked() {
                            ui.ctx().copy_text(format!("[[{tab_title}]]"));
                            action = Some(Action::Toast("已复制链接".into()));
                            ui.close();
                        }
                    }
                });
                if resp.clicked() {
                    self.active_pane = pi;
                    if let Some(p) = self.panes.get_mut(pi) {
                        p.active = ti;
                    }
                    self.title_dirty = true;
                }
                if resp.middle_clicked() {
                    action = Some(Action::CloseTab(pi, ti));
                }
                x += tw;
            }
            let plus =
                Rect::from_center_size(pos2(x + 18.0, pane_rect.center().y), vec2(28.0, 28.0));
            if x + 36.0 < pane_rect.right() - 8.0 {
                let resp = ui.interact(plus, Id::new(("newtab", pi)), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(plus, 4.0, pal.bg_hover);
                }
                icons::draw(
                    &ui.painter().clone(),
                    Rect::from_center_size(plus.center(), vec2(14.0, 14.0)),
                    Icon::Plus,
                    pal.text_muted,
                );
                if resp.clicked() {
                    self.active_pane = pi;
                    self.create = Some(CreateState {
                        kind: CreateKind::Note,
                        folder: String::new(),
                        value: String::new(),
                        focus: false,
                    });
                }
            }
        }
        if let Some(a) = action {
            self.open_action(a, now);
        }
    }

    pub(crate) fn tab_title(&self, tab: &Tab) -> String {
        match &tab.kind {
            TabKind::Graph => "关系图谱".to_string(),
            TabKind::Note(rel) => self
                .vault
                .find_by_rel(rel)
                .map(|i| self.vault.notes[i].title.clone())
                .unwrap_or_else(|| {
                    std::path::Path::new(rel)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "新标签页".to_string())
                }),
        }
    }

    // -------------------------------------------------------------- body

    pub(crate) fn body(&mut self, ui: &mut Ui, full: Rect, now: f64) {
        let pal = self.pal;
        let body = Rect::from_min_max(
            pos2(full.left(), full.top() + HEADER_H),
            pos2(full.right(), full.bottom() - STATUS_H),
        );
        let left_ribbon = Rect::from_min_size(body.min, vec2(RIBBON_W, body.height()));
        let right_ribbon = Rect::from_min_size(
            pos2(body.right() - RIBBON_W, body.top()),
            vec2(RIBBON_W, body.height()),
        );
        ui.painter().rect_filled(left_ribbon, 0.0, pal.bg_secondary);
        ui.painter().rect_filled(right_ribbon, 0.0, pal.bg_secondary);
        ui.painter().line_segment(
            [left_ribbon.right_top(), left_ribbon.right_bottom()],
            Stroke::new(1.0, pal.border),
        );
        ui.painter().line_segment(
            [right_ribbon.left_top(), right_ribbon.left_bottom()],
            Stroke::new(1.0, pal.border),
        );
        self.left_ribbon(ui, left_ribbon, now);
        self.right_ribbon(ui, right_ribbon, now);

        // At the original's 200x150 minimum the sidebars cannot fit; drop them rather
        // than squeezing the main area to a negative width.
        let mut left_w = if self.left_open {
            self.left_width.clamp(180.0, (body.width() * 0.5).max(220.0))
        } else {
            0.0
        };
        let mut right_w = if self.right_open {
            self.right_width.clamp(200.0, (body.width() * 0.5).max(240.0))
        } else {
            0.0
        };
        let chrome = RIBBON_W * 2.0;
        if body.width() - chrome - left_w - right_w < 160.0 {
            right_w = 0.0;
            if body.width() - chrome - left_w < 160.0 {
                left_w = 0.0;
            }
        }
        let mut main_left = left_ribbon.right();
        if left_w > 0.0 {
            let r = Rect::from_min_size(pos2(main_left, body.top()), vec2(left_w, body.height()));
            ui.painter().rect_filled(r, 0.0, pal.bg_secondary);
            ui.painter().line_segment(
                [r.right_top(), r.right_bottom()],
                Stroke::new(1.0, pal.border),
            );
            self.left_sidebar(ui, r, now);
            self.resize_handle(ui, pos2(r.right(), body.top()), body.height(), true);
            main_left = r.right();
        }
        let mut main_right = right_ribbon.left();
        if right_w > 0.0 {
            let r = Rect::from_min_size(pos2(main_right - right_w, body.top()), vec2(right_w, body.height()));
            ui.painter().rect_filled(r, 0.0, pal.bg_secondary);
            ui.painter().line_segment(
                [r.left_top(), r.left_bottom()],
                Stroke::new(1.0, pal.border),
            );
            self.right_sidebar(ui, r, now);
            self.resize_handle(ui, pos2(r.left(), body.top()), body.height(), false);
            main_right = r.left();
        }
        let main = Rect::from_min_max(pos2(main_left, body.top()), pos2(main_right, body.bottom()));
        self.main_area(ui, main, now);
    }

    fn resize_handle(&mut self, ui: &mut Ui, at: Pos2, height: f32, left_side: bool) {
        let r = Rect::from_min_size(pos2(at.x - 3.0, at.y), vec2(6.0, height));
        let resp = ui.interact(r, Id::new(("resize", at.x as i32)), Sense::hover());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        if resp.dragged() {
            let dx = resp.drag_delta().x;
            if left_side {
                self.left_width = (self.left_width + dx).clamp(180.0, 600.0);
            } else {
                self.right_width = (self.right_width - dx).clamp(200.0, 600.0);
            }
        }
    }

    fn left_ribbon(&mut self, ui: &mut Ui, area: Rect, now: f64) {
        let pal = self.pal;
        let mut y = area.top() + 4.0;
        let toggle = Rect::from_min_size(pos2(area.left() + 6.0, y), vec2(32.0, 32.0));
        let resp = self.icon_btn(
            ui,
            toggle,
            Icon::PanelLeft,
            pal.text_muted,
            self.left_open,
            "ribbon-toggle-left",
        );
        if resp.on_hover_text("收起/展开侧边栏").clicked() {
            self.left_open = !self.left_open;
            self.save_settings();
        }
        y += 40.0;
        let items: [(Icon, &str, &str); 6] = [
            (Icon::Folder, "文件列表", "files"),
            (Icon::Search, "搜索", "search"),
            (Icon::Graph, "关系图谱", "graph"),
            (Icon::Canvas, "白板", "canvas"),
            (Icon::Terminal, "命令面板", "commands"),
            (Icon::Bookmark, "书签", "bookmarks"),
        ];
        for (icon, tip, id) in items {
            let r = Rect::from_min_size(pos2(area.left() + 6.0, y), vec2(32.0, 32.0));
            let active = (id == "files" && self.left_open && self.left_tab == LeftTab::Files)
                || (id == "search" && self.left_open && self.left_tab == LeftTab::Search)
                || (id == "bookmarks" && self.left_open && self.left_tab == LeftTab::Bookmarks);
            let resp = self.icon_btn(ui, r, icon, pal.text_muted, active, ("ribbon", id));
            let resp = resp.on_hover_text(tip);
            if resp.clicked() {
                match id {
                    "files" => {
                        self.left_open = !self.left_open || self.left_tab != LeftTab::Files;
                        self.left_tab = LeftTab::Files;
                    }
                    "search" => {
                        self.left_open = !self.left_open || self.left_tab != LeftTab::Search;
                        self.left_tab = LeftTab::Search;
                    }
                    "bookmarks" => {
                        self.left_open = !self.left_open || self.left_tab != LeftTab::Bookmarks;
                        self.left_tab = LeftTab::Bookmarks;
                    }
                    "graph" => self.open_action(Action::OpenGraph, now),
                    "canvas" => self.toast("白板将在后续版本提供", now),
                    "commands" => {
                        self.palette = Some(crate::PaletteState {
                            kind: crate::PaletteKind::Commands,
                            query: String::new(),
                            selected: 0,
                            focus: false,
                        })
                    }
                    _ => {}
                }
                self.save_settings();
            }
            y += 36.0;
        }
        let mut by = area.bottom() - 40.0;
        for (icon, tip, id) in [(Icon::Help, "帮助", "help"), (Icon::Settings, "设置", "settings")] {
            let r = Rect::from_min_size(pos2(area.left() + 6.0, by), vec2(32.0, 32.0));
            let resp = self.icon_btn(ui, r, icon, pal.text_muted, false, ("ribbon-b", id));
            let resp = resp.on_hover_text(tip);
            if resp.clicked() {
                if id == "help" {
                    self.show_about = true;
                } else {
                    self.settings_open = true;
                }
            }
            by += 36.0;
        }
    }

    fn right_ribbon(&mut self, ui: &mut Ui, area: Rect, _now: f64) {
        let pal = self.pal;
        let mut y = area.top() + 4.0;
        let toggle = Rect::from_min_size(pos2(area.left() + 6.0, y), vec2(32.0, 32.0));
        let resp = self.icon_btn(
            ui,
            toggle,
            Icon::PanelRight,
            pal.text_muted,
            self.right_open,
            "ribbon-toggle-right",
        );
        if resp.on_hover_text("收起/展开右侧边栏").clicked() {
            self.right_open = !self.right_open;
            self.save_settings();
        }
        y += 40.0;
        for (i, tab) in RightTab::ALL.iter().enumerate() {
            let r = Rect::from_min_size(pos2(area.left() + 6.0, y), vec2(32.0, 32.0));
            let active = self.right_open && *tab == self.right_tab;
            let resp = self.icon_btn(ui, r, tab.icon(), pal.text_muted, active, ("rribbon", i));
            let resp = resp.on_hover_text(tab.title());
            if resp.clicked() {
                self.right_tab = *tab;
                self.right_open = true;
                self.save_settings();
            }
            y += 36.0;
        }
    }

    fn left_sidebar(&mut self, ui: &mut Ui, area: Rect, now: f64) {
        match self.left_tab {
            LeftTab::Files => self.file_explorer(ui, area, now),
            LeftTab::Search => self.search_panel(ui, area, now),
            LeftTab::Bookmarks => self.bookmarks_panel(ui, area, now),
        }
    }

    fn file_explorer(&mut self, ui: &mut Ui, area: Rect, now: f64) {
        let pal = self.pal;
        let profile_h = 42.0;
        let toolbar = Rect::from_min_size(pos2(area.left(), area.top()), vec2(area.width(), 38.0));
        ui.painter().rect_filled(toolbar, 0.0, pal.bg_secondary);
        let mut bx = toolbar.left() + 8.0;
        for (icon, tip, id) in [
            (Icon::Pen, "新建笔记", "new-note"),
            (Icon::FolderPlus, "新建文件夹", "new-folder"),
        ] {
            let r = Rect::from_min_size(pos2(bx, toolbar.center().y - 14.0), vec2(28.0, 28.0));
            let resp = self.icon_btn(ui, r, icon, pal.text_muted, false, ("fe", id));
            let resp = resp.on_hover_text(tip);
            if resp.clicked() {
                self.open_action(
                    if id == "new-note" {
                        Action::CreateNote(String::new())
                    } else {
                        Action::CreateFolder(String::new())
                    },
                    now,
                );
            }
            bx += 30.0;
        }
        let mut rx = toolbar.right() - 8.0 - 28.0;
        for (icon, tip, id) in [
            (Icon::Sort, "更改排序方式", "sort"),
            (Icon::Collapse, "折叠全部", "collapse"),
        ] {
            let r = Rect::from_min_size(pos2(rx, toolbar.center().y - 14.0), vec2(28.0, 28.0));
            let resp = self.icon_btn(ui, r, icon, pal.text_muted, false, ("fe2", id));
            let resp = resp.on_hover_text(tip);
            if resp.clicked() {
                if id == "collapse" {
                    self.vault.expanded.clear();
                } else {
                    self.toast("文件列表按名称排序", now);
                }
            }
            rx -= 30.0;
        }

        let list_rect = Rect::from_min_max(
            pos2(area.left(), toolbar.bottom()),
            pos2(area.right(), area.bottom() - profile_h),
        );
        ui.painter().rect_filled(list_rect, 0.0, pal.bg_secondary);
        let tree = self.vault.tree.clone();
        let mut actions: Vec<Action> = Vec::new();
        let active = self.active_note_idx();
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(list_rect)
                .layout(egui::Layout::top_down(Align::LEFT)),
        );
        child.set_clip_rect(list_rect);
        egui::ScrollArea::vertical()
            .id_salt("file-tree")
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.add_space(4.0);
                for e in &tree {
                    crate::ui::entry_row(ui, self, e, 0, active, &mut actions);
                }
                ui.add_space(8.0);
            });
        for a in actions {
            self.open_action(a, now);
        }

        let profile = Rect::from_min_size(
            pos2(area.left(), area.bottom() - profile_h),
            vec2(area.width(), profile_h),
        );
        ui.painter().rect_filled(profile, 0.0, pal.bg_secondary);
        ui.painter().line_segment(
            [profile.left_top(), profile.right_top()],
            Stroke::new(1.0, pal.border),
        );
        let name = self.vault_name();
        icons::draw(
            &ui.painter().clone(),
            Rect::from_center_size(pos2(profile.left() + 20.0, profile.center().y), vec2(16.0, 16.0)),
            Icon::Archive,
            pal.text_muted,
        );
        let galley = ui.ctx().fonts_mut(|f| {
            f.layout_no_wrap(theme::truncate(&name, 22), theme::font(13.0), pal.text_muted)
        });
        let name_w = galley.size().x;
        ui.painter().galley(
            pos2(profile.left() + 34.0, profile.center().y - galley.size().y * 0.5),
            galley,
            pal.text_muted,
        );
        icons::draw(
            &ui.painter().clone(),
            Rect::from_center_size(
                pos2(profile.left() + 40.0 + name_w, profile.center().y),
                vec2(12.0, 12.0),
            ),
            Icon::ChevronDown,
            pal.text_faint,
        );
        let gear = Rect::from_min_size(
            pos2(profile.right() - 36.0, profile.center().y - 14.0),
            vec2(28.0, 28.0),
        );
        let resp = self.icon_btn(ui, gear, Icon::Settings, pal.text_muted, false, "vault-gear");
        if resp.on_hover_text("设置").clicked() {
            self.settings_open = true;
        }
    }

    fn search_panel(&mut self, ui: &mut Ui, area: Rect, now: f64) {
        let pal = self.pal;
        let input_h = 42.0;
        let input_row = Rect::from_min_size(pos2(area.left(), area.top()), vec2(area.width(), input_h));
        ui.painter().rect_filled(input_row, 0.0, pal.bg_secondary);
        let field = Rect::from_min_size(
            pos2(input_row.left() + 8.0, input_row.top() + 6.0),
            vec2(input_row.width() - 16.0, 30.0),
        );
        ui.painter().rect_filled(field, 4.0, pal.bg_secondary_alt);
        icons::draw(
            &ui.painter().clone(),
            Rect::from_center_size(pos2(field.left() + 15.0, field.center().y), vec2(14.0, 14.0)),
            Icon::Search,
            pal.text_faint,
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    pos2(field.left() + 28.0, field.top()),
                    pos2(field.right() - 8.0, field.bottom()),
                ))
                .layout(egui::Layout::left_to_right(Align::Center)),
        );
        let edit = egui::TextEdit::singleline(&mut self.search_query)
            .id(Id::new("search-input"))
            .hint_text("搜索…")
            .font(theme::font(13.0))
            .background_color(Color32::TRANSPARENT)
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::ZERO)
            .text_color(pal.text);
        child.add(edit);

        let result_rect = Rect::from_min_max(
            pos2(area.left(), input_row.bottom()),
            pos2(area.right(), area.bottom()),
        );
        let hits = self.vault.search(&self.search_query.clone(), self.match_case);
        let mut actions = Vec::new();
        let mut list = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(result_rect)
                .layout(egui::Layout::top_down(Align::LEFT)),
        );
        list.set_clip_rect(result_rect);
        let query = self.search_query.trim().to_string();
        let match_case = self.match_case;
        egui::ScrollArea::vertical()
            .id_salt("search-results")
            .auto_shrink([false, false])
            .show(&mut list, |ui| {
                if query.is_empty() {
                    ui.add_space(24.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("输入关键词以搜索整个仓库")
                                .font(theme::font(13.0))
                                .color(pal.text_faint),
                        );
                    });
                    return;
                }
                if hits.is_empty() {
                    ui.add_space(24.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("没有找到结果")
                                .font(theme::font(13.0))
                                .color(pal.text_faint),
                        );
                    });
                    return;
                }
                ui.add_space(6.0);
                let mut last_note: Option<usize> = None;
                for hit in &hits {
                    let note = &self.vault.notes[hit.note];
                    if last_note != Some(hit.note) {
                        ui.add_space(4.0);
                        let (r, _) =
                            ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
                        ui.painter().text(
                            pos2(r.left() + 12.0, r.center().y),
                            Align2::LEFT_CENTER,
                            &note.title,
                            theme::font_bold(13.0),
                            pal.text,
                        );
                        last_note = Some(hit.note);
                    }
                    let (r, resp) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
                    if resp.hovered() {
                        ui.painter()
                            .rect_filled(r.shrink2(vec2(4.0, 0.0)), 4.0, pal.bg_hover);
                    }
                    let text = theme::truncate(hit.text.trim(), 60);
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap = egui::text::TextWrapping {
                        max_width: (r.width() - 24.0).max(40.0),
                        max_rows: 1,
                        break_anywhere: true,
                        ..Default::default()
                    };
                    if !query.is_empty() {
                        let hay = if match_case {
                            text.clone()
                        } else {
                            text.to_lowercase()
                        };
                        let needle = if match_case {
                            query.clone()
                        } else {
                            query.to_lowercase()
                        };
                        let mut pos = 0;
                        let base = egui::TextFormat {
                            font_id: theme::font(12.5),
                            color: pal.text_muted,
                            ..Default::default()
                        };
                        let mut found_any = false;
                        while let Some(found) = hay[pos..].find(&needle) {
                            let start = pos + found;
                            let end = start + needle.len();
                            found_any = true;
                            if start > pos {
                                job.append(&text[pos..start], 0.0, base.clone());
                            }
                            job.append(
                                &text[start..end],
                                0.0,
                                egui::TextFormat {
                                    font_id: theme::font(12.5),
                                    color: pal.text,
                                    background: pal.accent.gamma_multiply(0.35),
                                    ..Default::default()
                                },
                            );
                            pos = end;
                        }
                        if pos < text.len() {
                            job.append(&text[pos..], 0.0, base.clone());
                        }
                        if !found_any {
                            job.append(&text, 0.0, base);
                        }
                    } else {
                        job.append(
                            &text,
                            0.0,
                            egui::TextFormat {
                                font_id: theme::font(12.5),
                                color: pal.text_muted,
                                ..Default::default()
                            },
                        );
                    }
                    let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
                    ui.painter().galley(
                        pos2(r.left() + 12.0, r.center().y - galley.size().y * 0.5),
                        galley,
                        pal.text_muted,
                    );
                    if resp.clicked() {
                        actions.push(Action::Open(hit.note));
                    }
                    let hit_rel = note.rel.clone();
                    let hit_title = note.title.clone();
                    let root = self.vault.root.clone();
                    let vault_name = self.vault_name();
                    resp.context_menu(|ui| {
                        if ui.button("打开").clicked() {
                            actions.push(Action::Open(hit.note));
                            ui.close();
                        }
                        if ui.button("在新标签页中打开").clicked() {
                            actions.push(Action::OpenNewTab(hit.note));
                            ui.close();
                        }
                        ui.separator();
                        copy_path_menu(ui, &hit_rel, &root, &vault_name, true, &mut actions);
                        if ui.button("复制链接").clicked() {
                            ui.ctx().copy_text(format!("[[{hit_title}]]"));
                            actions.push(Action::Toast("已复制链接".into()));
                            ui.close();
                        }
                    });
                }
                ui.add_space(12.0);
            });
        for a in actions {
            self.open_action(a, now);
        }
    }

    fn bookmarks_panel(&mut self, ui: &mut Ui, area: Rect, now: f64) {
        let pal = self.pal;
        let bookmarks = self.bookmarks.clone();
        let mut actions = Vec::new();
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(egui::Layout::top_down(Align::LEFT)),
        );
        child.set_clip_rect(area);
        egui::ScrollArea::vertical()
            .id_salt("bookmarks")
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.add_space(16.0);
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
                ui.painter().text(
                    pos2(r.left() + 14.0, r.center().y),
                    Align2::LEFT_CENTER,
                    format!("书签 ({})", bookmarks.len()),
                    theme::font_bold(13.0),
                    pal.text,
                );
                ui.add_space(4.0);
                if bookmarks.is_empty() {
                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("还没有书签\n用命令面板添加当前笔记")
                                .font(theme::font(12.0))
                                .color(pal.text_faint),
                        );
                    });
                }
                for b in &bookmarks {
                    let Some(idx) = self.vault.find_by_rel(b) else {
                        continue;
                    };
                    let (r, resp) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
                    if resp.hovered() {
                        ui.painter()
                            .rect_filled(r.shrink2(vec2(4.0, 0.0)), 4.0, pal.bg_hover);
                    }
                    icons::draw(
                        &ui.painter().clone(),
                        Rect::from_center_size(
                            pos2(r.left() + 16.0, r.center().y),
                            vec2(13.0, 13.0),
                        ),
                        Icon::FileText,
                        pal.text_muted,
                    );
                    ui.painter().text(
                        pos2(r.left() + 30.0, r.center().y),
                        Align2::LEFT_CENTER,
                        &self.vault.notes[idx].title,
                        theme::font(13.0),
                        pal.text_muted,
                    );
                    if resp.clicked() {
                        actions.push(Action::Open(idx));
                    }
                    let bookmark_rel = b.clone();
                    let root = self.vault.root.clone();
                    let vault_name = self.vault_name();
                    resp.context_menu(|ui| {
                        if ui.button("打开").clicked() {
                            actions.push(Action::Open(idx));
                            ui.close();
                        }
                        if ui.button("在新标签页中打开").clicked() {
                            actions.push(Action::OpenNewTab(idx));
                            ui.close();
                        }
                        ui.separator();
                        copy_path_menu(ui, &bookmark_rel, &root, &vault_name, true, &mut actions);
                        if ui.button("复制链接").clicked() {
                            ui.ctx().copy_text(format!("[[{}]]", self.vault.notes[idx].title));
                            actions.push(Action::Toast("已复制链接".into()));
                            ui.close();
                        }
                        if ui.button("移除书签").clicked() {
                            actions.push(Action::ToggleBookmark(bookmark_rel.clone()));
                            ui.close();
                        }
                    });
                }
            });
        for a in actions {
            self.open_action(a, now);
        }
    }

    fn right_sidebar(&mut self, ui: &mut Ui, area: Rect, now: f64) {
        let pal = self.pal;
        let header = Rect::from_min_size(pos2(area.left(), area.top()), vec2(area.width(), 38.0));
        ui.painter().rect_filled(header, 0.0, pal.bg_secondary);
        ui.painter().text(
            pos2(header.left() + 14.0, header.center().y),
            Align2::LEFT_CENTER,
            self.right_tab.short(),
            theme::font_bold(13.0),
            pal.text,
        );
        let r = Rect::from_center_size(
            pos2(header.right() - 20.0, header.center().y),
            vec2(16.0, 16.0),
        );
        let resp = ui.interact(r, Id::new("right-collapse"), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        icons::draw(&ui.painter().clone(), r, Icon::PanelRight, pal.text_muted);
        if resp.clicked() {
            self.right_open = false;
            self.save_settings();
        }

        let content = Rect::from_min_max(
            pos2(area.left(), header.bottom()),
            pos2(area.right(), area.bottom()),
        );
        let active = self.active_note_idx();
        let mut actions = Vec::new();
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(content)
                .layout(egui::Layout::top_down(Align::LEFT)),
        );
        child.set_clip_rect(content);
        egui::ScrollArea::vertical()
            .id_salt(("right-pane", self.right_tab.title()))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.add_space(8.0);
                match self.right_tab {
                    RightTab::Backlinks => {
                        if let Some(idx) = active {
                            let backlinks = self.vault.backlinks(idx);
                            Self::panel_heading(
                                ui,
                                &pal,
                                &format!("已链接的笔记 ({})", backlinks.len()),
                            );
                            if backlinks.is_empty() {
                                Self::panel_empty(ui, &pal, "没有笔记链接到这里");
                            }
                            for b in &backlinks {
                                let title = self.vault.notes[b.note].title.clone();
                                let (r, resp) = ui
                                    .allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                                if resp.hovered() {
                                    ui.painter().rect_filled(
                                        r.shrink2(vec2(6.0, 0.0)),
                                        4.0,
                                        pal.bg_hover,
                                    );
                                }
                                icons::draw(
                                    &ui.painter().clone(),
                                    Rect::from_center_size(
                                        pos2(r.left() + 18.0, r.center().y),
                                        vec2(12.0, 12.0),
                                    ),
                                    Icon::FileText,
                                    pal.text_muted,
                                );
                                ui.painter().text(
                                    pos2(r.left() + 32.0, r.center().y),
                                    Align2::LEFT_CENTER,
                                    &title,
                                    theme::font(13.0),
                                    pal.text_muted,
                                );
                                if resp.clicked() {
                                    actions.push(Action::Open(b.note));
                                }
                                let bl_note = b.note;
                                let bl_title = title.clone();
                                let bl_rel = self.vault.notes[bl_note].rel.clone();
                                let root = self.vault.root.clone();
                                let vault_name = self.vault_name();
                                resp.context_menu(|ui| {
                                    if ui.button("打开").clicked() {
                                        actions.push(Action::Open(bl_note));
                                        ui.close();
                                    }
                                    if ui.button("在新标签页中打开").clicked() {
                                        actions.push(Action::OpenNewTab(bl_note));
                                        ui.close();
                                    }
                                    ui.separator();
                                    copy_path_menu(
                                        ui,
                                        &bl_rel,
                                        &root,
                                        &vault_name,
                                        true,
                                        &mut actions,
                                    );
                                    if ui.button("复制链接").clicked() {
                                        ui.ctx().copy_text(format!("[[{bl_title}]]"));
                                        actions.push(Action::Toast("已复制链接".into()));
                                        ui.close();
                                    }
                                });
                                if !b.text.is_empty() {
                                    let (r2, _) = ui.allocate_exact_size(
                                        vec2(ui.available_width() - 20.0, 20.0),
                                        Sense::hover(),
                                    );
                                    ui.painter().text(
                                        pos2(r2.left() + 32.0, r2.center().y),
                                        Align2::LEFT_CENTER,
                                        theme::truncate(&b.text, 40),
                                        theme::font(11.5),
                                        pal.text_faint,
                                    );
                                }
                            }
                        } else {
                            Self::panel_empty(ui, &pal, "打开一篇笔记以查看反向链接");
                        }
                    }
                    RightTab::Outgoing => {
                        if let Some(idx) = active {
                            let links = self.vault.links_in(idx);
                            Self::panel_heading(ui, &pal, &format!("出链 ({})", links.len()));
                            if links.is_empty() {
                                Self::panel_empty(ui, &pal, "这篇笔记没有链接");
                            }
                            for l in &links {
                                let (r, resp) = ui
                                    .allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
                                if resp.hovered() {
                                    ui.painter().rect_filled(
                                        r.shrink2(vec2(6.0, 0.0)),
                                        4.0,
                                        pal.bg_hover,
                                    );
                                }
                                icons::draw(
                                    &ui.painter().clone(),
                                    Rect::from_center_size(
                                        pos2(r.left() + 18.0, r.center().y),
                                        vec2(12.0, 12.0),
                                    ),
                                    Icon::Link,
                                    if l.resolved.is_some() {
                                        pal.text_muted
                                    } else {
                                        pal.text_faint
                                    },
                                );
                                let label = l
                                    .resolved
                                    .map(|i| self.vault.notes[i].title.clone())
                                    .unwrap_or_else(|| l.target.clone());
                                ui.painter().text(
                                    pos2(r.left() + 32.0, r.center().y),
                                    Align2::LEFT_CENTER,
                                    theme::truncate(&label, 26),
                                    theme::font(13.0),
                                    if l.resolved.is_some() {
                                        pal.text_muted
                                    } else {
                                        pal.text_faint
                                    },
                                );
                                if resp.clicked() {
                                    match l.resolved {
                                        Some(i) => actions.push(Action::Open(i)),
                                        None => actions.push(Action::CreateNote(String::new())),
                                    }
                                }
                                if let Some(i) = l.resolved {
                                    let out_title = self.vault.notes[i].title.clone();
                                    let out_rel = self.vault.notes[i].rel.clone();
                                    let root = self.vault.root.clone();
                                    let vault_name = self.vault_name();
                                    resp.context_menu(|ui| {
                                        if ui.button("打开").clicked() {
                                            actions.push(Action::Open(i));
                                            ui.close();
                                        }
                                        if ui.button("在新标签页中打开").clicked() {
                                            actions.push(Action::OpenNewTab(i));
                                            ui.close();
                                        }
                                        ui.separator();
                                        copy_path_menu(
                                            ui,
                                            &out_rel,
                                            &root,
                                            &vault_name,
                                            true,
                                            &mut actions,
                                        );
                                        if ui.button("复制链接").clicked() {
                                            ui.ctx().copy_text(format!("[[{out_title}]]"));
                                            actions.push(Action::Toast("已复制链接".into()));
                                            ui.close();
                                        }
                                    });
                                }
                            }
                        }
                    }
                    RightTab::Tags => {
                        let tags = self.vault.all_tags();
                        Self::panel_heading(ui, &pal, &format!("标签 ({})", tags.len()));
                        if tags.is_empty() {
                            Self::panel_empty(ui, &pal, "仓库里还没有标签");
                        }
                        for t in &tags {
                            let (r, resp) = ui
                                .allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
                            if resp.hovered() {
                                ui.painter().rect_filled(r.shrink2(vec2(6.0, 0.0)), 4.0, pal.bg_hover);
                            }
                            icons::draw(
                                &ui.painter().clone(),
                                Rect::from_center_size(
                                    pos2(r.left() + 18.0, r.center().y),
                                    vec2(12.0, 12.0),
                                ),
                                Icon::Hash,
                                pal.accent,
                            );
                            ui.painter().text(
                                pos2(r.left() + 32.0, r.center().y),
                                Align2::LEFT_CENTER,
                                &t.tag,
                                theme::font(13.0),
                                pal.text_muted,
                            );
                            ui.painter().text(
                                pos2(r.right() - 16.0, r.center().y),
                                Align2::RIGHT_CENTER,
                                t.count.to_string(),
                                theme::font(11.5),
                                pal.text_faint,
                            );
                            if resp.clicked() {
                                self.search_query = t.tag.clone();
                                self.left_tab = LeftTab::Search;
                                self.left_open = true;
                            }
                        }
                    }
                    RightTab::Outline => {
                        if let Some(idx) = active {
                            let headings = self.vault.headings(idx);
                            Self::panel_heading(ui, &pal, &format!("大纲 ({})", headings.len()));
                            if headings.is_empty() {
                                Self::panel_empty(ui, &pal, "这篇笔记没有标题");
                            }
                            for (hi, h) in headings.iter().enumerate() {
                                let indent = 14.0 + (h.level as f32 - 1.0) * 12.0;
                                let (r, resp) =
                                    ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                                if resp.hovered() {
                                    ui.painter().rect_filled(
                                        r.shrink2(vec2(6.0, 0.0)),
                                        4.0,
                                        pal.bg_hover,
                                    );
                                }
                                ui.painter().text(
                                    pos2(r.left() + indent, r.center().y),
                                    Align2::LEFT_CENTER,
                                    theme::truncate(&h.text, 30),
                                    theme::font(if h.level <= 2 { 13.0 } else { 12.0 }),
                                    if resp.hovered() { pal.text } else { pal.text_muted },
                                );
                                if resp.clicked() {
                                    let pane = self.active_pane;
                                    let tab = self.panes[pane].active;
                                    actions.push(Action::ScrollTo(pane, tab, hi));
                                }
                            }
                        }
                    }
                    RightTab::Properties => {
                        if let Some(idx) = active {
                            let props = self.vault.frontmatter(idx);
                            Self::panel_heading(ui, &pal, "笔记属性");
                            if props.is_empty() {
                                Self::panel_empty(ui, &pal, "这篇笔记没有属性\n在笔记顶部用 --- 添加");
                            }
                            for (k, v) in &props {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
                                ui.painter().text(
                                    pos2(r.left() + 14.0, r.center().y),
                                    Align2::LEFT_CENTER,
                                    k,
                                    theme::font(12.5),
                                    pal.text_faint,
                                );
                                ui.painter().text(
                                    pos2(r.left() + 120.0, r.center().y),
                                    Align2::LEFT_CENTER,
                                    theme::truncate(v, 24),
                                    theme::font(12.5),
                                    pal.text_muted,
                                );
                            }
                        }
                    }
                }
                ui.add_space(16.0);
            });
        for a in actions {
            self.open_action(a, now);
        }
    }

    fn panel_heading(ui: &mut Ui, pal: &crate::theme::Palette, text: &str) {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
        ui.painter().text(
            pos2(r.left() + 14.0, r.center().y),
            Align2::LEFT_CENTER,
            text,
            theme::font(12.5),
            pal.text_faint,
        );
    }

    fn panel_empty(ui: &mut Ui, pal: &crate::theme::Palette, text: &str) {
        ui.add_space(10.0);
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            text,
            theme::font(12.0),
            pal.text_faint,
        );
    }

    // -------------------------------------------------------------- main area

    fn main_area(&mut self, ui: &mut Ui, area: Rect, now: f64) {
        let pal = self.pal;
        if self.panes.is_empty() {
            self.panes.push(Pane::with_note(""));
        }
        let n = self.panes.len();
        let pane_w = area.width() / n as f32;
        for pi in 0..n {
            let r = Rect::from_min_size(
                pos2(area.left() + pane_w * pi as f32, area.top()),
                vec2(pane_w, area.height()),
            );
            if pi > 0 {
                ui.painter().line_segment(
                    [r.left_top(), r.left_bottom()],
                    Stroke::new(1.0, pal.border),
                );
            }
            self.pane_view(ui, r, pi, now);
        }
    }

    fn pane_view(&mut self, ui: &mut Ui, area: Rect, pane_idx: usize, now: f64) {
        // Obsidian makes the leaf under the pointer the active one as soon as a
        // mouse button goes down. Without this, Ctrl+E and the pane actions
        // targeted whichever pane happened to be active, so clicking in the
        // other reading pane seemed to make its note uneditable.
        let pressed_here = ui.ctx().input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .hover_pos()
                    .map(|p| area.contains(p))
                    .unwrap_or(false)
        });
        if pressed_here && self.active_pane != pane_idx {
            self.active_pane = pane_idx;
            self.title_dirty = true;
        }
        let pal = self.pal;
        let Some(pane) = self.panes.get(pane_idx) else {
            return;
        };
        let Some(tab) = pane.tabs.get(pane.active) else {
            self.empty_pane(ui, area);
            return;
        };
        let tab_index = pane.active;
        let mode = tab.mode;
        match tab.kind.clone() {
            TabKind::Graph => {
                // Obsidian lays a 40px view header (breadcrumb) under the tab
                // strip; the graph canvas and its controls start below it.
                let header = Rect::from_min_size(area.min, vec2(area.width(), HEADER_H));
                ui.painter().rect_filled(header, 0.0, pal.bg_primary);
                for (i, icon) in [(0.0, Icon::ChevronLeft), (1.0, Icon::ChevronRight)] {
                    let r = Rect::from_min_size(
                        pos2(header.left() + 10.0 + i * 26.0, header.center().y - 12.0),
                        vec2(24.0, 24.0),
                    );
                    icons::draw(
                        &ui.painter().clone(),
                        Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
                        icon,
                        pal.text_faint,
                    );
                }
                ui.painter().text(
                    pos2(header.left() + 76.0, header.center().y),
                    Align2::LEFT_CENTER,
                    "关系图谱",
                    theme::font(13.0),
                    pal.text_muted,
                );
                let canvas = Rect::from_min_max(
                    pos2(area.left(), header.bottom()),
                    area.max,
                );
                let active = self.active_note_idx();
                if let Some(act) = self.graph.show(ui, canvas, &self.vault, &pal, active) {
                    match act {
                        crate::graph::GraphAction::Open(idx) => self.open_note(idx, false),
                        crate::graph::GraphAction::OpenNewTab(idx) => self.open_note(idx, true),
                        crate::graph::GraphAction::Copy(text) => {
                            ui.ctx().copy_text(text);
                            self.toast("已复制", now);
                        }
                    }
                }
            }
            TabKind::Note(rel) => {
                let Some(idx) = self.vault.find_by_rel(&rel) else {
                    self.empty_pane(ui, area);
                    return;
                };
                // Every note gets the original's 40px view header under the tab
                // strip. It holds the reading/editing toggle (the book/pencil
                // button), which is the visible way into the editor.
                let header = Rect::from_min_size(area.min, vec2(area.width(), HEADER_H));
                self.note_view_header(ui, header, pane_idx, tab_index, idx, mode, now);
                let content = Rect::from_min_max(
                    pos2(area.left(), header.bottom()),
                    area.max,
                );
                match mode {
                    ViewMode::Read => self.read_view(ui, content, pane_idx, idx, now),
                    ViewMode::Edit => self.edit_view(ui, content, pane_idx, idx, now),
                }
            }
        }
    }

    /// the original's `view-header` for a markdown leaf: back/forward, the file
    /// breadcrumb, then the "reading/edit" toggle and the "..." pane menu.
    /// The toggle is `MarkdownView.updateButtons()`: a pencil while reading and
    /// a book while editing, with those exact tooltips.
    fn note_view_header(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        pane_idx: usize,
        tab_index: usize,
        idx: usize,
        mode: ViewMode,
        now: f64,
    ) {
        let pal = self.pal;
        let (rel, title) = {
            let Some(note) = self.vault.notes.get(idx) else {
                return;
            };
            (note.rel.clone(), note.title.clone())
        };
        ui.painter().rect_filled(rect, 0.0, pal.bg_primary);
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, pal.border),
        );

        for (i, icon) in [(0usize, Icon::ChevronLeft), (1, Icon::ChevronRight)] {
            let r = Rect::from_center_size(
                pos2(rect.left() + 20.0 + i as f32 * 26.0, rect.center().y),
                vec2(22.0, 22.0),
            );
            icons::draw(
                &ui.painter().clone(),
                Rect::from_center_size(r.center(), vec2(15.0, 15.0)),
                icon,
                pal.text_faint,
            );
        }

        let more = Rect::from_center_size(
            pos2(rect.right() - 20.0, rect.center().y),
            vec2(28.0, 28.0),
        );
        let mode_btn = Rect::from_center_size(
            pos2(rect.right() - 48.0, rect.center().y),
            vec2(28.0, 28.0),
        );

        let (icon, tip) = if mode == ViewMode::Read {
            (Icon::Pen, "阅读视图\n点击此处切换至编辑视图")
        } else {
            (Icon::BookOpen, "编辑视图\n点击此处切换至阅读视图")
        };
        let resp = self.icon_btn(
            ui,
            mode_btn,
            icon,
            pal.text_muted,
            false,
            ("view-mode", pane_idx, tab_index),
        );
        let resp = resp.on_hover_text(tip);
        if resp.clicked() {
            self.toggle_mode(pane_idx, tab_index, now);
        }

        let mut crumb = String::new();
        if let Some((dir, _)) = rel.rsplit_once('/') {
            crumb.push_str(dir);
            crumb.push_str(" / ");
        }
        crumb.push_str(&title);
        let clip = Rect::from_min_max(
            pos2(rect.left() + 58.0, rect.top()),
            pos2(mode_btn.left() - 6.0, rect.bottom()),
        );
        let galley = ui.painter().layout_no_wrap(
            theme::truncate(&crumb, 72),
            theme::font(13.0),
            pal.text_muted,
        );
        ui.painter().with_clip_rect(clip).galley(
            pos2(clip.left(), rect.center().y - galley.size().y * 0.5),
            galley,
            pal.text_muted,
        );

        let more_resp = self.icon_btn(
            ui,
            more,
            Icon::More,
            pal.text_muted,
            false,
            ("view-more", pane_idx, tab_index),
        );
        let more_resp = more_resp.on_hover_text("更多选项");
        let bookmarked = self.bookmarks.iter().any(|b| b == &rel);
        let root = self.vault.root.clone();
        let vault_name = self.vault_name();
        let rel_menu = rel.clone();
        let mut actions: Vec<Action> = Vec::new();
        egui::Popup::menu(&more_resp).show(|ui| {
            if ui
                .button(if mode == ViewMode::Read {
                    "编辑此笔记"
                } else {
                    "阅读此笔记"
                })
                .clicked()
            {
                actions.push(Action::ToggleMode(pane_idx, tab_index));
                ui.close();
            }
            if ui
                .button(if bookmarked { "移除书签" } else { "添加书签" })
                .clicked()
            {
                actions.push(Action::ToggleBookmark(rel_menu.clone()));
                ui.close();
            }
            ui.separator();
            copy_path_menu(ui, &rel_menu, &root, &vault_name, true, &mut actions);
            if ui.button("重命名").clicked() {
                actions.push(Action::Rename(idx));
                ui.close();
            }
            if ui.button("删除文件").clicked() {
                actions.push(Action::Delete(idx));
                ui.close();
            }
        });
        for a in actions {
            self.open_action(a, now);
        }
    }

    fn empty_pane(&mut self, ui: &mut Ui, area: Rect) {
        let pal = self.pal;
        ui.painter().rect_filled(area, 0.0, pal.bg_primary);
        ui.painter().text(
            area.center() - vec2(0.0, 24.0),
            Align2::CENTER_CENTER,
            "没有打开的笔记",
            theme::font(16.0),
            pal.text_faint,
        );
        let btn = Rect::from_center_size(area.center() + vec2(0.0, 16.0), vec2(120.0, 32.0));
        let resp = ui.interact(btn, Id::new("empty-new"), Sense::click());
        ui.painter().rect_filled(
            btn,
            6.0,
            if resp.hovered() {
                pal.bg_hover
            } else {
                pal.bg_secondary
            },
        );
        ui.painter().text(
            btn.center(),
            Align2::CENTER_CENTER,
            "新建笔记",
            theme::font(13.0),
            pal.text_muted,
        );
        if resp.clicked() {
            self.create = Some(CreateState {
                kind: CreateKind::Note,
                folder: String::new(),
                value: String::new(),
                focus: false,
            });
        }
    }

    fn content_width(&self, avail: f32) -> f32 {
        (avail - theme::FILE_MARGIN * 2.0)
            .min(theme::FILE_LINE_WIDTH)
            .max(140.0)
    }

    fn read_view(&mut self, ui: &mut Ui, area: Rect, pane_idx: usize, idx: usize, now: f64) {
        let pal = self.pal;
        let tab_index = self.panes[pane_idx].active;
        ui.painter().rect_filled(area, 0.0, pal.bg_primary);
        let note_title = self.vault.notes[idx].title.clone();
        let content = self.vault.notes[idx].content.clone();
        let blocks = markdown::parse(&content);
        let scroll_target = match self.scroll_to {
            Some((p, t, h)) if p == pane_idx && t == tab_index => Some(h),
            _ => None,
        };
        let mut renderer = markdown::Renderer::new(&pal, &self.vault, self.base_size);
        renderer.scroll_to_heading = scroll_target;
        let base = self.base_size;
        let content_w = self.content_width(area.width());

        // Right-click anywhere in the reading view for the note's own actions.
        let mut menu_actions: Vec<Action> = Vec::new();
        let read_ctx = ui.interact(
            area,
            Id::new(("read-ctx", pane_idx, tab_index)),
            Sense::click(),
        );
        let rel = self.vault.notes[idx].rel.clone();
        let bookmarked = self.bookmarks.iter().any(|b| b == &rel);
        let root_for_menu = self.vault.root.clone();
        let vault_name_for_menu = self.vault_name();
        read_ctx.context_menu(|ui| {
            if ui.button("编辑此笔记").clicked() {
                menu_actions.push(Action::ToggleMode(pane_idx, tab_index));
                ui.close();
            }
            if ui
                .button(if bookmarked {
                    "移除书签"
                } else {
                    "添加书签"
                })
                .clicked()
            {
                menu_actions.push(Action::ToggleBookmark(rel.clone()));
                ui.close();
            }
            ui.separator();
            if ui.button("复制笔记正文").clicked() {
                ui.ctx().copy_text(content.clone());
                menu_actions.push(Action::Toast("已复制正文".into()));
                ui.close();
            }
            copy_path_menu(
                ui,
                &rel,
                &root_for_menu,
                &vault_name_for_menu,
                true,
                &mut menu_actions,
            );
            if ui.button("复制链接").clicked() {
                ui.ctx().copy_text(format!("[[{note_title}]]"));
                menu_actions.push(Action::Toast("已复制链接".into()));
                ui.close();
            }
            ui.separator();
            if ui.button("重命名").clicked() {
                menu_actions.push(Action::Rename(idx));
                ui.close();
            }
            if ui.button("删除文件").clicked() {
                menu_actions.push(Action::Delete(idx));
                ui.close();
            }
        });

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(egui::Layout::top_down(Align::LEFT)),
        );
        child.set_clip_rect(area);
        egui::ScrollArea::vertical()
            .id_salt(("read", pane_idx, tab_index))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.add_space(theme::FILE_MARGIN);
                let side = ((ui.available_width() - content_w) * 0.5).max(theme::FILE_MARGIN);
                ui.horizontal(|ui| {
                    ui.add_space(side);
                    ui.vertical(|ui| {
                        ui.set_width(content_w);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&note_title)
                                    .font(theme::font_bold(base * 1.618))
                                    .color(pal.text),
                            )
                            .selectable(false),
                        );
                        ui.add_space(base * 0.5);
                        renderer.render(ui, &blocks);
                        ui.add_space(theme::FILE_MARGIN);
                    });
                });
            });
        let clicked = renderer.clicked.take();
        let task_toggle = renderer.toggle_task.take();
        if scroll_target.is_some() {
            self.scroll_to = None;
        }
        if let Some(target) = clicked {
            self.open_link(&target, now);
        }
        if let Some(line) = task_toggle {
            self.toggle_task(idx, line, now);
        }
        for action in menu_actions {
            self.open_action(action, now);
        }
    }

    fn toggle_task(&mut self, idx: usize, line: usize, now: f64) {
        let Some(note) = self.vault.notes.get(idx) else {
            return;
        };
        let mut lines: Vec<String> = note.content.split('\n').map(|s| s.to_string()).collect();
        if line >= lines.len() {
            return;
        }
        let l = lines[line].clone();
        let updated = if let Some(p) = l.find("[ ] ") {
            format!("{}[x] {}", &l[..p], &l[p + 4..])
        } else if let Some(p) = l.find("[x] ").or_else(|| l.find("[X] ")) {
            format!("{}[ ] {}", &l[..p], &l[p + 4..])
        } else {
            l
        };
        lines[line] = updated;
        let text = lines.join("\n");
        self.save_note_now(idx, text);
        self.toast("已更新任务", now);
    }

    fn edit_view(&mut self, ui: &mut Ui, area: Rect, pane_idx: usize, idx: usize, now: f64) {
        let pal = self.pal;
        ui.painter().rect_filled(area, 0.0, pal.bg_primary);
        let tab_index = self.panes[pane_idx].active;
        // New notes (and Ctrl+E) put the caret in the editor immediately, so
        // the person can type without having to find and click the text first.
        if self.focus_editor == Some((pane_idx, tab_index)) {
            ui.memory_mut(|m| m.request_focus(Id::new(("editor", pane_idx, tab_index))));
            self.focus_editor = None;
        }
        let content = self.vault.notes[idx].content.clone();
        let Some(tab) = self
            .panes
            .get_mut(pane_idx)
            .and_then(|p| p.tabs.get_mut(tab_index))
        else {
            return;
        };
        if tab.edit.is_none() {
            tab.edit = Some(EditBuf {
                text: content,
                last_edit: now,
            });
        }
        let mut buf = tab.edit.as_ref().map(|e| e.text.clone()).unwrap_or_default();
        let mut last_edit = tab.edit.as_ref().map(|e| e.last_edit).unwrap_or(now);
        let mut changed = false;
        let base = self.base_size;
        let content_w = self.content_width(area.width());

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(egui::Layout::top_down(Align::LEFT)),
        );
        child.set_clip_rect(area);
        egui::ScrollArea::vertical()
            .id_salt(("edit", pane_idx, tab_index))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.add_space(theme::FILE_MARGIN);
                let side =
                    ((ui.available_width() - content_w) * 0.5).max(theme::FILE_MARGIN);
                ui.horizontal(|ui| {
                    ui.add_space(side);
                    ui.vertical(|ui| {
                        ui.set_width(content_w);
                        let edit = egui::TextEdit::multiline(&mut buf)
                            .id(Id::new(("editor", pane_idx, tab_index)))
                            .font(theme::font(base))
                            .text_color(pal.text)
                            .background_color(Color32::TRANSPARENT)
                            .desired_width(content_w)
                            .desired_rows(24)
                            .margin(egui::Margin::ZERO)
                            .lock_focus(true);
                        let resp = ui.add(edit);
                        if resp.changed() {
                            changed = true;
                        }
                        ui.add_space(theme::FILE_MARGIN);
                    });
                });
            });

        if changed {
            last_edit = now;
        }
        if let Some(tab) = self
            .panes
            .get_mut(pane_idx)
            .and_then(|p| p.tabs.get_mut(tab_index))
        {
            tab.edit = Some(EditBuf {
                text: buf.clone(),
                last_edit,
            });
        }
        if changed || (now - last_edit > 0.6 && self.vault.notes[idx].content != buf) {
            self.pending_save = Some((idx, buf, now + 0.45));
        }
    }

    // -------------------------------------------------------------- window edges

    pub(crate) fn window_resize(&mut self, ui: &mut Ui, full: Rect) {
        use egui::{CursorIcon, ResizeDirection};
        let t = 4.0;
        let c = 12.0;
        let edges: [(&str, Rect, ResizeDirection, CursorIcon); 8] = [
            (
                "n",
                Rect::from_min_max(full.min, pos2(full.right(), full.top() + t)),
                ResizeDirection::North,
                CursorIcon::ResizeVertical,
            ),
            (
                "s",
                Rect::from_min_max(pos2(full.left(), full.bottom() - t), full.max),
                ResizeDirection::South,
                CursorIcon::ResizeVertical,
            ),
            (
                "w",
                Rect::from_min_max(full.min, pos2(full.left() + t, full.bottom())),
                ResizeDirection::West,
                CursorIcon::ResizeHorizontal,
            ),
            (
                "e",
                Rect::from_min_max(pos2(full.right() - t, full.top()), full.max),
                ResizeDirection::East,
                CursorIcon::ResizeHorizontal,
            ),
            (
                "nw",
                Rect::from_min_size(full.min, vec2(c, c)),
                ResizeDirection::NorthWest,
                CursorIcon::ResizeNwSe,
            ),
            (
                "ne",
                Rect::from_min_size(pos2(full.right() - c, full.top()), vec2(c, c)),
                ResizeDirection::NorthEast,
                CursorIcon::ResizeNeSw,
            ),
            (
                "sw",
                Rect::from_min_size(pos2(full.left(), full.bottom() - c), vec2(c, c)),
                ResizeDirection::SouthWest,
                CursorIcon::ResizeNeSw,
            ),
            (
                "se",
                Rect::from_min_size(pos2(full.right() - c, full.bottom() - c), vec2(c, c)),
                ResizeDirection::SouthEast,
                CursorIcon::ResizeNwSe,
            ),
        ];
        for (id, rect, dir, cursor) in edges {
            let resp = ui.interact(rect, Id::new(("wresize", id)), Sense::drag());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(cursor);
            }
            if resp.drag_started() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
            }
        }
    }

    pub(crate) fn status_bar(&mut self, ui: &mut Ui, full: Rect, now: f64) {
        let pal = self.pal;
        let bar = Rect::from_min_size(
            pos2(full.left(), full.bottom() - STATUS_H),
            vec2(full.width(), STATUS_H),
        );
        ui.painter().rect_filled(bar, 0.0, pal.bg_secondary);
        ui.painter().line_segment(
            [bar.left_top(), bar.right_top()],
            Stroke::new(1.0, pal.border),
        );
        let name = self.vault_name();
        ui.painter().text(
            pos2(bar.left() + 12.0, bar.center().y),
            Align2::LEFT_CENTER,
            theme::truncate(&name, 28),
            theme::font(12.0),
            pal.text_faint,
        );
        let mut x = bar.right() - 12.0;
        let note = self.active_note_idx().and_then(|i| self.vault.notes.get(i));
        let (words, chars) = match note {
            Some(n) => (
                vault::word_count(&n.content),
                vault::char_count(&n.content),
            ),
            None => (0, 0),
        };
        for text in [format!("{chars} 字符"), format!("{words} 字")] {
            let galley = ui.ctx().fonts_mut(|f| {
                f.layout_no_wrap(text, theme::font(12.0), pal.text_muted)
            });
            let w = galley.size().x;
            ui.painter().galley(
                pos2(x - w, bar.center().y - galley.size().y * 0.5),
                galley,
                pal.text_muted,
            );
            x -= w + 14.0;
        }
        if let Some((msg, until)) = &self.notice {
            if *until > now {
                let galley = ui.ctx().fonts_mut(|f| {
                    f.layout_no_wrap(msg.clone(), theme::font(12.0), pal.text_muted)
                });
                let w = galley.size().x;
                ui.painter().galley(
                    pos2(x - w - 10.0, bar.center().y - galley.size().y * 0.5),
                    galley,
                    pal.text_muted,
                );
            }
        }
    }
}

/// One row of the file explorer. Free function to keep the borrow of the tree simple.
pub(crate) fn entry_row(
    ui: &mut Ui,
    app: &mut crate::App,
    entry: &vault::Entry,
    depth: usize,
    active: Option<usize>,
    actions: &mut Vec<Action>,
) {
    let pal = app.pal;
    let row_h = 26.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, row_h), Sense::click());
    let indent = 10.0 + depth as f32 * 16.0;
    match entry {
        vault::Entry::Folder(folder) => {
            let open = app.vault.expanded.contains(&folder.rel);
            if resp.hovered() {
                ui.painter()
                    .rect_filled(rect.shrink2(vec2(4.0, 0.0)), 4.0, pal.bg_hover);
            }
            icons::draw(
                &ui.painter().clone(),
                Rect::from_center_size(
                    pos2(rect.left() + indent + 6.0, rect.center().y),
                    vec2(12.0, 12.0),
                ),
                if open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                },
                pal.text_faint,
            );
            icons::draw(
                &ui.painter().clone(),
                Rect::from_center_size(
                    pos2(rect.left() + indent + 22.0, rect.center().y),
                    vec2(15.0, 15.0),
                ),
                Icon::Folder,
                pal.text_muted,
            );
            ui.painter().text(
                pos2(rect.left() + indent + 35.0, rect.center().y),
                Align2::LEFT_CENTER,
                &folder.name,
                theme::font(13.0),
                pal.text_muted,
            );
            if resp.clicked() {
                actions.push(Action::ToggleFolder(folder.rel.clone()));
            }
            let rel = folder.rel.clone();
            resp.context_menu(|ui| {
                if ui.button("新建笔记").clicked() {
                    actions.push(Action::CreateNote(rel.clone()));
                    ui.close();
                }
                if ui.button("新建文件夹").clicked() {
                    actions.push(Action::CreateFolder(rel.clone()));
                    ui.close();
                }
                if ui
                    .button(if open { "折叠文件夹" } else { "展开文件夹" })
                    .clicked()
                {
                    actions.push(Action::ToggleFolder(rel.clone()));
                    ui.close();
                }
                ui.separator();
                let root = app.vault.root.clone();
                let vault_name = app.vault_name();
                copy_path_menu(ui, &rel, &root, &vault_name, false, actions);
                if ui.button("删除文件夹").clicked() {
                    actions.push(Action::DeleteFolder(rel.clone()));
                    ui.close();
                }
            });
            if open {
                let children = folder.children.clone();
                for child in &children {
                    entry_row(ui, app, child, depth + 1, active, actions);
                }
            }
        }
        vault::Entry::Note(idx) => {
            let note = &app.vault.notes[*idx];
            let is_active = active == Some(*idx);
            if is_active {
                ui.painter()
                    .rect_filled(rect.shrink2(vec2(4.0, 0.0)), 4.0, pal.bg_hover);
            } else if resp.hovered() {
                ui.painter()
                    .rect_filled(rect.shrink2(vec2(4.0, 0.0)), 4.0, pal.bg_secondary_alt);
            }
            icons::draw(
                &ui.painter().clone(),
                Rect::from_center_size(
                    pos2(rect.left() + indent + 12.0, rect.center().y),
                    vec2(14.0, 14.0),
                ),
                Icon::FileText,
                if is_active { pal.text } else { pal.text_muted },
            );
            let clip = Rect::from_min_max(
                pos2(rect.left() + indent + 24.0, rect.top()),
                pos2(rect.right() - 6.0, rect.bottom()),
            );
            ui.painter().with_clip_rect(clip).text(
                pos2(rect.left() + indent + 24.0, rect.center().y),
                Align2::LEFT_CENTER,
                &note.title,
                theme::font(13.0),
                if is_active { pal.text } else { pal.text_muted },
            );
            if resp.clicked() {
                actions.push(Action::Open(*idx));
            }
            if resp.double_clicked() {
                actions.push(Action::OpenNewTab(*idx));
            }
            let note_idx = *idx;
            let rel = note.rel.clone();
            let title = note.title.clone();
            let bookmarked = app.bookmarks.iter().any(|b| b == &rel);
            resp.context_menu(|ui| {
                if ui.button("打开").clicked() {
                    actions.push(Action::Open(note_idx));
                    ui.close();
                }
                if ui.button("在新标签页中打开").clicked() {
                    actions.push(Action::OpenNewTab(note_idx));
                    ui.close();
                }
                ui.separator();
                if ui.button("重命名").clicked() {
                    actions.push(Action::Rename(note_idx));
                    ui.close();
                }
                if ui.button("删除文件").clicked() {
                    actions.push(Action::Delete(note_idx));
                    ui.close();
                }
                if ui
                    .button(if bookmarked {
                        "移除书签"
                    } else {
                        "添加书签"
                    })
                    .clicked()
                {
                    actions.push(Action::ToggleBookmark(rel.clone()));
                    ui.close();
                }
                ui.separator();
                let root = app.vault.root.clone();
                let vault_name = app.vault_name();
                copy_path_menu(ui, &rel, &root, &vault_name, true, actions);
                if ui.button("复制链接").clicked() {
                    ui.ctx().copy_text(format!("[[{title}]]"));
                    actions.push(Action::Toast("已复制链接".into()));
                    ui.close();
                }
            });
        }
    }
}

// keep the Vault import used for clarity of the helper signature

/// `encodeURIComponent` equivalence used by the original's `getObsidianUrl`.
fn encode_uri_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        let keep = b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')');
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `obsidian://open?vault=<vault>&file=<vault-relative path>`
pub(crate) fn obsidian_url(vault_name: &str, rel: &str) -> String {
    format!(
        "obsidian://open?vault={}&file={}",
        encode_uri_component(vault_name),
        encode_uri_component(rel)
    )
}

/// the original's file-menu "复制路径" submenu: Note URL / absolute / vault-relative.
pub(crate) fn copy_path_menu(
    ui: &mut Ui,
    rel: &str,
    root: &std::path::Path,
    vault_name: &str,
    with_url: bool,
    actions: &mut Vec<Action>,
) {
    ui.menu_button("复制路径", |ui| {
        if with_url && ui.button("Note URL").clicked() {
            ui.ctx().copy_text(obsidian_url(vault_name, rel));
            actions.push(Action::Toast("已复制 Note URL".into()));
            ui.close();
        }
        if ui.button("绝对路径").clicked() {
            ui.ctx()
                .copy_text(root.join(rel).display().to_string());
            actions.push(Action::Toast("已复制绝对路径".into()));
            ui.close();
        }
        if ui.button("基于库的相对路径").clicked() {
            ui.ctx().copy_text(rel.to_string());
            actions.push(Action::Toast("已复制相对路径".into()));
            ui.close();
        }
    });
}

