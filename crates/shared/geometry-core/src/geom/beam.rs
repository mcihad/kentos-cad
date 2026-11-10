//! A beam's plan (docs/adr/3041 §1): its band along its axis, moved to the side its justification says, drawn dashed and
//! narrow as what lies over the plan's cut (ISO 128-23's dashed narrow line, 02.1, half the wide line: the slab of the
//! level above's own, [`crate::geom::slab::OVERHEAD_PEN`]); its words along it as a column's beside it (docs/adr/3011):
//! none at 1:200, its number at 1:100, its number and size from 1:50 ("K101 25/50").

use crate::Vec2;
use crate::block::{Piece, PieceStroke};
use crate::entity::Shape;
use crate::geom::arch::{LABEL_MM, Level};
use crate::text::TextAlign;

/// A beam's fields beside its axis.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BeamSpec {
    /// Its web's width, metres.
    pub width: f64,
    /// Where its middle lies from its axis, metres, to the axis's left (its turn counter-clockwise): 0 centred, minus
    /// half its width for an axis that is its left face (its body to the right), plus half for its right face.
    pub middle: f64,
    /// Its height, metres: written in its size.
    pub height: f64,
    /// Its number ("K101").
    pub tag: Option<String>,
}

impl BeamSpec {
    /// Its size as a construction plan writes it: its width over its height in centimetres ("25/50").
    pub fn size(&self) -> String {
        let cm = |m: f64| {
            let c = crate::jsmath::js_round(m * 1000.0) / 10.0;
            if c.fract() == 0.0 {
                format!("{c:.0}")
            } else {
                format!("{c:.1}")
            }
        };
        format!("{}/{}", cm(self.width), cm(self.height))
    }
}

fn unit(v: Vec2) -> Vec2 {
    let l = (v.x * v.x + v.y * v.y).sqrt();
    if l > 0.0 {
        Vec2::new(v.x / l, v.y / l)
    } else {
        Vec2::new(0.0, 0.0)
    }
}

fn left(d: Vec2) -> Vec2 {
    Vec2::new(-d.y, d.x)
}

/// The line `d` to the left of the axis, mitred at its turns.
fn side(pts: &[Vec2], d: f64) -> Vec<Vec2> {
    let n = pts.len();
    let dirs: Vec<Vec2> = pts
        .windows(2)
        .map(|w| unit(Vec2::new(w[1].x - w[0].x, w[1].y - w[0].y)))
        .collect();
    (0..n)
        .map(|i| {
            let (a, b) = (
                left(dirs[i.saturating_sub(1).min(n - 2)]),
                left(dirs[i.min(n - 2)]),
            );
            let m = if i == 0 {
                b
            } else if i == n - 1 {
                a
            } else {
                let dot = a.x * b.x + a.y * b.y;
                if 1.0 + dot < 1e-12 {
                    b
                } else {
                    Vec2::new((a.x + b.x) / (1.0 + dot), (a.y + b.y) / (1.0 + dot))
                }
            };
            Vec2::new(pts[i].x + m.x * d, pts[i].y + m.y * d)
        })
        .collect()
}

/// Its band: its left face from its start, then its right face back; empty for an axis of fewer than two points.
pub fn band(pts: &[Vec2], spec: &BeamSpec) -> Vec<Vec2> {
    if pts.len() < 2 {
        return Vec::new();
    }
    let half = spec.width / 2.0;
    let mut out = side(pts, spec.middle + half);
    let mut right = side(pts, spec.middle - half);
    right.reverse();
    out.extend(right);
    out
}

/// Its band as one shape: a closed outline.
pub fn outline(pts: &[Vec2], spec: &BeamSpec) -> Shape {
    Shape::Polygon {
        pts: band(pts, spec),
        bulges: None,
        holes: None,
        parts: None,
    }
}

fn piece(shape: Shape) -> Piece {
    Piece {
        shape,
        color: None,
        line_weight: None,
        by_layer: Vec::new(),
        gates: Vec::new(),
        stroke: PieceStroke::default(),
        attribute: None,
        annotation_origin: None,
    }
}

/// The gap between a beam's band and its words, millimetres on paper.
const WORDS_GAP_MM: f64 = 1.0;

/// Its pieces at 1:`scale` and `level`: its band dashed and narrow; its words as a column's: none in the simplest
/// ([`Level::Simple`], 1:200), its number at 1:100 and in its tool's preview, its number and size from 1:50
/// ([`Level::Construction`]). They run along its longest stretch, read from below or from the right, beside its band
/// (over it as read): written on the band they would cross its dashes and the wall's faces under it.
pub(crate) fn pieces_at(pts: &[Vec2], spec: &BeamSpec, scale: f64, level: Level) -> Vec<Piece> {
    let ring = band(pts, spec);
    if ring.is_empty() {
        return Vec::new();
    }
    let mut outline = piece(Shape::Polyline {
        pts: ring,
        bulges: None,
        holes: None,
        parts: None,
        closed: true,
        widths: None,
    });
    outline.stroke.line_type = Some("dashed".into());
    outline.stroke.weight_scale = Some(crate::geom::slab::OVERHEAD_PEN);
    let mut out = vec![outline];
    let words: Vec<String> = match level {
        Level::Simple => Vec::new(),
        Level::Whole | Level::Schematic => spec.tag.iter().cloned().collect(),
        Level::Construction => spec
            .tag
            .iter()
            .cloned()
            .chain(std::iter::once(spec.size()))
            .collect(),
    };
    if !words.is_empty() {
        let middle = side(pts, spec.middle);
        if let Some(w) = middle.windows(2).max_by(|a, b| {
            let l = |w: &[Vec2]| crate::jsmath::js_hypot(w[1].x - w[0].x, w[1].y - w[0].y);
            l(a).total_cmp(&l(b))
        }) {
            let (a, b) = (w[0], w[1]);
            let mut turn = crate::jsmath::atan2(b.y - a.y, b.x - a.x);
            // Read from below or from the right (ISO 128's reading directions).
            if turn > std::f64::consts::FRAC_PI_2 + 1e-9 {
                turn -= std::f64::consts::PI;
            } else if turn <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
                turn += std::f64::consts::PI;
            }
            // Over its band as read: up from the reading direction, past half its width and the gap.
            let up = Vec2::new(-crate::jsmath::sin(turn), crate::jsmath::cos(turn));
            let off = spec.width / 2.0 + WORDS_GAP_MM * scale / 1000.0;
            let h = LABEL_MM * scale / 1000.0;
            out.push(piece(Shape::Text {
                p: Vec2::new(
                    (a.x + b.x) / 2.0 + up.x * off,
                    (a.y + b.y) / 2.0 + up.y * off,
                ),
                text: words.join(" "),
                height: h,
                // A text's turn is in degrees.
                rotation: turn.to_degrees(),
                align: Some(TextAlign::BottomCenter),
                width_factor: None,
                mask: None,
                box_width: None,
                line_spacing: None,
                runs: None,
                flow: None,
                face: Default::default(),
                path: None,
            }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(middle: f64) -> BeamSpec {
        BeamSpec {
            width: 0.25,
            middle,
            height: 0.5,
            tag: Some("K101".into()),
        }
    }

    #[test]
    fn a_beam_s_band_lies_where_its_justification_says() {
        let axis = [Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0)];
        // Centred: 0.125 each side.
        assert_eq!(
            band(&axis, &spec(0.0)),
            [
                Vec2::new(0.0, 0.125),
                Vec2::new(4.0, 0.125),
                Vec2::new(4.0, -0.125),
                Vec2::new(0.0, -0.125)
            ]
        );
        // Its left face on the axis: its body to the right.
        let b = band(&axis, &spec(-0.125));
        assert_eq!((b[0], b[3]), (Vec2::new(0.0, 0.0), Vec2::new(0.0, -0.25)));
        // A turn is mitred: the corner of an L's outer face.
        let l = band(
            &[
                Vec2::new(0.0, 0.0),
                Vec2::new(4.0, 0.0),
                Vec2::new(4.0, 3.0),
            ],
            &spec(0.0),
        );
        assert!(
            (l[1].x - 3.875).abs() < 1e-12 && (l[1].y - 0.125).abs() < 1e-12,
            "{l:?}"
        );
        assert!(
            (l[4].x - 4.125).abs() < 1e-12 && (l[4].y + 0.125).abs() < 1e-12,
            "{l:?}"
        );
    }

    fn words(pieces: &[Piece]) -> Vec<&str> {
        pieces
            .iter()
            .filter_map(|p| match &p.shape {
                Shape::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_beam_is_dashed_and_narrow_and_says_its_size_from_1_50() {
        let axis = [Vec2::new(0.0, 0.0), Vec2::new(0.0, 4.0)];
        let simple = pieces_at(&axis, &spec(0.0), 200.0, Level::Simple);
        assert_eq!(simple.len(), 1, "no words at 1:200");
        assert_eq!(simple[0].stroke.line_type.as_deref(), Some("dashed"));
        assert_eq!(simple[0].stroke.weight_scale, Some(0.5));
        // Its number at 1:100 and in its tool's preview; a beam without one says nothing there.
        assert_eq!(
            words(&pieces_at(&axis, &spec(0.0), 100.0, Level::Schematic)),
            ["K101"]
        );
        assert_eq!(
            words(&pieces_at(&axis, &spec(0.0), 100.0, Level::Whole)),
            ["K101"]
        );
        let untagged = BeamSpec {
            tag: None,
            ..spec(0.0)
        };
        assert!(words(&pieces_at(&axis, &untagged, 100.0, Level::Schematic)).is_empty());
        assert_eq!(
            words(&pieces_at(&axis, &untagged, 50.0, Level::Construction)),
            ["25/50"]
        );
        let detail = pieces_at(&axis, &spec(0.0), 50.0, Level::Construction);
        let Some(Shape::Text {
            p,
            text,
            rotation,
            height,
            align,
            ..
        }) = detail.get(1).map(|p| &p.shape)
        else {
            panic!("{detail:?}")
        };
        assert_eq!(text, "K101 25/50");
        // A vertical beam is read from the right: turned a quarter counter-clockwise (a text's turn in degrees), its
        // words over its band as read: to its left, 0.125 + 1 mm at 1:50 off its axis, at its middle.
        assert!((rotation - 90.0).abs() < 1e-9, "{rotation}");
        assert_eq!(*align, Some(TextAlign::BottomCenter));
        assert!(
            (p.x + 0.175).abs() < 1e-12 && (p.y - 2.0).abs() < 1e-12,
            "{p:?}"
        );
        assert!((height - 0.125).abs() < 1e-12);
        // Drawn right to left, the same words read from below, over the band: turned back to 0°.
        let back = pieces_at(
            &[Vec2::new(4.0, 0.0), Vec2::new(0.0, 0.0)],
            &spec(0.0),
            50.0,
            Level::Construction,
        );
        let Some(Shape::Text { p, rotation, .. }) = back.get(1).map(|p| &p.shape) else {
            panic!("{back:?}")
        };
        assert!(rotation.abs() < 1e-9, "{rotation}");
        assert!(
            (p.x - 2.0).abs() < 1e-12 && (p.y - 0.175).abs() < 1e-12,
            "{p:?}"
        );
    }
}
