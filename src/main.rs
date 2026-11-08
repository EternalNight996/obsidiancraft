#![allow(dead_code)]

mod graph;
mod icons;
mod markdown;
mod modals;
mod png;
mod theme;
mod ui;
mod vault;

use eframe::egui::{self, Color32, Id, Rect, Sense, Ui};
use icons::Icon;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use theme::{Palette, HEADER_H, RIBBON_W, SIDEBAR_W, STATUS_H};

pub const VERSION: &str = "1.15.0";

// ------------------------------------------------------------------ state

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LeftTab {
    Files,
    Search,
    Bookmarks,
}

impl LeftTab {
    pub fn title(self) -> &'static str {
        match self {
            LeftTab::Files => "文件列表",
            LeftTab::Search => "搜索",
            LeftTab::Bookmarks => "书签",
        }
    }
    pub fn icon(self) -> Icon {
        match self {
            LeftTab::Files => Icon::Folder,
            LeftTab::Search => Icon::Search,
            LeftTab::Bookmarks => Icon::Bookmark,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RightTab {
    Backlinks,
    Outgoing,
    Tags,
    Outline,
    Properties,
}

impl RightTab {
    pub fn title(self) -> &'static str {
        match self {
            RightTab::Backlinks => "反向链接",
            RightTab::Outgoing => "出链",
            RightTab::Tags => "标签",
            RightTab::Outline => "大纲",
            RightTab::Properties => "添加笔记属性",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            RightTab::Backlinks => "反向链接",
            RightTab::Outgoing => "出链",
            RightTab::Tags => "标签",
            RightTab::Outline => "大纲",
            RightTab::Properties => "属性",
        }
    }
    pub fn icon(self) -> Icon {
        match self {
            RightTab::Backlinks => Icon::Link,
            RightTab::Outgoing => Icon::External,
            RightTab::Tags => Icon::Tag,
            RightTab::Outline => Icon::List,
            RightTab::Properties => Icon::Archive,
        }
    }
    pub const ALL: [RightTab; 5] = [
        RightTab::Backlinks,
        RightTab::Outgoing,
        RightTab::Tags,
        RightTab::Outline,
        RightTab::Properties,
    ];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Read,
    Edit,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TabKind {
    Note(String),
    Graph,
}

#[derive(Clone)]
pub struct EditBuf {
    pub text: String,
    pub last_edit: f64,
}

#[derive(Clone)]
pub struct Tab {
    pub kind: TabKind,
    pub mode: ViewMode,
    pub edit: Option<EditBuf>,
}

impl Tab {
    pub fn note(rel: &str, mode: ViewMode) -> Self {
        Self {
            kind: TabKind::Note(rel.to_string()),
            mode,
            edit: None,
        }
    }
    pub fn graph() -> Self {
        Self {
            kind: TabKind::Graph,
            mode: ViewMode::Read,
            edit: None,
        }
    }
    pub fn path(&self) -> Option<&str> {
        match &self.kind {
            TabKind::Note(p) => Some(p),
            TabKind::Graph => None,
        }
    }
}

#[derive(Clone)]
pub struct Pane {
    pub tabs: Vec<Tab>,
    pub active: usize,
}

impl Pane {
    pub fn with_note(rel: &str) -> Self {
        Self {
            tabs: vec![Tab::note(rel, ViewMode::Read)],
            active: 0,
        }
    }
    pub fn current(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }
    pub fn current_path(&self) -> Option<&str> {
        self.current().and_then(|t| t.path())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    Commands,
    Files,
}

pub struct PaletteState {
    pub kind: PaletteKind,
    pub query: String,
    pub selected: usize,
    pub focus: bool,
}

pub struct RenameState {
    pub idx: usize,
    pub value: String,
    pub focus: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CreateKind {
    Note,
    Folder,
}

pub struct CreateState {
    pub kind: CreateKind,
    pub folder: String,
    pub value: String,
    pub focus: bool,
}

pub struct Shot {
    pub path: PathBuf,
    pub requested: bool,
    pub done: Arc<AtomicBool>,
}

#[derive(Clone)]
pub enum Action {
    Open(usize),
    OpenNewTab(usize),
    ToggleFolder(String),
    CreateNote(String),
    CreateFolder(String),
    Rename(usize),
    Delete(usize),
    ToggleBookmark(String),
    CloseTab(usize, usize),
    CloseOtherTabs(usize, usize),
    CloseTabsRight(usize, usize),
    CloseAllTabs(usize),
    DeleteFolder(String),
    /// Switch one tab between reading and editing. Carries the pane and tab
    /// because the view-header button and the right-click menu must act on the
    /// pane that was clicked, not whichever pane happens to be active.
    ToggleMode(usize, usize),
    OpenGraph,
    ScrollTo(usize, usize, usize),
    Toast(String),
}

pub struct App {
    pub vault: vault::Vault,
    pub pal: Palette,
    pub dark: bool,
    pub accent_h: f32,
    pub base_size: f32,
    pub left_open: bool,
    pub left_width: f32,
    pub left_tab: LeftTab,
    pub right_open: bool,
    pub right_width: f32,
    pub right_tab: RightTab,
    pub panes: Vec<Pane>,
    pub active_pane: usize,
    pub search_query: String,
    pub match_case: bool,
    pub bookmarks: Vec<String>,
    pub palette: Option<PaletteState>,
    pub settings_open: bool,
    pub settings_section: usize,
    pub rename: Option<RenameState>,
    pub create: Option<CreateState>,
    pub notice: Option<(String, f64)>,
    pub graph: graph::GraphState,
    pub shot: Option<Shot>,
    pub frames: u64,
    pub pending_save: Option<(usize, String, f64)>,
    /// Editor to focus after this frame (a newly created note, or Ctrl+E).
    pub focus_editor: Option<(usize, usize)>,
    pub scroll_to: Option<(usize, usize, usize)>,
    pub show_about: bool,
    pub title_dirty: bool,
    /// Last title sent to the OS, so we only call SetWindowText when it changes
    /// (native `sub_1411c1210` compares GetWindowText before SetWindowTextW).
    pub last_title: String,
    /// The original stores a per-vault zoom level; levels are 0.5 apart, clamped to [-2.5, 3]
    /// and the factor is 1.2^level (the original/main.js `setFrameZoomLevel` callers).
    pub zoom_level: f32,
    /// `frame` config: "native" keeps the OS frame, otherwise the title bar is hidden
    /// (the original/main.js: `Ve = F.frame === "native"; Je = Ve ? "default" : "hidden"`).
    pub native_frame: bool,
    pub window_w: f32,
    pub window_h: f32,
    pub window_max: bool,
    pub last_size: Option<(f32, f32)>,
    pub size_check_at: f64,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, vault_path: PathBuf, shot: Option<PathBuf>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let mut app = Self {
            vault: vault::Vault::load(&vault_path),
            pal: Palette::dark().with_accent_hue(258.0),
            dark: true,
            accent_h: 258.0,
            base_size: 16.0,
            left_open: true,
            left_width: SIDEBAR_W,
            left_tab: LeftTab::Files,
            right_open: false,
            right_width: SIDEBAR_W,
            right_tab: RightTab::Backlinks,
            panes: Vec::new(),
            active_pane: 1,
            search_query: String::new(),
            match_case: false,
            bookmarks: Vec::new(),
            palette: None,
            settings_open: false,
            settings_section: 0,
            rename: None,
            create: None,
            notice: None,
            graph: graph::GraphState::default(),
            shot: shot.map(|p| Shot {
                path: p,
                requested: false,
                done: Arc::new(AtomicBool::new(false)),
            }),
            frames: 0,
            pending_save: None,
            focus_editor: None,
            scroll_to: None,
            show_about: false,
            title_dirty: true,
            last_title: String::new(),
            zoom_level: 0.0,
            native_frame: false,
            window_w: 800.0,
            window_h: 600.0,
            window_max: false,
            last_size: None,
            size_check_at: 0.0,
        };
        app.load_settings();
        theme::apply_style(&cc.egui_ctx, &app.pal);
        if app.zoom_level != 0.0 {
            cc.egui_ctx.set_zoom_factor(1.2f32.powf(app.zoom_level));
        }

        let start = app
            .vault
            .find_by_title("欢迎")
            .or_else(|| if app.vault.notes.is_empty() { None } else { Some(0) });
        if let Some(idx) = start {
            let rel = app.vault.notes[idx].rel.clone();
            app.panes = vec![Pane::with_note(&rel), Pane::with_note(&rel)];
            app.active_pane = 1;
        } else {
            app.panes = vec![Pane::with_note(""), Pane::with_note("")];
            app.active_pane = 1;
        }
        app
    }

    // -------------------------------------------------------------- settings

    fn settings_path() -> PathBuf {
        settings_file()
    }

    fn load_settings(&mut self) {
        let Ok(text) = std::fs::read_to_string(Self::settings_path()) else {
            return;
        };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "dark" => self.dark = v.trim() == "1",
                "accent" => self.accent_h = v.trim().parse().unwrap_or(258.0),
                "font" => self.base_size = v.trim().parse().unwrap_or(16.0),
                "left_width" => self.left_width = v.trim().parse().unwrap_or(SIDEBAR_W),
                "right_width" => self.right_width = v.trim().parse().unwrap_or(SIDEBAR_W),
                "left_open" => self.left_open = v.trim() != "0",
                "right_open" => self.right_open = v.trim() == "1",
                "zoom" => self.zoom_level = clamp_zoom(v.trim().parse().unwrap_or(0.0)),
                "native_frame" => self.native_frame = v.trim() == "1",
                "window_w" => self.window_w = v.trim().parse().unwrap_or(800.0),
                "window_h" => self.window_h = v.trim().parse().unwrap_or(600.0),
                "window_max" => self.window_max = v.trim() == "1",
                "bookmark" => self.bookmarks.push(v.trim().to_string()),
                _ => {}
            }
        }
        // Native `sub_1411ad730` treats a zero width *or* zero height as "use
        // CW_USEDEFAULT for x, y, width and height"; the BrowserWindow minimum
        // from main.js is 200x150. A saved 0 must not collapse the window.
        let (w, h) = restored_window_size(self.window_w, self.window_h);
        self.window_w = w;
        self.window_h = h;
        self.zoom_level = clamp_zoom(self.zoom_level);
        self.pal = Palette::for_dark(self.dark).with_accent_hue(self.accent_h);
    }

    pub fn save_settings(&self) {
        let mut out = String::new();
        out.push_str(&format!("dark={}\n", if self.dark { 1 } else { 0 }));
        out.push_str(&format!("accent={}\n", self.accent_h));
        out.push_str(&format!("font={}\n", self.base_size));
        out.push_str(&format!("left_width={}\n", self.left_width));
        out.push_str(&format!("right_width={}\n", self.right_width));
        out.push_str(&format!("left_open={}\n", if self.left_open { 1 } else { 0 }));
        out.push_str(&format!("right_open={}\n", if self.right_open { 1 } else { 0 }));
        out.push_str(&format!("zoom={}\n", self.zoom_level));
        out.push_str(&format!("native_frame={}\n", if self.native_frame { 1 } else { 0 }));
        out.push_str(&format!("window_w={}\n", self.window_w));
        out.push_str(&format!("window_h={}\n", self.window_h));
        out.push_str(&format!("window_max={}\n", if self.window_max { 1 } else { 0 }));
        for b in &self.bookmarks {
            out.push_str(&format!("bookmark={b}\n"));
        }
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, out);
    }

    pub fn set_theme(&mut self, dark: bool, ctx: &egui::Context) {
        self.dark = dark;
        self.pal = Palette::for_dark(dark).with_accent_hue(self.accent_h);
        theme::apply_style(ctx, &self.pal);
        self.save_settings();
    }

    pub fn toast(&mut self, msg: impl Into<String>, now: f64) {
        self.notice = Some((msg.into(), now + 2.4));
    }

    // -------------------------------------------------------------- vault ops

    pub fn open_action(&mut self, action: Action, now: f64) {
        match action {
            Action::Open(idx) => self.open_note(idx, false),
            Action::OpenNewTab(idx) => self.open_note(idx, true),
            Action::ToggleFolder(rel) => {
                if self.vault.expanded.contains(&rel) {
                    self.vault.expanded.remove(&rel);
                } else {
                    self.vault.expanded.insert(rel);
                }
            }
            Action::CreateNote(folder) => {
                self.create = Some(CreateState {
                    kind: CreateKind::Note,
                    folder,
                    value: String::new(),
                    focus: false,
                });
            }
            Action::CreateFolder(folder) => {
                self.create = Some(CreateState {
                    kind: CreateKind::Folder,
                    folder,
                    value: String::new(),
                    focus: false,
                });
            }
            Action::Rename(idx) => {
                if let Some(note) = self.vault.notes.get(idx) {
                    self.rename = Some(RenameState {
                        idx,
                        value: note.title.clone(),
                        focus: false,
                    });
                }
            }
            Action::Delete(idx) => {
                if let Some(note) = self.vault.notes.get(idx).cloned() {
                    match self.vault.delete_note(idx) {
                        Ok(()) => {
                            for pane in &mut self.panes {
                                pane.tabs.retain(|t| t.path() != Some(note.rel.as_str()));
                                if pane.active >= pane.tabs.len() && !pane.tabs.is_empty() {
                                    pane.active = pane.tabs.len() - 1;
                                }
                            }
                            self.toast(format!("已删除 {}", note.title), now);
                            self.title_dirty = true;
                        }
                        Err(e) => self.toast(format!("删除失败：{e}"), now),
                    }
                }
            }
            Action::ToggleBookmark(rel) => {
                if let Some(pos) = self.bookmarks.iter().position(|b| b == &rel) {
                    self.bookmarks.remove(pos);
                } else {
                    self.bookmarks.push(rel);
                }
                self.save_settings();
            }
            Action::CloseTab(pane, tab) => {
                if let Some(p) = self.panes.get_mut(pane) {
                    if tab < p.tabs.len() {
                        p.tabs.remove(tab);
                        if p.active >= p.tabs.len() && !p.tabs.is_empty() {
                            p.active = p.tabs.len() - 1;
                        }
                    }
                }
                self.title_dirty = true;
            }
            Action::CloseOtherTabs(pane, keep) => {
                if let Some(p) = self.panes.get_mut(pane) {
                    if let Some(tab) = p.tabs.get(keep).cloned() {
                        p.tabs = vec![tab];
                        p.active = 0;
                    }
                }
                self.title_dirty = true;
            }
            Action::CloseTabsRight(pane, keep) => {
                if let Some(p) = self.panes.get_mut(pane) {
                    if keep + 1 < p.tabs.len() {
                        p.tabs.truncate(keep + 1);
                        if p.active >= p.tabs.len() && !p.tabs.is_empty() {
                            p.active = p.tabs.len() - 1;
                        }
                    }
                }
                self.title_dirty = true;
            }
            Action::CloseAllTabs(pane) => {
                if let Some(p) = self.panes.get_mut(pane) {
                    p.tabs.clear();
                    p.active = 0;
                }
                self.title_dirty = true;
            }
            Action::DeleteFolder(rel) => {
                match self.vault.delete_folder(&rel) {
                    Ok(()) => {
                        let prefix = format!("{rel}/");
                        for pane in &mut self.panes {
                            pane.tabs.retain(|t| {
                                !t.path().map(|p| p.starts_with(&prefix)).unwrap_or(false)
                            });
                            if pane.active >= pane.tabs.len() && !pane.tabs.is_empty() {
                                pane.active = pane.tabs.len() - 1;
                            }
                        }
                        self.toast(format!("已删除文件夹 {rel}"), now);
                        self.title_dirty = true;
                    }
                    Err(e) => self.toast(format!("删除失败：{e}"), now),
                }
            }
            Action::ToggleMode(pane, tab) => self.toggle_mode(pane, tab, now),
            Action::Toast(msg) => self.toast(msg, now),
            Action::OpenGraph => {
                let pane = self.active_pane.min(self.panes.len().saturating_sub(1));
                if let Some(p) = self.panes.get_mut(pane) {
                    if let Some(pos) = p.tabs.iter().position(|t| t.kind == TabKind::Graph) {
                        p.active = pos;
                    } else {
                        p.tabs.push(Tab::graph());
                        p.active = p.tabs.len() - 1;
                    }
                }
                self.title_dirty = true;
            }
            Action::ScrollTo(pane, tab, heading) => {
                self.scroll_to = Some((pane, tab, heading));
            }
        }
    }

    pub fn open_note(&mut self, idx: usize, new_tab: bool) {
        let Some(note) = self.vault.notes.get(idx) else {
            return;
        };
        let rel = note.rel.clone();
        let pane_idx = self.active_pane.min(self.panes.len().saturating_sub(1));
        let Some(pane) = self.panes.get_mut(pane_idx) else {
            return;
        };
        if new_tab || pane.tabs.is_empty() {
            if let Some(pos) = pane.tabs.iter().position(|t| t.path() == Some(rel.as_str())) {
                pane.active = pos;
            } else {
                pane.tabs.push(Tab::note(&rel, ViewMode::Read));
                pane.active = pane.tabs.len() - 1;
            }
        } else {
            let t = &mut pane.tabs[pane.active];
            t.kind = TabKind::Note(rel.clone());
            t.mode = ViewMode::Read;
            t.edit = None;
        }
        self.title_dirty = true;
    }

    pub fn open_rel(&mut self, rel: &str) {
        if let Some(idx) = self.vault.find_by_rel(rel) {
            self.open_note(idx, false);
        }
    }

    /// Open a file straight into a view mode. New notes use editing mode, as
    /// the original's `defaultViewMode` defaults to `source`, and the editor is
    /// focused so the person can start typing immediately.
    pub fn open_rel_in_mode(&mut self, rel: &str, mode: ViewMode) {
        if let Some(idx) = self.vault.find_by_rel(rel) {
            self.open_note(idx, false);
            let pane_idx = self.active_pane.min(self.panes.len().saturating_sub(1));
            let tab_index = self.panes.get(pane_idx).map(|p| p.active).unwrap_or(0);
            if let Some(tab) = self
                .panes
                .get_mut(pane_idx)
                .and_then(|p| p.tabs.get_mut(p.active))
            {
                tab.mode = mode;
                tab.edit = None;
                if mode == ViewMode::Edit {
                    self.focus_editor = Some((pane_idx, tab_index));
                }
            }
        }
    }

    pub fn open_link(&mut self, target: &markdown::LinkTarget, now: f64) {
        match target {
            markdown::LinkTarget::Internal(name) => {
                if let Some(idx) = self.vault.resolve(name) {
                    self.open_note(idx, false);
                } else {
                    match self.vault.create_note("", name) {
                        Ok(rel) => {
                            self.vault.refresh();
                            self.open_rel_in_mode(&rel, ViewMode::Edit);
                            self.toast(format!("已创建 {name}"), now);
                        }
                        Err(e) => self.toast(format!("创建失败：{e}"), now),
                    }
                }
            }
            markdown::LinkTarget::External(url) => {
                self.toast(format!("外部链接：{url}"), now);
                open_external(url);
            }
        }
    }

    pub fn save_note_now(&mut self, idx: usize, text: String) {
        if let Some(note) = self.vault.notes.get(idx) {
            if note.content == text {
                return;
            }
        }
        if self.vault.save_note(idx, text).is_ok() {
            self.title_dirty = true;
        }
    }

    pub fn active_note_idx(&self) -> Option<usize> {
        let pane = self.panes.get(self.active_pane)?;
        let tab = pane.current()?;
        let path = tab.path()?;
        self.vault.find_by_rel(path)
    }

    pub fn toggle_mode(&mut self, pane_idx: usize, tab_index: usize, now: f64) {
        let pane_idx = pane_idx.min(self.panes.len().saturating_sub(1));
        let Some(pane) = self.panes.get(pane_idx) else {
            return;
        };
        let Some(tab) = pane.tabs.get(tab_index) else {
            return;
        };
        let Some(rel) = tab.path().map(|s| s.to_string()) else {
            return;
        };
        let mode = tab.mode;
        let edit_text = tab.edit.as_ref().map(|e| e.text.clone());
        let Some(idx) = self.vault.find_by_rel(&rel) else {
            return;
        };
        self.active_pane = pane_idx;
        match mode {
            ViewMode::Read => {
                let content = self.vault.notes[idx].content.clone();
                if let Some(t) = self
                    .panes
                    .get_mut(pane_idx)
                    .and_then(|p| p.tabs.get_mut(tab_index))
                {
                    t.mode = ViewMode::Edit;
                    t.edit = Some(EditBuf {
                        text: content,
                        last_edit: now,
                    });
                }
                // The original focuses the editor after switching, so typing works
                // without an extra click.
                self.focus_editor = Some((pane_idx, tab_index));
            }
            ViewMode::Edit => {
                if let Some(text) = edit_text {
                    self.save_note_now(idx, text);
                }
                if let Some(t) = self
                    .panes
                    .get_mut(pane_idx)
                    .and_then(|p| p.tabs.get_mut(tab_index))
                {
                    t.mode = ViewMode::Read;
                    t.edit = None;
                }
            }
        }
        self.title_dirty = true;
    }

    pub fn set_zoom(&mut self, ctx: &egui::Context, level: f32) {
        self.zoom_level = normalize_zoom(level);
        ctx.set_zoom_factor(1.2f32.powf(self.zoom_level));
        self.save_settings();
    }

    pub fn change_zoom(&mut self, ctx: &egui::Context, delta: f32) {
        self.set_zoom(ctx, self.zoom_level + delta);
    }

    pub fn toggle_frame(&mut self, ctx: &egui::Context) {
        self.native_frame = !self.native_frame;
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(self.native_frame));
        self.save_settings();
        self.title_dirty = true;
    }

    // -------------------------------------------------------------- commands

    pub fn run_command(&mut self, name: &str, ctx: &egui::Context, now: f64) {
        match name {
            "新建笔记" => {
                self.create = Some(CreateState {
                    kind: CreateKind::Note,
                    folder: String::new(),
                    value: String::new(),
                    focus: false,
                });
            }
            "新建文件夹" => {
                self.create = Some(CreateState {
                    kind: CreateKind::Folder,
                    folder: String::new(),
                    value: String::new(),
                    focus: false,
                });
            }
            "打开快速切换" => {
                self.palette = Some(PaletteState {
                    kind: PaletteKind::Files,
                    query: String::new(),
                    selected: 0,
                    focus: false,
                });
            }
            "打开命令面板" => {
                self.palette = Some(PaletteState {
                    kind: PaletteKind::Commands,
                    query: String::new(),
                    selected: 0,
                    focus: false,
                });
            }
            "搜索" => {
                self.left_open = true;
                self.left_tab = LeftTab::Search;
            }
            "关系图谱" => self.open_action(Action::OpenGraph, now),
            "切换编辑/阅读视图" => {
                let pane = self.active_pane.min(self.panes.len().saturating_sub(1));
                if let Some(tab) = self.panes.get(pane).map(|p| p.active) {
                    self.toggle_mode(pane, tab, now);
                }
            }
            "关闭当前标签页" => {
                let pane = self.active_pane.min(self.panes.len().saturating_sub(1));
                if let Some(p) = self.panes.get(pane) {
                    if !p.tabs.is_empty() {
                        self.open_action(Action::CloseTab(pane, p.active), now);
                    }
                }
            }
            "保存当前文件" => {
                if let Some(idx) = self.active_note_idx() {
                    let text = self
                        .panes
                        .get(self.active_pane)
                        .and_then(|p| p.current())
                        .and_then(|t| t.edit.as_ref())
                        .map(|e| e.text.clone())
                        .unwrap_or_else(|| self.vault.notes[idx].content.clone());
                    self.save_note_now(idx, text);
                    self.toast("已保存", now);
                }
            }
            "重命名当前文件" => {
                if let Some(idx) = self.active_note_idx() {
                    self.open_action(Action::Rename(idx), now);
                }
            }
            "删除当前文件" => {
                if let Some(idx) = self.active_note_idx() {
                    self.open_action(Action::Delete(idx), now);
                }
            }
            "添加/移除书签" => {
                if let Some(idx) = self.active_note_idx() {
                    let rel = self.vault.notes[idx].rel.clone();
                    self.open_action(Action::ToggleBookmark(rel), now);
                }
            }
            "切换深色/浅色主题" => {
                let dark = !self.dark;
                self.set_theme(dark, ctx);
            }
            "放大" => self.change_zoom(ctx, 0.5),
            "缩小" => self.change_zoom(ctx, -0.5),
            "实际大小" => self.set_zoom(ctx, 0.0),
            "切换窗口边框" => self.toggle_frame(ctx),
            "打开设置" => self.settings_open = true,
            "打开今天的日记" => {
                let today = today_string();
                match self.vault.create_note("", &today) {
                    Ok(rel) => {
                        self.vault.refresh();
                        self.open_rel_in_mode(&rel, ViewMode::Edit);
                    }
                    Err(e) => self.toast(format!("创建失败：{e}"), now),
                }
            }
            "打开其他仓库文件夹…" => {
                if let Some(path) = pick_folder() {
                    let mut v = vault::Vault::load(&path);
                    if v.notes.is_empty() {
                        seed_vault(&path);
                        v = vault::Vault::load(&path);
                    }
                    self.vault = v;
                    self.panes = vec![Pane::with_note(""), Pane::with_note("")];
                    if !self.vault.notes.is_empty() {
                        let rel = self.vault.notes[0].rel.clone();
                        self.panes = vec![Pane::with_note(&rel), Pane::with_note(&rel)];
                    }
                    self.title_dirty = true;
                    self.toast("已打开仓库", now);
                }
            }
            "帮助" => self.show_about = true,
            _ => {}
        }
    }

    pub fn commands(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("新建笔记", "Ctrl+N"),
            ("新建文件夹", ""),
            ("打开快速切换", "Ctrl+O"),
            ("打开命令面板", "Ctrl+P"),
            ("搜索", "Ctrl+Shift+F"),
            ("关系图谱", "Ctrl+G"),
            ("切换编辑/阅读视图", "Ctrl+E"),
            ("关闭当前标签页", "Ctrl+W"),
            ("保存当前文件", "Ctrl+S"),
            ("重命名当前文件", "F2"),
            ("删除当前文件", ""),
            ("添加/移除书签", ""),
            ("切换深色/浅色主题", ""),
            ("放大", "Ctrl+="),
            ("缩小", "Ctrl+-"),
            ("实际大小", "Ctrl+0"),
            ("切换窗口边框", ""),
            ("打开今天的日记", ""),
            ("打开其他仓库文件夹…", ""),
            ("打开设置", "Ctrl+,"),
            ("帮助", ""),
        ]
    }

    // -------------------------------------------------------------- shortcuts

    fn shortcuts(&mut self, ctx: &egui::Context, now: f64) {
        let (
            cmd,
            shift,
            key_p,
            key_o,
            key_n,
            key_e,
            key_w,
            key_s,
            key_g,
            key_f,
            key_comma,
            f2,
            esc,
        ) = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.shift,
                i.key_pressed(egui::Key::P),
                i.key_pressed(egui::Key::O),
                i.key_pressed(egui::Key::N),
                i.key_pressed(egui::Key::E),
                i.key_pressed(egui::Key::W),
                i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::G),
                i.key_pressed(egui::Key::F),
                i.key_pressed(egui::Key::Comma),
                i.key_pressed(egui::Key::F2),
                i.key_pressed(egui::Key::Escape),
            )
        });

        if esc {
            self.palette = None;
            self.settings_open = false;
            self.show_about = false;
            self.rename = None;
            self.create = None;
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if cmd && key_p {
            self.palette = Some(PaletteState {
                kind: PaletteKind::Commands,
                query: String::new(),
                selected: 0,
                focus: false,
            });
        }
        if cmd && key_o {
            self.palette = Some(PaletteState {
                kind: PaletteKind::Files,
                query: String::new(),
                selected: 0,
                focus: false,
            });
        }
        if cmd && shift && key_f {
            self.left_open = true;
            self.left_tab = LeftTab::Search;
        }
        let (zoom_in, zoom_out, zoom_reset) = ctx.input(|i| {
            (
                i.modifiers.command
                    && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)),
                i.modifiers.command && i.key_pressed(egui::Key::Minus),
                i.modifiers.command && i.key_pressed(egui::Key::Num0),
            )
        });
        if zoom_in {
            self.change_zoom(ctx, 0.5);
        }
        if zoom_out {
            self.change_zoom(ctx, -0.5);
        }
        if zoom_reset {
            self.set_zoom(ctx, 0.0);
        }
        // Ctrl+E must keep working while the editor has focus: The original toggles
        // reading/editing from inside the note, and blocking it made the app
        // feel like the text could not be edited.
        if cmd && key_e {
            let pane = self.active_pane.min(self.panes.len().saturating_sub(1));
            if let Some(tab) = self.panes.get(pane).map(|p| p.active) {
                self.toggle_mode(pane, tab, now);
            }
        }
        // Saving is safe from inside the editor too (The original keeps Ctrl+S there).
        if cmd && key_s {
            self.run_command("保存当前文件", ctx, now);
        }
        if typing {
            return;
        }
        if cmd && key_n {
            self.create = Some(CreateState {
                kind: CreateKind::Note,
                folder: String::new(),
                value: String::new(),
                focus: false,
            });
        }
        if cmd && key_w {
            let pane = self.active_pane.min(self.panes.len().saturating_sub(1));
            if let Some(p) = self.panes.get(pane) {
                if !p.tabs.is_empty() {
                    self.open_action(Action::CloseTab(pane, p.active), now);
                }
            }
        }
        if cmd && key_g {
            self.run_command("关系图谱", ctx, now);
        }
        if cmd && key_comma {
            self.settings_open = true;
        }
        if f2 {
            self.run_command("重命名当前文件", ctx, now);
        }
    }

    // -------------------------------------------------------------- helpers

    pub fn vault_name(&self) -> String {
        self.vault
            .root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "仓库".to_string())
    }

    pub fn icon_btn(
        &self,
        ui: &mut Ui,
        rect: Rect,
        icon: Icon,
        tint: Color32,
        active: bool,
        id: impl std::hash::Hash + std::fmt::Debug,
    ) -> egui::Response {
        let resp = ui.interact(rect, Id::new(id), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(rect, 6.0, self.pal.bg_hover);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let size = rect.width().min(rect.height());
        let inner = Rect::from_center_size(rect.center(), egui::vec2(size * 0.46, size * 0.46));
        icons::draw(
            &ui.painter().clone(),
            inner,
            icon,
            if active { self.pal.text } else { tint },
        );
        resp
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frames = self.frames.wrapping_add(1);
        let now = ctx.input(|i| i.time);

        // Window bounds are remembered like the original: only while neither maximized
        // nor minimized, and at most once a second.
        if now - self.size_check_at >= 1.0 {
            self.size_check_at = now;
            let (inner, maximized, minimized) = ctx.input(|i| {
                (
                    i.viewport().inner_rect,
                    i.viewport().maximized.unwrap_or(false),
                    i.viewport().minimized.unwrap_or(false),
                )
            });
            if maximized {
                if !self.window_max {
                    self.window_max = true;
                    self.save_settings();
                }
            } else if let (Some(rect), false) = (inner, minimized) {
                let size = (rect.width().round(), rect.height().round());
                if self.last_size != Some(size) && size.0 >= 200.0 && size.1 >= 150.0 {
                    self.last_size = Some(size);
                    self.window_w = size.0;
                    self.window_h = size.1;
                    self.window_max = false;
                    self.save_settings();
                }
            }
        }

        if let Some((idx, text, when)) = self.pending_save.clone() {
            if now >= when {
                self.save_note_now(idx, text);
                self.pending_save = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(120));
            }
        }

        self.shortcuts(&ctx, now);

        if self.title_dirty {
            let note = self
                .active_note_idx()
                .and_then(|i| self.vault.notes.get(i))
                .map(|n| n.title.clone())
                .unwrap_or_else(|| self.vault_name());
            let title = format!("{note} - {} - ObsidianCraft", self.vault_name());
            // Only touch the window title when it actually changed. Native
            // `sub_1411c1210` compares GetWindowTextLengthW first, then every
            // UTF-16 code unit, and calls SetWindowTextW only on a difference.
            if window_title_changed(&self.last_title, &title) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
                self.last_title = title;
            }
            self.title_dirty = false;
        }

        let pal = self.pal;
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, pal.bg_primary);
        self.top_bar(ui, full, now);
        self.body(ui, full, now);
        self.status_bar(ui, full, now);
        self.overlays(ui, &ctx, full, now);
        self.window_resize(ui, full);

        // screenshot mode: draw ~120 frames, save the window picture, exit
        if let Some(shot) = &mut self.shot {
            if !shot.requested && self.frames >= 120 {
                shot.requested = true;
                let path = shot.path.clone();
                let done = shot.done.clone();
                ctx.request_screenshot(move |image| {
                    let size = image.size;
                    let mut rgba: Vec<u8> = Vec::with_capacity(size[0] * size[1] * 4);
                    for px in &image.pixels {
                        let [r, g, b, a] = px.to_srgba_unmultiplied();
                        rgba.extend_from_slice(&[r, g, b, a]);
                    }
                    let _ = png::write_png(&path, size[0], size[1], &rgba);
                    done.store(true, Ordering::SeqCst);
                });
            }
            if shot.requested && shot.done.load(Ordering::SeqCst) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint();
            }
        }
    }
}

// ------------------------------------------------------------------ helpers

pub fn settings_file() -> PathBuf {
    std::env::var("APPDATA")
        .map(|p| PathBuf::from(p).join("ObsidianCraft").join("settings.txt"))
        .unwrap_or_else(|_| PathBuf::from("obsidiancraft-settings.txt"))
}

pub fn today_string() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn open_external(url: &str) {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
}

fn pick_folder() -> Option<PathBuf> {
    let script = r#"Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description = '选择仓库文件夹'; if ($d.ShowDialog() -eq 'OK') { Write-Output $d.SelectedPath }"#;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-STA", "-Command", script])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    }
}

fn find_default_vault() -> PathBuf {
    if let Ok(p) = std::env::var("OBSIDIANCRAFT_VAULT") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..5 {
            if let Some(d) = &dir {
                let cand = d.join("vault");
                if cand.is_dir() && has_notes(&cand) {
                    return cand;
                }
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
        if let Some(d) = exe.parent() {
            return d.join("vault");
        }
    }
    PathBuf::from("vault")
}

fn has_notes(dir: &Path) -> bool {
    fn scan(dir: &Path, depth: usize) -> bool {
        if depth > 3 {
            return false;
        }
        if let Ok(read) = std::fs::read_dir(dir) {
            for e in read.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let p = e.path();
                if p.is_dir() {
                    if scan(&p, depth + 1) {
                        return true;
                    }
                } else if name.to_lowercase().ends_with(".md") {
                    return true;
                }
            }
        }
        false
    }
    scan(dir, 0)
}

fn seed_vault(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    if has_notes(dir) {
        return;
    }
    let welcome = "这是你的新*仓库*。\n\n写点笔记，[[创建链接]]，或者试一试[[创建链接]]!\n\n当你准备好了，就将该笔记文件删除，使这个仓库为你所用。\n";
    let _ = std::fs::write(dir.join("欢迎.md"), welcome);
    let _ = std::fs::write(dir.join("创建链接.md"), "");
}

fn parse_args() -> (Option<PathBuf>, Option<PathBuf>) {
    let mut screenshot: Option<PathBuf> = None;
    let mut vault: Option<PathBuf> = None;
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                if i + 1 < args.len() {
                    screenshot = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--vault" => {
                if i + 1 < args.len() {
                    vault = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    (screenshot, vault)
}

/// Window bounds restore rule from native `sub_1411ad730` (0x1411ad730):
/// `if (width == 0 || height == 0) { x = y = width = height = CW_USEDEFAULT; }`
/// with the BrowserWindow defaults 800x600 and minimum 200x150 from
/// `the original/main.js` `he()`. The 8192 ceiling is only defensive; the
/// original leaves the upper bound to Windows.
pub fn restored_window_size(w: f32, h: f32) -> (f32, f32) {
    if w <= 0.0 || h <= 0.0 {
        return (800.0, 600.0);
    }
    (w.clamp(200.0, 8192.0), h.clamp(150.0, 8192.0))
}

/// Display zoom range and step from `main.js`: `Math.max(-2.5, Math.min(3, level))`
/// and 0.5-level menu increments. The factor is `1.2^level`.
pub fn clamp_zoom(level: f32) -> f32 {
    level.clamp(-2.5, 3.0)
}

pub fn normalize_zoom(level: f32) -> f32 {
    clamp_zoom((level * 2.0).round() / 2.0)
}

/// Native `sub_1411c1210` (0x1411c1210) only calls `SetWindowTextW` after
/// `GetWindowTextLengthW` differs, or after comparing every UTF-16 code unit.
/// A string-equality check has the same result for valid UTF-16 titles.
pub fn window_title_changed(current: &str, next: &str) -> bool {
    let cur: Vec<u16> = current.encode_utf16().collect();
    let nxt: Vec<u16> = next.encode_utf16().collect();
    if cur.len() != nxt.len() {
        return true;
    }
    cur != nxt
}

fn main() -> eframe::Result {
    let (screenshot, vault_arg) = parse_args();
    let vault_path = vault_arg.unwrap_or_else(find_default_vault);
    seed_vault(&vault_path);

    // Real defaults from the original/main.js (`he()`): 800x600, min 200x150,
    // hidden title bar unless the "native frame" setting is on.
    let prefs = std::fs::read_to_string(settings_file()).unwrap_or_default();
    let pref = |key: &str| -> Option<String> {
        prefs
            .lines()
            .filter_map(|l| l.split_once('='))
            .find(|(k, _)| k.trim() == key)
            .map(|(_, v)| v.trim().to_string())
    };
    let saved_w = pref("window_w")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(800.0);
    let saved_h = pref("window_h")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(600.0);
    // Native rule: a zero width or height means CW_USEDEFAULT for all bounds,
    // which is the 800x600 BrowserWindow default here.
    let (win_w, win_h) = restored_window_size(saved_w, saved_h);
    let native = pref("native_frame").map(|v| v == "1").unwrap_or(false);
    let maximized = pref("window_max").map(|v| v == "1").unwrap_or(false);

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([win_w, win_h])
        .with_min_inner_size([200.0, 150.0])
        .with_decorations(native)
        .with_transparent(true)
        .with_maximized(maximized)
        .with_title("ObsidianCraft");
    if screenshot.is_some() {
        viewport = viewport
            .with_inner_size([800.0, 600.0])
            .with_maximized(false);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let shot = screenshot.clone();
    eframe::run_native(
        "obsidiancraft",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, vault_path.clone(), shot.clone())))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        use std::sync::atomic::AtomicU64;
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-vaults")
            .join(format!(
                "{}-{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::SeqCst)
            ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "alpha").unwrap();
        std::fs::write(dir.join("b.md"), "beta").unwrap();
        let vault = vault::Vault::load(&dir);
        let a = vault
            .notes
            .iter()
            .find(|n| n.title == "a")
            .map(|n| n.rel.clone())
            .unwrap();
        App {
            vault,
            pal: Palette::dark(),
            dark: true,
            accent_h: 258.0,
            base_size: 16.0,
            left_open: true,
            left_width: SIDEBAR_W,
            left_tab: LeftTab::Files,
            right_open: false,
            right_width: SIDEBAR_W,
            right_tab: RightTab::Backlinks,
            panes: vec![Pane::with_note(&a), Pane::with_note(&a)],
            active_pane: 1,
            search_query: String::new(),
            match_case: false,
            bookmarks: Vec::new(),
            palette: None,
            settings_open: false,
            settings_section: 0,
            rename: None,
            create: None,
            notice: None,
            graph: graph::GraphState::default(),
            shot: None,
            frames: 0,
            pending_save: None,
            focus_editor: None,
            scroll_to: None,
            show_about: false,
            title_dirty: true,
            last_title: String::new(),
            zoom_level: 0.0,
            native_frame: false,
            window_w: 800.0,
            window_h: 600.0,
            window_max: false,
            last_size: None,
            size_check_at: 0.0,
        }
    }

    /// The editor button/contents must act on the pane that was clicked. Before
    /// this, `ToggleMode` used `active_pane` (the other pane at startup), so the
    /// note under the pointer never switched to editing.
    #[test]
    fn toggle_mode_targets_the_given_tab_not_the_active_pane() {
        let mut app = test_app();
        app.active_pane = 1;
        let tab0 = app.panes[0].active;
        let tab1 = app.panes[1].active;
        app.toggle_mode(0, tab0, 1.0);
        assert_eq!(app.panes[0].tabs[tab0].mode, ViewMode::Edit);
        assert_eq!(app.panes[1].tabs[tab1].mode, ViewMode::Read);
        assert_eq!(app.active_pane, 0);
        assert_eq!(app.focus_editor, Some((0, tab0)));
        app.toggle_mode(0, tab0, 2.0);
        assert_eq!(app.panes[0].tabs[tab0].mode, ViewMode::Read);
    }

    /// `sub_1411ad730`: zero width or height means CW_USEDEFAULT for x/y/w/h,
    /// so a saved zero must fall back to 800x600; the BrowserWindow minimum is
    /// 200x150, not the 320x240 the old restore clamp used.
    #[test]
    fn restore_uses_cw_usedefault_and_the_200x150_minimum() {
        assert_eq!(restored_window_size(0.0, 600.0), (800.0, 600.0));
        assert_eq!(restored_window_size(800.0, 0.0), (800.0, 600.0));
        assert_eq!(restored_window_size(0.0, 0.0), (800.0, 600.0));
        assert_eq!(restored_window_size(100.0, 100.0), (200.0, 150.0));
        assert_eq!(restored_window_size(1200.0, 800.0), (1200.0, 800.0));
    }

    /// `main.js` clamps the restored zoom level to [-2.5, 3]; menu steps are
    /// 0.5 and the factor is 1.2^level.
    #[test]
    fn zoom_restore_is_clamped_and_steps_are_half_level() {
        assert_eq!(clamp_zoom(9.0), 3.0);
        assert_eq!(clamp_zoom(-9.0), -2.5);
        assert_eq!(clamp_zoom(1.25), 1.25);
        assert_eq!(normalize_zoom(1.24), 1.0);
        assert_eq!(normalize_zoom(1.26), 1.5);
        assert_eq!(normalize_zoom(8.0), 3.0);
    }

    /// `sub_1411c1210`: compare the length, then every UTF-16 code unit; only
    /// call SetWindowTextW when something differs.
    #[test]
    fn title_is_only_set_when_its_utf16_units_change() {
        assert!(!window_title_changed("欢迎 - 仓库 - ObsidianCraft", "欢迎 - 仓库 - ObsidianCraft"));
        assert!(window_title_changed("欢迎 - 仓库 - ObsidianCraft", "创建链接 - 仓库 - Obsidian"));
        assert!(window_title_changed("a", "a "));
        assert!(window_title_changed("", "a"));
    }

    /// New notes open with the caret in the editor, as the original's default
    /// `defaultViewMode` is `source`.
    #[test]
    fn a_new_note_opens_in_editing_mode() {
        let mut app = test_app();
        let b = app
            .vault
            .notes
            .iter()
            .find(|n| n.title == "b")
            .map(|n| n.rel.clone())
            .unwrap();
        app.active_pane = 1;
        app.open_rel_in_mode(&b, ViewMode::Edit);
        let pane = &app.panes[1];
        assert_eq!(pane.current().unwrap().mode, ViewMode::Edit);
        assert_eq!(app.focus_editor, Some((1, pane.active)));
    }
}
