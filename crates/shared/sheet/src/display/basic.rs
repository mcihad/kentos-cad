//! Text, pictures, shapes and lines.

use super::{Ctx, Note, Pen};
use crate::kinds::*;
use crate::model::Item;
use crate::style::Stroke;
use crate::text::{self, Layout};
use crate::units::*;

pub(crate) fn text(ctx: &Ctx, it: &Item, t: &TextItem, pen: &mut Pen, notes: &mut Vec<Note>) {
    let content = ctx.render(it, &t.content, notes);
    let b = text::layout(
        &content,
        &t.style,
        &it.content_rect(),
        &Layout {
            align: t.align,
            valign: t.valign,
            line_height: t.line_height,
            wrap: t.wrap,
            fit: t.fit,
        },
    );
    if b.overflow {
        notes.push(Note {
            item: it.id.clone(),
            code: "text_overflow",
            detail: String::new(),
        });
    }
    pen.block(&b, &t.style);
}

/// An empty picture's box: light grey, crossed.
pub(crate) fn placeholder(pen: &mut Pen, r: RectUm) {
    let line = Stroke::solid("#b4b4b4", 180);
    pen.rect(r, Some("#f2f2f2"), Some(&line), 0);
    let (x0, y0, x1, y1) = (
        f64::from(r.left),
        f64::from(r.top),
        r.right() as f64,
        r.bottom() as f64,
    );
    pen.lines(&[[[x0, y0], [x1, y1]], [[x0, y1], [x1, y0]]], &line);
}

pub(crate) fn picture(ctx: &Ctx, it: &Item, p: &PictureItem, pen: &mut Pen, notes: &mut Vec<Note>) {
    let r = it.content_rect();
    let Some(sha) = &p.asset else {
        notes.push(Note {
            item: it.id.clone(),
            code: "picture_empty",
            detail: String::new(),
        });
        placeholder(pen, r);
        return;
    };
    let meta = ctx.book.asset(sha);
    let have = ctx
        .inputs
        .assets
        .as_ref()
        .is_none_or(|a| a.iter().any(|x| x == sha));
    let Some(m) = meta.filter(|m| have && m.width > 0 && m.height > 0) else {
        notes.push(Note {
            item: it.id.clone(),
            code: "missing_asset",
            detail: sha.clone(),
        });
        placeholder(pen, r);
        return;
    };
    let (iw, ih) = (f64::from(m.width), f64::from(m.height));
    let (rw, rh) = (f64::from(r.width), f64::from(r.height));
    let (w, h) = match p.fit {
        PictureFit::Contain => {
            let s = (rw / iw).min(rh / ih);
            (iw * s, ih * s)
        }
        PictureFit::Cover => {
            let s = (rw / iw).max(rh / ih);
            (iw * s, ih * s)
        }
        PictureFit::Stretch => (rw, rh),
        PictureFit::Original => {
            let dpi = f64::from(m.dpi.unwrap_or(96).max(1));
            (iw * 25_400.0 / dpi, ih * 25_400.0 / dpi)
        }
    };
    let x = f64::from(r.left) + (rw - w) / 2.0;
    let y = f64::from(r.top) + (rh - h) / 2.0;
    let img = RectUm::new(round_um(x), round_um(y), round_um(w), round_um(h));
    let clip = p.clip && !r.contains_rect(&img);
    if clip {
        pen.push_clip(r);
    }
    pen.image(sha, img, 100);
    if clip {
        pen.pop_clip();
    }
}

pub(crate) fn shape(it: &Item, s: &ShapeItem, pen: &mut Pen) {
    let r = it.content_rect();
    let fill = s.fill.as_deref();
    let stroke = s.stroke.as_ref();
    let (x0, y0, x1, y1) = (
        f64::from(r.left),
        f64::from(r.top),
        r.right() as f64,
        r.bottom() as f64,
    );
    match &s.shape {
        ShapeKind::Rect(rs) => pen.rect(r, fill, stroke, rs.radius.min(r.width.min(r.height) / 2)),
        ShapeKind::Ellipse(_) => {
            let c = r.center();
            pen.ellipse(
                c,
                f64::from(r.width) / 2.0,
                f64::from(r.height) / 2.0,
                fill,
                stroke,
            );
        }
        ShapeKind::Triangle(_) => pen.path(
            &[[(x0 + x1) / 2.0, y0], [x1, y1], [x0, y1]],
            true,
            fill,
            stroke,
        ),
        ShapeKind::Polygon(p) => {
            let pts: Vec<[f64; 2]> = p.points.iter().map(|q| frame_point(&r, *q)).collect();
            pen.path(&pts, true, fill, stroke);
        }
    }
}

pub(crate) fn line(it: &Item, l: &LineItem, pen: &mut Pen) {
    let pts: Vec<[f64; 2]> = l
        .points
        .iter()
        .map(|q| frame_point(&it.frame, *q))
        .collect();
    pen.path(&pts, false, None, Some(&l.stroke));
    let n = pts.len();
    if n < 2 {
        return;
    }
    let ends = [(l.start, pts[0], pts[1]), (l.end, pts[n - 1], pts[n - 2])];
    let w = f64::from(l.stroke.width.max(100));
    for (kind, tip, from) in ends {
        let (dx, dy) = (tip[0] - from[0], tip[1] - from[1]);
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        match kind {
            LineEnd::None => {}
            LineEnd::Arrow => {
                let a = (6.0 * w).max(2_500.0);
                let half = a * 0.35;
                let base = [tip[0] - ux * a, tip[1] - uy * a];
                pen.path(
                    &[
                        tip,
                        [base[0] - uy * half, base[1] + ux * half],
                        [base[0] + uy * half, base[1] - ux * half],
                    ],
                    true,
                    Some(&l.stroke.color),
                    None,
                );
            }
            LineEnd::Dot => {
                let rad = (1.6 * w).max(600.0);
                pen.ellipse(tip, rad, rad, Some(&l.stroke.color), None);
            }
            LineEnd::Bar => {
                let half = (3.0 * w).max(1_200.0);
                pen.lines(
                    &[[
                        [tip[0] - uy * half, tip[1] + ux * half],
                        [tip[0] + uy * half, tip[1] - ux * half],
                    ]],
                    &Stroke {
                        dash: Vec::new(),
                        ..l.stroke.clone()
                    },
                );
            }
        }
    }
}
