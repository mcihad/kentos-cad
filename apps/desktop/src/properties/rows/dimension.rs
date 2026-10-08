//! A dimension's rows of docs/adr/0147 §7 in Öznitelikler, for one dimension
//! or the dimensions of a selection (the web's `dimensionRows.ts`): Zemin; its
//! lines' colours, weights and types and its value's colour (docs/adr/0205
//! §6); an ordinate's Koordinat (Y or X); a slope's two elevations; an arc
//! length's radius and angle, shown only. Each row is the dimensions' common
//! value, or “Çeşitli”; a change writes them in one step “Değiştir”, those
//! that already have the value left out. On a locked layer the rows only show.

use kentos_contracts::{DimensionEntity, DimensionStyle, Entity, LineType};
use kentos_domain::Slot;
use kentos_interaction::{Format, dimension_layout, dist};

use super::{Choice, Editor, Row, v};
use crate::app::Message;
use crate::properties::{Event, Field, Part};
use crate::ribbon_panels::{LINE_TYPES, LINE_WEIGHTS, line_colors, weight_text};

const MIXED: &str = "Çeşitli";
/// A line's colour when its look names none: the object's.
const OBJECT_COLOR: &str = "Nesnenin rengi";
/// A line's weight when its look names none.
const HAIRLINE: &str = "Kılcal";

/// A colour's name among the line colours, else its value (the web's `lineColorName`).
pub(crate) fn line_color_name(v: Option<&str>) -> String {
    match v {
        None => OBJECT_COLOR.to_owned(),
        Some(v) => line_colors()
            .find(|(_, c)| c.eq_ignore_ascii_case(v))
            .map_or_else(|| v.to_uppercase(), |(name, _)| name.to_owned()),
    }
}

/// A part's colour, weight and type fields of a look.
fn color_of(d: &DimensionEntity, part: Part) -> Option<&str> {
    match part {
        Part::Line => d.look.dim_line_color.as_deref(),
        Part::Ext => d.look.ext_color.as_deref(),
        Part::Value => d.look.text_color.as_deref(),
    }
}

fn weight_of(d: &DimensionEntity, part: Part) -> Option<f64> {
    match part {
        Part::Line => d.look.dim_line_weight,
        Part::Ext | Part::Value => d.look.ext_weight,
    }
}

fn type_of(d: &DimensionEntity, part: Part) -> LineType {
    match part {
        Part::Line => d.look.dim_line_type,
        Part::Ext | Part::Value => d.look.ext_line_type,
    }
    .unwrap_or(LineType::Continuous)
}

/// A line's colour row: the object's, or one of the line colours (one of
/// another program's as it is), the swatch beside it.
fn color_row(
    label: &'static str,
    part: Part,
    dims: &[&DimensionEntity],
    slots: &[Slot],
    locked: bool,
) -> Row {
    let c = common(dims, |d| color_of(d, part).map(str::to_uppercase));
    let text = c
        .as_ref()
        .map_or_else(|| MIXED.to_owned(), |c| line_color_name(c.as_deref()));
    let pick =
        |label: String, swatch: Option<String>, chosen: bool, color: Option<String>| Choice::Pick {
            label,
            swatch,
            icon: None,
            wide: false,
            chosen,
            enabled: true,
            message: Message::Properties(Event::DimensionLineColor(slots.to_vec(), part, color)),
        };
    let mut items = vec![
        pick(OBJECT_COLOR.to_owned(), None, c == Some(None), None),
        Choice::Separator,
    ];
    items.extend(line_colors().map(|(name, value)| {
        pick(
            name.to_owned(),
            Some(value.to_owned()),
            c == Some(Some(value.to_uppercase())),
            Some(value.to_owned()),
        )
    }));
    let editor = Editor::Select {
        text: text.clone(),
        swatch: c.clone().flatten(),
        icon: None,
        items,
    };
    Row::text(label, text).editor((!locked).then_some(editor))
}

/// A line's weight row on paper: a hairline, or one of the drawing's
/// (another program's among them).
fn weight_row(
    label: &'static str,
    part: Part,
    dims: &[&DimensionEntity],
    slots: &[Slot],
    locked: bool,
) -> Row {
    let w = common(dims, |d| weight_of(d, part));
    let name = |w: Option<f64>| w.map_or_else(|| HAIRLINE.to_owned(), weight_text);
    let text = w.map_or_else(|| MIXED.to_owned(), name);
    let mut weights = LINE_WEIGHTS.to_vec();
    if let Some(Some(x)) = w
        && !weights.contains(&x)
    {
        weights.push(x);
        weights.sort_by(f64::total_cmp);
    }
    let pick = |value: Option<f64>| Choice::Pick {
        label: name(value),
        swatch: None,
        icon: None,
        wide: false,
        chosen: w == Some(value),
        enabled: true,
        message: Message::Properties(Event::DimensionLineWeight(slots.to_vec(), part, value)),
    };
    let mut items = vec![pick(None), Choice::Separator];
    items.extend(weights.into_iter().map(|x| pick(Some(x))));
    let editor = Editor::Select {
        text: text.clone(),
        swatch: None,
        icon: None,
        items,
    };
    Row::text(label, text).editor((!locked).then_some(editor))
}

/// A line's type row: continuous (none) or another.
fn type_row(
    label: &'static str,
    part: Part,
    dims: &[&DimensionEntity],
    slots: &[Slot],
    locked: bool,
) -> Row {
    let t = common(dims, |d| type_of(d, part));
    let name = |t: LineType| {
        LINE_TYPES
            .iter()
            .find(|(x, _)| *x == t)
            .map_or("", |(_, l)| *l)
    };
    let text = t.map_or(MIXED, name);
    let editor = select(
        text,
        LINE_TYPES.iter().map(|(x, l)| {
            (
                *l,
                t == Some(*x),
                Event::DimensionLineType(
                    slots.to_vec(),
                    part,
                    (*x != LineType::Continuous).then_some(*x),
                ),
            )
        }),
    );
    Row::text(label, text).editor((!locked).then_some(editor))
}

/// The value every dimension has, or none when they differ (NaN never matches, as the web's `===`).
fn common<T: PartialEq>(
    dims: &[&DimensionEntity],
    of: impl Fn(&DimensionEntity) -> T,
) -> Option<T> {
    let first = of(dims[0]);
    dims[1..].iter().all(|d| of(d) == first).then_some(first)
}

/// An ordinate's axis: Y for its angle 0 (or none), X for 90.
pub(crate) fn is_y_axis(angle: Option<f64>) -> bool {
    angle.unwrap_or(0.0) == 0.0
}

fn on_off(on: bool) -> &'static str {
    if on { "Açık" } else { "Kapalı" }
}

/// A drop-down of `choices`: each one's label, whether it is the value, and its event.
fn select(text: &str, choices: impl IntoIterator<Item = (&'static str, bool, Event)>) -> Editor {
    Editor::Select {
        text: text.to_owned(),
        swatch: None,
        icon: None,
        items: choices
            .into_iter()
            .map(|(label, chosen, event)| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen,
                enabled: true,
                message: Message::Properties(event),
            })
            .collect(),
    }
}

pub(super) fn rows(
    dims: &[&DimensionEntity],
    slots: &[Slot],
    locked: bool,
    f: &Format,
) -> Vec<Row> {
    if dims.is_empty() {
        return Vec::new();
    }
    let edit = |editor: Editor| (!locked).then_some(editor);
    let mask = common(dims, |d| d.mask);
    let mask_text = mask.map_or(MIXED, on_off);
    let mut rows = vec![Row::text("Zemin", mask_text).editor(edit(select(
        mask_text,
        [true, false].map(|on| {
            (
                on_off(on),
                mask == Some(on),
                Event::DimensionMask(slots.to_vec(), on),
            )
        }),
    )))];
    // Its lines (docs/adr/0205 §6): the dimension line's (and its
    // arrowheads'), the extension lines', the value's colour.
    rows.extend([
        color_row("Çizgi rengi", Part::Line, dims, slots, locked),
        weight_row("Çizgi kalınlığı", Part::Line, dims, slots, locked),
        type_row("Çizgi tipi", Part::Line, dims, slots, locked),
        color_row("Uzatma rengi", Part::Ext, dims, slots, locked),
        weight_row("Uzatma kalınlığı", Part::Ext, dims, slots, locked),
        type_row("Uzatma tipi", Part::Ext, dims, slots, locked),
        color_row("Değer rengi", Part::Value, dims, slots, locked),
    ]);
    // An ordinate's axis: its Y (0) or its X (90).
    if dims
        .iter()
        .all(|d| d.style == Some(DimensionStyle::Ordinate))
    {
        let axis = common(dims, |d| if is_y_axis(d.angle) { "Y" } else { "X" });
        let text = axis.unwrap_or(MIXED);
        rows.push(Row::text("Koordinat", text).editor(edit(select(
            text,
            [("Y", 0.0), ("X", 90.0)].map(|(name, angle)| {
                (
                    name,
                    axis == Some(name),
                    Event::DimensionAxis(slots.to_vec(), angle),
                )
            }),
        ))));
    }
    // A slope's two elevations, metres.
    if dims.iter().all(|d| d.style == Some(DimensionStyle::Slope)) {
        for (label, second) in [("Birinci kot", false), ("İkinci kot", true)] {
            let z = common(dims, |d| {
                if second { d.zb } else { d.za }.unwrap_or(f64::NAN)
            });
            let row = match z {
                Some(z) => Row::figure(label, f.length_bare(z)).unit(f.length_unit_label()),
                None => Row::text(label, MIXED),
            };
            let field = if second {
                Field::DimensionZb(slots.to_vec())
            } else {
                Field::DimensionZa(slots.to_vec())
            };
            rows.push(row.editor(edit(Editor::Number(field))));
        }
    }
    // An arc length's radius and the arc's angle, shown only.
    if dims
        .iter()
        .all(|d| d.style == Some(DimensionStyle::ArcLength))
    {
        let radius_of = |d: &DimensionEntity| d.c.map_or(f64::NAN, |c| dist(v(d.a), v(c)));
        let radius = common(dims, radius_of);
        // The layout's value is the arc's length: its angle is that over its radius.
        let sweep = common(dims, |d| {
            dimension_layout(&Entity::Dimension(d.clone()))
                .map_or(f64::NAN, |l| l.value / radius_of(d))
        });
        rows.push(match radius {
            Some(r) => Row::figure("Yarıçap", f.length_bare(r)).unit(f.length_unit_label()),
            None => Row::text("Yarıçap", MIXED),
        });
        rows.push(match sweep {
            Some(s) => Row::figure("Açı", f.angle_bare(s)).unit(f.angle_unit_label()),
            None => Row::text("Açı", MIXED),
        });
    }
    rows
}
