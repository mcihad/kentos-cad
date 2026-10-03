//! A legend: the host's rows (its legend engine, ADR 0093) filtered and
//! ordered as the item says, laid out here in columns of symbol and label,
//! under a title, with group headings.

use super::{Ctx, LegendEntryInput, LegendSymbol, MarkerShape, Note, Pen};
use crate::kinds::*;
use crate::model::Item;
use crate::style::{HAlign, Stroke, VAlign};
use crate::text::{self, Layout};
use crate::units::*;

enum Row<'a> {
    Heading(text::Block),
    Entry(&'a LegendSymbol, text::Block, i64),
}

fn rows<'a>(ctx: &'a Ctx, it: &Item, l: &LegendItem) -> Vec<(&'a LegendEntryInput, String)> {
    let Some(input) = ctx.inputs.legends.iter().find(|x| x.item == it.id) else {
        return Vec::new();
    };
    let keep = |e: &LegendEntryInput| match l.filter {
        LegendFilter::All => true,
        LegendFilter::VisibleInMap => e.in_map,
        LegendFilter::AtlasFeature => e.in_atlas,
    };
    match &l.entries {
        LegendEntries::Auto(_) => input
            .entries
            .iter()
            .filter(|e| keep(e))
            .map(|e| (e, e.label.clone()))
            .collect(),
        LegendEntries::Custom(c) => c
            .entries
            .iter()
            .filter(|o| !o.hidden)
            .filter_map(|o| {
                input
                    .entries
                    .iter()
                    .find(|e| e.layer == o.layer && keep(e))
                    .map(|e| (e, o.label.clone().unwrap_or_else(|| e.label.clone())))
            })
            .collect(),
    }
}

pub(crate) fn draw(ctx: &Ctx, it: &Item, l: &LegendItem, pen: &mut Pen, notes: &mut Vec<Note>) {
    let c = it.content_rect();
    let mut top = i64::from(c.top);
    let bottom = c.bottom();
    let mut title_over = false;
    let title = ctx.render(it, &l.title, notes);
    if !title.trim().is_empty() {
        let b = text::layout(
            &title,
            &l.title_style,
            &RectUm::new(c.left, sat(top), c.width, sat(bottom - top)),
            &Layout {
                align: HAlign::Left,
                valign: VAlign::Top,
                line_height: 115,
                wrap: true,
                fit: TextFit::None,
            },
        );
        pen.block(&b, &l.title_style);
        top += b.height + i64::from(l.spacing.title_gap);
        // A title taller than the frame, or a word of it wider: said with the labels' below.
        title_over = b.overflow;
    }
    let entries = rows(ctx, it, l);
    if entries.is_empty() {
        notes.push(Note {
            item: it.id.clone(),
            code: "legend_empty",
            detail: String::new(),
        });
        return;
    }
    let cols = i64::from(l.columns.max(1));
    let gap = i64::from(l.spacing.column_gap);
    let col_w = ((i64::from(c.width) - (cols - 1) * gap) / cols).max(1_000);
    let sym_w = i64::from(l.symbol.width);
    let sym_h = i64::from(l.symbol.height);
    let label_w = {
        let room = (col_w - sym_w - i64::from(l.spacing.symbol_gap)).max(1_000);
        if l.wrap_width > 0 {
            room.min(i64::from(l.wrap_width))
        } else {
            room
        }
    };
    let lay = |t: &str, style: &crate::style::TextStyle, w: i64| {
        text::layout(
            t,
            style,
            &RectUm::new(0, 0, sat(w), sat(1_000_000)),
            &Layout {
                align: HAlign::Left,
                valign: VAlign::Top,
                line_height: 115,
                wrap: true,
                fit: TextFit::None,
            },
        )
    };
    // Rows with their heights; a heading when the group changes.
    let mut list: Vec<(Row, i64, bool)> = Vec::new();
    let mut group: Option<&str> = None;
    // A word of a heading or a label wider than its column: it runs past the column.
    let mut wide = title_over;
    for (e, label) in &entries {
        if e.group.as_deref() != group {
            group = e.group.as_deref();
            if let Some(g) = group {
                let b = lay(g, &l.group_style, col_w);
                let h = b.height;
                wide |= b.overflow;
                list.push((Row::Heading(b), h, true));
            }
        }
        let b = lay(label, &l.label_style, label_w);
        let h = b.height.max(sym_h);
        wide |= b.overflow;
        list.push((Row::Entry(&e.symbol, b, h), h, false));
    }
    if wide {
        notes.push(super::marks::written_overflow(&it.id, "lejantın yazısı"));
    }
    // Columns of about equal height, filled in order.
    let row_gap = i64::from(l.spacing.row_gap);
    let group_gap = i64::from(l.spacing.group_gap);
    let total: i64 = list.iter().map(|(_, h, _)| h + row_gap).sum();
    let target = (total + cols - 1) / cols;
    let mut col = 0i64;
    let mut y = top;
    let mut used = 0i64;
    let mut overflow = false;
    for (k, (row, h, heading)) in list.iter().enumerate() {
        let lead = if *heading && used > 0 {
            group_gap - row_gap
        } else {
            0
        };
        if used > 0 && used + lead + h > target && col + 1 < cols {
            col += 1;
            y = top;
            used = 0;
        }
        let lead = if *heading && used > 0 {
            group_gap - row_gap
        } else {
            0
        };
        y += lead;
        if y + h > bottom {
            overflow = true;
            break;
        }
        let x = i64::from(c.left) + col * (col_w + gap);
        match row {
            Row::Heading(b) => {
                draw_block(pen, b, x, y, &l.group_style);
            }
            Row::Entry(sym, b, rh) => {
                let sy = y + (rh - sym_h) / 2;
                symbol(
                    pen,
                    sym,
                    RectUm::new(sat(x), sat(sy), sat(sym_w), sat(sym_h)),
                );
                let ly = y + (rh - b.height) / 2;
                draw_block(
                    pen,
                    b,
                    x + sym_w + i64::from(l.spacing.symbol_gap),
                    ly,
                    &l.label_style,
                );
            }
        }
        y += h + row_gap;
        used += lead + h + row_gap;
        let _ = k;
    }
    if overflow {
        notes.push(Note {
            item: it.id.clone(),
            code: "legend_overflow",
            detail: String::new(),
        });
    }
}

/// A block laid out at the origin, drawn with its top-left at `(x, y)`.
fn draw_block(pen: &mut Pen, b: &text::Block, x: i64, y: i64, style: &crate::style::TextStyle) {
    for line in &b.lines {
        pen.text(
            &line.text,
            [(x + line.x) as f64, (y + line.baseline) as f64],
            style,
            b.size,
            line.width,
            0,
        );
    }
}

pub(crate) fn symbol(pen: &mut Pen, sym: &LegendSymbol, r: RectUm) {
    let (x0, y0) = (f64::from(r.left), f64::from(r.top));
    let (w, h) = (f64::from(r.width), f64::from(r.height));
    match sym {
        LegendSymbol::Patch(p) => {
            pen.rect(r, p.fill.as_deref(), None, 0);
            if let Some(hc) = &p.hatch {
                pen.push_clip(r);
                let step = 1_000.0;
                let mut segs = Vec::new();
                let mut t = -h;
                while t < w {
                    segs.push([[x0 + t, y0 + h], [x0 + t + h, y0]]);
                    t += step;
                }
                pen.lines(&segs, &Stroke::solid(hc, 150));
                pen.pop_clip();
            }
            if let Some(s) = &p.stroke {
                pen.rect(r, None, Some(s), 0);
            }
        }
        LegendSymbol::Line(l) => {
            let y = y0 + h / 2.0;
            pen.path(&[[x0, y], [x0 + w, y]], false, None, Some(&l.stroke));
        }
        LegendSymbol::Marker(m) => {
            let d = f64::from(m.size).min(h).min(w);
            let (cx, cy) = (x0 + w / 2.0, y0 + h / 2.0);
            let half = d / 2.0;
            match m.shape {
                MarkerShape::Circle => {
                    pen.ellipse([cx, cy], half, half, m.fill.as_deref(), m.stroke.as_ref())
                }
                MarkerShape::Square => pen.rect(
                    RectUm::new(
                        round_um(cx - half),
                        round_um(cy - half),
                        round_um(d),
                        round_um(d),
                    ),
                    m.fill.as_deref(),
                    m.stroke.as_ref(),
                    0,
                ),
                MarkerShape::Triangle => pen.path(
                    &[
                        [cx, cy - half],
                        [cx + half, cy + half],
                        [cx - half, cy + half],
                    ],
                    true,
                    m.fill.as_deref(),
                    m.stroke.as_ref(),
                ),
                MarkerShape::Cross => {
                    let s = m.stroke.clone().unwrap_or_else(|| {
                        Stroke::solid(m.fill.as_deref().unwrap_or("#000000"), 250)
                    });
                    pen.lines(
                        &[
                            [[cx - half, cy], [cx + half, cy]],
                            [[cx, cy - half], [cx, cy + half]],
                        ],
                        &s,
                    );
                }
            }
        }
        LegendSymbol::Paths(p) => {
            for path in &p.paths {
                let pts: Vec<[f64; 2]> = path
                    .points
                    .iter()
                    .map(|q| {
                        [
                            x0 + w * f64::from(q[0]) / 1000.0,
                            y0 + h * f64::from(q[1]) / 1000.0,
                        ]
                    })
                    .collect();
                pen.path(
                    &pts,
                    path.closed,
                    path.fill.as_deref(),
                    path.stroke.as_ref(),
                );
            }
        }
        LegendSymbol::Image(i) => pen.image(&i.asset, r, 100),
    }
}
