//! Tables and coordinate lists: one engine. A fixed table's cells are
//! `[% … %]` text; an attribute table's rows are the host's objects,
//! filtered, sorted and written here with the expression language; a
//! coordinate list's are the host's points (Y east, X north when the
//! project has a coordinate system; the drawing's own X and Y otherwise).
//! Columns are as wide as their content (or fixed); rows that do not fit
//! continue in the frames the item names, then are cut off and reported.

use std::rc::Rc;

use super::{Ctx, Note, Pen};
use crate::expr::{self, Eval, Scope};
use crate::kinds::*;
use crate::model::{Item, ItemId, VarValue};
use crate::style::{HAlign, TextStyle, VAlign};
use crate::text::{self, Layout};
use crate::units::*;

#[derive(Clone, Debug)]
pub(crate) enum RowData {
    Cells(Vec<String>),
    /// One cell across every column (a list's area).
    Span(String),
}

#[derive(Clone, Debug)]
pub(crate) struct Column {
    pub width: Um,
    pub align: HAlign,
}

#[derive(Clone, Debug)]
pub(crate) struct Model {
    pub title: String,
    pub title_style: TextStyle,
    pub header: Option<Vec<String>>,
    pub columns: Vec<Column>,
    pub rows: Vec<RowData>,
    pub header_style: CellStyle,
    pub cell_style: CellStyle,
    pub lines: TableLines,
    pub zebra: Option<String>,
    pub empty: EmptyTable,
}

/// The rows a frame draws.
#[derive(Clone, Debug)]
pub(crate) struct Chunk {
    pub model: Rc<Model>,
    pub start: usize,
    pub end: usize,
    pub first: bool,
}

fn cell_layout(align: HAlign) -> Layout {
    Layout {
        align,
        valign: VAlign::Middle,
        line_height: 115,
        wrap: true,
        fit: TextFit::None,
    }
}

fn number_text(v: &VarValue, decimals: Option<u8>) -> String {
    match (v, decimals) {
        (VarValue::Number(x), Some(d)) => kentos_geometry_core::display::fixed(*x, usize::from(d)),
        _ => expr::value_text(v),
    }
}

fn compare(a: &VarValue, b: &VarValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (VarValue::Null, VarValue::Null) => Ordering::Equal,
        (VarValue::Null, _) => Ordering::Greater,
        (_, VarValue::Null) => Ordering::Less,
        (VarValue::Number(x), VarValue::Number(y)) => x.total_cmp(y),
        _ => kentos_expression::js::collate::compare_tr(&expr::value_text(a), &expr::value_text(b)),
    }
}

/// The model of a table or a coordinate list; none for a continuation frame.
pub(crate) fn model_of(ctx: &Ctx, it: &Item, notes: &mut Vec<Note>) -> Option<Model> {
    match &it.kind {
        ItemKind::Table(t) => {
            let columns: Vec<Column> = t
                .columns
                .iter()
                .map(|c| Column {
                    width: c.width,
                    align: c.align,
                })
                .collect();
            let rows = match &t.source {
                TableSource::Continued(_) => return None,
                TableSource::Fixed(f) => f
                    .rows
                    .iter()
                    .map(|r| RowData::Cells(r.iter().map(|c| ctx.render(it, c, notes)).collect()))
                    .collect(),
                TableSource::Layer(l) => layer_rows(ctx, it, t, l, notes),
            };
            let header = t.header.then(|| {
                t.columns
                    .iter()
                    .map(|c| ctx.render(it, &c.heading, notes))
                    .collect()
            });
            Some(Model {
                title: ctx.render(it, &t.title, notes),
                title_style: t.title_style.clone(),
                header,
                columns,
                rows,
                header_style: t.header_style.clone(),
                cell_style: t.cell_style.clone(),
                lines: t.lines.clone(),
                zebra: t.zebra.clone(),
                empty: t.empty.clone(),
            })
        }
        ItemKind::CoordinateList(c) => Some(coordinate_model(ctx, it, c, notes)),
        _ => None,
    }
}

fn layer_rows(
    ctx: &Ctx,
    it: &Item,
    t: &TableItem,
    l: &LayerRows,
    notes: &mut Vec<Note>,
) -> Vec<RowData> {
    let Some(input) = ctx.inputs.tables.iter().find(|x| x.item == it.id) else {
        notes.push(Note {
            item: it.id.clone(),
            code: "table_data_missing",
            detail: l.layer.clone(),
        });
        return Vec::new();
    };
    if let Some(m) = &l.only_in_map
        && !ctx.maps.contains_key(m)
    {
        notes.push(Note {
            item: it.id.clone(),
            code: "broken_link",
            detail: m.clone(),
        });
    }
    let base = ctx.scope_for(it);
    let row_scope = |f: &super::FeatureInput| -> Scope {
        let mut s = base.clone();
        s.fields = f
            .attributes
            .iter()
            .map(|a| (a.name.clone(), a.value.clone()))
            .collect();
        s.area = f.area;
        s.length = f.length;
        s
    };
    let mut reported = false;
    let mut kept: Vec<(&super::FeatureInput, Scope)> = Vec::new();
    for f in &input.features {
        if (l.only_in_map.is_some() && !f.in_map) || (l.atlas_filter && !f.in_atlas) {
            continue;
        }
        let s = row_scope(f);
        if !l.filter.trim().is_empty() {
            match expr::evaluate(&l.filter, &s) {
                Eval::Value(VarValue::Bool(true)) => {}
                Eval::Value(v)
                    if !matches!(v, VarValue::Bool(false) | VarValue::Null)
                        && expr::value_text(&v) != "0" => {}
                Eval::Error(e) if !reported => {
                    reported = true;
                    notes.push(super::eval_note(&it.id, &l.filter, &Eval::Error(e)));
                    continue;
                }
                _ => continue,
            }
        }
        kept.push((f, s));
    }
    if !l.sort.is_empty() {
        let keys: Vec<Vec<VarValue>> = kept
            .iter()
            .map(|(_, s)| {
                l.sort
                    .iter()
                    .map(|k| match expr::evaluate(&k.expression, s) {
                        Eval::Value(v) => v,
                        _ => VarValue::Null,
                    })
                    .collect()
            })
            .collect();
        let mut order: Vec<usize> = (0..kept.len()).collect();
        order.sort_by(|&a, &b| {
            for (i, k) in l.sort.iter().enumerate() {
                let o = compare(&keys[a][i], &keys[b][i]);
                let o = if k.descending { o.reverse() } else { o };
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            a.cmp(&b)
        });
        kept = order.into_iter().map(|i| kept[i].clone()).collect();
    }
    let mut missing_reported = false;
    kept.iter()
        .map(|(_, s)| {
            RowData::Cells(
                t.columns
                    .iter()
                    .map(|c| match expr::evaluate(&c.value, s) {
                        Eval::Value(v) => number_text(&v, c.decimals),
                        Eval::Null => String::new(),
                        e => {
                            if !missing_reported {
                                missing_reported = true;
                                notes.push(super::eval_note(&it.id, &c.value, &e));
                            }
                            match e {
                                Eval::Missing(n) => expr::missing_mark(&n),
                                _ => expr::missing_mark("ifade"),
                            }
                        }
                    })
                    .collect(),
            )
        })
        .collect()
}

/// What a coordinate list's source gives, as its section says it
/// (docs/adr/0206 §2): whose objects, how many, and their points (a
/// closed figure's corners and area); `input` the host's for the list,
/// `layer` the source layer's name when it is one.
pub fn coordinate_summary(
    source: &CoordSource,
    input: Option<&super::CoordinateInput>,
    layer: Option<&str>,
) -> String {
    let objects = input.and_then(|i| i.objects).unwrap_or(0);
    let missing = input.and_then(|i| i.missing).unwrap_or(0);
    let points = input.map_or(0, |i| i.points.len());
    let what = match input {
        Some(i) if i.closed && points > 2 => {
            let area = i
                .area
                .map(|a| format!(", alanı {} m²", kentos_geometry_core::display::fixed(a, 2)))
                .unwrap_or_default();
            format!("kapalı şeklin {points} köşesi{area}")
        }
        _ => format!("{points} nokta"),
    };
    match source {
        CoordSource::Objects(o) if o.uids.is_empty() => {
            "Nesne alınmadı: çizimde nesneleri seçip “Seçimi al”a basın.".to_owned()
        }
        CoordSource::Objects(o) => {
            let gone = if missing > 0 {
                format!(" (çizimde olmayan: {missing})")
            } else {
                String::new()
            };
            format!("Seçilen {} nesne{gone}: {what}.", o.uids.len())
        }
        CoordSource::Layer(_) => {
            let name = layer.unwrap_or("Bulunamayan");
            if objects == 0 {
                format!("“{name}” katmanında nesne yok: liste boş.")
            } else {
                format!("“{name}” katmanı: {objects} nesne, {what}.")
            }
        }
        CoordSource::Selection(_) if objects == 0 => {
            "Çizimde seçili nesne yok: liste boş. Seçim değiştikçe liste de değişir; sabitlemek için “Seçimi al”.".to_owned()
        }
        CoordSource::Selection(_) => format!(
            "Çizimde şu an seçili {objects} nesne: {what}. Seçim değiştikçe liste de değişir; sabitlemek için “Seçimi al”."
        ),
    }
}

/// A coordinate list's headings and its area row's word (docs/adr/0206 §3):
/// each one given, else the default; east first in both, Y east in
/// surveying, X east in a local project's own axes.
pub fn coordinate_headings(c: &CoordinateListItem, georeferenced: bool) -> [String; 5] {
    let (e, n) = if georeferenced {
        ("Y (m)", "X (m)")
    } else {
        ("X (m)", "Y (m)")
    };
    let cols = c.columns.clone().unwrap_or_default();
    let or = |v: Option<String>, d: &str| {
        v.filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| d.to_owned())
    };
    [
        or(cols.point, "Nokta"),
        or(cols.east, e),
        or(cols.north, n),
        or(cols.z, "Z (m)"),
        or(cols.area, "Alan"),
    ]
}

fn coordinate_model(ctx: &Ctx, it: &Item, c: &CoordinateListItem, notes: &mut Vec<Note>) -> Model {
    let [point, e, n, z, area_word] = coordinate_headings(c, ctx.inputs.capabilities.georeferenced);
    let mut header = vec![point, e, n];
    if c.z {
        header.push(z);
    }
    let mut columns = vec![
        Column {
            width: 0,
            align: HAlign::Center,
        },
        Column {
            width: 0,
            align: HAlign::Right,
        },
        Column {
            width: 0,
            align: HAlign::Right,
        },
    ];
    if c.z {
        columns.push(Column {
            width: 0,
            align: HAlign::Right,
        });
    }
    let d = usize::from(c.decimals);
    let mut rows = Vec::new();
    let input = ctx.inputs.coordinates.iter().find(|x| x.item == it.id);
    if input.is_none() {
        notes.push(Note {
            item: it.id.clone(),
            code: "coordinates_missing",
            detail: String::new(),
        });
    }
    if let Some(input) = input {
        let name = |i: usize, p: &super::CoordPoint| match &c.naming {
            PointNaming::Given(_) => p.name.clone().unwrap_or_else(|| (i + 1).to_string()),
            PointNaming::Sequence(s) => format!("{}{}", s.prefix, u64::from(s.start) + i as u64),
        };
        let row = |i: usize, p: &super::CoordPoint| {
            let mut cells = vec![
                name(i, p),
                kentos_geometry_core::display::fixed(p.x, d),
                kentos_geometry_core::display::fixed(p.y, d),
            ];
            if c.z {
                cells.push(
                    p.z.map_or_else(String::new, |z| kentos_geometry_core::display::fixed(z, d)),
                );
            }
            RowData::Cells(cells)
        };
        for (i, p) in input.points.iter().enumerate() {
            rows.push(row(i, p));
        }
        if c.closing_row
            && input.closed
            && input.points.len() > 2
            && let Some(p) = input.points.first()
        {
            rows.push(row(0, p));
        }
        if c.area_row {
            let area = input.area.unwrap_or_else(|| {
                let pts: Vec<kentos_geometry_core::Vec2> = input
                    .points
                    .iter()
                    .map(|p| kentos_geometry_core::Vec2::new(p.x, p.y))
                    .collect();
                kentos_geometry_core::geometry::signed_area(&pts).abs()
            });
            rows.push(RowData::Span(format!(
                "{area_word} = {} m²",
                kentos_geometry_core::display::fixed(area, d.max(2))
            )));
        }
    }
    Model {
        title: ctx.render(it, &c.title, notes),
        title_style: c.title_style.clone(),
        header: Some(header),
        columns,
        rows,
        header_style: c.header_style.clone(),
        cell_style: c.cell_style.clone(),
        lines: c.lines.clone(),
        zebra: None,
        empty: EmptyTable::ShowHeader(Empty {}),
    }
}

/// Column widths for a frame `w` wide.
fn widths(m: &Model, w: i64) -> Vec<i64> {
    let n = m
        .columns
        .len()
        .max(m.header.as_ref().map_or(0, Vec::len))
        .max(1);
    let col = |i: usize| {
        m.columns.get(i).cloned().unwrap_or(Column {
            width: 0,
            align: HAlign::Left,
        })
    };
    let pad_c = 2 * i64::from(m.cell_style.padding);
    let pad_h = 2 * i64::from(m.header_style.padding);
    let mut natural = vec![0i64; n];
    if let Some(h) = &m.header {
        for (i, t) in h.iter().enumerate().take(n) {
            natural[i] = natural[i].max(text::text_width(t, &m.header_style.text) + pad_h);
        }
    }
    for r in &m.rows {
        if let RowData::Cells(cells) = r {
            for (i, t) in cells.iter().enumerate().take(n) {
                natural[i] = natural[i].max(text::text_width(t, &m.cell_style.text) + pad_c);
            }
        }
    }
    let fixed: i64 = (0..n)
        .filter(|&i| col(i).width > 0)
        .map(|i| i64::from(col(i).width))
        .sum();
    let autos: Vec<usize> = (0..n).filter(|&i| col(i).width <= 0).collect();
    let room = (w - fixed).max(0);
    let want: i64 = autos.iter().map(|&i| natural[i].max(1)).sum();
    let mut out: Vec<i64> = (0..n).map(|i| i64::from(col(i).width.max(0))).collect();
    if autos.is_empty() {
        return out;
    }
    let mut given = 0;
    for (k, &i) in autos.iter().enumerate() {
        let share = if k + 1 == autos.len() {
            room - given
        } else {
            room * natural[i].max(1) / want.max(1)
        };
        out[i] = share.max(pad_c + 1_000);
        given += share;
    }
    out
}

/// A row's height in a frame with these column widths.
fn row_height(m: &Model, r: &RowData, ws: &[i64], header: bool) -> i64 {
    let cs = if header {
        &m.header_style
    } else {
        &m.cell_style
    };
    let pad = 2 * i64::from(cs.padding);
    let h_of = |t: &str, w: i64| {
        text::layout(
            t,
            &cs.text,
            &RectUm::new(0, 0, sat(w - pad), sat(1_000_000)),
            &cell_layout(HAlign::Left),
        )
        .height
    };
    let line = text::line_height_of(&cs.text, 115);
    match r {
        RowData::Cells(cells) => {
            cells
                .iter()
                .zip(ws)
                .map(|(t, w)| h_of(t, *w))
                .max()
                .unwrap_or(0)
                .max(line)
                + pad
        }
        RowData::Span(t) => h_of(t, ws.iter().sum()).max(line) + pad,
    }
}

fn title_height(m: &Model, w: i64) -> i64 {
    if m.title.trim().is_empty() {
        return 0;
    }
    text::layout(
        &m.title,
        &m.title_style,
        &RectUm::new(0, 0, sat(w), sat(1_000_000)),
        &Layout {
            align: HAlign::Left,
            valign: VAlign::Top,
            line_height: 115,
            wrap: true,
            fit: TextFit::None,
        },
    )
    .height
        + 1_200
}

/// How many rows from `start` fit a frame.
fn fit(m: &Model, frame: &RectUm, start: usize, first: bool) -> usize {
    let w = i64::from(frame.width);
    let ws = widths(m, w);
    let mut used = if first { title_height(m, w) } else { 0 };
    if let Some(h) = &m.header {
        used += row_height(m, &RowData::Cells(h.clone()), &ws, true);
    }
    let mut k = start;
    while k < m.rows.len() {
        let h = row_height(m, &m.rows[k], &ws, false);
        if used + h > i64::from(frame.height) {
            break;
        }
        used += h;
        k += 1;
    }
    k
}

/// Lays out a table that continues in other frames, so they find their rows.
pub(crate) fn prepare(ctx: &Ctx, it: &Item, notes: &mut Vec<Note>) {
    let overflow = match &it.kind {
        ItemKind::Table(t) => &t.overflow,
        ItemKind::CoordinateList(c) => &c.overflow,
        _ => return,
    };
    let Overflow::ContinueIn(cont) = overflow else {
        return;
    };
    let mut scratch = Vec::new();
    let Some(model) = model_of(ctx, it, &mut scratch) else {
        return;
    };
    let _ = notes;
    let model = Rc::new(model);
    let mut frames: Vec<(ItemId, RectUm, bool)> = vec![(it.id.clone(), it.content_rect(), true)];
    for id in &cont.items {
        if let Some(target) = ctx.scene.item(id) {
            frames.push((id.clone(), target.content_rect(), false));
        }
    }
    let mut start = 0;
    let mut map = ctx.continued.borrow_mut();
    for (id, frame, first) in frames {
        let end = fit(&model, &frame, start, first);
        map.insert(
            id,
            Chunk {
                model: Rc::clone(&model),
                start,
                end,
                first,
            },
        );
        start = end;
    }
}

pub(crate) fn draw(ctx: &Ctx, it: &Item, pen: &mut Pen, notes: &mut Vec<Note>) {
    let prepared: Option<Chunk> = ctx.continued.borrow().get(&it.id).cloned();
    let chunk = match prepared {
        Some(c) => {
            if c.first {
                // The table itself: its notes (values not there) are said once, here.
                let _ = model_of(ctx, it, notes);
                let total = c.model.rows.len();
                let last_end = ctx
                    .continued
                    .borrow()
                    .values()
                    .filter(|x| Rc::ptr_eq(&x.model, &c.model))
                    .map(|x| x.end)
                    .max()
                    .unwrap_or(c.end);
                if last_end < total {
                    notes.push(Note {
                        item: it.id.clone(),
                        code: "table_overflow",
                        detail: format!("{}", total - last_end),
                    });
                }
            }
            c
        }
        None => {
            let Some(model) = model_of(ctx, it, notes) else {
                // A continuation frame no table continues in.
                notes.push(Note {
                    item: it.id.clone(),
                    code: "continuation_unused",
                    detail: String::new(),
                });
                return;
            };
            let model = Rc::new(model);
            let end = fit(&model, &it.content_rect(), 0, true);
            if end < model.rows.len() {
                notes.push(Note {
                    item: it.id.clone(),
                    code: "table_overflow",
                    detail: format!("{}", model.rows.len() - end),
                });
            }
            Chunk {
                model,
                start: 0,
                end,
                first: true,
            }
        }
    };
    draw_chunk(pen, &it.content_rect(), &chunk);
}

fn draw_chunk(pen: &mut Pen, frame: &RectUm, c: &Chunk) {
    let m = &c.model;
    // A continuation frame the rows did not reach stays empty.
    if !c.first && c.start >= c.end.min(m.rows.len()) {
        return;
    }
    let w = i64::from(frame.width);
    let ws = widths(m, w);
    let table_w: i64 = ws.iter().sum();
    let x0 = i64::from(frame.left);
    let mut y = i64::from(frame.top);
    if c.first && !m.title.trim().is_empty() {
        let b = text::layout(
            &m.title,
            &m.title_style,
            &RectUm::new(frame.left, frame.top, frame.width, frame.height),
            &Layout {
                align: HAlign::Left,
                valign: VAlign::Top,
                line_height: 115,
                wrap: true,
                fit: TextFit::None,
            },
        );
        pen.block(&b, &m.title_style);
        y += title_height(m, w);
    }
    let empty = m.rows.is_empty();
    if empty && matches!(m.empty, EmptyTable::Hide(_)) {
        return;
    }
    let top = y;
    let mut rules_h: Vec<i64> = Vec::new();
    let mut header_rule = None;
    if let Some(h) = &m.header {
        let row = RowData::Cells(h.clone());
        let rh = row_height(m, &row, &ws, true);
        draw_row(pen, m, &row, &ws, x0, y, rh, true, false);
        y += rh;
        header_rule = Some(y);
    }
    let rows: Vec<RowData> = if empty {
        match &m.empty {
            EmptyTable::Message(t) => vec![RowData::Span(t.text.clone())],
            _ => Vec::new(),
        }
    } else {
        m.rows[c.start..c.end.min(m.rows.len())].to_vec()
    };
    for (k, r) in rows.iter().enumerate() {
        let rh = row_height(m, r, &ws, false);
        if k > 0 {
            rules_h.push(y);
        }
        let zebra = m.zebra.is_some() && (c.start + k) % 2 == 1;
        draw_row(pen, m, r, &ws, x0, y, rh, false, zebra);
        y += rh;
    }
    let bottom = y;
    if bottom <= top {
        return;
    }
    // Rules: between rows, between columns (spanning rows break them), under the header, around.
    if let Some(s) = &m.lines.rows {
        let segs: Vec<[[f64; 2]; 2]> = rules_h
            .iter()
            .map(|&yy| [[x0 as f64, yy as f64], [(x0 + table_w) as f64, yy as f64]])
            .collect();
        pen.lines(&segs, s);
    }
    if let Some(s) = &m.lines.columns {
        let mut segs = Vec::new();
        let mut x = x0;
        for wi in ws.iter().take(ws.len().saturating_sub(1)) {
            x += wi;
            // Down to the first spanning row.
            let mut yy = top;
            let mut seg_top = top;
            if m.header.is_some() {
                yy += row_height(
                    m,
                    &RowData::Cells(m.header.clone().unwrap_or_default()),
                    &ws,
                    true,
                );
            }
            for r in &rows {
                let rh = row_height(m, r, &ws, false);
                if matches!(r, RowData::Span(_)) {
                    if yy > seg_top {
                        segs.push([[x as f64, seg_top as f64], [x as f64, yy as f64]]);
                    }
                    seg_top = yy + rh;
                }
                yy += rh;
            }
            if yy > seg_top {
                segs.push([[x as f64, seg_top as f64], [x as f64, yy as f64]]);
            }
        }
        pen.lines(&segs, s);
    }
    if let (Some(s), Some(hy)) = (&m.lines.header, header_rule)
        && hy < bottom
    {
        pen.lines(
            &[[[x0 as f64, hy as f64], [(x0 + table_w) as f64, hy as f64]]],
            s,
        );
    }
    if let Some(s) = &m.lines.outer {
        pen.rect(
            RectUm::new(sat(x0), sat(top), sat(table_w), sat(bottom - top)),
            None,
            Some(s),
            0,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_row(
    pen: &mut Pen,
    m: &Model,
    r: &RowData,
    ws: &[i64],
    x0: i64,
    y: i64,
    h: i64,
    header: bool,
    zebra: bool,
) {
    let cs = if header {
        &m.header_style
    } else {
        &m.cell_style
    };
    let total: i64 = ws.iter().sum();
    let fill = if header {
        cs.fill.clone()
    } else if zebra {
        m.zebra.clone()
    } else {
        cs.fill.clone()
    };
    if let Some(f) = &fill {
        pen.rect(
            RectUm::new(sat(x0), sat(y), sat(total), sat(h)),
            Some(f),
            None,
            0,
        );
    }
    let pad = i64::from(cs.padding);
    match r {
        RowData::Cells(cells) => {
            let mut x = x0;
            for (i, t) in cells.iter().enumerate() {
                let w = ws.get(i).copied().unwrap_or(0);
                let align = if header {
                    HAlign::Center
                } else {
                    m.columns.get(i).map_or(HAlign::Left, |c| c.align)
                };
                let b = text::layout(
                    t,
                    &cs.text,
                    &RectUm::new(
                        sat(x + pad),
                        sat(y + pad),
                        sat(w - 2 * pad),
                        sat(h - 2 * pad),
                    ),
                    &cell_layout(align),
                );
                pen.block(&b, &cs.text);
                x += w;
            }
        }
        RowData::Span(t) => {
            let b = text::layout(
                t,
                &cs.text,
                &RectUm::new(
                    sat(x0 + pad),
                    sat(y + pad),
                    sat(total - 2 * pad),
                    sat(h - 2 * pad),
                ),
                &cell_layout(HAlign::Left),
            );
            let style = TextStyle {
                weight: 600,
                ..cs.text.clone()
            };
            pen.block(&b, &style);
        }
    }
}
