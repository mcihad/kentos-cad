//! The title block (antet): rows of cells by weight, a cell split into rows
//! of its own where it spans its neighbours; each cell a small caption in
//! its corner and a value under it (shrunk to fit, never cut), a signature
//! place or a logo. One item, moved and resized whole (QGIS Q6).

use super::{Ctx, Note, Pen};
use crate::kinds::*;
use crate::model::Item;
use crate::style::{HAlign, TextStyle, VAlign};
use crate::text::{self, Layout};
use crate::units::*;

pub(crate) fn draw(
    ctx: &Ctx,
    it: &Item,
    tb: &TitleBlockItem,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
) {
    let r = it.content_rect();
    let mut seps: Vec<[[f64; 2]; 2]> = Vec::new();
    rows(ctx, it, tb, &tb.rows, r, pen, notes, &mut seps);
    pen.lines(&seps, &tb.inner);
    pen.rect(r, None, Some(&tb.outer), 0);
}

fn split(total: i64, weights: &[i64], k: usize, start: i64) -> (i64, i64) {
    let sum: i64 = weights.iter().sum::<i64>().max(1);
    let before: i64 = weights[..k].iter().sum();
    let a = start + total * before / sum;
    let b = if k + 1 == weights.len() {
        start + total
    } else {
        start + total * (before + weights[k]) / sum
    };
    (a, b)
}

#[allow(clippy::too_many_arguments)]
fn rows(
    ctx: &Ctx,
    it: &Item,
    tb: &TitleBlockItem,
    list: &[TitleRow],
    r: RectUm,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
    seps: &mut Vec<[[f64; 2]; 2]>,
) {
    let weights: Vec<i64> = list.iter().map(|x| i64::from(x.height.max(1))).collect();
    for (k, row) in list.iter().enumerate() {
        let (y0, y1) = split(i64::from(r.height), &weights, k, i64::from(r.top));
        if k > 0 {
            seps.push([
                [f64::from(r.left), y0 as f64],
                [r.right() as f64, y0 as f64],
            ]);
        }
        let ws: Vec<i64> = row
            .cells
            .iter()
            .map(|c| i64::from(c.width.max(1)))
            .collect();
        for (j, cell) in row.cells.iter().enumerate() {
            let (x0, x1) = split(i64::from(r.width), &ws, j, i64::from(r.left));
            if j > 0 {
                seps.push([[x0 as f64, y0 as f64], [x0 as f64, y1 as f64]]);
            }
            let cr = RectUm::new(sat(x0), sat(y0), sat(x1 - x0), sat(y1 - y0));
            if cell.rows.is_empty() {
                draw_cell(ctx, it, tb, cell, cr, pen, notes);
            } else {
                if let Some(f) = &cell.fill {
                    pen.rect(cr, Some(f), None, 0);
                }
                rows(ctx, it, tb, &cell.rows, cr, pen, notes, seps);
            }
        }
    }
}

fn draw_cell(
    ctx: &Ctx,
    it: &Item,
    tb: &TitleBlockItem,
    cell: &TitleCell,
    r: RectUm,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
) {
    if let Some(f) = &cell.fill {
        pen.rect(r, Some(f), None, 0);
    }
    let pad = i64::from(tb.cell_padding);
    let inner = r.inset(sat(pad));
    let mut top = i64::from(inner.top);
    if !cell.label.is_empty() {
        let lh = text::line_height_of(&tb.label_style, 100);
        let b = text::layout(
            &cell.label,
            &tb.label_style,
            &RectUm::new(inner.left, inner.top, inner.width, sat(lh)),
            &Layout {
                align: HAlign::Left,
                valign: VAlign::Top,
                line_height: 100,
                wrap: false,
                fit: TextFit::ShrinkToFit,
            },
        );
        pen.block(&b, &tb.label_style);
        top += lh + 300;
    }
    let value_box = RectUm::new(
        inner.left,
        sat(top),
        inner.width,
        sat((inner.bottom() - top).max(0)),
    );
    if let Some(sha) = &cell.picture {
        picture_in(ctx, it, sha, value_box, pen, notes);
    }
    let style: TextStyle = cell.style.clone().unwrap_or_else(|| tb.value_style.clone());
    let value = ctx.render(it, &cell.value, notes);
    if value.is_empty() {
        return;
    }
    if cell.signature {
        // The name at the bottom, a line to sign on above it.
        let lh = text::line_height_of(&style, 110);
        let name_box = RectUm::new(
            value_box.left,
            sat(value_box.bottom() - lh),
            value_box.width,
            sat(lh),
        );
        let b = text::layout(
            &value,
            &style,
            &name_box,
            &Layout {
                align: HAlign::Center,
                valign: VAlign::Bottom,
                line_height: 110,
                wrap: false,
                fit: TextFit::ShrinkToFit,
            },
        );
        pen.block(&b, &style);
        let y = (i64::from(name_box.top) - 600) as f64;
        let inset = f64::from(value_box.width) * 0.12;
        pen.lines(
            &[[
                [f64::from(value_box.left) + inset, y],
                [value_box.right() as f64 - inset, y],
            ]],
            &crate::style::Stroke {
                dash: vec![600, 400],
                ..tb.inner.clone()
            },
        );
        return;
    }
    let b = text::layout(
        &value,
        &style,
        &value_box,
        &Layout {
            align: cell.align,
            valign: cell.valign,
            line_height: 115,
            wrap: true,
            fit: TextFit::ShrinkToFit,
        },
    );
    if b.overflow {
        notes.push(Note {
            item: it.id.clone(),
            code: "text_overflow",
            detail: cell.label.clone(),
        });
    }
    pen.block(&b, &style);
}

/// A picture as large as fits a box, centred.
pub(crate) fn picture_in(
    ctx: &Ctx,
    it: &Item,
    sha: &str,
    r: RectUm,
    pen: &mut Pen,
    notes: &mut Vec<Note>,
) {
    let meta = ctx.book.asset(sha);
    let have = ctx
        .inputs
        .assets
        .as_ref()
        .is_none_or(|a| a.iter().any(|x| x == sha));
    match meta {
        Some(m) if have && m.width > 0 && m.height > 0 => {
            let s = (f64::from(r.width) / f64::from(m.width))
                .min(f64::from(r.height) / f64::from(m.height));
            let (w, h) = (f64::from(m.width) * s, f64::from(m.height) * s);
            let x = f64::from(r.left) + (f64::from(r.width) - w) / 2.0;
            let y = f64::from(r.top) + (f64::from(r.height) - h) / 2.0;
            pen.image(
                sha,
                RectUm::new(round_um(x), round_um(y), round_um(w), round_um(h)),
                100,
            );
        }
        _ => {
            notes.push(Note {
                item: it.id.clone(),
                code: "missing_asset",
                detail: sha.to_owned(),
            });
            super::basic::placeholder(pen, r);
        }
    }
}
