//! Small line icons drawn with the painter (Lucide-style, 24-unit grid).

use eframe::egui::{pos2, Color32, Painter, Pos2, Rect, Stroke};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    PanelLeft,
    PanelRight,
    FileText,
    Search,
    Bookmark,
    BookOpen,
    Graph,
    Canvas,
    Terminal,
    Settings,
    Help,
    Plus,
    FolderPlus,
    Sort,
    Collapse,
    Folder,
    ChevronRight,
    ChevronLeft,
    ChevronDown,
    X,
    Link,
    Tag,
    List,
    Archive,
    More,
    Pen,
    Trash,
    Minus,
    Square,
    Restore,
    RotateCcw,
    Wand,
    Hash,
    External,
    History,
    Check,
    Note,
    FileMove,
}

fn seg(p: &Painter, a: Pos2, b: Pos2, st: Stroke) {
    p.line_segment([a, b], st);
}

fn poly(p: &Painter, pts: &[Pos2], st: Stroke) {
    for w in pts.windows(2) {
        p.line_segment([w[0], w[1]], st);
    }
}

pub fn draw(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height());
    if s < 4.0 {
        return;
    }
    let c = rect.center();
    let u = s / 24.0;
    let pt = |x: f32, y: f32| pos2(c.x + (x - 12.0) * u, c.y + (y - 12.0) * u);
    let st = Stroke::new((s / 14.0).clamp(1.0, 2.0), color);
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(pt(x0, y0), pt(x1, y1));
    let circle = |cx: f32, cy: f32, rad: f32| {
        p.add(eframe::egui::Shape::circle_stroke(
            pt(cx, cy),
            rad * u,
            st,
        ));
    };

    match icon {
        Icon::PanelLeft => {
            p.rect_stroke(r(3.0, 4.0, 21.0, 20.0), 2.0, st, eframe::egui::StrokeKind::Inside);
            seg(p, pt(9.0, 4.0), pt(9.0, 20.0), st);
        }
        Icon::PanelRight => {
            p.rect_stroke(r(3.0, 4.0, 21.0, 20.0), 2.0, st, eframe::egui::StrokeKind::Inside);
            seg(p, pt(15.0, 4.0), pt(15.0, 20.0), st);
        }
        Icon::FileText | Icon::Note | Icon::FileMove => {
            poly(
                p,
                &[
                    pt(6.0, 3.0),
                    pt(14.0, 3.0),
                    pt(18.0, 7.0),
                    pt(18.0, 21.0),
                    pt(6.0, 21.0),
                    pt(6.0, 3.0),
                ],
                st,
            );
            poly(p, &[pt(14.0, 3.0), pt(14.0, 7.0), pt(18.0, 7.0)], st);
            if icon == Icon::FileText {
                seg(p, pt(9.0, 12.0), pt(15.0, 12.0), st);
                seg(p, pt(9.0, 16.0), pt(15.0, 16.0), st);
            } else if icon == Icon::Note {
                seg(p, pt(9.0, 14.0), pt(15.0, 14.0), st);
                seg(p, pt(12.0, 11.0), pt(12.0, 17.0), st);
            }
        }
        Icon::Search => {
            circle(10.5, 10.5, 6.5);
            seg(p, pt(15.3, 15.3), pt(20.5, 20.5), st);
        }
        Icon::Bookmark => {
            poly(
                p,
                &[
                    pt(6.0, 3.0),
                    pt(18.0, 3.0),
                    pt(18.0, 21.0),
                    pt(12.0, 16.0),
                    pt(6.0, 21.0),
                    pt(6.0, 3.0),
                ],
                st,
            );
        }
        Icon::BookOpen => {
            // lucide book-open: a centre spine and two open pages.
            seg(p, pt(12.0, 7.0), pt(12.0, 21.0), st);
            poly(
                p,
                &[
                    pt(12.0, 7.0),
                    pt(8.5, 5.0),
                    pt(4.0, 4.0),
                    pt(3.0, 5.0),
                    pt(3.0, 18.0),
                    pt(4.0, 18.5),
                    pt(8.5, 19.5),
                    pt(12.0, 21.0),
                ],
                st,
            );
            poly(
                p,
                &[
                    pt(12.0, 7.0),
                    pt(15.5, 5.0),
                    pt(20.0, 4.0),
                    pt(21.0, 5.0),
                    pt(21.0, 18.0),
                    pt(20.0, 18.5),
                    pt(15.5, 19.5),
                    pt(12.0, 21.0),
                ],
                st,
            );
        }
        Icon::Graph => {
            circle(6.0, 6.0, 2.6);
            circle(18.5, 7.0, 2.6);
            circle(12.0, 18.0, 2.6);
            seg(p, pt(6.0, 6.0), pt(18.5, 7.0), st);
            seg(p, pt(6.0, 6.0), pt(12.0, 18.0), st);
            seg(p, pt(18.5, 7.0), pt(12.0, 18.0), st);
        }
        Icon::Canvas => {
            p.rect_stroke(r(3.0, 4.0, 21.0, 20.0), 2.0, st, eframe::egui::StrokeKind::Inside);
            seg(p, pt(3.0, 10.0), pt(21.0, 10.0), st);
            seg(p, pt(10.0, 10.0), pt(10.0, 20.0), st);
        }
        Icon::Terminal => {
            p.rect_stroke(r(3.0, 4.0, 21.0, 20.0), 2.0, st, eframe::egui::StrokeKind::Inside);
            poly(p, &[pt(7.0, 9.0), pt(11.0, 12.0), pt(7.0, 15.0)], st);
            seg(p, pt(13.0, 15.5), pt(17.0, 15.5), st);
        }
        Icon::Settings => {
            circle(12.0, 12.0, 3.4);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let (sa, ca) = a.sin_cos();
                seg(
                    p,
                    pt(12.0 + ca * 6.4, 12.0 + sa * 6.4),
                    pt(12.0 + ca * 8.6, 12.0 + sa * 8.6),
                    st,
                );
            }
        }
        Icon::Help => {
            circle(12.0, 12.0, 8.6);
            let f = eframe::egui::FontId::proportional(11.0 * u);
            p.text(pt(12.0, 12.0), eframe::egui::Align2::CENTER_CENTER, "?", f, color);
        }
        Icon::Plus => {
            seg(p, pt(12.0, 5.0), pt(12.0, 19.0), st);
            seg(p, pt(5.0, 12.0), pt(19.0, 12.0), st);
        }
        Icon::FolderPlus => {
            poly(
                p,
                &[
                    pt(3.0, 6.0),
                    pt(10.0, 6.0),
                    pt(12.0, 9.0),
                    pt(21.0, 9.0),
                    pt(21.0, 19.0),
                    pt(3.0, 19.0),
                    pt(3.0, 6.0),
                ],
                st,
            );
            seg(p, pt(12.0, 12.0), pt(12.0, 17.0), st);
            seg(p, pt(9.5, 14.5), pt(14.5, 14.5), st);
        }
        Icon::Sort => {
            seg(p, pt(4.0, 7.0), pt(20.0, 7.0), st);
            seg(p, pt(4.0, 12.0), pt(14.0, 12.0), st);
            seg(p, pt(4.0, 17.0), pt(9.0, 17.0), st);
        }
        Icon::Collapse => {
            poly(p, &[pt(8.0, 9.0), pt(12.0, 5.0), pt(16.0, 9.0)], st);
            poly(p, &[pt(8.0, 15.0), pt(12.0, 19.0), pt(16.0, 15.0)], st);
        }
        Icon::Folder => {
            poly(
                p,
                &[
                    pt(3.0, 6.0),
                    pt(10.0, 6.0),
                    pt(12.0, 9.0),
                    pt(21.0, 9.0),
                    pt(21.0, 19.0),
                    pt(3.0, 19.0),
                    pt(3.0, 6.0),
                ],
                st,
            );
        }
        Icon::ChevronRight => {
            poly(p, &[pt(9.0, 6.0), pt(15.0, 12.0), pt(9.0, 18.0)], st);
        }
        Icon::ChevronLeft => {
            poly(p, &[pt(15.0, 6.0), pt(9.0, 12.0), pt(15.0, 18.0)], st);
        }
        Icon::ChevronDown => {
            poly(p, &[pt(6.0, 9.0), pt(12.0, 15.0), pt(18.0, 9.0)], st);
        }
        Icon::X => {
            seg(p, pt(6.0, 6.0), pt(18.0, 18.0), st);
            seg(p, pt(18.0, 6.0), pt(6.0, 18.0), st);
        }
        Icon::Link => {
            // two chain links
            poly(
                p,
                &[
                    pt(9.5, 14.5),
                    pt(7.0, 17.0),
                    pt(4.6, 14.6),
                    pt(4.6, 12.6),
                    pt(9.0, 8.2),
                    pt(11.0, 8.2),
                ],
                st,
            );
            poly(
                p,
                &[
                    pt(14.5, 9.5),
                    pt(17.0, 7.0),
                    pt(19.4, 9.4),
                    pt(19.4, 11.4),
                    pt(15.0, 15.8),
                    pt(13.0, 15.8),
                ],
                st,
            );
            seg(p, pt(9.0, 15.0), pt(15.0, 9.0), st);
        }
        Icon::Tag => {
            poly(
                p,
                &[
                    pt(3.5, 12.0),
                    pt(12.0, 3.5),
                    pt(20.5, 12.0),
                    pt(12.0, 20.5),
                    pt(3.5, 12.0),
                ],
                st,
            );
            circle(15.5, 8.5, 1.4);
        }
        Icon::List => {
            for y in [6.0f32, 12.0, 18.0] {
                circle(5.0, y, 1.2);
                seg(p, pt(10.0, y), pt(20.0, y), st);
            }
        }
        Icon::Archive => {
            seg(p, pt(3.0, 5.0), pt(21.0, 5.0), st);
            poly(
                p,
                &[pt(4.0, 5.0), pt(4.0, 20.0), pt(20.0, 20.0), pt(20.0, 5.0)],
                st,
            );
            seg(p, pt(4.0, 11.0), pt(20.0, 11.0), st);
        }
        Icon::More => {
            for x in [6.0f32, 12.0, 18.0] {
                p.circle_filled(pt(x, 12.0), 1.5 * u, color);
            }
        }
        Icon::Pen => {
            poly(
                p,
                &[
                    pt(4.0, 20.0),
                    pt(4.0, 15.5),
                    pt(15.5, 4.0),
                    pt(20.0, 8.5),
                    pt(8.5, 20.0),
                    pt(4.0, 20.0),
                ],
                st,
            );
        }
        Icon::Trash => {
            seg(p, pt(4.0, 7.0), pt(20.0, 7.0), st);
            poly(
                p,
                &[pt(6.0, 7.0), pt(6.0, 20.0), pt(18.0, 20.0), pt(18.0, 7.0)],
                st,
            );
            seg(p, pt(10.0, 11.0), pt(10.0, 16.5), st);
            seg(p, pt(14.0, 11.0), pt(14.0, 16.5), st);
            poly(p, &[pt(9.0, 7.0), pt(9.5, 4.0), pt(14.5, 4.0), pt(15.0, 7.0)], st);
        }
        Icon::Minus => seg(p, pt(6.0, 12.0), pt(18.0, 12.0), st),
        Icon::Square => {
            p.rect_stroke(r(5.0, 5.0, 19.0, 19.0), 1.0, st, eframe::egui::StrokeKind::Inside);
        }
        Icon::Restore => {
            p.rect_stroke(r(4.0, 8.0, 16.0, 20.0), 1.0, st, eframe::egui::StrokeKind::Inside);
            poly(p, &[pt(8.0, 4.0), pt(20.0, 4.0), pt(20.0, 16.0)], st);
        }
        Icon::RotateCcw => {
            // Three-quarter arc with an arrow, like lucide rotate-ccw.
            let mut prev = pt(12.0 + 8.0, 12.0);
            for i in 1..=24 {
                let a = -0.35 + i as f32 / 24.0 * std::f32::consts::TAU * 0.8;
                let next = pt(12.0 + a.cos() * 8.0, 12.0 + a.sin() * 8.0);
                seg(p, prev, next, st);
                prev = next;
            }
            poly(p, &[pt(3.0, 4.0), pt(3.0, 10.0), pt(9.0, 10.0)], st);
        }
        Icon::Wand => {
            // Diagonal wand with sparkles (lucide wand-2).
            seg(p, pt(4.0, 20.0), pt(15.5, 8.5), st);
            seg(p, pt(14.0, 7.0), pt(17.0, 10.0), st);
            seg(p, pt(16.0, 3.0), pt(21.0, 3.0), st);
            seg(p, pt(18.5, 0.5), pt(18.5, 5.5), st);
            seg(p, pt(6.0, 5.0), pt(6.0, 9.0), st);
            seg(p, pt(4.0, 7.0), pt(8.0, 7.0), st);
            seg(p, pt(18.0, 16.0), pt(18.0, 20.0), st);
            seg(p, pt(16.0, 18.0), pt(20.0, 18.0), st);
        }
        Icon::Hash => {
            seg(p, pt(9.0, 4.0), pt(7.0, 20.0), st);
            seg(p, pt(17.0, 4.0), pt(15.0, 20.0), st);
            seg(p, pt(4.0, 9.0), pt(20.0, 9.0), st);
            seg(p, pt(4.0, 15.0), pt(20.0, 15.0), st);
        }
        Icon::External => {
            poly(
                p,
                &[
                    pt(13.0, 5.0),
                    pt(19.0, 5.0),
                    pt(19.0, 11.0),
                    pt(19.0, 19.0),
                    pt(5.0, 19.0),
                    pt(5.0, 5.0),
                    pt(11.0, 5.0),
                ],
                st,
            );
            seg(p, pt(13.0, 11.0), pt(19.0, 5.0), st);
        }
        Icon::History => {
            for i in 0..9 {
                let a0 = -2.2 + i as f32 * 0.45;
                let a1 = a0 + 0.45;
                let (s0, c0) = a0.sin_cos();
                let (s1, c1) = a1.sin_cos();
                seg(
                    p,
                    pt(12.0 + c0 * 8.0, 12.0 + s0 * 8.0),
                    pt(12.0 + c1 * 8.0, 12.0 + s1 * 8.0),
                    st,
                );
            }
            poly(p, &[pt(3.5, 12.0), pt(3.5, 7.0), pt(7.0, 9.5)], st);
        }
        Icon::Check => {
            poly(p, &[pt(5.0, 12.5), pt(10.0, 17.5), pt(19.0, 6.5)], st);
        }
    }
}
