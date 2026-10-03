//! A dimension's rows of docs/adr/0147 §7 in Öznitelikler, for one dimension
//! or the dimensions of a selection (the web's `dimensionRows.ts`): Zemin; an
//! ordinate's Koordinat (Y or X); a slope's two elevations; an arc length's
//! radius and angle, shown only. Each row is the dimensions' common value, or
//! “Çeşitli”; a change writes them in one step “Değiştir”, those that already
//! have the value left out. On a locked layer the rows only show.

use kentos_contracts::{DimensionEntity, DimensionStyle, Entity};
use kentos_domain::Slot;
use kentos_interaction::{Format, dimension_layout, dist};

use super::{Choice, Editor, Row, v};
use crate::app::Message;
use crate::properties::{Event, Field};

const MIXED: &str = "Çeşitli";

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
