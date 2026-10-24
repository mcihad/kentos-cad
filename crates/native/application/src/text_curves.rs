//! Texts on curves as the commands write them (docs/adr/0196 §4, 0622 §9): `cad.annotations.write`'s `along` work
//! (Eğri boyunca yazı) and `cad.texts.edit`'s `fit`, `turn` and `straighten` (Yazıyı eğriye oturt, Doğrultuya döndür,
//! Düzleştir). The letters, the curve's piece and the straightening are the core's (`text::along`); the tools draw
//! their previews with the same functions.

use kentos_commands::{Entity, TextEntity, TextFace, TextPath, Vec2 as Wire};
use kentos_geometry_core::entity::{Shape, TextPlace};
use kentos_geometry_core::ops::edges::nearest_edge;
use kentos_geometry_core::text::Font;
use kentos_geometry_core::text::along::{Along, Curve, piece, readable_turn};
use kentos_geometry_core::tools::locks::edge_direction;
use kentos_geometry_core::vec2::Vec2;

use crate::geometry::{contract_curve, core_face, shape};

/// The kinds a text may follow (§4): their paths, by the core's `paths_of`.
pub const CURVE_KINDS: [&str; 7] = [
    "line", "arc", "circle", "polyline", "polygon", "ellipse", "spline",
];

/// The kinds whose edge gives Doğrultuya döndür its direction (the locks' edge kinds).
pub const EDGE_KINDS: [&str; 7] = [
    "line", "polyline", "polygon", "arc", "circle", "xline", "ray",
];

/// The letters' length of `text` at `height`, in `face` or the drawing's typeface (§2.2): what the curve's piece is
/// cut to.
pub fn letters_length(
    text: &str,
    height: f64,
    width_factor: f64,
    face: &TextFace,
    font: Font,
) -> f64 {
    let core = core_face(face);
    let straight = Curve {
        pts: vec![Vec2::new(1.0, 0.0)],
        bulges: None,
    };
    Along {
        p: Vec2::new(0.0, 0.0),
        rotation: 0.0,
        curve: &straight,
        text,
        runs: &[],
        height,
        width_factor,
        align: None,
        font: core.font_or(font),
        bold: core.is_bold(),
        lean: core.lean(),
    }
    .advances()
    .iter()
    .sum()
}

fn wire(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

/// The letters of `words` (at `height`, `width` times as wide, in `face`) on the curve `curve` where `at` falls, the
/// click being `share` of them along: the text's point, turn and piece. `None` off any path.
#[allow(clippy::too_many_arguments)]
pub fn placed(
    words: &str,
    height: f64,
    width: f64,
    face: &TextFace,
    curve: &Shape,
    at: Vec2,
    share: f64,
    font: Font,
) -> Option<(Wire, f64, TextPath)> {
    let length = letters_length(words, height, width, face, font);
    let (p, rotation, path) = piece(curve, at, length, share)?;
    Some((wire(p), rotation, contract_curve(path)))
}

/// The text `t` (its words, height, face and width) put on the curve `curve` where `at` falls, the click being
/// `share` of its letters along. `None` off any path.
pub fn on_curve(
    t: &TextEntity,
    curve: &Shape,
    at: Vec2,
    share: f64,
    font: Font,
) -> Option<TextEntity> {
    let (p, rotation, path) = placed(
        &t.text,
        t.height,
        t.width_factor.unwrap_or(1.0),
        &t.face,
        curve,
        at,
        share,
        font,
    )?;
    Some(TextEntity {
        p,
        rotation,
        path: Some(path),
        ..t.clone()
    })
}

/// Whether Eğriye oturt takes the text: one line, no box, not linked to an object (§1).
pub fn fits(t: &TextEntity) -> bool {
    !t.text.contains('\n')
        && t.paragraph.box_width.is_none()
        && t.paragraph.line_spacing.is_none()
        && t.label_of.is_none()
}

/// Eğriye oturt: the text on the curve where its straight box's middle falls, its letters' middle there.
pub fn fitted(t: &TextEntity, curve: &Shape, font: Font) -> Option<TextEntity> {
    let straight = TextEntity {
        path: None,
        ..t.clone()
    };
    let s = shape(&Entity::Text(straight));
    let place = TextPlace::of(&s)?;
    let ring = place.outline(font);
    let first = *ring.first()?;
    let (mut lo, mut hi) = (first, first);
    for q in &ring {
        lo = Vec2::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Vec2::new(hi.x.max(q.x), hi.y.max(q.y));
    }
    let middle = Vec2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    on_curve(t, curve, middle, 0.5, font)
}

/// Doğrultuya döndür's turn: the direction of `edge_of`'s edge nearest `at`, readable. `None` without an edge.
pub fn turn_toward(edge_of: &Shape, at: Vec2) -> Option<f64> {
    let edge = nearest_edge(edge_of, at)?;
    let u = edge_direction(&edge, at)?;
    Some(readable_turn(u.x, u.y))
}

/// Düzleştir: the curved text straight at its first letter, along its direction; its alignment gone.
pub fn straightened(t: &TextEntity, font: Font) -> Option<TextEntity> {
    let s = shape(&Entity::Text(t.clone()));
    let (p, rotation) = TextPlace::of(&s)?.along(font)?.straight();
    Some(TextEntity {
        p: wire(p),
        rotation,
        align: None,
        path: None,
        ..t.clone()
    })
}
