//! The graph view: notes as nodes, wikilinks as edges.
//!
//! Visual rules follow the shipped app: node fill is `--graph-node` (#b3b3b3),
//! labels are `--graph-text` (#dadada) at `14 + size/4` and offset below the
//! node, edges are `--graph-line` (#3f3f3f) at 20% with a 1px stroke, and the
//! settings live in the real top-right control panel (closed by default).

use crate::icons::{self, Icon};
use crate::theme::{self, Palette};
use crate::vault::Vault;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CursorIcon, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};
use std::collections::{HashMap, HashSet};

const MIN_ZOOM: f32 = 1.0 / 128.0;
const MAX_ZOOM: f32 = 8.0;
const WHEEL_ZOOM_FACTOR: f32 = 1.5;
const ZOOM_SMOOTHING: f32 = 0.85;
const ALPHA_MIN: f32 = 0.001;
const ALPHA_DECAY: f32 = 0.022_626_5;
const VELOCITY_DECAY: f32 = 0.6;
const COLLIDE_RADIUS: f32 = 60.0;
const COLLIDE_STRENGTH: f32 = 0.5;
const NODE_FADE: f32 = 0.2;
const LINE_ALPHA: f32 = 0.2;
/// d3's default alphaTarget while dragging, as the graph view posts it.
const DRAG_ALPHA: f32 = 0.3;

const PANEL_W: f32 = 240.0;
const PANEL_MARGIN: f32 = 12.0;
const SECTION_H: f32 = 30.0;

/// What a click or a context-menu item in the graph asks the app to do.
pub enum GraphAction {
    Open(usize),
    OpenNewTab(usize),
    Copy(String),
}

#[derive(Clone)]
pub struct ColorGroup {
    pub query: String,
    pub color: Color32,
}

pub struct GraphState {
    pub pos: HashMap<usize, Pos2>,
    pub vel: HashMap<usize, Vec2>,
    pub zoom: f32,
    pub target_zoom: f32,
    pub pan: Vec2,
    pub drag_node: Option<usize>,
    /// The node under the pointer when the button went down. egui only reports
    /// a drag after the pointer has moved past its threshold, by which time the
    /// pointer may already be off the node, so the pressed node is kept here.
    pub press_node: Option<usize>,
    pub initialized: bool,
    pub hovered: Option<usize>,
    /// Which node (if any) the open right-click menu belongs to.
    pub menu_node: Option<usize>,
    pub alpha: f32,
    /// d3's alphaTarget: 0.3 while a node is held, so the layout keeps running.
    pub alpha_target: f32,
    pub last_nodes: usize,
    pub last_edges: usize,
    // Real graph settings (app.js rDe / nDe / graph.json defaults).
    pub show_arrow: bool,
    pub text_fade: f32,
    pub node_size_mult: f32,
    pub line_size_mult: f32,
    pub show_orphans: bool,
    pub show_tags: bool,
    pub show_attachments: bool,
    pub hide_unresolved: bool,
    pub filter_query: String,
    pub groups: Vec<ColorGroup>,
    /// Slider-space force values; the engine wants `bhe(value, 0.01)`.
    pub center_slider: f32,
    pub link_slider: f32,
    pub link_distance: f32,
    pub repel_strength: f32,
    // The top-right control panel starts closed, as saved in graph.json.
    pub controls_open: bool,
    pub collapse_filter: bool,
    pub collapse_colors: bool,
    pub collapse_display: bool,
    pub collapse_forces: bool,
    zoom_anchor: Option<Pos2>,
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            pos: HashMap::new(),
            vel: HashMap::new(),
            zoom: 1.0,
            target_zoom: 1.0,
            pan: Vec2::ZERO,
            drag_node: None,
            press_node: None,
            initialized: false,
            hovered: None,
            menu_node: None,
            alpha: 1.0,
            alpha_target: 0.0,
            last_nodes: 0,
            last_edges: 0,
            show_arrow: false,
            text_fade: 0.0,
            node_size_mult: 1.0,
            line_size_mult: 1.0,
            show_orphans: true,
            show_tags: false,
            show_attachments: false,
            hide_unresolved: false,
            filter_query: String::new(),
            groups: Vec::new(),
            center_slider: 0.518_713_25,
            link_slider: 1.0,
            link_distance: 250.0,
            repel_strength: 10.0,
            controls_open: false,
            collapse_filter: true,
            collapse_colors: true,
            collapse_display: true,
            collapse_forces: true,
            zoom_anchor: None,
        }
    }
}

impl GraphState {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        area: Rect,
        vault: &Vault,
        pal: &Palette,
        active: Option<usize>,
    ) -> Option<GraphAction> {
        ui.painter().rect_filled(area, 0.0, pal.bg_primary);
        if vault.notes.is_empty() {
            ui.painter().text(
                area.center(),
                Align2::CENTER_CENTER,
                "仓库里还没有笔记",
                theme::font(14.0),
                pal.text_faint,
            );
            return None;
        }
        let n = vault.notes.len();
        if !self.initialized {
            // New nodes start around the origin (the original's worker then settles).
            let radius = 90.0 + n as f32 * 6.0;
            for i in 0..n {
                let a = i as f32 / n.max(1) as f32 * std::f32::consts::TAU;
                self.pos
                    .insert(i, pos2(a.cos() * radius, a.sin() * radius));
                self.vel.insert(i, Vec2::ZERO);
            }
            self.initialized = true;
            self.alpha = 1.0;
            self.alpha_target = 0.0;
        }
        for i in 0..n {
            self.pos.entry(i).or_insert_with(|| {
                let a = i as f32 * 2.399963;
                pos2(a.cos() * 120.0, a.sin() * 120.0)
            });
            self.vel.entry(i).or_insert(Vec2::ZERO);
        }

        let all_edges = build_edges(vault);
        let mut linked = HashSet::new();
        for (a, b) in &all_edges {
            linked.insert(*a);
            linked.insert(*b);
        }
        let query = self.filter_query.trim().to_lowercase();
        let visible: Vec<usize> = (0..n)
            .filter(|i| self.show_orphans || linked.contains(i))
            .filter(|i| {
                query.is_empty()
                    || vault.notes[*i].title.to_lowercase().contains(&query)
                    || vault.notes[*i].rel.to_lowercase().contains(&query)
            })
            .collect();
        let visible_set: HashSet<usize> = visible.iter().copied().collect();
        let edges: Vec<(usize, usize)> = all_edges
            .iter()
            .filter(|(a, b)| visible_set.contains(a) && visible_set.contains(b))
            .copied()
            .collect();
        if visible.len() != self.last_nodes || edges.len() != self.last_edges {
            self.last_nodes = visible.len();
            self.last_edges = edges.len();
            if self.initialized {
                self.alpha = self.alpha.max(0.3);
            }
        }

        let controls_rect = self.controls_rect(area);
        let resp = ui.interact(area, Id::new("graph-canvas"), Sense::click_and_drag());
        let pointer = ui.ctx().pointer_interact_pos();
        let pointer_over_controls = pointer.map(|p| controls_rect.contains(p)).unwrap_or(false);
        let center = area.center();
        let to_screen = |p: Pos2, zoom: f32, pan: Vec2| -> Pos2 { center + pan + p.to_vec2() * zoom };

        // Node weights drive both the radius and the hover hit-test.
        let mut degree: HashMap<usize, usize> = visible.iter().map(|&i| (i, 0)).collect();
        for (a, b) in &edges {
            *degree.entry(*a).or_insert(0) += 1;
            *degree.entry(*b).or_insert(0) += 1;
        }
        let hit_mult = self.node_size_mult;
        let hit_zoom = self.zoom;
        self.hovered = None;
        if let Some(p) = pointer {
            if area.contains(p) && !pointer_over_controls {
                let mut best: Option<(usize, f32)> = None;
                for &i in &visible {
                    let sp = to_screen(self.pos[&i], self.zoom, self.pan);
                    let d = sp.distance(p);
                    if d < screen_radius(*degree.get(&i).unwrap_or(&0), hit_mult, hit_zoom) + 2.0
                        && best.map(|(_, bd)| d < bd).unwrap_or(true)
                    {
                        best = Some((i, d));
                    }
                }
                self.hovered = best.map(|(i, _)| i);
            }
        }

        // Smooth zoom toward the target scale, keeping the stored anchor fixed.
        let zooming = (self.zoom - self.target_zoom).abs() > 0.0001;
        if zooming {
            let old = self.zoom;
            let anchor = self.zoom_anchor.unwrap_or(center);
            let world = (anchor - center - self.pan) / old;
            self.zoom = old * ZOOM_SMOOTHING + self.target_zoom * (1.0 - ZOOM_SMOOTHING);
            if (self.zoom - self.target_zoom).abs() < 0.0001 {
                self.zoom = self.target_zoom;
            }
            self.pan = anchor - center - world * self.zoom;
            ui.ctx().request_repaint();
        }

        // Right-click remembers what it was over; the menu is built on later frames.
        if resp.secondary_clicked() && !pointer_over_controls {
            self.menu_node = self.hovered;
        }
        let mut out: Option<GraphAction> = None;
        if self.menu_node.is_some() {
            resp.context_menu(|ui| {
                ui.set_min_width(180.0);
                let Some(node) = self.menu_node else {
                    return;
                };
                if let Some(note) = vault.notes.get(node) {
                    ui.add(egui::Label::new(
                        egui::RichText::new(theme::truncate(&note.title, 24))
                            .font(theme::font(12.5))
                            .color(pal.text_faint),
                    ));
                    ui.separator();
                    if ui.button("打开").clicked() {
                        out = Some(GraphAction::Open(node));
                        ui.close();
                    }
                    if ui.button("在新标签页中打开").clicked() {
                        out = Some(GraphAction::OpenNewTab(node));
                        ui.close();
                    }
                    ui.separator();
                    ui.menu_button("复制路径", |ui| {
                        let vault_name = vault
                            .root
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        if ui.button("Note URL").clicked() {
                            out = Some(GraphAction::Copy(crate::ui::obsidian_url(
                                &vault_name, &note.rel,
                            )));
                            ui.close();
                        }
                        if ui.button("绝对路径").clicked() {
                            out = Some(GraphAction::Copy(
                                vault.root.join(&note.rel).display().to_string(),
                            ));
                            ui.close();
                        }
                        if ui.button("基于库的相对路径").clicked() {
                            out = Some(GraphAction::Copy(note.rel.clone()));
                            ui.close();
                        }
                    });
                    if ui.button("复制链接").clicked() {
                        out = Some(GraphAction::Copy(format!("[[{}]]", note.title)));
                        ui.close();
                    }
                }
            });
        }

        // Wheel zoom: the original scales by 1.5 per 120 px of wheel delta.
        if resp.hovered() && !pointer_over_controls {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                let factor = WHEEL_ZOOM_FACTOR.powf(-scroll / (120.0 / 3.0));
                let target = (self.target_zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
                let anchor = if target < self.zoom { None } else { pointer };
                self.zoom_to(target, anchor);
            }
        }

        // Remember the node under the pointer at press time (the original's
        // onPointerDown does this before any movement); egui only reports a
        // drag once the pointer has moved, when it may be off the node already.
        let (primary_pressed, primary_down) =
            ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down()));
        if primary_pressed && !pointer_over_controls {
            self.press_node = self.hovered;
        }
        if !primary_down {
            self.press_node = None;
        }
        if resp.drag_started() && !pointer_over_controls {
            self.drag_node = self.press_node;
            if self.drag_node.is_some() {
                // The original posts {alpha:.3, alphaTarget:.3, run:true} on
                // every pointermove: the layout runs for the whole drag.
                self.alpha = self.alpha.max(DRAG_ALPHA);
                self.alpha_target = DRAG_ALPHA;
                ui.ctx().request_repaint();
            }
        }
        if resp.dragged() {
            if let Some(node) = self.drag_node {
                // Pinned to the pointer, even when it wanders over the panel.
                if let Some(p) = pointer {
                    let world = (p - center - self.pan) / self.zoom;
                    self.pos.insert(node, pos2(world.x, world.y));
                    self.vel.insert(node, Vec2::ZERO);
                    self.hovered = Some(node);
                }
                self.alpha = self.alpha.max(DRAG_ALPHA);
                self.alpha_target = DRAG_ALPHA;
                ui.ctx().request_repaint();
            } else if !pointer_over_controls {
                self.pan += resp.drag_delta();
            }
        }
        // A release outside the window may never arrive as drag_stopped.
        if resp.drag_stopped() || (self.drag_node.is_some() && !primary_down) {
            if self.drag_node.is_some() {
                // alphaTarget back to 0: the graph settles after release.
                self.alpha_target = 0.0;
                self.alpha = self.alpha.max(DRAG_ALPHA);
            }
            self.drag_node = None;
            self.press_node = None;
        }

        // d3-force style tick with the original's values. While a node is held the
        // layout keeps ticking (alphaTarget 0.3) with that node pinned, like
        // the original worker's forceNode message, so neighbours react live
        // instead of only after the button is released.
        if (self.alpha > ALPHA_MIN || self.alpha_target > 0.0) && visible.len() > 1 {
            ui.ctx().request_repaint();
            let alpha = self.alpha;
            let center_strength = bhe(self.center_slider, 0.01);
            let link_strength = bhe(self.link_slider, 0.01);
            let mut force: HashMap<usize, Vec2> =
                visible.iter().map(|&i| (i, Vec2::ZERO)).collect();

            let link_alpha = alpha * link_strength;
            for (a, b) in &edges {
                let dx = self.pos[b].x - self.pos[a].x;
                let dy = self.pos[b].y - self.pos[a].y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let spring = (dist - self.link_distance) / dist * link_alpha;
                let fx = dx * spring;
                let fy = dy * spring;
                let da = *degree.get(a).unwrap_or(&1) as f32;
                let db = *degree.get(b).unwrap_or(&1) as f32;
                let bias = db / (da + db);
                *force.get_mut(a).unwrap() += vec2(fx * (1.0 - bias), fy * (1.0 - bias));
                *force.get_mut(b).unwrap() -= vec2(fx * bias, fy * bias);
            }

            let charge = -(self.repel_strength.powi(3)) * alpha;
            for (k, &i) in visible.iter().enumerate() {
                for &j in &visible[k + 1..] {
                    let dx = self.pos[&i].x - self.pos[&j].x;
                    let dy = self.pos[&i].y - self.pos[&j].y;
                    let dist2 = (dx * dx + dy * dy).max(1.0);
                    let w = charge / dist2;
                    *force.get_mut(&i).unwrap() += vec2(dx * w, dy * w);
                    *force.get_mut(&j).unwrap() -= vec2(dx * w, dy * w);
                }
            }

            for &i in &visible {
                let p = self.pos[&i];
                *force.get_mut(&i).unwrap() += vec2(
                    -p.x * center_strength * alpha,
                    -p.y * center_strength * alpha,
                );
            }

            for (k, &i) in visible.iter().enumerate() {
                for &j in &visible[k + 1..] {
                    let dx = self.pos[&j].x - self.pos[&i].x;
                    let dy = self.pos[&j].y - self.pos[&i].y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                    let overlap = COLLIDE_RADIUS * 2.0 - dist;
                    if overlap > 0.0 {
                        let l = overlap / dist * COLLIDE_STRENGTH * alpha;
                        *force.get_mut(&i).unwrap() -= vec2(dx * l, dy * l);
                        *force.get_mut(&j).unwrap() += vec2(dx * l, dy * l);
                    }
                }
            }

            for &i in &visible {
                if Some(i) == self.drag_node {
                    // Pinned to the pointer: forces still act on it, but it
                    // does not integrate away from the cursor.
                    self.vel.insert(i, Vec2::ZERO);
                    continue;
                }
                let v = (self.vel[&i] + force[&i]) * VELOCITY_DECAY;
                self.vel.insert(i, v);
                let p = self.pos[&i];
                self.pos.insert(i, pos2(p.x + v.x, p.y + v.y));
            }
            self.alpha += (self.alpha_target - self.alpha) * ALPHA_DECAY;
        } else {
            self.alpha = self.alpha.max(0.0);
        }

        // Highlight: hovered node and its neighbours stay lit, the rest fades.
        let neighbours: HashSet<usize> = if let Some(h) = self.hovered {
            edges
                .iter()
                .filter_map(|&(a, b)| {
                    if a == h {
                        Some(b)
                    } else if b == h {
                        Some(a)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            HashSet::new()
        };
        let hovering = self.hovered.is_some();
        let zoom_fade = ((self.zoom - 0.3) * 2.0).clamp(0.0, 1.0);
        let sqrt_zoom = self.zoom.max(MIN_ZOOM).sqrt();

        // Colour groups tint matching notes; the focused and hovered notes win.
        let mut group_color: HashMap<usize, Color32> = HashMap::new();
        for &i in &visible {
            let hay = format!("{} {}", vault.notes[i].title, vault.notes[i].rel).to_lowercase();
            for g in &self.groups {
                let q = g.query.trim().to_lowercase();
                if !q.is_empty() && hay.contains(&q) {
                    group_color.insert(i, g.color);
                    break;
                }
            }
        }

        // edges
        for (a, b) in &edges {
            let pa = to_screen(self.pos[a], self.zoom, self.pan);
            let pb = to_screen(self.pos[b], self.zoom, self.pan);
            let hot = self.hovered == Some(*a) || self.hovered == Some(*b);
            let color = if hot { pal.accent } else { pal.border_strong };
            let alpha = if hot { 1.0 } else { LINE_ALPHA } * zoom_fade;
            let width = self.line_size_mult.max(0.5);
            ui.painter().line_segment(
                [pa, pb],
                Stroke::new(width, with_alpha(color, alpha)),
            );
            if self.show_arrow && zoom_fade > 0.0 {
                let arrow_color = with_alpha(
                    pal.text,
                    (if hot { 1.0 } else { LINE_ALPHA }) * zoom_fade,
                );
                draw_arrow(
                    &ui.painter().clone(),
                    pa,
                    pb,
                    screen_radius(*degree.get(b).unwrap_or(&0), self.node_size_mult, self.zoom)
                        + 1.0,
                    arrow_color,
                    2.0 * self.line_size_mult.sqrt(),
                );
            }
        }

        // nodes
        let text_alpha = (self.zoom.log2() + 1.0 - self.text_fade).clamp(0.0, 1.0);
        for &i in &visible {
            let sp = to_screen(self.pos[&i], self.zoom, self.pan);
            let r = screen_radius(*degree.get(&i).unwrap_or(&0), self.node_size_mult, self.zoom);
            let r_world = base_node_size(*degree.get(&i).unwrap_or(&0)) * self.node_size_mult;
            let is_active = active == Some(i);
            let is_hover = self.hovered == Some(i);
            let is_neighbour = neighbours.contains(&i);
            let fade = if hovering && !is_neighbour && !is_hover {
                NODE_FADE
            } else {
                1.0
            };
            let color = if is_active || is_hover {
                pal.accent
            } else if let Some(c) = group_color.get(&i) {
                *c
            } else {
                pal.text_muted
            };
            ui.painter()
                .circle_filled(sp, r, with_alpha(color, fade));
            if is_active {
                ui.painter().circle_stroke(
                    sp,
                    r + 1.0,
                    Stroke::new(self.zoom.max(1.0).sqrt(), with_alpha(pal.accent, fade)),
                );
            }
            let label_alpha = if is_hover { 1.0 } else { text_alpha * fade };
            if label_alpha > 0.001 {
                let label_size = (14.0 + r_world / 4.0) * sqrt_zoom;
                let offset_y = (r_world + 5.0) * sqrt_zoom + if is_hover { 15.0 } else { 0.0 };
                ui.painter().text(
                    sp + vec2(0.0, offset_y),
                    Align2::CENTER_TOP,
                    theme::truncate(&vault.notes[i].title, 24),
                    theme::font(label_size),
                    with_alpha(pal.text, label_alpha),
                );
            }
        }

        if resp.double_clicked() && !pointer_over_controls {
            if let Some(h) = self.hovered {
                out = Some(GraphAction::OpenNewTab(h));
            }
        } else if resp.clicked() && !pointer_over_controls {
            if let Some(h) = self.hovered {
                out = Some(GraphAction::Open(h));
            }
        }

        self.controls(ui, area, pal);
        out
    }

    fn controls(&mut self, ui: &mut Ui, area: Rect, pal: &Palette) {
        let right = area.right() - PANEL_MARGIN;
        let top = area.top() + PANEL_MARGIN;
        if !self.controls_open {
            // Closed: a small primary-coloured pill with the settings gear and
            // the growth-animation wand, exactly where the original puts them.
            let rect = Rect::from_min_size(pos2(right - 36.0, top), vec2(36.0, 60.0));
            ui.painter().rect_filled(rect, 8.0, pal.bg_primary);
            let gear = Rect::from_min_size(pos2(rect.left() + 6.0, rect.top() + 6.0), vec2(24.0, 24.0));
            let wand = Rect::from_min_size(pos2(rect.left() + 6.0, rect.top() + 30.0), vec2(24.0, 24.0));
            if icon_button(ui, gear, Icon::Settings, "graph-open-settings", pal) {
                self.controls_open = true;
            }
            if icon_button(ui, wand, Icon::Wand, "graph-animate", pal) {
                self.alpha = 1.0;
                self.alpha_target = 0.0;
            }
            return;
        }

        let panel = Rect::from_min_size(
            pos2(right - PANEL_W, top),
            vec2(PANEL_W, self.panel_height().min((area.height() - 24.0).max(120.0))),
        );
        ui.painter().rect_filled(
            panel.translate(vec2(0.0, 2.0)).expand(2.0),
            10.0,
            Color32::from_black_alpha(50),
        );
        ui.painter()
            .rect_filled(panel, 8.0, pal.bg_secondary);
        ui.painter()
            .rect_stroke(panel, 8.0, Stroke::new(1.0, pal.border_strong), egui::StrokeKind::Inside);

        let mut y = panel.top() + 8.0;
        let titles = ["筛选", "颜色组", "外观", "力度"];
        for i in 0..4 {
            let collapsed = match i {
                0 => self.collapse_filter,
                1 => self.collapse_colors,
                2 => self.collapse_display,
                _ => self.collapse_forces,
            };
            let mut hdr = Rect::from_min_size(pos2(panel.left(), y), vec2(panel.width(), SECTION_H));
            if i == 0 {
                hdr.max.x -= 72.0;
            }
            let hdr_resp = ui.interact(hdr, Id::new(("graph-section", i)), Sense::click());
            if hdr_resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        pos2(panel.left() + 6.0, y + 2.0),
                        pos2(panel.right() - 6.0, y + SECTION_H - 2.0),
                    ),
                    4.0,
                    pal.bg_hover,
                );
            }
            icons::draw(
                &ui.painter().clone(),
                Rect::from_center_size(pos2(panel.left() + 16.0, y + SECTION_H / 2.0), vec2(12.0, 12.0)),
                if collapsed { Icon::ChevronRight } else { Icon::ChevronDown },
                pal.text_faint,
            );
            ui.painter().text(
                pos2(panel.left() + 30.0, y + SECTION_H / 2.0),
                Align2::LEFT_CENTER,
                titles[i],
                theme::font(13.0),
                pal.text,
            );
            if hdr_resp.clicked() {
                match i {
                    0 => self.collapse_filter = !self.collapse_filter,
                    1 => self.collapse_colors = !self.collapse_colors,
                    2 => self.collapse_display = !self.collapse_display,
                    _ => self.collapse_forces = !self.collapse_forces,
                }
            }
            y += SECTION_H;
            if !collapsed {
                let extra = match i {
                    0 => self.filter_section(ui, panel, y, pal),
                    1 => self.colors_section(ui, panel, y, pal),
                    2 => self.display_section(ui, panel, y, pal),
                    _ => self.forces_section(ui, panel, y, pal),
                };
                y += extra;
            }
            if i < 3 {
                ui.painter().line_segment(
                    [pos2(panel.left(), y), pos2(panel.right(), y)],
                    Stroke::new(1.0, pal.border),
                );
            }
        }

        // Overlay buttons, as in app.css `.graph-controls-button.mod-close/-reset`.
        let close = Rect::from_min_size(pos2(panel.right() - 32.0, panel.top() + 6.0), vec2(24.0, 24.0));
        let reset = Rect::from_min_size(pos2(panel.right() - 60.0, panel.top() + 6.0), vec2(24.0, 24.0));
        if icon_button(ui, close, Icon::X, "graph-close-settings", pal) {
            self.controls_open = false;
        }
        if icon_button(ui, reset, Icon::RotateCcw, "graph-reset-settings", pal) {
            self.reset_settings();
        }
    }

    fn panel_height(&self) -> f32 {
        let mut h = 16.0 + SECTION_H * 4.0;
        if !self.collapse_filter {
            h += 34.0 + 4.0 * 30.0 + 8.0;
        }
        if !self.collapse_colors {
            h += 34.0 + self.groups.len() as f32 * 30.0 + 8.0;
        }
        if !self.collapse_display {
            h += 30.0 + 3.0 * 44.0 + 8.0;
        }
        if !self.collapse_forces {
            h += 4.0 * 44.0 + 8.0;
        }
        h
    }

    fn controls_rect(&self, area: Rect) -> Rect {
        let right = area.right() - PANEL_MARGIN;
        let top = area.top() + PANEL_MARGIN;
        if self.controls_open {
            Rect::from_min_size(
                pos2(right - PANEL_W, top),
                vec2(PANEL_W, self.panel_height().min((area.height() - 24.0).max(120.0))),
            )
        } else {
            Rect::from_min_size(pos2(right - 36.0, top), vec2(36.0, 60.0))
        }
    }

    fn filter_section(&mut self, ui: &mut Ui, panel: Rect, y: f32, pal: &Palette) -> f32 {
        let x = panel.left() + 12.0;
        let w = panel.width() - 24.0;
        let search = Rect::from_min_size(pos2(x, y + 6.0), vec2(w, 30.0));
        ui.painter().rect_filled(search, 5.0, pal.bg_primary);
        ui.painter()
            .rect_stroke(search, 5.0, Stroke::new(1.0, pal.border), egui::StrokeKind::Inside);
        let edit = egui::TextEdit::singleline(&mut self.filter_query)
            .id(Id::new("graph-filter-input"))
            .hint_text("搜索文件…")
            .font(theme::font(13.0))
            .background_color(Color32::TRANSPARENT)
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::symmetric(8, 4))
            .text_color(pal.text);
        if ui.put(search.shrink2(vec2(6.0, 3.0)), edit).changed() {
            self.alpha = self.alpha.max(0.3);
        }
        let mut row_y = y + 6.0 + 34.0;
        for (label, value) in [
            ("标签", &mut self.show_tags),
            ("附件", &mut self.show_attachments),
            ("仅显示已创建的笔记", &mut self.hide_unresolved),
            ("孤立文件", &mut self.show_orphans),
        ] {
            let row = Rect::from_min_size(pos2(x, row_y), vec2(w, 28.0));
            ui.painter().text(
                pos2(row.left(), row.center().y),
                Align2::LEFT_CENTER,
                label,
                theme::font(13.0),
                pal.text,
            );
            let tr = Rect::from_min_size(pos2(row.right() - 40.0, row.center().y - 11.0), vec2(40.0, 22.0));
            let id = Id::new(("graph-toggle", label));
            if draw_toggle(ui, tr, *value, id, pal) {
                *value = !*value;
                if label == "孤立文件" {
                    self.alpha = self.alpha.max(0.3);
                }
            }
            row_y += 30.0;
        }
        row_y + 6.0 - y
    }

    fn colors_section(&mut self, ui: &mut Ui, panel: Rect, y: f32, pal: &Palette) -> f32 {
        let x = panel.left() + 12.0;
        let w = panel.width() - 24.0;
        let btn = Rect::from_min_size(pos2(x, y + 6.0), vec2(w, 30.0));
        if draw_button(ui, btn, "新建颜色组", "graph-new-group", pal) {
            let colors = group_palette(pal);
            let color = colors[self.groups.len() % colors.len()];
            self.groups.push(ColorGroup {
                query: String::new(),
                color,
            });
        }
        let mut row_y = y + 6.0 + 34.0;
        let mut remove: Option<usize> = None;
        for (i, group) in self.groups.iter_mut().enumerate() {
            let row = Rect::from_min_size(pos2(x, row_y), vec2(w, 28.0));
            let swatch = Rect::from_min_size(pos2(row.left(), row.center().y - 9.0), vec2(18.0, 18.0));
            let swatch_resp = ui.interact(swatch, Id::new(("graph-swatch", i)), Sense::click());
            ui.painter().rect_filled(swatch, 4.0, group.color);
            if swatch_resp.clicked() {
                let colors = group_palette(pal);
                let cur = colors.iter().position(|c| *c == group.color).unwrap_or(0);
                group.color = colors[(cur + 1) % colors.len()];
            }
            let input = Rect::from_min_size(
                pos2(swatch.right() + 8.0, row.center().y - 13.0),
                vec2(w - 18.0 - 8.0 - 24.0, 26.0),
            );
            ui.painter().rect_filled(input, 4.0, pal.bg_primary);
            ui.painter()
                .rect_stroke(input, 4.0, Stroke::new(1.0, pal.border), egui::StrokeKind::Inside);
            let edit = egui::TextEdit::singleline(&mut group.query)
                .id(Id::new(("graph-group-input", i)))
                .hint_text("请输入检索式…")
                .font(theme::font(12.5))
                .background_color(Color32::TRANSPARENT)
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(6, 3))
                .text_color(pal.text);
            if ui.put(input.shrink2(vec2(4.0, 2.0)), edit).changed() {
                ui.ctx().request_repaint();
            }
            let del = Rect::from_min_size(pos2(row.right() - 22.0, row.center().y - 11.0), vec2(22.0, 22.0));
            if icon_button(ui, del, Icon::X, &format!("graph-group-del-{i}"), pal) {
                remove = Some(i);
            }
            row_y += 30.0;
        }
        if let Some(i) = remove {
            self.groups.remove(i);
        }
        row_y + 6.0 - y
    }

    fn display_section(&mut self, ui: &mut Ui, panel: Rect, y: f32, pal: &Palette) -> f32 {
        let x = panel.left() + 12.0;
        let w = panel.width() - 24.0;
        let row = Rect::from_min_size(pos2(x, y + 4.0), vec2(w, 28.0));
        ui.painter().text(
            pos2(row.left(), row.center().y),
            Align2::LEFT_CENTER,
            "箭头",
            theme::font(13.0),
            pal.text,
        );
        let tr = Rect::from_min_size(pos2(row.right() - 40.0, row.center().y - 11.0), vec2(40.0, 22.0));
        if draw_toggle(ui, tr, self.show_arrow, Id::new("graph-toggle-arrow"), pal) {
            self.show_arrow = !self.show_arrow;
        }
        let mut sy = y + 38.0;
        if slider_row(ui, x, sy, w, "文本透明度", &mut self.text_fade, -3.0, 3.0, pal, Id::new("graph-fade")) {
            self.alpha = self.alpha.max(0.3);
        }
        sy += 44.0;
        if slider_row(ui, x, sy, w, "节点大小", &mut self.node_size_mult, 0.1, 5.0, pal, Id::new("graph-node-size")) {
            ui.ctx().request_repaint();
        }
        sy += 44.0;
        if slider_row(ui, x, sy, w, "连线粗细", &mut self.line_size_mult, 0.1, 5.0, pal, Id::new("graph-line-size")) {
            ui.ctx().request_repaint();
        }
        sy + 44.0 - y
    }

    fn forces_section(&mut self, ui: &mut Ui, panel: Rect, y: f32, pal: &Palette) -> f32 {
        let x = panel.left() + 12.0;
        let w = panel.width() - 24.0;
        let mut sy = y + 6.0;
        if slider_row(ui, x, sy, w, "图谱向心力", &mut self.center_slider, 0.0, 1.0, pal, Id::new("graph-center")) {
            self.alpha = self.alpha.max(0.3);
        }
        sy += 44.0;
        if slider_row(ui, x, sy, w, "相连节点间的吸引力", &mut self.link_slider, 0.0, 1.0, pal, Id::new("graph-link")) {
            self.alpha = self.alpha.max(0.3);
        }
        sy += 44.0;
        if slider_row(ui, x, sy, w, "连线长度", &mut self.link_distance, 30.0, 500.0, pal, Id::new("graph-distance")) {
            self.alpha = self.alpha.max(0.3);
        }
        sy += 44.0;
        if slider_row(ui, x, sy, w, "节点间的排斥力", &mut self.repel_strength, 0.0, 20.0, pal, Id::new("graph-repel")) {
            self.alpha = self.alpha.max(0.3);
        }
        sy + 44.0 - y
    }

    fn reset_settings(&mut self) {
        self.show_arrow = false;
        self.text_fade = 0.0;
        self.node_size_mult = 1.0;
        self.line_size_mult = 1.0;
        self.show_orphans = true;
        self.show_tags = false;
        self.show_attachments = false;
        self.hide_unresolved = false;
        self.filter_query.clear();
        self.groups.clear();
        self.center_slider = 0.518_713_25;
        self.link_slider = 1.0;
        self.link_distance = 250.0;
        self.repel_strength = 10.0;
        self.alpha = self.alpha.max(0.3);
    }

    fn zoom_to(&mut self, target: f32, anchor: Option<Pos2>) {
        self.target_zoom = target.clamp(MIN_ZOOM, MAX_ZOOM);
        self.zoom_anchor = anchor;
    }
}

/// `Bhe(slider, min)` from app.js: the force slider's non-linear mapping.
fn bhe(value: f32, min: f32) -> f32 {
    (min.powf(1.0 - value) - min) / (1.0 - min)
}

/// Base node size (radius): `clamp(3*sqrt(weight+1), 8, 30)`.
fn base_node_size(weight: usize) -> f32 {
    (3.0 * ((weight + 1) as f32).sqrt()).clamp(8.0, 30.0)
}

/// the original's node layer is scaled by `sqrt(1/scale)`, so on screen the radius
/// grows sub-linearly: `size * sqrt(scale)`.
fn screen_radius(weight: usize, multiplier: f32, zoom: f32) -> f32 {
    base_node_size(weight) * multiplier * zoom.max(MIN_ZOOM).sqrt()
}

fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (color.a() as f32 * alpha.clamp(0.0, 1.0)) as u8,
    )
}

fn draw_arrow(
    p: &egui::Painter,
    from: Pos2,
    to: Pos2,
    offset: f32,
    color: Color32,
    scale: f32,
) {
    let d = to - from;
    let len = d.length();
    if len <= offset + 1.0 {
        return;
    }
    let dir = d / len;
    let tip = to - dir * offset;
    let back = tip - dir * (4.0 * scale);
    let side = vec2(-dir.y, dir.x) * (2.0 * scale);
    p.add(egui::Shape::convex_polygon(
        vec![tip, back + side, back - side],
        color,
        Stroke::NONE,
    ));
}

/// A 24×24 clickable icon (the graph panel buttons).
fn icon_button(ui: &mut Ui, r: Rect, icon: Icon, id: &str, pal: &Palette) -> bool {
    let resp = ui.interact(r, Id::new(id), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        ui.painter().rect_filled(r, 4.0, pal.bg_hover);
    }
    icons::draw(
        &ui.painter().clone(),
        Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
        icon,
        if resp.hovered() { pal.text } else { pal.text_muted },
    );
    resp.clicked()
}

/// The real 40×22 toggle from the settings panel.
fn draw_toggle(ui: &mut Ui, r: Rect, on: bool, id: Id, pal: &Palette) -> bool {
    let resp = ui.interact(r, id, Sense::click());
    let bg = if on { pal.accent } else { pal.border_strong };
    ui.painter().rect_filled(r, r.height() / 2.0, bg);
    let knob = r.height() - 4.0;
    let x = if on {
        r.right() - knob / 2.0 - 2.0
    } else {
        r.left() + knob / 2.0 + 2.0
    };
    ui.painter()
        .circle_filled(pos2(x, r.center().y), knob / 2.0, Color32::WHITE);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// A labelled slider row (label above, 4px track + 18px white thumb).
#[allow(clippy::too_many_arguments)]
fn slider_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    pal: &Palette,
    id: Id,
) -> bool {
    ui.painter().text(
        pos2(x, y + 8.0),
        Align2::LEFT_CENTER,
        label,
        theme::font(13.0),
        pal.text,
    );
    let track = Rect::from_min_size(pos2(x, y + 26.0), vec2(w, 4.0));
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    ui.painter().rect_filled(track, 2.0, pal.border_strong);
    if t > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_max(track.min, pos2(track.left() + track.width() * t, track.bottom())),
            2.0,
            pal.accent,
        );
    }
    let thumb = pos2(track.left() + track.width() * t, track.center().y);
    ui.painter().circle_filled(thumb, 9.0, Color32::WHITE);
    let hit = Rect::from_min_size(pos2(x, y + 16.0), vec2(w, 24.0));
    let resp = ui.interact(hit, id, Sense::click_and_drag());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if (resp.dragged() || resp.clicked()) && resp.hovered() {
        if let Some(p) = ui.ctx().pointer_interact_pos() {
            let nt = ((p.x - track.left()) / track.width()).clamp(0.0, 1.0);
            *value = min + nt * (max - min);
            return true;
        }
    }
    false
}

fn draw_button(ui: &mut Ui, r: Rect, label: &str, id: &str, pal: &Palette) -> bool {
    let resp = ui.interact(r, Id::new(id), Sense::click());
    let bg = if resp.hovered() {
        pal.bg_hover
    } else {
        pal.bg_secondary_alt
    };
    ui.painter().rect_filled(r, 5.0, bg);
    ui.painter()
        .rect_stroke(r, 5.0, Stroke::new(1.0, pal.border), egui::StrokeKind::Inside);
    ui.painter().text(
        r.center(),
        Align2::CENTER_CENTER,
        label,
        theme::font(13.0),
        pal.text,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn group_palette(pal: &Palette) -> [Color32; 8] {
    [
        pal.accent,
        pal.red,
        pal.yellow,
        pal.green,
        Color32::from_rgb(0x4C, 0xA9, 0xF0),
        Color32::from_rgb(0xF0, 0x8A, 0x4C),
        Color32::from_rgb(0x3F, 0xC1, 0xC1),
        Color32::from_rgb(0xE0, 0x6C, 0xC0),
    ]
}

fn build_edges(vault: &Vault) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, note) in vault.notes.iter().enumerate() {
        for (target, _, _) in crate::vault::extract_links(&note.content) {
            if let Some(j) = vault.resolve(&target) {
                if i != j && seen.insert((i.min(j), i.max(j))) {
                    out.push((i.min(j), i.max(j)));
                }
            }
        }
    }
    out
}

/// Regression test for live node dragging: while the button is still held the
/// dragged node follows the pointer and the linked node already moves.
#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Event, PointerButton, RawInput};

    const AREA: Rect = Rect {
        min: Pos2 { x: 0.0, y: 0.0 },
        max: Pos2 { x: 800.0, y: 600.0 },
    };

    struct Harness {
        ctx: egui::Context,
        state: GraphState,
        vault: Vault,
        pal: Palette,
        time: f64,
    }

    impl Harness {
        fn new() -> Self {
            let notes = vec![
                crate::vault::Note {
                    rel: "a.md".into(),
                    title: "a".into(),
                    content: "[[b]]".into(),
                    modified: 0,
                },
                crate::vault::Note {
                    rel: "b.md".into(),
                    title: "b".into(),
                    content: "b".into(),
                    modified: 0,
                },
                crate::vault::Note {
                    rel: "c.md".into(),
                    title: "c".into(),
                    content: "c".into(),
                    modified: 0,
                },
            ];
            let mut title_index = HashMap::new();
            let mut path_index = HashMap::new();
            for (i, n) in notes.iter().enumerate() {
                title_index.insert(n.title.to_lowercase(), i);
                path_index.insert(n.rel.to_lowercase(), i);
                path_index.insert(n.rel.trim_end_matches(".md").to_lowercase(), i);
            }
            let vault = Vault {
                root: std::path::PathBuf::from("."),
                notes,
                tree: Vec::new(),
                expanded: HashSet::new(),
                title_index,
                path_index,
            };

            let mut state = GraphState::default();
            state.initialized = true;
            for (title, p) in [
                ("a", pos2(0.0, 0.0)),
                ("b", pos2(100.0, 0.0)),
                ("c", pos2(-100.0, 0.0)),
            ] {
                let i = vault.find_by_title(title).unwrap();
                state.pos.insert(i, p);
                state.vel.insert(i, Vec2::ZERO);
            }
            state.alpha = 0.0;
            state.last_nodes = vault.notes.len();
            state.last_edges = 1;
            Self {
                ctx: egui::Context::default(),
                state,
                vault,
                pal: Palette::dark(),
                time: 0.0,
            }
        }

        fn idx(&self, title: &str) -> usize {
            self.vault.find_by_title(title).unwrap()
        }

        /// One frame: move the pointer and optionally press/release it.
        fn frame(&mut self, pos: Pos2, button: Option<bool>) {
            self.time += 1.0 / 60.0;
            let mut input = RawInput::default();
            input.time = Some(self.time);
            input.screen_rect = Some(AREA);
            input.events.push(Event::PointerMoved(pos));
            if let Some(pressed) = button {
                input.events.push(Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                });
            }
            let state = &mut self.state;
            let vault = &self.vault;
            let pal = &self.pal;
            self.ctx
                .run_ui(input, |ui| {
                    state.show(ui, AREA, vault, pal, None);
                })
                .drop_without_applying_deltas();
        }
    }

    #[test]
    fn dragging_a_node_runs_the_layout_before_the_button_is_released() {
        let mut h = Harness::new();
        let a = h.idx("a");
        let b = h.idx("b");

        h.frame(pos2(400.0, 300.0), None); // hover warms up
        h.frame(pos2(400.0, 300.0), None);
        assert_eq!(h.state.hovered, Some(a), "node a must be hoverable");
        h.frame(pos2(400.0, 300.0), Some(true)); // press on a
        assert_eq!(h.state.press_node, Some(a), "press must remember the node");

        let b_before = h.state.pos[&b];
        h.frame(pos2(500.0, 320.0), None); // move past egui's drag threshold
        assert_eq!(
            h.state.drag_node,
            Some(a),
            "the drag must stick to the node pressed, not the hovered node"
        );
        assert_eq!(h.state.alpha_target, DRAG_ALPHA);

        h.frame(pos2(600.0, 340.0), None); // still held down
        let a_world = pos2(200.0, 40.0);
        assert!(
            h.state.pos[&a].distance(a_world) < 0.01,
            "the dragged node must follow the pointer while held: {:?}",
            h.state.pos[&a]
        );
        assert!(
            h.state.pos[&b].distance(b_before) > 0.1,
            "the linked node must react while the button is still held \
             (before: {b_before:?}, now: {:?})",
            h.state.pos[&b]
        );
        assert_eq!(h.state.drag_node, Some(a));
        assert_eq!(h.state.alpha_target, DRAG_ALPHA);

        h.frame(pos2(600.0, 340.0), Some(false)); // release
        assert_eq!(h.state.drag_node, None);
        assert_eq!(h.state.alpha_target, 0.0);
        assert!(h.state.alpha > 0.0, "the graph must settle after release");
    }
}
