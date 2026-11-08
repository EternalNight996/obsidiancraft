//! Markdown parsing and the reading-view renderer.

use crate::theme::{self, Palette};
use crate::vault::Vault;
use eframe::egui::{
    self, pos2, vec2, Align, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use std::ops::Range;

#[derive(Clone, PartialEq, Debug)]
pub enum LinkTarget {
    Internal(String),
    External(String),
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub highlight: bool,
    pub code: bool,
    pub tag: bool,
    pub math: bool,
    pub link: Option<LinkTarget>,
}

#[derive(Clone, Debug)]
pub struct Inline {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Debug)]
pub struct ListItem {
    pub indent: usize,
    pub marker: String,
    pub task: Option<bool>,
    pub content: Vec<Inline>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub enum Block {
    Heading { level: u8, inlines: Vec<Inline> },
    Paragraph(Vec<Inline>),
    Code { lang: String, text: String },
    Quote(Vec<Block>),
    Callout { kind: String, title: Vec<Inline>, body: Vec<Block> },
    List(Vec<ListItem>),
    Table { head: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>> },
    Rule,
    Image { alt: String, target: String },
    Math(String),
    Properties(Vec<(String, String)>),
}

// ---------------------------------------------------------------- parsing

pub fn parse(content: &str) -> Vec<Block> {
    let body = crate::vault::body_without_frontmatter(content);
    let props = crate::vault::parse_frontmatter(content);
    let mut blocks = Vec::new();
    if !props.is_empty() {
        blocks.push(Block::Properties(props));
    }
    let lines: Vec<&str> = body.split('\n').collect();
    parse_lines(&lines, &mut blocks);
    blocks
}

fn is_fence(line: &str) -> Option<String> {
    let t = line.trim_start();
    for marker in ["```", "~~~"] {
        if let Some(rest) = t.strip_prefix(marker) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 3 {
        return false;
    }
    let c = t.chars().next().unwrap_or(' ');
    if c != '-' && c != '*' && c != '_' {
        return false;
    }
    t.chars().all(|x| x == c || x == ' ')
}

fn is_table_sep(line: &str) -> bool {
    let t = line.trim();
    if !t.contains('-') {
        return false;
    }
    let t = t.trim_start_matches('|').trim_end_matches('|');
    let cells: Vec<&str> = t.split('|').collect();
    !cells.is_empty()
        && cells.iter().all(|c| {
            let c = c.trim();
            !c.is_empty()
                && c.chars()
                    .all(|ch| ch == '-' || ch == ':' || ch == ' ')
                && c.chars().filter(|ch| *ch == '-').count() >= 2
        })
}

fn split_table_row(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn block_start(line: &str) -> bool {
    let t = line.trim_start();
    if t.trim().is_empty() {
        return true;
    }
    if is_fence(line).is_some() {
        return true;
    }
    if t.starts_with('#') {
        let n = t.chars().take_while(|c| *c == '#').count();
        if n <= 6 && t[n..].starts_with(' ') {
            return true;
        }
    }
    if is_rule(line) {
        return true;
    }
    if t.starts_with('>') || t.starts_with("$$") {
        return true;
    }
    if list_marker(t).is_some() {
        return true;
    }
    false
}

fn list_marker(t: &str) -> Option<(usize, String, Option<bool>, usize)> {
    let indent = t.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    let rest = &t[indent..];
    let (marker, after) = if let Some(r) = rest.strip_prefix("- ") {
        ("-".to_string(), r)
    } else if let Some(r) = rest.strip_prefix("* ") {
        ("*".to_string(), r)
    } else if let Some(r) = rest.strip_prefix("+ ") {
        ("+".to_string(), r)
    } else {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            let after_digits = &rest[digits.len()..];
            if let Some(r) = after_digits.strip_prefix(". ") {
                (format!("{digits}."), r)
            } else if let Some(r) = after_digits.strip_prefix(") ") {
                (format!("{digits})"), r)
            } else {
                return None;
            }
        } else {
            return None;
        }
    };
    let mut task = None;
    let mut content = after;
    if let Some(r) = content.strip_prefix("[ ] ") {
        task = Some(false);
        content = r;
    } else if let Some(r) = content
        .strip_prefix("[x] ")
        .or_else(|| content.strip_prefix("[X] "))
    {
        task = Some(true);
        content = r;
    }
    Some((
        indent / 2,
        marker,
        task,
        after.len() - content.len(),
    ))
}

fn parse_lines(lines: &[&str], out: &mut Vec<Block>) {
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if let Some(lang) = is_fence(line) {
            let mut code = String::new();
            i += 1;
            while i < lines.len() && is_fence(lines[i]).is_none() {
                code.push_str(lines[i]);
                code.push('\n');
                i += 1;
            }
            i += 1; // closing fence
            out.push(Block::Code { lang, text: code });
            continue;
        }

        if trimmed.starts_with('#') {
            let hashes = trimmed.chars().take_while(|c| *c == '#').count();
            if hashes <= 6 && trimmed[hashes..].starts_with(' ') {
                let text = trimmed[hashes..].trim().trim_end_matches('#').trim();
                out.push(Block::Heading {
                    level: hashes as u8,
                    inlines: parse_inline(text),
                });
                i += 1;
                continue;
            }
        }

        if is_rule(line) {
            out.push(Block::Rule);
            i += 1;
            continue;
        }

        if trimmed.starts_with("$$") {
            let mut math = String::new();
            let rest = trimmed.trim_start_matches('$');
            if rest.ends_with("$$") && rest.len() > 2 {
                math = rest.trim_end_matches('$').to_string();
                out.push(Block::Math(math));
                i += 1;
                continue;
            }
            if !rest.is_empty() {
                math.push_str(rest);
                math.push('\n');
            }
            i += 1;
            while i < lines.len() && !lines[i].trim().ends_with("$$") {
                math.push_str(lines[i]);
                math.push('\n');
                i += 1;
            }
            if i < lines.len() {
                let last = lines[i].trim();
                math.push_str(last.trim_end_matches('$'));
                i += 1;
            }
            out.push(Block::Math(math));
            continue;
        }

        if trimmed.starts_with('>') {
            let mut quoted: Vec<&str> = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                let t = lines[i].trim_start();
                let rest = t[1..].strip_prefix(' ').unwrap_or(&t[1..]);
                quoted.push(rest);
                i += 1;
            }
            let first = quoted.first().map(|s| s.trim()).unwrap_or("");
            if let Some(callout) = parse_callout_header(first) {
                let (kind, title) = callout;
                let mut body_blocks = Vec::new();
                let rest: Vec<&str> = quoted.iter().skip(1).copied().collect();
                parse_lines(&rest, &mut body_blocks);
                out.push(Block::Callout {
                    kind,
                    title: parse_inline(&title),
                    body: body_blocks,
                });
            } else {
                let mut inner = Vec::new();
                parse_lines(&quoted, &mut inner);
                out.push(Block::Quote(inner));
            }
            continue;
        }

        if trimmed.contains('|') && i + 1 < lines.len() && is_table_sep(lines[i + 1]) {
            let head = split_table_row(line)
                .into_iter()
                .map(|c| parse_inline(&c))
                .collect();
            let mut rows = Vec::new();
            i += 2;
            while i < lines.len() && lines[i].contains('|') && !lines[i].trim().is_empty() {
                rows.push(
                    split_table_row(lines[i])
                        .into_iter()
                        .map(|c| parse_inline(&c))
                        .collect(),
                );
                i += 1;
            }
            out.push(Block::Table { head, rows });
            continue;
        }

        if let Some((indent, marker, task, _offset)) = list_marker(line) {
            let mut items = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                let t = l.trim_start();
                if t.is_empty() {
                    break;
                }
                let Some((ind, mark, tsk, off)) = list_marker(l) else {
                    break;
                };
                let content = &l.trim_start()[0..];
                let raw = content;
                let rest = raw[ind * 2 + mark.len() + 1 + off..].to_string();
                items.push(ListItem {
                    indent: ind,
                    marker: mark,
                    task: tsk,
                    content: parse_inline(rest.trim()),
                    line: i,
                });
                i += 1;
            }
            let _ = (indent, marker, task);
            out.push(Block::List(items));
            continue;
        }

        if trimmed.starts_with("![") {
            if let Some((alt, target)) = parse_image(trimmed) {
                out.push(Block::Image { alt, target });
                i += 1;
                continue;
            }
        }

        // paragraph
        let mut parts: Vec<&str> = vec![line.trim()];
        i += 1;
        while i < lines.len() {
            let l = lines[i];
            if l.trim().is_empty() || block_start(l) {
                break;
            }
            parts.push(l.trim());
            i += 1;
        }
        let joined = parts.join("\n");
        out.push(Block::Paragraph(parse_inline(&joined)));
    }
}

fn parse_callout_header(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    let rest = t.strip_prefix("[!")?;
    let end = rest.find(']')?;
    let kind = rest[..end].to_lowercase();
    let title = rest[end + 1..].trim().to_string();
    Some((kind, title))
}

fn parse_image(s: &str) -> Option<(String, String)> {
    let end_alt = s.find("](")?;
    let alt = s[2..end_alt].to_string();
    let rest = &s[end_alt + 2..];
    let end = rest.find(')')?;
    Some((alt, rest[..end].to_string()))
}

fn flush(out: &mut Vec<Inline>, buf: &mut String, style: &Style) {
    if !buf.is_empty() {
        out.push(Inline {
            text: std::mem::take(buf),
            style: style.clone(),
        });
    }
}

fn find_seq(chars: &[char], from: usize, seq: &str) -> Option<usize> {
    let needle: Vec<char> = seq.chars().collect();
    if needle.is_empty() || from >= chars.len() {
        return None;
    }
    let mut i = from;
    while i + needle.len() <= chars.len() {
        if chars[i..i + needle.len()] == needle[..] {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_wikilink(chars: &[char], i: usize) -> Option<(String, usize)> {
    let mut j = i + 2;
    while j + 1 < chars.len() {
        if chars[j] == ']' && chars[j + 1] == ']' {
            let target: String = chars[i + 2..j].iter().collect();
            return Some((target.trim().to_string(), j + 2 - i));
        }
        j += 1;
    }
    None
}

fn parse_mdlink(chars: &[char], i: usize) -> Option<(String, String, usize)> {
    let mut j = i + 1;
    while j < chars.len() {
        if chars[j] == ']' && j + 1 < chars.len() && chars[j + 1] == '(' {
            let text: String = chars[i + 1..j].iter().collect();
            let mut k = j + 2;
            while k < chars.len() {
                if chars[k] == ')' {
                    let url: String = chars[j + 2..k].iter().collect();
                    return Some((text, url, k + 1 - i));
                }
                k += 1;
            }
            return None;
        }
        if chars[j] == '[' {
            return None;
        }
        j += 1;
    }
    None
}

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || c == '/' || c == '-' || c == '_'
}

pub fn parse_inline(text: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    parse_inline_into(text, Style::default(), &mut out);
    out
}

fn parse_inline_into(text: &str, style: Style, out: &mut Vec<Inline>) {
    let chars: Vec<char> = text.chars().collect();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            buf.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '\n' {
            buf.push(' ');
            i += 1;
            continue;
        }
        if c == '[' && i + 1 < chars.len() && chars[i + 1] == '[' {
            if let Some((target, consumed)) = parse_wikilink(&chars, i) {
                flush(out, &mut buf, &style);
                let (link, label) = match target.split_once('|') {
                    Some((t, a)) => (t.trim().to_string(), a.trim().to_string()),
                    None => {
                        let t = target.split('#').next().unwrap_or(&target).to_string();
                        let label = if target.contains('|') {
                            target.clone()
                        } else {
                            t.clone()
                        };
                        (t, label)
                    }
                };
                if !link.is_empty() {
                    out.push(Inline {
                        text: label,
                        style: Style {
                            link: Some(LinkTarget::Internal(link)),
                            ..style.clone()
                        },
                    });
                } else {
                    buf.push_str(&target);
                }
                i += consumed;
                continue;
            }
        }
        if c == '[' {
            if let Some((label, url, consumed)) = parse_mdlink(&chars, i) {
                flush(out, &mut buf, &style);
                out.push(Inline {
                    text: label,
                    style: Style {
                        link: Some(LinkTarget::External(url)),
                        ..style.clone()
                    },
                });
                i += consumed;
                continue;
            }
        }
        if c == '`' {
            if let Some(end) = find_seq(&chars, i + 1, "`") {
                flush(out, &mut buf, &style);
                let code: String = chars[i + 1..end].iter().collect();
                out.push(Inline {
                    text: code,
                    style: Style {
                        code: true,
                        ..style.clone()
                    },
                });
                i = end + 1;
                continue;
            }
        }
        if c == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
            if let Some((alt, src, consumed)) = parse_mdlink(&chars, i + 1) {
                flush(out, &mut buf, &style);
                out.push(Inline {
                    text: if alt.is_empty() {
                        src.clone()
                    } else {
                        alt
                    },
                    style: Style {
                        link: Some(LinkTarget::External(src)),
                        ..style.clone()
                    },
                });
                i += 1 + consumed;
                continue;
            }
        }
        if c == '#' {
            let prev_ok = i == 0
                || !(chars[i - 1].is_alphanumeric() || is_tag_char(chars[i - 1]));
            if prev_ok && i + 1 < chars.len() && is_tag_char(chars[i + 1]) {
                let mut j = i + 1;
                while j < chars.len() && is_tag_char(chars[j]) {
                    j += 1;
                }
                let tag: String = chars[i..j].iter().collect();
                if tag.chars().any(|x| x.is_alphabetic()) {
                    flush(out, &mut buf, &style);
                    out.push(Inline {
                        text: tag,
                        style: Style {
                            tag: true,
                            ..style.clone()
                        },
                    });
                    i = j;
                    continue;
                }
            }
        }
        if c == '$' {
            if let Some(end) = find_seq(&chars, i + 1, "$") {
                if end > i + 1 {
                    flush(out, &mut buf, &style);
                    let math: String = chars[i + 1..end].iter().collect();
                    out.push(Inline {
                        text: math,
                        style: Style {
                            math: true,
                            ..style.clone()
                        },
                    });
                    i = end + 1;
                    continue;
                }
            }
        }
        if (c == '*' || c == '_') && i + 1 < chars.len() && chars[i + 1] == c {
            let marker: String = [c, c].iter().collect();
            if let Some(end) = find_seq(&chars, i + 2, &marker) {
                if end > i + 2 {
                    flush(out, &mut buf, &style);
                    let inner: String = chars[i + 2..end].iter().collect();
                    let mut st = style.clone();
                    st.bold = true;
                    parse_inline_into(&inner, st, out);
                    i = end + 2;
                    continue;
                }
            }
        }
        if c == '=' && i + 1 < chars.len() && chars[i + 1] == '=' {
            if let Some(end) = find_seq(&chars, i + 2, "==") {
                if end > i + 2 {
                    flush(out, &mut buf, &style);
                    let inner: String = chars[i + 2..end].iter().collect();
                    let mut st = style.clone();
                    st.highlight = true;
                    parse_inline_into(&inner, st, out);
                    i = end + 2;
                    continue;
                }
            }
        }
        if c == '~' && i + 1 < chars.len() && chars[i + 1] == '~' {
            if let Some(end) = find_seq(&chars, i + 2, "~~") {
                if end > i + 2 {
                    flush(out, &mut buf, &style);
                    let inner: String = chars[i + 2..end].iter().collect();
                    let mut st = style.clone();
                    st.strike = true;
                    parse_inline_into(&inner, st, out);
                    i = end + 2;
                    continue;
                }
            }
        }
        if c == '*' || c == '_' {
            let marker: String = c.to_string();
            if let Some(end) = find_seq(&chars, i + 1, &marker) {
                if end > i + 1 {
                    flush(out, &mut buf, &style);
                    let inner: String = chars[i + 1..end].iter().collect();
                    let mut st = style.clone();
                    st.italic = true;
                    parse_inline_into(&inner, st, out);
                    i = end + 1;
                    continue;
                }
            }
        }
        buf.push(c);
        i += 1;
    }
    flush(out, &mut buf, &style);
}

// ---------------------------------------------------------------- rendering

pub struct Renderer<'a> {
    pub pal: &'a Palette,
    pub vault: &'a Vault,
    pub size: f32,
    pub clicked: Option<LinkTarget>,
    pub toggle_task: Option<usize>,
    pub scroll_to_heading: Option<usize>,
    heading_no: usize,
    hovered_link: bool,
}

impl<'a> Renderer<'a> {
    pub fn new(pal: &'a Palette, vault: &'a Vault, size: f32) -> Self {
        Self {
            pal,
            vault,
            size,
            clicked: None,
            toggle_task: None,
            scroll_to_heading: None,
            heading_no: 0,
            hovered_link: false,
        }
    }

    fn gap(&self) -> f32 {
        self.size * 1.0
    }

    pub fn render(&mut self, ui: &mut Ui, blocks: &[Block]) {
        for (i, block) in blocks.iter().enumerate() {
            let first = i == 0;
            self.block(ui, block, first);
        }
    }

    fn block(&mut self, ui: &mut Ui, block: &Block, first: bool) {
        match block {
            Block::Properties(props) => {
                self.properties(ui, props);
                ui.add_space(self.gap() * 0.75);
            }
            Block::Heading { level, inlines } => {
                let em = match level {
                    1 => 1.618,
                    2 => 1.462,
                    3 => 1.318,
                    4 => 1.188,
                    5 => 1.076,
                    _ => 1.0,
                };
                let fs = self.size * em;
                if !first {
                    ui.add_space(fs * 0.85);
                }
                if self.scroll_to_heading == Some(self.heading_no) {
                    ui.scroll_to_rect(ui.available_rect_before_wrap(), Some(Align::TOP));
                }
                let width = ui.available_width();
                let job = self.job(inlines, width, fs, true, self.pal.text);
                let galley = ui.ctx().fonts_mut(|f| f.layout_job(job.job));
                let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::click());
                ui.painter().galley(rect.min, galley.clone(), self.pal.text);
                self.hit_test(ui, &resp, rect, &galley, &job.links);
                ui.add_space(fs * 0.3);
                self.heading_no += 1;
            }
            Block::Paragraph(inlines) => {
                self.paragraph(ui, inlines, self.size, self.pal.text, false);
                ui.add_space(self.gap());
            }
            Block::Code { lang, text } => {
                self.code_block(ui, lang, text);
            }
            Block::Quote(inner) => {
                self.quote(ui, inner);
            }
            Block::Callout { kind, title, body } => {
                self.callout(ui, kind, title, body);
            }
            Block::List(items) => {
                self.list(ui, items);
            }
            Block::Table { head, rows } => {
                self.table(ui, head, rows);
            }
            Block::Rule => {
                ui.add_space(self.gap() * 0.6);
                let (rect, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, 0.0, self.pal.border_strong);
                ui.add_space(self.gap() * 1.2);
            }
            Block::Image { alt, target } => {
                self.image(ui, alt, target);
                ui.add_space(self.gap());
            }
            Block::Math(text) => {
                self.code_block(ui, "math", text);
            }
        }
    }

    fn properties(&mut self, ui: &mut Ui, props: &[(String, String)]) {
        let row_h = self.size * 1.9;
        for (k, v) in props {
            let (rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), row_h),
                Sense::hover(),
            );
            ui.painter().text(
                pos2(rect.left(), rect.center().y),
                egui::Align2::LEFT_CENTER,
                k,
                theme::font(self.size * 0.8125),
                self.pal.text_muted,
            );
            ui.painter().text(
                pos2(rect.left() + 180.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                v,
                theme::font(self.size * 0.875),
                self.pal.text,
            );
            ui.painter().line_segment(
                [
                    pos2(rect.left(), rect.bottom()),
                    pos2(rect.right(), rect.bottom()),
                ],
                Stroke::new(1.0, self.pal.border),
            );
        }
    }

    fn paragraph(
        &mut self,
        ui: &mut Ui,
        inlines: &[Inline],
        size: f32,
        color: Color32,
        bold: bool,
    ) -> egui::Response {
        let width = ui.available_width();
        let job = self.job(inlines, width, size, bold, color);
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job.job));
        let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::click());
        ui.painter().galley(rect.min, galley.clone(), color);
        self.hit_test(ui, &resp, rect, &galley, &job.links);
        resp
    }

    fn code_block(&mut self, ui: &mut Ui, _lang: &str, text: &str) {
        let width = ui.available_width();
        let fs = self.size * 0.85;
        let mut job = LayoutJob::default();
        job.wrap = TextWrapping {
            max_width: (width - 24.0).max(60.0),
            break_anywhere: true,
            ..Default::default()
        };
        job.break_on_newline = true;
        job.append(
            text.trim_end_matches('\n'),
            0.0,
            TextFormat {
                font_id: theme::font_mono(fs),
                color: self.pal.text,
                line_height: Some(fs * 1.55),
                ..Default::default()
            },
        );
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        let h = galley.size().y + 24.0;
        let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
        ui.painter()
            .rect_filled(rect, 6.0, self.pal.code_bg);
        ui.painter()
            .rect_stroke(rect, 6.0, Stroke::new(1.0, self.pal.border), egui::StrokeKind::Inside);
        ui.painter()
            .galley(rect.min + vec2(12.0, 12.0), galley, self.pal.text);
        ui.add_space(self.gap());
    }

    fn quote(&mut self, ui: &mut Ui, inner: &[Block]) {
        let start = ui.cursor().top();
        let avail = ui.available_width();
        let indent = 18.0;
        let rect = Rect::from_min_size(
            pos2(ui.max_rect().left() + indent, start),
            vec2((avail - indent).max(40.0), 0.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(Align::LEFT)),
        );
        child.set_width(rect.width());
        self.render(&mut child, inner);
        let h = child.min_rect().height();
        ui.allocate_space(vec2(avail, h));
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(start_x(ui) + 2.0, start), vec2(3.0, h)),
            1.5,
            self.pal.border_strong,
        );
        ui.add_space(self.gap());
    }

    fn callout(&mut self, ui: &mut Ui, kind: &str, title: &[Inline], body: &[Block]) {
        let color = match kind {
            "tip" | "success" | "done" => self.pal.green,
            "warning" | "question" => self.pal.yellow,
            "caution" | "danger" | "error" | "failure" | "bug" => self.pal.red,
            "important" | "example" => self.pal.accent,
            "info" | "todo" | "abstract" | "summary" | "quote" => self.pal.accent,
            _ => self.pal.accent,
        };
        let avail = ui.available_width();
        let start = ui.cursor().top();
        let pad = 14.0;
        let inner_rect = Rect::from_min_size(
            pos2(ui.max_rect().left(), start),
            vec2(avail, 0.0),
        );
        let body_rect = Rect::from_min_size(
            pos2(inner_rect.left() + pad + 6.0, start + pad),
            vec2((avail - pad * 2.0 - 6.0).max(40.0), 0.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body_rect)
                .layout(egui::Layout::top_down(Align::LEFT)),
        );
        child.set_width(body_rect.width());
        // Reserve a shape slot now, so the background is painted *behind* the text
        // once the callout's height is known.
        let bg_idx = ui.painter().add(egui::Shape::Noop);
        let label = if title.is_empty() {
            let mut chars = kind.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_uppercase().collect::<String>(),
                    chars.as_str()
                ),
                None => kind.to_string(),
            }
        } else {
            plain_text(title)
        };
        child.label(
            egui::RichText::new(label)
                .font(theme::font_bold(self.size))
                .color(color),
        );
        child.add_space(6.0);
        self.render(&mut child, body);
        let h = child.min_rect().height() + pad * 2.0;
        ui.allocate_space(vec2(avail, h));
        let bg = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 20);
        let bg_rect = Rect::from_min_size(pos2(inner_rect.left(), start), vec2(avail, h));
        ui.painter().set(
            bg_idx,
            egui::Shape::rect_filled(bg_rect, 6.0, bg),
        );
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(inner_rect.left(), start), vec2(3.0, h)),
            1.5,
            color,
        );
        ui.add_space(self.gap());
    }

    fn list(&mut self, ui: &mut Ui, items: &[ListItem]) {
        for item in items {
            let start = ui.cursor().top();
            let avail = ui.available_width();
            let indent = item.indent as f32 * 18.0;
            let marker_w = 22.0;
            let left = ui.max_rect().left() + indent;
            let text_rect = Rect::from_min_size(
                pos2(left + marker_w, start),
                vec2((avail - indent - marker_w).max(40.0), 0.0),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(text_rect)
                    .layout(egui::Layout::top_down(Align::LEFT)),
            );
            child.set_width(text_rect.width());
            let text_color = if item.task == Some(true) {
                self.pal.text_faint
            } else {
                self.pal.text
            };
            self.paragraph(&mut child, &item.content, self.size, text_color, false);
            let h = child.min_rect().height().max(self.size * 1.5);
            ui.allocate_space(vec2(avail, h));
            let line_h = self.size * 1.5;
            if let Some(done) = item.task {
                let box_rect = Rect::from_min_size(
                    pos2(left + 2.0, start + (line_h - 16.0) * 0.5 - 1.0),
                    vec2(16.0, 16.0),
                );
                let resp = ui.interact(
                    box_rect,
                    ui.scope_id().with(("task", item.line)),
                    Sense::click(),
                );
                if done {
                    ui.painter()
                        .rect_filled(box_rect, 4.0, self.pal.accent);
                    crate::icons::draw(
                        &ui.painter().clone(),
                        box_rect.shrink(3.0),
                        crate::icons::Icon::Check,
                        Color32::WHITE,
                    );
                } else {
                    ui.painter().rect_stroke(
                        box_rect,
                        4.0,
                        Stroke::new(1.2, self.pal.border_strong),
                        egui::StrokeKind::Inside,
                    );
                }
                if resp.clicked() {
                    self.toggle_task = Some(item.line);
                }
            } else if item.marker.ends_with('.') || item.marker.ends_with(')') {
                ui.painter().text(
                    pos2(left + marker_w - 6.0, start + 2.0),
                    egui::Align2::RIGHT_TOP,
                    &item.marker,
                    theme::font(self.size),
                    self.pal.text_muted,
                );
            } else {
                ui.painter().text(
                    pos2(left + marker_w - 9.0, start + 2.0),
                    egui::Align2::CENTER_TOP,
                    "•",
                    theme::font(self.size),
                    self.pal.text_muted,
                );
            }
            ui.add_space(4.0);
        }
        ui.add_space(self.gap() * 0.6);
    }

    fn table(&mut self, ui: &mut Ui, head: &[Vec<Inline>], rows: &[Vec<Vec<Inline>>]) {
        let ncol = head.len().max(1);
        let avail = ui.available_width();
        let colw = avail / ncol as f32;
        let pad = 9.0;
        let text_w = (colw - pad * 2.0).max(20.0);
        let size = self.size;

        let mut draw_row = |ui: &mut Ui, cells: &[Vec<Inline>], bold: bool| {
            let mut galleys = Vec::new();
            let mut h = 0.0f32;
            for c in 0..ncol {
                let inl = cells.get(c);
                let job = match inl {
                    Some(inl) => {
                        let mut job = LayoutJob::default();
                        job.wrap = TextWrapping {
                            max_width: text_w,
                            break_anywhere: true,
                            ..Default::default()
                        };
                        job = self.fill_job(job, inl, size, bold, self.pal.text);
                        job
                    }
                    None => LayoutJob::default(),
                };
                let g = ui.ctx().fonts_mut(|f| f.layout_job(job));
                h = h.max(g.size().y);
                galleys.push(g);
            }
            let row_h = h + pad * 2.0;
            let (rect, _) =
                ui.allocate_exact_size(vec2(avail, row_h), Sense::hover());
            for (c, g) in galleys.into_iter().enumerate() {
                let cell = Rect::from_min_size(
                    pos2(rect.left() + colw * c as f32 + pad, rect.top() + pad),
                    vec2(text_w, h),
                );
                ui.painter().galley(cell.min, g, self.pal.text);
                if c > 0 {
                    ui.painter().line_segment(
                        [
                            pos2(rect.left() + colw * c as f32, rect.top()),
                            pos2(rect.left() + colw * c as f32, rect.bottom()),
                        ],
                        Stroke::new(1.0, self.pal.border),
                    );
                }
            }
            ui.painter().line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(1.0, self.pal.border),
            );
        };

        draw_row(ui, head, true);
        for row in rows {
            draw_row(ui, row, false);
        }
        ui.add_space(self.gap());
    }

    fn image(&mut self, ui: &mut Ui, alt: &str, target: &str) {
        let w = ui.available_width().min(360.0);
        let h = 150.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        ui.painter()
            .rect_filled(rect, 6.0, self.pal.bg_secondary_alt);
        ui.painter().rect_stroke(
            rect,
            6.0,
            Stroke::new(1.0, self.pal.border),
            egui::StrokeKind::Inside,
        );
        let icon_rect = Rect::from_center_size(
            rect.center() - vec2(0.0, 14.0),
            vec2(28.0, 28.0),
        );
        crate::icons::draw(
            &ui.painter().clone(),
            icon_rect,
            crate::icons::Icon::Canvas,
            self.pal.text_faint,
        );
        let label = if alt.is_empty() { target } else { alt };
        ui.painter().text(
            rect.center() + vec2(0.0, 18.0),
            egui::Align2::CENTER_CENTER,
            crate::theme::truncate(label, 60),
            theme::font(self.size * 0.8125),
            self.pal.text_muted,
        );
    }

    fn fill_job(
        &self,
        mut job: LayoutJob,
        inlines: &[Inline],
        size: f32,
        bold: bool,
        color: Color32,
    ) -> LayoutJob {
        let mut pos = 0usize;
        for inline in inlines {
            let fmt = self.format(&inline.style, size, bold, color);
            job.append(&inline.text, 0.0, fmt);
            pos += inline.text.chars().count();
        }
        let _ = pos;
        job
    }

    fn format(&self, style: &Style, size: f32, bold: bool, color: Color32) -> TextFormat {
        let mut c = color;
        let mut font_id = if style.bold || bold {
            theme::font_bold(size)
        } else if style.italic {
            theme::font_italic(size)
        } else {
            theme::font(size)
        };
        if style.code {
            font_id = theme::font_mono(size * 0.88);
            c = self.pal.text;
        }
        if style.math {
            font_id = theme::font_italic(size);
            c = self.pal.text;
        }
        if style.link.is_some() {
            c = match &style.link {
                Some(LinkTarget::Internal(t)) => {
                    if self.vault.resolve(t).is_some() {
                        self.pal.accent
                    } else {
                        self.pal.text_muted
                    }
                }
                _ => self.pal.accent,
            };
        }
        if style.tag {
            c = self.pal.accent;
        }
        if style.strike {
            c = self.pal.text_faint;
        }
        TextFormat {
            font_id,
            color: c,
            // Obsidian: body line-height 1.5, headings 1.2 (app.css).
            line_height: Some(if bold { size * 1.2 } else { size * 1.5 }),
            background: if style.code {
                self.pal.code_bg
            } else if style.highlight {
                self.pal.yellow.gamma_multiply(0.35)
            } else if style.tag {
                self.pal.accent.gamma_multiply(0.18)
            } else {
                Color32::TRANSPARENT
            },
            expand_bg: if style.code || style.highlight || style.tag {
                2.0
            } else {
                0.0
            },
            italics: style.italic,
            strikethrough: if style.strike {
                Stroke::new(1.0, self.pal.text_faint)
            } else {
                Stroke::NONE
            },
            underline: if style.tag {
                Stroke::NONE
            } else {
                Stroke::NONE
            },
            ..Default::default()
        }
    }

    fn job(
        &self,
        inlines: &[Inline],
        width: f32,
        size: f32,
        bold: bool,
        color: Color32,
    ) -> Job {
        let mut job = LayoutJob::default();
        let has_cjk = inlines.iter().any(|i| i.text.chars().any(is_cjk));
        job.wrap = TextWrapping {
            max_width: width,
            break_anywhere: has_cjk,
            ..Default::default()
        };
        job.break_on_newline = true;
        let mut links: Vec<(Range<usize>, LinkTarget)> = Vec::new();
        let mut pos = 0usize;
        for inline in inlines {
            let fmt = self.format(&inline.style, size, bold, color);
            if let Some(target) = &inline.style.link {
                let n = inline.text.chars().count();
                links.push((pos..pos + n, target.clone()));
            }
            job.append(&inline.text, 0.0, fmt);
            pos += inline.text.chars().count();
        }
        Job { job, links }
    }

    fn hit_test(
        &mut self,
        ui: &mut Ui,
        resp: &egui::Response,
        rect: Rect,
        galley: &egui::Galley,
        links: &[(Range<usize>, LinkTarget)],
    ) {
        if links.is_empty() {
            return;
        }
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            if resp.hovered() {
                let local = pos - rect.min;
                let cursor = galley.cursor_from_pos(local);
                let index = cursor.index.0;
                for (range, target) in links {
                    if range.contains(&index) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        self.hovered_link = true;
                        if resp.clicked() {
                            self.clicked = Some(target.clone());
                        }
                        break;
                    }
                }
            }
        }
    }
}

struct Job {
    job: LayoutJob,
    links: Vec<(Range<usize>, LinkTarget)>,
}

fn start_x(ui: &Ui) -> f32 {
    ui.max_rect().left()
}

fn is_cjk(c: char) -> bool {
    let u = c as u32;
    (0x3040..=0x30FF).contains(&u)
        || (0x3400..=0x4DBF).contains(&u)
        || (0x4E00..=0x9FFF).contains(&u)
        || (0xAC00..=0xD7AF).contains(&u)
        || (0xF900..=0xFAFF).contains(&u)
}

pub fn plain_text(inlines: &[Inline]) -> String {
    inlines.iter().map(|i| i.text.clone()).collect()
}

/// The first paragraph/heading text of a note, for previews.
pub fn note_preview(content: &str, max: usize) -> String {
    for line in crate::vault::body_without_frontmatter(content).lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("```") || t.starts_with("---") {
            continue;
        }
        let clean = t
            .trim_start_matches(['-', '*', '>', ' '])
            .replace("**", "")
            .replace("[[", "")
            .replace("]]", "")
            .replace('`', "");
        return crate::theme::truncate(&clean, max);
    }
    String::new()
}
