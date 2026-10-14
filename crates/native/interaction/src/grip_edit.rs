//! Grip editing's modes (docs/adr/0280; plan/cad.md CAD-INPUT-16), as AutoCAD's grip modes in KentOS's words: a hot
//! grip stretches its vertex by default; its letters turn it into Taşı, Döndür, Ölçekle or Aynala of the whole
//! selection, with Taban noktası (another base point than the grip), Kopya (each placement adds copies and the grip
//! stays hot for the next) and Referans (Döndür's reference direction, Ölçekle's reference length, shown with two
//! points or typed). Several grips are made hot with Shift and a click on each; the next click on one of them starts
//! the edit, and Esnet moves every hot grip by the same distance.
//!
//! The modes write through the product commands: Esnet through `cad.entities.edit`'s `grip` (the object's whole new
//! geometry, a copy as a new object), the others through `cad.entities.transform` of the selection, in place or as
//! copies. The geometry is the shared core's (`move_grip`, the affine transforms).

use kentos_contracts::Transform;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::{Entity as CoreEntity, area_parts};
use kentos_geometry_core::geom::affine::{Affine, mirror, rotation, scaling, translation};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::grips::{entity_grips, grip_part, mid_grip_segment, move_grip};
use kentos_geometry_core::tools::drawing::direction_angle;
use kentos_geometry_core::tools::editing::{rotation_angle, scale_factor};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{shape, with_shape};

use crate::Vec2;
use crate::format::{Format, degrees, fixed};
use crate::points::{SAME, plain_number, wire};
use crate::prompt::{Prompt, upper_tr};

/// What a hot grip does where it is placed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GripMode {
    /// Esnet: the grip's vertex (every hot grip's) goes there.
    #[default]
    Stretch,
    /// Taşı: the selection moves from the base point.
    Move,
    /// Döndür: the selection turns about the base point.
    Rotate,
    /// Ölçekle: the selection scales about the base point.
    Scale,
    /// Aynala: the selection mirrors in the line from the base point.
    Mirror,
}

impl GripMode {
    /// The mode's name, as the prompt and the log say it.
    pub fn label(self) -> &'static str {
        match self {
            GripMode::Stretch => "Esnet",
            GripMode::Move => "Taşı",
            GripMode::Rotate => "Döndür",
            GripMode::Scale => "Ölçekle",
            GripMode::Mirror => "Aynala",
        }
    }

    /// The mode's letter.
    fn key(self) -> &'static str {
        match self {
            GripMode::Stretch => "E",
            GripMode::Move => "T",
            GripMode::Rotate => "D",
            GripMode::Scale => "L",
            GripMode::Mirror => "A",
        }
    }

    const ALL: [GripMode; 5] = [
        GripMode::Stretch,
        GripMode::Move,
        GripMode::Rotate,
        GripMode::Scale,
        GripMode::Mirror,
    ];
}

/// Aynala's Yazılar as the prompt says it (docs/adr/0292).
pub fn texts_word(mirrored: bool) -> &'static str {
    if mirrored { "aynalı" } else { "okunur" }
}

/// Aynala's Yazılar as the command takes it (docs/adr/0292).
pub fn mirror_text(mirrored: bool) -> kentos_contracts::MirrorText {
    if mirrored {
        kentos_contracts::MirrorText::Mirrored
    } else {
        kentos_contracts::MirrorText::Readable
    }
}

/// A grip made hot with Shift: its object, its index among the object's grips, where it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HotGrip {
    pub slot: Slot,
    pub index: usize,
    pub origin: Vec2,
}

/// What the next point is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Stage {
    /// The mode's point: where the grip goes, the turn, the scale, the mirror line's end.
    #[default]
    Point,
    /// Taban noktası: the new base point.
    Base,
    /// Referans: its first point.
    RefFrom,
    /// Referans: its second point, from the first.
    RefTo(Vec2),
}

/// A hot grip's mode and its settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GripModes {
    pub mode: GripMode,
    /// Where the modes measure from: the grip, until Taban noktası gives another.
    pub base: Vec2,
    pub stage: Stage,
    /// Döndür's reference direction (radians) or Ölçekle's reference length (metres).
    pub reference: Option<f64>,
    /// Kopya: each placement adds copies; the grip stays hot.
    pub copy: bool,
    /// How many placements were written while Kopya was on.
    pub placed: usize,
    /// Aynala's Yazılar (docs/adr/0292): the texts turn with the mirror; else they stay readable.
    pub text_mirrored: bool,
}

/// What a letter or typed text did to the modes.
#[derive(Clone, Debug, PartialEq)]
pub enum Typed {
    /// A setting changed; the line to say.
    Said(String),
    /// A typed value gives the transform to write.
    Write(Transform),
}

impl GripModes {
    pub fn new(base: Vec2) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    /// [`GripModes::new`] with the session's Yazılar (docs/adr/0292).
    pub fn with_texts(base: Vec2, text_mirrored: bool) -> Self {
        Self {
            text_mirrored,
            ..Self::new(base)
        }
    }

    /// Where the constraints (ortho, polar) and the rubber line start: the reference's first point, else the base.
    pub fn anchor(&self) -> Vec2 {
        match self.stage {
            Stage::RefTo(from) => from,
            _ => self.base,
        }
    }

    /// The prompt: the mode's step and its options.
    pub fn prompt(&self) -> Prompt {
        let step = match (self.stage, self.mode) {
            (Stage::Base, _) => "yeni taban noktasını belirtin",
            (Stage::RefFrom, GripMode::Rotate) => {
                "referans doğrultunun ilk noktasını gösterin ya da referans açıyı yazın"
            }
            (Stage::RefFrom, _) => "referans uzunluğun ilk ucunu gösterin ya da uzunluğu yazın",
            (Stage::RefTo(_), GripMode::Rotate) => "referans doğrultunun ikinci noktasını gösterin",
            (Stage::RefTo(_), _) => "referans uzunluğun ikinci ucunu gösterin",
            (Stage::Point, GripMode::Stretch) => {
                "yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)"
            }
            (Stage::Point, GripMode::Move) => "hedef noktayı belirtin ya da koordinat yazın",
            (Stage::Point, GripMode::Rotate) => "döndürme açısını gösterin ya da yazın",
            (Stage::Point, GripMode::Scale) => "ölçek faktörünü gösterin ya da yazın",
            (Stage::Point, GripMode::Mirror) => "simetri ekseninin ikinci noktasını belirtin",
        };
        let title = match self.mode {
            GripMode::Stretch => "Tutamaç",
            GripMode::Move => "Tutamaç · Taşı",
            GripMode::Rotate => "Tutamaç · Döndür",
            GripMode::Scale => "Tutamaç · Ölçekle",
            GripMode::Mirror => "Tutamaç · Aynala",
        };
        let mut prompt = Prompt::new(title, step);
        if self.stage != Stage::Point {
            return prompt;
        }
        for mode in GripMode::ALL.into_iter().filter(|m| *m != self.mode) {
            prompt = prompt.option(mode.label(), mode.key());
        }
        prompt = prompt.option("Taban noktası", "B");
        if matches!(self.mode, GripMode::Rotate | GripMode::Scale) {
            prompt = prompt.option("Referans", "R");
        }
        if self.mode == GripMode::Mirror {
            prompt = prompt.option_with("Yazılar", "Y", texts_word(self.text_mirrored));
        }
        prompt.option_with("Kopya", "K", if self.copy { "açık" } else { "kapalı" })
    }

    /// A letter: a mode, Taban noktası, Referans or Kopya; a typed value at a reference's or a turn's or a scale's
    /// step. `None` when the text is none of these (a typed point, then). Döndür's typed angles are the project's
    /// (docs/adr/0291, as the Döndür tool's): a turn in degrees counter-clockwise in a CAD project, in grads
    /// clockwise in a CBS one; with Referans both the reference and the new angle are directions.
    pub fn typed(&mut self, text: &str, format: &Format) -> Option<Typed> {
        let t = upper_tr(js_trim(text));
        if self.stage == Stage::Point {
            if let Some(mode) = GripMode::ALL.into_iter().find(|m| m.key() == t) {
                self.mode = mode;
                self.reference = None;
                return Some(Typed::Said(format!("Tutamaç: {}.", mode.label())));
            }
            match t.as_str() {
                "B" => {
                    self.stage = Stage::Base;
                    return Some(Typed::Said("Taban noktasını gösterin.".to_owned()));
                }
                "R" if matches!(self.mode, GripMode::Rotate | GripMode::Scale) => {
                    self.stage = Stage::RefFrom;
                    return Some(Typed::Said("Referansı gösterin ya da yazın.".to_owned()));
                }
                "Y" if self.mode == GripMode::Mirror => {
                    self.text_mirrored = !self.text_mirrored;
                    return Some(Typed::Said(format!(
                        "Yazılar: {}.",
                        texts_word(self.text_mirrored)
                    )));
                }
                "K" => {
                    self.copy = !self.copy;
                    let on = if self.copy { "açık" } else { "kapalı" };
                    return Some(Typed::Said(format!("Kopya {on}.")));
                }
                _ => {}
            }
        }
        let n = plain_number(text)?;
        match (self.stage, self.mode) {
            (Stage::RefFrom, GripMode::Rotate) => {
                self.reference = Some(format.direction_from_typed(n));
                self.stage = Stage::Point;
                Some(Typed::Said(format!(
                    "Referans açı {}{}.",
                    fixed(n, 4),
                    format.angle_unit_label()
                )))
            }
            (Stage::RefFrom, GripMode::Scale) if n > 0.0 => {
                self.reference = Some(n);
                self.stage = Stage::Point;
                Some(Typed::Said(format!("Referans uzunluk {}.", fixed(n, 4))))
            }
            (Stage::Point, GripMode::Rotate) => Some(Typed::Write(Transform::Rotate {
                center: wire(self.base),
                angle: match self.reference {
                    Some(r) => format.direction_from_typed(n) - r,
                    None => format.turn_from_typed(n),
                },
            })),
            (Stage::Point, GripMode::Scale) if n > 0.0 && n.is_finite() => {
                // With a reference, the typed number is the new length.
                let factor = self.reference.map_or(n, |r| n / r);
                Some(Typed::Write(Transform::Scale {
                    center: wire(self.base),
                    factor,
                }))
            }
            _ => None,
        }
    }

    /// A point at a step that is not the mode's own: the base or the reference; true when it was taken so.
    pub fn take_point(&mut self, at: Vec2) -> bool {
        match self.stage {
            Stage::Point => false,
            Stage::Base => {
                self.base = at;
                self.stage = Stage::Point;
                true
            }
            Stage::RefFrom => {
                self.stage = Stage::RefTo(at);
                true
            }
            Stage::RefTo(from) => {
                if dist(from, at) > SAME {
                    self.reference = Some(match self.mode {
                        GripMode::Rotate => direction_angle(from, at),
                        _ => dist(from, at),
                    });
                    self.stage = Stage::Point;
                }
                true
            }
        }
    }

    /// Ölçekle's factor at `at`: the distance from the base over the reference length (by default the distance
    /// from the base to the grip, so the grip follows the pointer; one drawing unit when they meet).
    fn factor(&self, at: Vec2, grip: Vec2, unit: f64) -> Option<f64> {
        let reference = self.reference.unwrap_or_else(|| {
            let d = dist(self.base, grip);
            if d > SAME { d } else { unit }
        });
        let f = scale_factor(self.base, at, reference);
        (f > 0.0 && f.is_finite()).then_some(f)
    }

    /// The transform a point gives in a mode other than Esnet: none in Esnet, or where it is undefined.
    pub fn transform(&self, at: Vec2, grip: Vec2, unit: f64) -> Option<Transform> {
        let base = self.base;
        match self.mode {
            GripMode::Stretch => None,
            GripMode::Move => (dist(base, at) > SAME).then_some(Transform::Move {
                dx: at.x - base.x,
                dy: at.y - base.y,
            }),
            GripMode::Rotate => (dist(base, at) > SAME).then(|| Transform::Rotate {
                center: wire(base),
                angle: rotation_angle(base, at, self.reference.unwrap_or(0.0)),
            }),
            GripMode::Scale => self.factor(at, grip, unit).map(|factor| Transform::Scale {
                center: wire(base),
                factor,
            }),
            GripMode::Mirror => (dist(base, at) > SAME).then(|| Transform::Mirror {
                a: wire(base),
                b: wire(at),
                text: Some(mirror_text(self.text_mirrored)),
            }),
        }
    }

    /// The transform as the preview draws it.
    pub fn affine(&self, at: Vec2, grip: Vec2, unit: f64) -> Option<Affine> {
        if self.stage != Stage::Point {
            return None;
        }
        match self.transform(at, grip, unit)? {
            Transform::Move { dx, dy } => Some(translation(dx, dy)),
            Transform::Rotate { center, angle } => {
                Some(rotation(angle, Vec2::new(center.x, center.y)))
            }
            Transform::Scale { center, factor } => {
                Some(scaling(factor, Vec2::new(center.x, center.y)))
            }
            Transform::Mirror { a, b, .. } => Some(mirror(Vec2::new(a.x, a.y), Vec2::new(b.x, b.y))),
            _ => None,
        }
    }

    /// The tag beside the pointer: the turn's angle or the scale's factor; none for the others.
    pub fn tag(&self, at: Vec2, grip: Vec2, unit: f64, format: &Format) -> Option<String> {
        if self.stage != Stage::Point {
            return None;
        }
        match self.transform(at, grip, unit)? {
            Transform::Rotate { angle, .. } => Some(format!("Açı {}°", fixed(degrees(angle), 2))),
            Transform::Scale { factor, .. } => Some(format!("Ölçek {}", fixed(factor, 4))),
            Transform::Move { dx, dy } => {
                Some(format.length(dist(Vec2::new(0.0, 0.0), Vec2::new(dx, dy))))
            }
            _ => None,
        }
    }

    /// What the log says once a transform was written over `n` objects.
    pub fn said(&self, transform: &Transform, n: usize) -> String {
        let copy = if self.copy { " (kopya)" } else { "" };
        match transform {
            Transform::Move { .. } => format!("{n} nesne taşındı{copy}."),
            Transform::Rotate { angle, .. } => {
                format!("{n} nesne {}° döndürüldü{copy}.", fixed(degrees(*angle), 4))
            }
            Transform::Scale { factor, .. } => {
                format!(
                    "{n} nesne {} faktörüyle ölçeklendi{copy}.",
                    fixed(*factor, 4)
                )
            }
            _ => format!("{n} nesnenin simetriği alındı{copy}."),
        }
    }
}

/// The objects as Esnet leaves them when every hot grip moves by `(dx, dy)`: each object's grips moved one after
/// the other, from where they stood; an edge's middle among several grips moves its edge's two vertices (as one
/// vertex would add a vertex and shift the others). An object whose grip cannot go there is left out; none moved,
/// `None`. Each object with its shape before.
pub fn stretched(
    doc: &Document,
    grips: &[HotGrip],
    dx: f64,
    dy: f64,
) -> Vec<(Slot, kentos_contracts::Entity, kentos_contracts::Entity)> {
    let mut slots: Vec<Slot> = Vec::new();
    for g in grips {
        if !slots.contains(&g.slot) {
            slots.push(g.slot);
        }
    }
    let mut out = Vec::new();
    for slot in slots {
        let Some(e) = doc.get(slot) else {
            continue;
        };
        let before = shape(e);
        let own: Vec<usize> = grips
            .iter()
            .filter(|g| g.slot == slot)
            .map(|g| g.index)
            .collect();
        let indices = if own.len() == 1 {
            own
        } else {
            vertex_grips(&before, &own)
        };
        let points = entity_grips(&before);
        let mut moved = CoreEntity::new(before.clone());
        let mut ok = true;
        for index in indices {
            let Some(p) = points.get(index) else {
                ok = false;
                break;
            };
            match move_grip(&moved, index, Vec2::new(p.x + dx, p.y + dy)) {
                Some(m) => moved = m,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if let (true, Some(after)) = (ok, with_shape(e, moved.shape)) {
            out.push((slot, e.clone(), after));
        }
    }
    out
}

/// Several grips of one object as vertex grips: an edge's middle as its two vertices, each vertex once.
fn vertex_grips(shape: &kentos_geometry_core::entity::Shape, grips: &[usize]) -> Vec<usize> {
    let parts = area_parts(shape);
    let mut out: Vec<usize> = Vec::new();
    for &index in grips {
        let both = mid_grip_segment(shape, index).and_then(|segment| {
            let (part, local) = grip_part(shape, index)?;
            let offset = index - local;
            let n = match parts.get(part)? {
                kentos_geometry_core::entity::Shape::Polyline { pts, .. }
                | kentos_geometry_core::entity::Shape::Polygon { pts, .. } => pts.len(),
                _ => return None,
            };
            Some([offset + segment, offset + (segment + 1) % n])
        });
        match both {
            Some(pair) => out.extend(pair),
            None => out.push(index),
        }
    }
    let mut seen = Vec::new();
    out.retain(|i| {
        let fresh = !seen.contains(i);
        seen.push(*i);
        fresh
    });
    out
}
