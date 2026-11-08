//! Colours, fonts and egui style for the Obsidian look.
//! Values come from the original app's stylesheet (app.css), see MECHANICS.md.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

pub const HEADER_H: f32 = 40.0; // --header-height
pub const RIBBON_W: f32 = 44.0; // --ribbon-width
pub const STATUS_H: f32 = 22.0; // .status-bar min-height 18 + padding
pub const SIDEBAR_W: f32 = 300.0;
pub const FILE_MARGIN: f32 = 32.0; // --file-margins 32px
pub const FILE_LINE_WIDTH: f32 = 700.0; // --file-line-width

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub bg_secondary_alt: Color32,
    pub bg_hover: Color32,
    pub bg_active: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub code_bg: Color32,
    pub red: Color32,
    pub yellow: Color32,
    pub green: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg_primary: Color32::from_rgb(0x1C, 0x1C, 0x1C),
            bg_secondary: Color32::from_rgb(0x28, 0x28, 0x28),
            bg_secondary_alt: Color32::from_rgb(0x23, 0x23, 0x23),
            bg_hover: Color32::from_rgb(0x33, 0x33, 0x33),
            bg_active: Color32::from_rgb(0x2E, 0x2E, 0x2E),
            border: Color32::from_rgb(0x33, 0x33, 0x33),
            border_strong: Color32::from_rgb(0x3F, 0x3F, 0x3F),
            text: Color32::from_rgb(0xDA, 0xDA, 0xDA),
            text_muted: Color32::from_rgb(0xB3, 0xB3, 0xB3),
            text_faint: Color32::from_rgb(0x66, 0x66, 0x66),
            accent: Color32::from_rgb(0x8A, 0x5C, 0xF5),
            accent_hover: Color32::from_rgb(0x9B, 0x74, 0xF7),
            accent_soft: Color32::from_rgba_unmultiplied(0x8A, 0x5C, 0xF5, 0x33),
            code_bg: Color32::from_rgb(0x23, 0x23, 0x23),
            red: Color32::from_rgb(0xFB, 0x46, 0x4C),
            yellow: Color32::from_rgb(0xE0, 0xDE, 0x71),
            green: Color32::from_rgb(0x44, 0xCF, 0x6E),
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg_primary: Color32::WHITE,
            bg_secondary: Color32::from_rgb(0xF6, 0xF6, 0xF6),
            bg_secondary_alt: Color32::from_rgb(0xFC, 0xFC, 0xFC),
            bg_hover: Color32::from_rgb(0xE4, 0xE4, 0xE4),
            bg_active: Color32::from_rgb(0xEF, 0xEF, 0xEF),
            border: Color32::from_rgb(0xE4, 0xE4, 0xE4),
            border_strong: Color32::from_rgb(0xDA, 0xDA, 0xDA),
            text: Color32::from_rgb(0x22, 0x22, 0x22),
            text_muted: Color32::from_rgb(0x5C, 0x5C, 0x5C),
            text_faint: Color32::from_rgb(0xAB, 0xAB, 0xAB),
            accent: Color32::from_rgb(0x8A, 0x5C, 0xF5),
            accent_hover: Color32::from_rgb(0x7A, 0x4B, 0xE8),
            accent_soft: Color32::from_rgba_unmultiplied(0x8A, 0x5C, 0xF5, 0x2E),
            code_bg: Color32::from_rgb(0xFA, 0xFA, 0xFA),
            red: Color32::from_rgb(0xE9, 0x31, 0x47),
            yellow: Color32::from_rgb(0xE0, 0xAC, 0x00),
            green: Color32::from_rgb(0x08, 0xB9, 0x4E),
        }
    }

    pub fn for_dark(dark: bool) -> Self {
        if dark { Self::dark() } else { Self::light() }
    }

    /// Accent colour with a custom hue (the original exposes --accent-h/s/l).
    pub fn with_accent_hue(mut self, h: f32) -> Self {
        self.accent = hsl(h, 0.88, 0.66);
        self.accent_hover = hsl(h + 2.0, 0.88, 0.72);
        let a = self.accent;
        self.accent_soft = Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 0x33);
        self
    }
}

pub fn hsl(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(to(r), to(g), to(b))
}

pub const BOLD_FAMILY: &str = "obsidian-bold";
pub const ITALIC_FAMILY: &str = "obsidian-italic";

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn font_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD_FAMILY.into()))
}
pub fn font_italic(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(ITALIC_FAMILY.into()))
}
pub fn font_mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

fn push_font(defs: &mut FontDefinitions, name: &str, path: &str, index: u32) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => {
            let blob: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
            defs.font_data
                .insert(name.to_string(), Arc::new(FontData::from_blob(blob, index)));
            true
        }
        Err(_) => false,
    }
}

/// Load the fonts the original uses: the system UI font (Segoe UI on Windows) with
/// Microsoft YaHei as the CJK fallback, plus real bold/italic faces and Consolas.
pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let win = "C:\\Windows\\Fonts\\";
    let have_ui = push_font(&mut defs, "ui", &format!("{win}segoeui.ttf"), 0);
    let have_ui_bold = push_font(&mut defs, "ui-bold", &format!("{win}segoeuib.ttf"), 0);
    let have_ui_italic = push_font(&mut defs, "ui-italic", &format!("{win}segoeuii.ttf"), 0);
    let have_cjk = push_font(&mut defs, "cjk", &format!("{win}msyh.ttc"), 0);
    let have_cjk_bold = push_font(&mut defs, "cjk-bold", &format!("{win}msyhbd.ttc"), 0);
    let have_mono = push_font(&mut defs, "mono", &format!("{win}consola.ttf"), 0);
    if have_mono {
        let mono = defs.families.entry(FontFamily::Monospace).or_default();
        mono.insert(0, "mono".to_string());
        if have_cjk {
            mono.push("cjk".to_string());
        }
    }

    let proportional = defs.families.entry(FontFamily::Proportional).or_default();
    if have_ui {
        proportional.insert(0, "ui".to_string());
    }
    if have_cjk {
        proportional.insert(if have_ui { 1 } else { 0 }, "cjk".to_string());
    }

    let mut bold: Vec<String> = Vec::new();
    if have_ui_bold {
        bold.push("ui-bold".to_string());
    }
    if have_cjk_bold {
        bold.push("cjk-bold".to_string());
    }
    if bold.is_empty() {
        bold = proportional.clone();
    } else if have_cjk {
        // CJK glyphs the bold face may lack fall back to the regular CJK face.
        bold.push("cjk".to_string());
        if have_ui {
            bold.push("ui".to_string());
        }
    }
    defs.families
        .insert(FontFamily::Name(BOLD_FAMILY.into()), bold);

    let mut italic: Vec<String> = Vec::new();
    if have_ui_italic {
        italic.push("ui-italic".to_string());
    }
    if have_cjk {
        italic.push("cjk".to_string());
    }
    if have_ui {
        italic.push("ui".to_string());
    }
    defs.families
        .insert(FontFamily::Name(ITALIC_FAMILY.into()), italic);

    ctx.set_fonts(defs);
}

pub fn apply_style(ctx: &egui::Context, pal: &Palette) {
    ctx.set_theme(if pal.dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    let pal = *pal;
    ctx.all_styles_mut(move |style| {
        style.visuals = if pal.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        style.visuals.override_text_color = Some(pal.text);
        style.visuals.panel_fill = pal.bg_primary;
        style.visuals.window_fill = pal.bg_secondary;
        style.visuals.extreme_bg_color = pal.bg_primary;
        style.visuals.faint_bg_color = pal.bg_secondary_alt;
        style.visuals.widgets.noninteractive.bg_fill = pal.bg_secondary;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, pal.border);
        style.visuals.widgets.inactive.bg_fill = pal.bg_secondary_alt;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.hovered.bg_fill = pal.bg_hover;
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.active.bg_fill = pal.bg_active;
        style.visuals.selection.bg_fill = pal.accent_soft;
        style.visuals.selection.stroke = egui::Stroke::new(1.0, pal.accent);
        style.visuals.hyperlink_color = pal.accent;
        style.spacing.item_spacing = egui::vec2(0.0, 0.0);
        style.spacing.button_padding = egui::vec2(0.0, 0.0);
        style.spacing.scroll.floating = true;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.content_margin = egui::Margin::same(0);
        style.animation_time = 0.08;
    });
}

/// Truncate a string to at most `max` chars, adding an ellipsis when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
