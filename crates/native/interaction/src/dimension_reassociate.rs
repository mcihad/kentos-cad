//! Yeniden bağla (docs/adr/0249 §3; AutoCAD's DIMREASSOCIATE): an
//! associative dimension's points bound to places on objects again. A
//! dimension is clicked; then each of its measured points in turn (Hizalı,
//! Doğrusal, Semt and Eğim: the first, then the second; Koordinat: its
//! point) is given its new place by a snap on an object's end, vertex,
//! centre, middle or point. Enter keeps the point as it is, Serbest (S) lets
//! it go. After the last point the bindings are written through
//! `cad.dimensions.reassociate` (step “Ölçüyü yeniden bağla”) and the tool
//! waits for the next dimension. The point asked for is ringed; the
//! dimension as it would be follows the cursor, its value beside it.

use kentos_contracts::{
    DimensionEntity, DimensionPoint, DimensionStyle, DimensionsReassociate, Entity, PlaceOn,
    PointBinding, SubPlace, SubRef,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, dimensions_reassociate};

use crate::Vec2;
use crate::dimension::snapped_place;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::spatial::dimension_layout;
use crate::tool::{Context, Cursor, Flow, Label, Pointer, Preview, Stroke, Tag, Tone, Tool};

/// The tool's id: its command is `tool.dimensionReassociate`.
pub const ID: &str = "dimensionReassociate";
pub const LABEL: &str = "Yeniden bağla";
/// A click where no dimension is.
pub const NO_DIMENSION_HERE: &str =
    "Tıklanan yerde ölçü yok; kilitsiz bir katmandaki ölçünün çizgisine tıklayın.";
/// A dimension of a kind taken from an object.
pub const NOT_BINDABLE: &str = "Bu ölçü nesneden alınır (Açı, Yarıçap, Çap, Yay uzunluğu, Kırıklı yarıçap); noktaları bağlanmaz.";
/// A click on no place of an object.
pub const NO_PLACE_HERE: &str = "Burada bir nesnenin yeri yok; kenetle bir ucuna, köşesine, merkezine, kenar ortasına ya da bir noktaya tıklayın.";

/// The measured points a dimension's kind binds, in order (docs/adr/0249 §3).
fn points_of(d: &DimensionEntity) -> &'static [DimensionPoint] {
    match d.style {
        None | Some(DimensionStyle::Linear | DimensionStyle::Azimuth | DimensionStyle::Slope) => {
            &[DimensionPoint::A, DimensionPoint::B]
        }
        Some(DimensionStyle::Ordinate) => &[DimensionPoint::A],
        _ => &[],
    }
}

fn point_at(d: &DimensionEntity, k: DimensionPoint) -> Vec2 {
    let p = match k {
        DimensionPoint::A => d.a,
        DimensionPoint::B => d.b,
        DimensionPoint::C => d.c.unwrap_or(d.a),
    };
    Vec2::new(p.x, p.y)
}

/// A place's name as the prompt and the tag say it.
pub fn place_words(place: &SubPlace) -> String {
    match *place {
        SubPlace::Node { .. } => "nokta".into(),
        SubPlace::Start => "başlangıç".into(),
        SubPlace::End => "bitiş".into(),
        SubPlace::Center => "merkez".into(),
        SubPlace::Vertex { index, .. } => format!("{}. köşe", index + 1),
        SubPlace::Mid { index, .. } => format!("{}. kenarın ortası", index + 1),
    }
}

/// A reference's object id as the command names it.
fn uid_text(r: &SubRef) -> String {
    kentos_domain::Uuid::from_bytes(r.uid.0).to_string()
}

/// A dimension whose points bind, on an unlocked layer: what the tool takes.
fn dimension(e: &Entity, doc: &Document) -> bool {
    matches!(e, Entity::Dimension(_)) && edge::unlocked(e, doc)
}

/// The dimension picked and what was given so far.
#[derive(Clone, Debug)]
struct Picking {
    slot: Slot,
    uid: String,
    was: DimensionEntity,
    points: &'static [DimensionPoint],
    /// The point asked for: an index into `points`.
    at: usize,
    bindings: Vec<PointBinding>,
    /// The dimension as the bindings so far and the cursor's place would make it.
    shown: DimensionEntity,
}

#[derive(Default)]
pub struct DimensionReassociate {
    picking: Option<Picking>,
    /// The cursor's place, when it stands on one.
    hovering: Option<(Vec2, SubRef)>,
    cursor: Option<Vec2>,
    format: Format,
}

fn ordinal(k: DimensionPoint) -> &'static str {
    match k {
        DimensionPoint::A => "birinci",
        DimensionPoint::B => "ikinci",
        DimensionPoint::C => "üçüncü",
    }
}

fn with_point(d: &DimensionEntity, k: DimensionPoint, p: Vec2) -> DimensionEntity {
    let mut d = d.clone();
    let p = kentos_contracts::Vec2 { x: p.x, y: p.y };
    match k {
        DimensionPoint::A => d.a = p,
        DimensionPoint::B => d.b = p,
        DimensionPoint::C => d.c = Some(p),
    }
    d
}

impl DimensionReassociate {
    pub fn new() -> Self {
        Self::default()
    }

    fn pick(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick(p, cx, dimension) else {
            cx.say(Level::Warn, NO_DIMENSION_HERE);
            return;
        };
        let Some(Entity::Dimension(d)) = cx.doc.get(slot) else {
            return;
        };
        let points = points_of(d);
        if points.is_empty() {
            cx.say(Level::Warn, NOT_BINDABLE);
            return;
        }
        self.picking = Some(Picking {
            slot,
            uid: edge::uid(cx.doc, slot),
            was: d.clone(),
            points,
            at: 0,
            bindings: Vec::new(),
            shown: d.clone(),
        });
        cx.selection.set_hover(None);
    }

    /// The point asked for gets `to` (none: let go, keep: none and not given); after the last the bindings are written.
    fn give(&mut self, to: Option<Option<(Vec2, SubRef)>>, cx: &mut Context<'_>) {
        let Some(picking) = self.picking.as_mut() else {
            return;
        };
        let k = picking.points[picking.at];
        match to {
            Some(Some((p, r))) => {
                picking.shown = with_point(&picking.shown, k, p);
                picking.bindings.push(PointBinding {
                    point: k,
                    to: Some(PlaceOn {
                        uid: uid_text(&r),
                        place: r.place,
                    }),
                });
            }
            Some(None) => picking.bindings.push(PointBinding { point: k, to: None }),
            None => {}
        }
        picking.at += 1;
        if picking.at < picking.points.len() {
            return;
        }
        self.write(cx);
    }

    fn write(&mut self, cx: &mut Context<'_>) {
        let Some(picking) = self.picking.take() else {
            return;
        };
        self.hovering = None;
        if picking.bindings.is_empty() {
            cx.say(Level::Info, format!("{LABEL}: ölçü değişmedi."));
            return;
        }
        let input = DimensionsReassociate {
            uid: picking.uid,
            points: picking.bindings,
            expected_revision: None,
        };
        let result = dimensions_reassociate::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            let said = if out.changed == 0 {
                format!("{LABEL}: ölçü değişmedi.")
            } else {
                let value = dimension_layout(&Entity::Dimension(picking.shown.clone()))
                    .map(|l| {
                        cx.format()
                            .dimension_in(l.prefix, l.unit, l.value, &picking.shown.look)
                    })
                    .unwrap_or_default();
                format!("{LABEL}: {} nokta bağlandı; ölçü {value}.", out.changed)
            };
            cx.say(Level::Success, said);
        }
    }
}

impl Tool for DimensionReassociate {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        match &self.picking {
            None => Prompt::new(LABEL, "yeniden bağlanacak ölçüye tıklayın"),
            Some(picking) => {
                let k = picking.points[picking.at];
                Prompt::new(
                    LABEL,
                    format!(
                        "ölçünün {} noktasının yeni yerine kenetle tıklayın (Enter: olduğu gibi)",
                        ordinal(k)
                    ),
                )
                .option("Serbest", "S")
            }
        }
    }

    fn point_count(&self) -> usize {
        self.picking.as_ref().map_or(0, |p| 1 + p.at)
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        Flow::Stay
    }

    /// The cursor snaps once the dimension is picked: the point is a place on an object.
    fn snaps(&self) -> bool {
        self.picking.is_some()
    }

    fn cursor(&self) -> Cursor {
        if self.picking.is_some() {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
        if self.picking.is_none() {
            edge::hover(p, cx, dimension);
            return;
        }
        self.cursor = Some(p.world);
        self.hovering = snapped_place(p.snap, p.world, cx).map(|r| (p.world, r));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.picking.is_none() {
            self.pick(p, cx);
            return;
        }
        let own = self.picking.as_ref().map(|k| k.uid.clone());
        match snapped_place(p.snap, p.world, cx) {
            Some(r) if Some(uid_text(&r)) == own => cx.say(
                Level::Warn,
                "Ölçünün noktası ölçünün kendisine bağlanamaz; ölçülen nesnenin bir yerine tıklayın.",
            ),
            Some(r) => self.give(Some(Some((p.world, r))), cx),
            None => cx.say(Level::Warn, NO_PLACE_HERE),
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.picking.is_some() && upper_tr(js_trim(text)) == "S" {
            self.give(Some(None), cx);
            return true;
        }
        false
    }

    /// Enter: the point asked for stays as it is; with no dimension the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.picking.is_some() {
            self.give(None, cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }

    /// Ctrl+Z: the last point given goes back, then the dimension.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        let Some(picking) = self.picking.as_mut() else {
            return false;
        };
        if picking.at == 0 {
            self.picking = None;
            self.hovering = None;
            return true;
        }
        picking.at -= 1;
        let k = picking.points[picking.at];
        if picking.bindings.last().is_some_and(|b| b.point == k) {
            picking.bindings.pop();
        }
        picking.shown = with_point(&picking.shown, k, point_at(&picking.was, k));
        true
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = Preview::default();
        let Some(picking) = &self.picking else {
            return out;
        };
        let k = picking.points[picking.at];
        let asked = point_at(&picking.shown, k);
        let shown = match &self.hovering {
            Some((p, _)) => with_point(&picking.shown, k, *p),
            None => picking.shown.clone(),
        };
        // The dimension as it would be, dashed; its points named.
        if let Some(layout) = dimension_layout(&Entity::Dimension(shown.clone())) {
            for [a, b] in &layout.lines {
                out.strokes
                    .push(Stroke::dashed(vec![*a, *b], false, [6.0, 4.0]).tone(Tone::Accent));
            }
            if let Some(at) = self.cursor {
                let mut lines = vec![format.dimension_in(
                    layout.prefix,
                    layout.unit,
                    layout.value,
                    &shown.look,
                )];
                match &self.hovering {
                    Some((_, r)) => lines.push(format!("Bağlanacak: {}", place_words(&r.place))),
                    None => {
                        lines.push("Kenetle bir nesnenin yerine gelin".into());
                        out.tag_tone = Tone::Danger;
                    }
                }
                out.tag = Some(Tag { at, lines });
            }
        }
        out.squares.push(asked);
        for (i, &q) in picking.points.iter().enumerate() {
            out.labels.push(Label {
                at: point_at(&shown, q),
                text: ["1", "2", "3"][i].into(),
                offset: [8.0, -8.0],
                tone: if q == k { Tone::Accent } else { Tone::Snap },
            });
        }
        if let (Some((p, _)), true) = (&self.hovering, picking.at < picking.points.len()) {
            out.guides.push([asked, *p]);
        }
        out
    }
}
