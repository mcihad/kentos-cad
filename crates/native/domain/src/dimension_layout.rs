//! A dimension recipe's members dressed (docs/adr/0730): drawn at the recipe's sheet scales too, each as large on paper
//! as at the drawing's own, and their values placed where they do not fit, by the core's rules
//! (`ops::dimension_layout`) with the values as a reader sees them.

use kentos_contracts::{
    Annotative, AnnotativeScale, DimensionEntity, DimensionSet, DimensionSetRow, DimensionSetTight,
    DimensionTextPlace, DrawingFont, TextShift, same_scale,
};
use kentos_geometry_core::geom::dimension::look::DEFAULT_TEXT_GAP;
use kentos_geometry_core::jsmath::{PI, atan2, cos, sin};
use kentos_geometry_core::ops::dimension_layout::{
    Placement, RowLine, ValueBox, fits, place_values, scaled_offset,
};
use kentos_geometry_core::text::{Font, width_em};
use kentos_geometry_core::vec2::Vec2;

use crate::dimension_sets::PlannedDimensionSetMember;
use crate::document::Document;
use crate::format::Format;

/// A recipe's rows' frame: a point of its reference line (the measured face, or the bodies' outermost face on the rows'
/// side), its direction and its normal (the direction's left).
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowFrame {
    pub origin: Vec2,
    pub d: Vec2,
    pub n: Vec2,
}

impl RowFrame {
    fn t(&self, p: kentos_contracts::Vec2) -> f64 {
        (p.x - self.origin.x) * self.d.x + (p.y - self.origin.y) * self.d.y
    }

    fn o(&self, p: kentos_contracts::Vec2) -> f64 {
        (p.x - self.origin.x) * self.n.x + (p.y - self.origin.y) * self.n.y
    }
}

/// What a planned member's value does at one scale (docs/adr/0730 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ValueNote {
    /// It does not fit between its extension lines.
    pub tight: bool,
    /// The recipe moved it aside, above or below its line.
    pub placed: bool,
}

/// A planned member's value at the drawing's scale and at each of the recipe's sheet scales (other than the drawing's).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValueNotes {
    pub own: ValueNote,
    pub scales: Vec<(f64, ValueNote)>,
}

/// A recipe member's value as a reader sees it (docs/adr/0730 §2): its measured length along its angle, as the core lays
/// a straight dimension out, and its text as the document writes it.
pub fn member_value(format: &Format, d: &DimensionEntity) -> (f64, String) {
    let t = (d.angle.unwrap_or(0.0) * PI) / 180.0;
    let u = Vec2::new(cos(t), sin(t));
    let measured = ((d.b.x - d.a.x) * u.x + (d.b.y - d.a.y) * u.y).abs();
    let text = d.text.as_deref().filter(|t| !t.is_empty());
    (
        measured,
        format.dimension_value("", "length", measured, &d.look, text),
    )
}

/// Its value's width as the document draws it, in em: in the dimension's own face, else the drawing's.
pub fn value_width_em(font: Option<DrawingFont>, d: &DimensionEntity, shown: &str) -> f64 {
    width_em(shown, Font::from_id(DrawingFont::id_of(d.look.font.or(font))))
}

/// Whether two value places are the same: none is its own place.
fn same_shift(a: Option<TextShift>, b: Option<TextShift>) -> bool {
    let v = |s: Option<TextShift>| s.map_or((0.0, 0.0), |s| (s.along, s.across));
    v(a) == v(b)
}

fn shift_of(p: Placement) -> Option<TextShift> {
    match p {
        Placement::Moved(along, across) => Some(TextShift { along, across }),
        _ => None,
    }
}

/// One scale's account: each member's offset there, whether its value fits and where it goes.
struct Scaled {
    offsets: Vec<f64>,
    tight: Vec<bool>,
    placements: Vec<Placement>,
}

impl Document {
    /// A recipe's planned members dressed (docs/adr/0730 §1, §2): with sheet scales, their representations at each;
    /// with `shift`, their values placed at the drawing's scale (a value the user moved keeps its place) and at each sheet
    /// scale; each one's account. Their sources and their own geometry are the plan's, unchanged.
    pub(crate) fn dress_dimension_set(
        &self,
        set: &DimensionSet,
        members: &mut [PlannedDimensionSetMember],
        frame: &RowFrame,
    ) {
        if members.is_empty() {
            return;
        }
        let own = self.settings.plot_scale;
        let format = Format::of(&self.settings);
        let font = self.settings.drawing_font;
        let shifting = set.tight == DimensionSetTight::Shift;
        let recorded: Vec<Option<TextShift>> = members
            .iter()
            .map(|m| {
                m.uid
                    .and_then(|u| set.members.iter().find(|old| old.uid == u))
                    .and_then(|old| old.shift)
            })
            .collect();
        // The user moved a value when its place is not the recipe's last one.
        let moved: Vec<Option<(f64, f64)>> = members
            .iter()
            .zip(&recorded)
            .map(|(m, r)| {
                (!same_shift(m.dimension.text_shift, *r))
                    .then(|| m.dimension.text_shift.map_or((0.0, 0.0), |s| (s.along, s.across)))
            })
            .collect();
        let widths: Vec<f64> = members
            .iter()
            .map(|m| {
                let (_, shown) = member_value(&format, &m.dimension);
                value_width_em(font, &m.dimension, &shown)
            })
            .collect();
        // Rows in the recipe's order, along each.
        let row_rank = |r: DimensionSetRow| set.rows.iter().position(|x| *x == r).unwrap_or(usize::MAX);
        let centre = |m: &PlannedDimensionSetMember| {
            (frame.t(m.dimension.a) + frame.t(m.dimension.b)) * 0.5
        };
        let mut order: Vec<usize> = (0..members.len()).collect();
        order.sort_by(|&i, &j| {
            row_rank(members[i].segment.row)
                .cmp(&row_rank(members[j].segment.row))
                .then(centre(&members[i]).total_cmp(&centre(&members[j])))
        });
        let at = |scale: f64, fixed: bool| -> Scaled {
            let offsets: Vec<f64> = members
                .iter()
                .map(|m| {
                    let d = &m.dimension;
                    scaled_offset(d.offset, frame.o(d.a) + d.offset, scale, own)
                })
                .collect();
            let mut rows: Vec<DimensionSetRow> = Vec::new();
            let mut lines: Vec<RowLine> = Vec::new();
            let mut line_of = vec![0usize; members.len()];
            for &i in &order {
                let d = &members[i].dimension;
                let (ta, tb) = (frame.t(d.a), frame.t(d.b));
                let (t0, t1) = if ta <= tb { (ta, tb) } else { (tb, ta) };
                let row = members[i].segment.row;
                match rows.iter().position(|r| *r == row) {
                    Some(k) => {
                        line_of[i] = k;
                        lines[k].t0 = kentos_geometry_core::jsmath::js_min(lines[k].t0, t0);
                        lines[k].t1 = kentos_geometry_core::jsmath::js_max(lines[k].t1, t1);
                    }
                    None => {
                        line_of[i] = rows.len();
                        rows.push(row);
                        lines.push(RowLine {
                            o: frame.o(d.a) + offsets[i],
                            t0,
                            t1,
                        });
                    }
                }
            }
            let boxes: Vec<ValueBox> = order
                .iter()
                .map(|&i| {
                    let d = &members[i].dimension;
                    let h = d.height * scale / own;
                    let gap = d.look.text_gap.unwrap_or(DEFAULT_TEXT_GAP);
                    let lift = if d.look.text_place == Some(DimensionTextPlace::Centre) {
                        -(DEFAULT_TEXT_GAP * h)
                    } else {
                        gap * h
                    };
                    let (ta, tb) = (frame.t(d.a), frame.t(d.b));
                    // As the core lays the value out: along the line from a's side to b's, turned to read.
                    let t = (d.angle.unwrap_or(0.0) * PI) / 180.0;
                    let u = Vec2::new(cos(t), sin(t));
                    let sign = if (d.b.x - d.a.x) * u.x + (d.b.y - d.a.y) * u.y < 0.0 { -1.0 } else { 1.0 };
                    let line = Vec2::new(u.x * sign, u.y * sign);
                    let rotation = atan2(line.y, line.x) * 180.0 / PI;
                    let flip = if rotation > 90.0 || rotation <= -90.0 { -1.0 } else { 1.0 };
                    let reads = line.x * flip * frame.d.x + line.y * flip * frame.d.y;
                    ValueBox {
                        line: line_of[i],
                        t0: if ta <= tb { ta } else { tb },
                        t1: if ta <= tb { tb } else { ta },
                        width: widths[i] * h,
                        height: h,
                        lift,
                        gap,
                        reading: if reads < 0.0 { -1.0 } else { 1.0 },
                        fixed: if fixed { moved[i] } else { None },
                    }
                })
                .collect();
            let placed = if shifting {
                place_values(&boxes, &lines)
            } else {
                boxes
                    .iter()
                    .map(|b| if fits(b) { Placement::Fits } else { Placement::Tight })
                    .collect()
            };
            let mut placements = vec![Placement::Fits; members.len()];
            let mut tight = vec![false; members.len()];
            for (k, &i) in order.iter().enumerate() {
                placements[i] = placed[k];
                tight[i] = !fits(&boxes[k]);
            }
            Scaled {
                offsets,
                tight,
                placements,
            }
        };
        // The drawing's scale: a value moved by the user stays (and is in the others' way) while the recipe shifts.
        let mine = at(own, shifting);
        let sheet_scales: Vec<f64> = set
            .scales
            .iter()
            .copied()
            .filter(|s| !same_scale(*s, own))
            .collect();
        let theirs: Vec<(f64, Scaled)> = sheet_scales.iter().map(|&s| (s, at(s, false))).collect();
        for (i, m) in members.iter_mut().enumerate() {
            let p = mine.placements[i];
            m.notes.own = ValueNote {
                tight: mine.tight[i],
                placed: matches!(p, Placement::Moved(..)),
            };
            if p == Placement::Fixed {
                // The user's place; the recipe's last place is kept as it was, so it stays the user's.
                m.shift = recorded[i];
                continue;
            }
            if shifting {
                m.dimension.text_shift = shift_of(p);
                m.shift = shift_of(p);
            } else {
                // Keeping: a place the recipe made before (it was shifting) goes back to the value's own.
                if moved[i].is_none() && recorded[i].is_some() {
                    m.dimension.text_shift = None;
                }
                m.shift = if moved[i].is_some() { recorded[i] } else { None };
            }
        }
        let mut entries: Vec<Vec<AnnotativeScale>> = vec![Vec::new(); members.len()];
        for (scale, theirs) in theirs {
            for (i, m) in members.iter_mut().enumerate() {
                let p = theirs.placements[i];
                m.notes.scales.push((
                    scale,
                    ValueNote {
                        tight: theirs.tight[i],
                        placed: matches!(p, Placement::Moved(..)),
                    },
                ));
                entries[i].push(AnnotativeScale {
                    scale,
                    point: None,
                    rotation: None,
                    offset: Some(theirs.offsets[i]),
                    text_shift: shifting
                        .then(|| shift_of(p).unwrap_or(TextShift { along: 0.0, across: 0.0 })),
                });
            }
        }
        if !set.scales.is_empty() {
            for (m, scales) in members.iter_mut().zip(entries) {
                m.dimension.annotative = Some(Box::new(Annotative { at: own, scales }));
            }
        } else if self.dimension_set(set.id).is_some_and(|before| !before.scales.is_empty()) {
            // Its sheet scales gone, the representations it wrote go too.
            for m in members.iter_mut() {
                m.dimension.annotative = None;
            }
        }
    }
}

/// The rows' frame of a recipe on one wall edge (docs/adr/0695): the measured face line.
pub(crate) fn edge_frame(line: [Vec2; 2]) -> Option<RowFrame> {
    let frame = kentos_geometry_core::ops::dimension_set::Frame::new(line).ok()?;
    Some(RowFrame {
        origin: frame.start,
        d: frame.direction,
        n: frame.normal,
    })
}
