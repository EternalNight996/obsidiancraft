//! The vault: a folder of markdown notes, its tree, links, tags and search.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Note {
    pub rel: String,
    pub title: String,
    pub content: String,
    pub modified: u64,
}

#[derive(Clone, Debug)]
pub enum Entry {
    Folder(Folder),
    Note(usize),
}

#[derive(Clone, Debug)]
pub struct Folder {
    pub rel: String,
    pub name: String,
    pub children: Vec<Entry>,
}

#[derive(Clone, Debug)]
pub struct LinkRef {
    pub target: String,
    pub resolved: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Backlink {
    pub note: usize,
    pub line: usize,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub note: usize,
    pub line: usize,
    pub text: String,
    pub count: usize,
}

#[derive(Clone, Debug)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct TagUse {
    pub tag: String,
    pub count: usize,
}

pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
    pub tree: Vec<Entry>,
    pub expanded: HashSet<String>,
    pub title_index: HashMap<String, usize>,
    pub path_index: HashMap<String, usize>,
}

impl Vault {
    pub fn load(root: &Path) -> Self {
        let mut vault = Self {
            root: root.to_path_buf(),
            notes: Vec::new(),
            tree: Vec::new(),
            expanded: HashSet::new(),
            title_index: HashMap::new(),
            path_index: HashMap::new(),
        };
        vault.refresh();
        vault
    }

    pub fn refresh(&mut self) {
        let mut notes = Vec::new();
        let mut expanded = std::mem::take(&mut self.expanded);
        let tree = walk(&self.root, "", &mut notes, &mut expanded);
        self.tree = tree;
        self.notes = notes;
        self.expanded = expanded;

        self.title_index.clear();
        self.path_index.clear();
        for (i, note) in self.notes.iter().enumerate() {
            self.title_index
                .entry(note.title.to_lowercase())
                .or_insert(i);
            self.path_index
                .insert(note.rel.to_lowercase(), i);
            self.path_index
                .insert(note.rel.trim_end_matches(".md").to_lowercase(), i);
        }
    }

    pub fn note(&self, idx: usize) -> Option<&Note> {
        self.notes.get(idx)
    }

    pub fn find_by_rel(&self, rel: &str) -> Option<usize> {
        self.path_index.get(&rel.to_lowercase()).copied()
    }

    pub fn find_by_title(&self, title: &str) -> Option<usize> {
        self.title_index.get(&title.to_lowercase()).copied()
    }

    /// Resolve a `[[wikilink]]` target to a note index.
    pub fn resolve(&self, target: &str) -> Option<usize> {
        let t = target.split('|').next().unwrap_or(target);
        let t = t.split('#').next().unwrap_or(t);
        let t = t.trim().trim_start_matches("./");
        if t.is_empty() {
            return None;
        }
        let key = t.to_lowercase();
        if let Some(i) = self.path_index.get(&key) {
            return Some(*i);
        }
        if let Some(i) = self.title_index.get(&key) {
            return Some(*i);
        }
        let key = key.trim_end_matches(".md").to_string();
        if let Some(i) = self.path_index.get(&key) {
            return Some(*i);
        }
        self.title_index.get(&key).copied()
    }

    pub fn save_note(&mut self, idx: usize, content: String) -> std::io::Result<()> {
        if let Some(note) = self.notes.get_mut(idx) {
            let path = self.root.join(&note.rel);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(&path, content.as_bytes())?;
            note.content = content;
            note.modified = now_secs();
        }
        Ok(())
    }

    fn unique_rel(&self, dir: &str, name: &str) -> String {
        let base = sanitize_name(name);
        let mut candidate = if dir.is_empty() {
            format!("{base}.md")
        } else {
            format!("{dir}/{base}.md")
        };
        let mut n = 1;
        while self.root.join(&candidate).exists() {
            let c = if dir.is_empty() {
                format!("{base} {n}.md")
            } else {
                format!("{dir}/{base} {n}.md")
            };
            candidate = c;
            n += 1;
            if n > 999 {
                break;
            }
        }
        candidate
    }

    /// Create a note, returning its new relative path.
    pub fn create_note(&mut self, dir: &str, name: &str) -> std::io::Result<String> {
        let rel = self.unique_rel(dir, name);
        let path = self.root.join(&rel);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, b"")?;
        self.refresh();
        Ok(rel)
    }

    pub fn create_folder(&mut self, dir: &str, name: &str) -> std::io::Result<String> {
        let base = sanitize_name(name);
        let mut rel = if dir.is_empty() {
            base.clone()
        } else {
            format!("{dir}/{base}")
        };
        let mut n = 1;
        while self.root.join(&rel).exists() {
            rel = if dir.is_empty() {
                format!("{base} {n}")
            } else {
                format!("{dir}/{base} {n}")
            };
            n += 1;
            if n > 999 {
                break;
            }
        }
        fs::create_dir_all(self.root.join(&rel))?;
        self.refresh();
        Ok(rel)
    }

    pub fn delete_note(&mut self, idx: usize) -> std::io::Result<()> {
        let Some(note) = self.notes.get(idx).cloned() else {
            return Ok(());
        };
        let path = self.root.join(&note.rel);
        let trash = self.root.join(".trash");
        let _ = fs::create_dir_all(&trash);
        let target = trash.join(format!("{} {}", now_secs(), note.rel.replace('/', "_")));
        match fs::rename(&path, &target) {
            Ok(()) => {}
            Err(_) => {
                fs::remove_file(&path)?;
            }
        }
        self.refresh();
        Ok(())
    }

    /// Move a folder (and everything in it) to the vault's `.trash`.
    pub fn delete_folder(&mut self, rel: &str) -> std::io::Result<()> {
        let path = self.root.join(rel);
        if !path.exists() {
            return Ok(());
        }
        let trash = self.root.join(".trash");
        let _ = fs::create_dir_all(&trash);
        let target = trash.join(format!("{} {}", now_secs(), rel.replace('/', "_")));
        match fs::rename(&path, &target) {
            Ok(()) => {}
            Err(_) => {
                fs::remove_dir_all(&path)?;
            }
        }
        self.refresh();
        Ok(())
    }

    /// Rename a note's file and update links to it in every note.
    pub fn rename_note(&mut self, idx: usize, new_title: &str) -> std::io::Result<String> {
        let Some(note) = self.notes.get(idx).cloned() else {
            return Ok(String::new());
        };
        let dir = match note.rel.rfind('/') {
            Some(p) => note.rel[..p].to_string(),
            None => String::new(),
        };
        let new_base = sanitize_name(new_title);
        let new_rel = if dir.is_empty() {
            format!("{new_base}.md")
        } else {
            format!("{dir}/{new_base}.md")
        };
        if new_rel == note.rel {
            return Ok(new_rel);
        }
        if self.root.join(&new_rel).exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "同名文件已存在",
            ));
        }
        fs::rename(self.root.join(&note.rel), self.root.join(&new_rel))?;
        let old = note.title.clone();
        for other in &self.notes {
            if other.rel == note.rel {
                continue;
            }
            let updated = other
                .content
                .replace(&format!("[[{old}]]"), &format!("[[{new_base}]]"))
                .replace(&format!("[[{old}|"), &format!("[[{new_base}|"))
                .replace(&format!("[[{old}#"), &format!("[[{new_base}#"));
            if updated != other.content {
                let _ = fs::write(self.root.join(&other.rel), updated.as_bytes());
            }
        }
        self.refresh();
        Ok(new_rel)
    }

    pub fn links_in(&self, idx: usize) -> Vec<LinkRef> {
        let Some(note) = self.notes.get(idx) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for (target, _, _) in extract_links(&note.content) {
            if seen.insert(target.clone()) {
                let resolved = self.resolve(&target);
                out.push(LinkRef { target, resolved });
            }
        }
        out
    }

    pub fn backlinks(&self, idx: usize) -> Vec<Backlink> {
        let Some(note) = self.notes.get(idx) else {
            return Vec::new();
        };
        let title = note.title.to_lowercase();
        let rel = note.rel.to_lowercase();
        let rel_no_ext = rel.trim_end_matches(".md").to_string();
        let mut out = Vec::new();
        for (other_idx, other) in self.notes.iter().enumerate() {
            if other_idx == idx {
                continue;
            }
            for (target, line, text) in extract_links(&other.content) {
                let t = target
                    .split('|')
                    .next()
                    .unwrap_or(&target)
                    .split('#')
                    .next()
                    .unwrap_or(&target)
                    .trim()
                    .to_lowercase();
                let t = t.trim_start_matches("./");
                if t == title || t == rel || t == rel_no_ext || t == other.title.to_lowercase() {
                    out.push(Backlink {
                        note: other_idx,
                        line,
                        text,
                    });
                }
            }
        }
        out
    }

    pub fn headings(&self, idx: usize) -> Vec<Heading> {
        let Some(note) = self.notes.get(idx) else {
            return Vec::new();
        };
        headings_in(&note.content)
    }

    pub fn all_tags(&self) -> Vec<TagUse> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for note in &self.notes {
            for tag in tags_in(&note.content) {
                *counts.entry(tag).or_insert(0) += 1;
            }
        }
        let mut out: Vec<TagUse> = counts
            .into_iter()
            .map(|(tag, count)| TagUse { tag, count })
            .collect();
        out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tag.cmp(&b.tag)));
        out
    }

    pub fn search(&self, query: &str, match_case: bool) -> Vec<SearchHit> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut tag_filter: Option<String> = None;
        let mut needle = q.to_string();
        if let Some(rest) = q.strip_prefix("tag:") {
            tag_filter = Some(rest.trim().trim_start_matches('#').to_lowercase());
            needle = String::new();
        } else if let Some(rest) = q.strip_prefix("path:") {
            let n = if match_case { rest.to_string() } else { rest.to_lowercase() };
            for (idx, note) in self.notes.iter().enumerate() {
                let name = if match_case { note.rel.clone() } else { note.rel.to_lowercase() };
                if name.contains(&n) {
                    out.push(SearchHit {
                        note: idx,
                        line: 0,
                        text: note.title.clone(),
                        count: 1,
                    });
                }
            }
            return out;
        }

        let pat_lower = needle.to_lowercase();
        for (idx, note) in self.notes.iter().enumerate() {
            if let Some(tag) = &tag_filter {
                let has = tags_in(&note.content)
                    .iter()
                    .any(|t| t.to_lowercase() == format!("#{tag}"));
                if has {
                    out.push(SearchHit {
                        note: idx,
                        line: 0,
                        text: note.title.clone(),
                        count: 1,
                    });
                }
                continue;
            }
            if needle.is_empty() {
                continue;
            }
            let mut file_count = 0;
            for (line_no, line) in note.content.split('\n').enumerate() {
                let hay = if match_case {
                    line.to_string()
                } else {
                    line.to_lowercase()
                };
                let pat = if match_case {
                    needle.clone()
                } else {
                    needle.to_lowercase()
                };
                let count = hay.matches(&pat).count();
                if count > 0 {
                    file_count += count;
                    out.push(SearchHit {
                        note: idx,
                        line: line_no,
                        text: line.trim().to_string(),
                        count,
                    });
                    if out.len() >= 200 {
                        return out;
                    }
                }
            }
            if file_count == 0 && out.len() < 200 {
                // title matches still count as a file hit
                let title_hay = if match_case {
                    note.title.clone()
                } else {
                    note.title.to_lowercase()
                };
                if title_hay.contains(if match_case {
                    needle.as_str()
                } else {
                    pat_lower.as_str()
                }) {
                    out.push(SearchHit {
                        note: idx,
                        line: 0,
                        text: String::new(),
                        count: 1,
                    });
                }
            }
        }
        out
    }

    pub fn frontmatter(&self, idx: usize) -> Vec<(String, String)> {
        self.notes
            .get(idx)
            .map(|n| parse_frontmatter(&n.content))
            .unwrap_or_default()
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim();
    if cleaned.is_empty() {
        "未命名".to_string()
    } else {
        cleaned.to_string()
    }
}

fn walk(
    dir: &Path,
    rel: &str,
    notes: &mut Vec<Note>,
    expanded: &mut HashSet<String>,
) -> Vec<Entry> {
    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<String> = Vec::new();
    if let Ok(read) = fs::read_dir(dir) {
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                dirs.push(name);
            } else if name.to_lowercase().ends_with(".md") {
                files.push(name);
            }
        }
    }
    let key = |s: &String| (s.to_lowercase(), s.clone());
    dirs.sort_by_key(key);
    files.sort_by_key(key);

    let mut out = Vec::new();
    for name in dirs {
        let rel_path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        // New folders default to expanded; the user's collapses are kept in `expanded`.
        expanded.insert(rel_path.clone());
        let children = walk(&dir.join(&name), &rel_path, notes, expanded);
        out.push(Entry::Folder(Folder {
            rel: rel_path,
            name,
            children,
        }));
    }
    for name in files {
        let rel_path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let path = dir.join(&name);
        let content = fs::read_to_string(&path).unwrap_or_default();
        let modified = fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let title = name.trim_end_matches(".md").to_string();
        let idx = notes.len();
        notes.push(Note {
            rel: rel_path,
            title,
            content,
            modified,
        });
        out.push(Entry::Note(idx));
    }
    out
}

/// Find `[[wikilinks]]`, skipping fenced code blocks.
pub fn extract_links(content: &str) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (line_no, line) in content.split('\n').enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let bytes: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == '[' && bytes[i + 1] == '[' {
                let start = i;
                let mut j = i + 2;
                let mut end = None;
                while j + 1 < bytes.len() {
                    if bytes[j] == ']' && bytes[j + 1] == ']' {
                        end = Some(j);
                        break;
                    }
                    j += 1;
                }
                if let Some(e) = end {
                    let target: String = bytes[start + 2..e].iter().collect();
                    if !target.trim().is_empty() {
                        out.push((target.trim().to_string(), line_no, line.trim().to_string()));
                    }
                    i = e + 2;
                    continue;
                }
            }
            i += 1;
        }
    }
    out
}

/// Headings, skipping fenced code blocks.
pub fn headings_in(content: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (line_no, line) in content.split('\n').enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if hashes >= 1 && hashes <= 6 {
            let rest = &trimmed[hashes..];
            if rest.starts_with(' ') {
                out.push(Heading {
                    level: hashes as u8,
                    text: rest.trim().trim_end_matches('#').trim().to_string(),
                    line: line_no,
                });
            }
        }
    }
    out
}

/// `#tags` in a note (skipping fenced code blocks).
pub fn tags_in(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in content.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '#' {
                let prev_ok = i == 0 || !chars[i - 1].is_alphanumeric();
                if prev_ok {
                    let mut j = i + 1;
                    while j < chars.len() {
                        let c = chars[j];
                        if c.is_alphanumeric() || c == '/' || c == '-' || c == '_' {
                            j += 1;
                        } else {
                            break;
                        }
                    }
                    if j > i + 1 {
                        let tag: String = chars[i..j].iter().collect();
                        if tag.chars().any(|c| c.is_alphabetic()) {
                            out.push(tag);
                        }
                        i = j;
                        continue;
                    }
                }
            }
            i += 1;
        }
    }
    out
}

pub fn parse_frontmatter(content: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if !content.starts_with("---") {
        return out;
    }
    let mut lines = content.split('\n');
    lines.next();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            if !key.is_empty() && !key.starts_with('#') {
                out.push((key.to_string(), v.trim().trim_matches('"').to_string()));
            }
        }
    }
    out
}

pub fn body_without_frontmatter(content: &str) -> &str {
    if content.starts_with("---") {
        if let Some(end) = content[3..].find("\n---") {
            let after = 3 + end + 4;
            let rest = &content[after..];
            return rest.strip_prefix('\n').unwrap_or(rest);
        }
    }
    content
}

fn is_cjk(c: char) -> bool {
    let u = c as u32;
    (0x3040..=0x30FF).contains(&u)   // kana
        || (0x3400..=0x4DBF).contains(&u)
        || (0x4E00..=0x9FFF).contains(&u)
        || (0xAC00..=0xD7AF).contains(&u)
        || (0xF900..=0xFAFF).contains(&u)
        || (0x20000..=0x2FA1F).contains(&u)
}

/// Word count, matching the original's idea: every CJK character is a word,
/// and runs of letters/digits count as one word each.
pub fn word_count(text: &str) -> usize {
    let mut count = 0usize;
    let mut in_word = false;
    for c in text.chars() {
        if is_cjk(c) {
            count += 1;
            in_word = false;
        } else if c.is_alphanumeric() || c == '\'' || c == '’' || c == '-' || c == '_' {
            if !in_word {
                count += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }
    count
}

pub fn char_count(text: &str) -> usize {
    text.chars().count()
}
