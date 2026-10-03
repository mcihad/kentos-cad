//! A map frame's content in the PDF (design §9a): the drawing's vectors
//! turned from the ground into the paper by the map's view (centre, scale,
//! turn) and the frame's own turn, cut to the frame by a clip path, each
//! drawing layer in an optional content group; or the host's picture of the
//! frame; or, without either, the light “harita” box the SVG draws too.

use std::collections::BTreeMap;

use pdf_writer::{Name, Ref};

use super::content::{Page, color, rad};
use super::{MapPath, MapStroke, MapText, TextAnchor, VectorMap};
use crate::display::{MapFrame, MapPrim};
use crate::units::{corners, rotate};

/// The map's frame: its own coordinates from the ground, then the frame's turn on the paper.
pub(super) struct Frame {
    view: MapFrame,
    clip: crate::units::RectUm,
    turn: crate::units::Mdeg,
}

impl Frame {
    pub fn of(m: &MapPrim) -> Frame {
        Frame {
            view: MapFrame {
                content: m.clip,
                center: m.view.center,
                scale: m.view.scale.max(1),
                view_rot: m.view.rotation,
                item_rot: m.rotation,
            },
            clip: m.clip,
            turn: m.rotation,
        }
    }

    /// A ground point (east, north) on the paper (µm); none for a map with no place.
    pub fn paper(&self, g: [f64; 2]) -> Option<[f64; 2]> {
        let local = self.view.to_local(g)?;
        Some(rotate(local, self.clip.center(), self.turn))
    }

    /// A paper point (µm) on the ground.
    pub fn ground(&self, p: [f64; 2]) -> Option<[f64; 2]> {
        let local = rotate(p, self.clip.center(), -self.turn);
        self.view.to_ground(local)
    }

    /// The content box's corners on the paper (µm): top left, top right, bottom right, bottom left.
    pub fn corners(&self) -> [[f64; 2]; 4] {
        corners(&self.clip, self.turn)
    }

    /// Whether a ground point is inside the content box (its turn taken back).
    fn holds(&self, g: [f64; 2]) -> bool {
        let Some(local) = self.view.to_local(g) else {
            return false;
        };
        let c = &self.clip;
        local[0] >= f64::from(c.left)
            && local[0] <= f64::from(c.left) + f64::from(c.width)
            && local[1] >= f64::from(c.top)
            && local[1] <= f64::from(c.top) + f64::from(c.height)
    }

    /// The ground the content covers (min east, min north, max east, max north).
    fn extent(&self) -> Option<[f64; 4]> {
        self.view.extent()
    }

    /// Metres on the ground per micrometre on the paper.
    fn metres_per_um(&self) -> f64 {
        f64::from(self.view.scale.max(1)) / 1_000_000.0
    }
}

/// The frame's clip (until the matching `restore`).
fn clip_to(page: &mut Page<'_>, frame: &Frame) {
    page.save();
    page.poly(&frame.corners(), true);
    page.c.clip_nonzero();
    page.c.end_path();
}

/// A vector map: each layer (its optional content group, when `groups` has it) bottom first.
pub(super) fn vector(
    page: &mut Page<'_>,
    m: &MapPrim,
    map: &VectorMap,
    groups: Option<&BTreeMap<String, (String, Ref)>>,
) {
    let frame = Frame::of(m);
    let Some(extent) = frame.extent() else {
        return placeholder(page, m);
    };
    clip_to(page, &frame);
    for layer in &map.layers {
        let group = groups.and_then(|g| g.get(&layer.id));
        if let Some((name, r)) = group {
            page.properties.insert(name.clone(), *r);
            page.c
                .begin_marked_content_with_properties(Name(b"OC"))
                .properties_named(Name(name.as_bytes()));
        }
        for p in &layer.paths {
            path(page, &frame, &extent, p);
        }
        for t in &layer.texts {
            text(page, &frame, t);
        }
        if group.is_some() {
            page.c.end_marked_content();
        }
    }
    page.restore();
}

/// Whether a path's box meets the frame's ground (a path wholly outside is not written).
fn meets(extent: &[f64; 4], pts: &[[f64; 2]], margin: f64) -> bool {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for p in pts {
        b = [
            b[0].min(p[0]),
            b[1].min(p[1]),
            b[2].max(p[0]),
            b[3].max(p[1]),
        ];
    }
    b[0] <= extent[2] + margin
        && b[2] >= extent[0] - margin
        && b[1] <= extent[3] + margin
        && b[3] >= extent[1] - margin
}

fn path(page: &mut Page<'_>, frame: &Frame, extent: &[f64; 4], p: &MapPath) {
    if p.points.len() < 2 && !(p.closed && p.points.len() >= 3) {
        return;
    }
    // A line's half width and its caps may reach in from outside the frame.
    let margin = p.stroke.as_ref().map_or(0.0, |s| s.width * 1000.0) * frame.metres_per_um();
    if !meets(extent, &p.points, margin) {
        return;
    }
    let paper: Option<Vec<[f64; 2]>> = p.points.iter().map(|g| frame.paper(*g)).collect();
    let Some(paper) = paper else {
        return;
    };
    let fill = p.fill.as_deref().filter(|_| p.closed).and_then(color);
    let line = p
        .stroke
        .as_ref()
        .and_then(|s| color(&s.color).map(|c| (s, c)));
    if fill.is_none() && line.is_none() {
        return;
    }
    page.poly(&paper, p.closed);
    if p.closed {
        for hole in &p.holes {
            let ring: Option<Vec<[f64; 2]>> = hole.iter().map(|g| frame.paper(*g)).collect();
            if let Some(ring) = ring.filter(|r| r.len() >= 3) {
                page.poly(&ring, true);
            }
        }
    }
    page.alpha_for(
        fill.map_or(1.0, |(_, a)| a),
        line.map_or(1.0, |(_, (_, a))| a),
    );
    if let Some((rgb, _)) = fill {
        page.c.set_fill_rgb(rgb[0], rgb[1], rgb[2]);
    }
    if let Some((s, (rgb, _))) = line {
        page.c.set_stroke_rgb(rgb[0], rgb[1], rgb[2]);
        stroke_style(page, s);
    }
    let even_odd = !p.holes.is_empty();
    match (fill.is_some(), line.is_some()) {
        (true, true) if even_odd => page.c.fill_even_odd_and_stroke(),
        (true, true) => page.c.fill_nonzero_and_stroke(),
        (true, false) if even_odd => page.c.fill_even_odd(),
        (true, false) => page.c.fill_nonzero(),
        _ => page.c.stroke(),
    };
}

/// Millimetres on the paper as points.
fn mm(v: f64) -> f64 {
    v * 72.0 / 25.4
}

fn stroke_style(page: &mut Page<'_>, s: &MapStroke) {
    let dash: Vec<f64> = s.dash.iter().map(|d| mm(*d)).collect();
    page.line_style(mm(s.width), &dash, s.cap, s.join);
}

fn text(page: &mut Page<'_>, frame: &Frame, t: &MapText) {
    if t.text.trim().is_empty() || t.size <= 0.0 {
        return;
    }
    // One rule on every platform (design §9a): a text anchored inside the frame is written and
    // cut at its edge; one anchored outside is not, however far it would reach in.
    if !frame.holds(t.at) {
        return;
    }
    let r = t.rotation.to_radians();
    let (Some(p0), Some(p1)) = (
        frame.paper(t.at),
        frame.paper([t.at[0] + r.cos(), t.at[1] + r.sin()]),
    ) else {
        return;
    };
    // The baseline's direction on the paper (y down: clockwise from the paper's x axis).
    let turn = libm::atan2(p1[1] - p0[1], p1[0] - p0[0]);
    let Some(font) = page.fonts.get(&t.font, t.weight, t.italic) else {
        return;
    };
    let size = t.size * 1000.0;
    let width = page.fonts.width_em(&t.font, t.weight, t.italic, &t.text) * size;
    let along = match t.anchor {
        TextAnchor::LeftBaseline | TextAnchor::LeftMiddle => 0.0,
        TextAnchor::CenterBaseline | TextAnchor::CenterMiddle => -width / 2.0,
        TextAnchor::RightBaseline | TextAnchor::RightMiddle => -width,
    };
    // A middle anchor centres the capitals on the point: the baseline is half their height below.
    let down = match t.anchor {
        TextAnchor::LeftMiddle | TextAnchor::CenterMiddle | TextAnchor::RightMiddle => {
            font.cap_em() * size / 2.0
        }
        _ => 0.0,
    };
    let (s, c) = (libm::sin(turn), libm::cos(turn));
    let start = [p0[0] + along * c - down * s, p0[1] + along * s + down * c];
    page.text_at(
        &t.text,
        (&t.font, t.weight, t.italic),
        mm(t.size),
        start,
        turn,
        &t.color,
        t.halo.as_deref(),
    );
}

/// A map's picture (the host's PNG of the content box) over the box, turned with the frame.
pub(super) fn raster(page: &mut Page<'_>, m: &MapPrim, image: (&str, Ref)) {
    page.xobjects.insert(image.0.to_owned(), image.1);
    let frame = Frame::of(m);
    clip_to(page, &frame);
    page.place(&m.clip, m.rotation);
    page.c.x_object(Name(image.0.as_bytes()));
    page.restore();
}

/// The text the light “harita” box writes, and its face (what the document must embed).
pub(super) const PLACEHOLDER: (&str, &str, u16) = ("harita", "barlow", 400);

/// A map without content: a light grey box with “harita”, as the SVG writer draws it.
pub(super) fn placeholder(page: &mut Page<'_>, m: &MapPrim) {
    let r = &m.clip;
    page.rect(r, m.rotation, 0);
    page.paint(Some("#ececec"), None, false);
    let size = f64::from((r.height.min(r.width) / 12).clamp(2_000, 12_000));
    if page
        .fonts
        .get(PLACEHOLDER.1, PLACEHOLDER.2, false)
        .is_none()
    {
        return;
    }
    let width = page
        .fonts
        .width_em(PLACEHOLDER.1, PLACEHOLDER.2, false, PLACEHOLDER.0)
        * size;
    let c = r.center();
    let start = rotate([c[0] - width / 2.0, c[1] + size / 3.0], c, m.rotation);
    page.text_at(
        PLACEHOLDER.0,
        (PLACEHOLDER.1, PLACEHOLDER.2, false),
        size * super::content::PT,
        start,
        rad(m.rotation),
        "#a0a0a0",
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::MapViewPrim;
    use crate::kinds::{GroundPoint, MapLayers};
    use crate::units::RectUm;

    fn prim(rotation: i32, view_rot: i32) -> MapPrim {
        MapPrim {
            item: "m".into(),
            clip: RectUm::new(20_000, 10_000, 200_000, 100_000),
            rotation,
            view: MapViewPrim {
                center: Some(GroundPoint {
                    x: 500_000.0,
                    y: 4_420_000.0,
                }),
                scale: 1000,
                rotation: view_rot,
            },
            layers: MapLayers::default(),
            crs: None,
            extent: None,
            clip_feature: None,
        }
    }

    /// The view's centre is the box's middle; a metre east is a millimetre right at 1/1000, a
    /// metre north a millimetre up; the paper and the ground go both ways, turned or not.
    #[test]
    fn the_ground_and_the_paper_go_both_ways() {
        let f = Frame::of(&prim(0, 0));
        let mid = f.paper([500_000.0, 4_420_000.0]).unwrap();
        assert_eq!(mid, [120_000.0, 60_000.0]);
        let east = f.paper([500_001.0, 4_420_000.0]).unwrap();
        assert!((east[0] - 121_000.0).abs() < 1e-6 && (east[1] - 60_000.0).abs() < 1e-6);
        let north = f.paper([500_000.0, 4_420_001.0]).unwrap();
        assert!((north[1] - 59_000.0).abs() < 1e-6);
        for (turn, view) in [(0, 0), (30_000, 0), (0, -15_000), (25_000, 10_000)] {
            let f = Frame::of(&prim(turn, view));
            let g = [500_037.5, 4_419_981.25];
            let back = f.ground(f.paper(g).unwrap()).unwrap();
            assert!(
                (back[0] - g[0]).abs() < 1e-6 && (back[1] - g[1]).abs() < 1e-6,
                "{turn} {view}"
            );
        }
    }

    /// The anchor decides: inside the content box (turned or not) a text is written, outside not.
    #[test]
    fn a_text_is_written_when_its_anchor_is_inside_the_frame() {
        for (turn, view) in [(0, 0), (30_000, 0), (0, -15_000)] {
            let f = Frame::of(&prim(turn, view));
            assert!(f.holds([500_000.0, 4_420_000.0]), "the middle");
            // The box is 200 × 100 mm at 1/1000: 100 m east of the middle is its edge.
            // Along the frame's own axes on the ground: the view's turn (display::MapFrame).
            let (s, c) = crate::units::sin_cos(view);
            let at = |e: f64, n: f64| [500_000.0 + e * c - n * s, 4_420_000.0 + e * s + n * c];
            assert!(
                f.holds(at(99.0, 49.0)),
                "{turn} {view}: inside, near a corner"
            );
            assert!(
                !f.holds(at(101.0, 0.0)),
                "{turn} {view}: past the right edge"
            );
            assert!(!f.holds(at(0.0, 51.0)), "{turn} {view}: past the top edge");
        }
    }

    #[test]
    fn a_path_outside_the_frame_is_left_out() {
        let e = [499_900.0, 4_419_950.0, 500_100.0, 4_420_050.0];
        assert!(meets(&e, &[[500_000.0, 4_420_000.0]], 0.0));
        assert!(!meets(
            &e,
            &[[500_200.0, 4_420_000.0], [500_300.0, 4_420_010.0]],
            0.0
        ));
        assert!(meets(&e, &[[500_200.0, 4_420_000.0]], 150.0));
    }
}
