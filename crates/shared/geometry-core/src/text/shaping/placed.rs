//! The model-space glyph plan. Coordinates stay relative to the text anchor,
//! including on a curve, so a renderer subtracts its camera in f64 before f32.
//! No consumer measures or shapes these glyphs again.
use super::{Error, Glyph, OutlineOp, Style, paragraph, rich};

fn rules_of(flow: &rich::Layout, piece: usize) -> &[rich::Rule] {
    let start = flow.rules.partition_point(|r| r.piece < piece);
    let end = flow.rules[start..].partition_point(|r| r.piece == piece) + start;
    &flow.rules[start..end]
}
use crate::{
    entity::TextPlace,
    geom::affine::{self, Affine},
    jsmath::{PI, atan2, js_max, js_min},
    text::{Font, TextAlign, along::Walk},
    vec2::Vec2,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedGlyph {
    pub glyph: Glyph,
    /// Shaped em coordinates to model coordinates relative to the text's p.
    pub transform: Affine,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedRule {
    pub corners: [[f64; 2]; 4],
    pub run: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub glyphs: Vec<PlacedGlyph>,
    /// Curved grapheme/ligature/fraction groups, relative to the anchor.
    pub clusters: Vec<crate::text::along::Letter>,
    pub rules: Vec<PlacedRule>,
    pub width: f64,
    pub pitch: f64,
    pub depth: usize,
    /// Conservative actual ink, relative to p; absent for whitespace only.
    pub ink: Option<[f64; 4]>,
    /// Pick/mask frame in the text axes, relative to p before its rotation.
    pub frame: Option<[f64; 4]>,
}
fn at(m: &Affine, [x, y]: [f64; 2]) -> [f64; 2] {
    let p = affine::apply(m, Vec2::new(x, y));
    [p.x, p.y]
}
fn rect(m: &Affine, [a, b, c, d]: [f64; 4]) -> [[f64; 2]; 4] {
    [[a, b], [c, b], [c, d], [a, d]].map(|p| at(m, p))
}
fn include(ink: &mut Option<[f64; 4]>, p: [f64; 2]) {
    match ink {
        Some(b) => {
            b[0] = js_min(b[0], p[0]);
            b[1] = js_min(b[1], p[1]);
            b[2] = js_max(b[2], p[0]);
            b[3] = js_max(b[3], p[1]);
        }
        None => *ink = Some([p[0], p[1], p[0], p[1]]),
    }
}
impl PlacedGlyph {
    /// Actual Bézier contour, in the same model frame as the measured ink.
    pub fn outline(&self) -> Result<Vec<OutlineOp>, Error> {
        let m = &self.transform;
        let ops = super::outline(&self.glyph)?
            .into_iter()
            .map(|op| match op {
                OutlineOp::Move(p) => OutlineOp::Move(at(m, p)),
                OutlineOp::Line(p) => OutlineOp::Line(at(m, p)),
                OutlineOp::Quad(c, p) => OutlineOp::Quad(at(m, c), at(m, p)),
                OutlineOp::Cubic(a, b, p) => OutlineOp::Cubic(at(m, a), at(m, b), at(m, p)),
                OutlineOp::Close => OutlineOp::Close,
            })
            .collect::<Vec<_>>();
        // Very large valid input must not turn into infinite path vertices.
        for op in &ops {
            let points: &[[f64; 2]] = match op {
                OutlineOp::Move(p) | OutlineOp::Line(p) => std::slice::from_ref(p),
                OutlineOp::Quad(a, b) => &[*a, *b],
                OutlineOp::Cubic(a, b, c) => &[*a, *b, *c],
                OutlineOp::Close => &[],
            };
            if points.iter().flatten().any(|v| !v.is_finite()) {
                return Err(Error::InvalidGeometry);
            }
        }
        Ok(ops)
    }
}
impl Layout {
    /// Prepared contours share a bounded budget across the complete object.
    /// A complex text is an explicit failure, never a partially drawn suffix.
    pub fn contours(&self) -> Result<Vec<Vec<OutlineOp>>, Error> {
        let mut paths = Vec::with_capacity(self.glyphs.len());
        let mut count = 0;
        for glyph in &self.glyphs {
            let path = glyph.outline()?;
            count += path.len();
            if count > 1_000_000 {
                return Err(Error::TooManyContours);
            }
            paths.push(path);
        }
        Ok(paths)
    }
    fn measure(&mut self, unrotate: &Affine) -> Result<(), Error> {
        for g in &self.glyphs {
            let face = super::parsed(g.glyph.font)?;
            let Some(b) = face.glyph_bounding_box(rustybuzz::ttf_parser::GlyphId(g.glyph.id))
            else {
                continue;
            };
            let em = f64::from(face.units_per_em());
            for [x, y] in [
                [b.x_min, b.y_min],
                [b.x_max, b.y_min],
                [b.x_max, b.y_max],
                [b.x_min, b.y_max],
            ]
            .map(|p| p.map(f64::from))
            {
                let x = x / em * g.glyph.size;
                let y = y / em * g.glyph.size;
                let p = at(
                    &g.transform,
                    [
                        g.glyph.origin[0] + x + g.glyph.slant * y,
                        g.glyph.origin[1] + y,
                    ],
                );
                include(&mut self.ink, p);
                include(&mut self.frame, at(unrotate, p));
            }
        }
        for r in &self.rules {
            for &p in &r.corners {
                include(&mut self.ink, p);
                include(&mut self.frame, at(unrotate, p));
            }
        }
        if self.ink.is_some_and(|b| b.iter().any(|x| !x.is_finite()))
            || self.frame.is_some_and(|b| b.iter().any(|x| !x.is_finite()))
            || self
                .glyphs
                .iter()
                .any(|g| g.transform.iter().any(|x| !x.is_finite()))
        {
            return Err(Error::InvalidGeometry);
        }
        Ok(())
    }
}

/// The selected face's italic flag is explicit: legacy text tables never
/// encoded it in Font, and that public font identity stays unchanged.
pub fn layout(t: &TextPlace<'_>, font: Font, italic: bool) -> Result<Layout, Error> {
    if !t.rotation.is_finite() || !t.lean.is_finite() {
        return Err(Error::InvalidGeometry);
    }
    let curved = t.path.is_some();
    if curved && (t.text.contains(['\r', '\n', '\t'])) {
        return Err(Error::LineBreak);
    }
    let flow = rich::layout(
        t.text,
        t.runs,
        Style {
            family: t.font_or(font).id(),
            weight: if t.bold { 600 } else { 400 },
            italic,
            ..Style::default()
        },
        paragraph::Settings {
            height: t.height,
            width_factor: t.width_factor.unwrap_or(1.),
            box_width: if curved { None } else { t.box_width },
            line_spacing: t.line_spacing.unwrap_or(1.),
            along: if curved {
                0.
            } else {
                t.align.map_or(0., TextAlign::along)
            },
            flow: if curved { None } else { t.flow },
        },
    )?;
    let mut out = Layout {
        glyphs: Vec::new(),
        clusters: Vec::new(),
        rules: Vec::new(),
        width: flow.flow.width,
        pitch: flow.flow.pitch,
        depth: flow.flow.depth,
        ink: None,
        frame: None,
    };
    let h = t.height;
    let sx = h * t.width_factor.unwrap_or(1.);
    let rotate = affine::rotation(t.rotation * PI / 180., Vec2::new(0., 0.));
    let unrotate = affine::rotation(-t.rotation * PI / 180., Vec2::new(0., 0.));
    let scale = [sx, 0., 0., h, 0., 0.];
    if let Some(curve) = t.path {
        let walk = Walk::of(curve, Vec2::new(0., 0.), 0.);
        let locate = walk.locator();
        let start = t.align.map_or(0., TextAlign::along) * (walk.total() - out.width);
        let up = t.align.map_or(0., TextAlign::up) * h;
        for (piece_index, piece) in flow.flow.pieces.iter().enumerate() {
            let glyphs = &piece.line.glyphs;
            let mut rules = rules_of(&flow, piece_index).iter().collect::<Vec<_>>();
            rules.sort_by(|a, b| a.rect[0].total_cmp(&b.rect[0]));
            let mut next_rule = 0;
            let mut active: Vec<&rich::Rule> = Vec::new();
            let mut from = 0;
            while from < glyphs.len() {
                let first = &glyphs[from];
                let run_index = t.runs.partition_point(|r| r.end <= first.scalars.start);
                let stack = t.runs.get(run_index).filter(|r| {
                    r.script == Some(crate::text::paragraph::Script::Stack)
                        && r.start <= first.scalars.start
                        && first.scalars.end <= r.end
                });
                let mut to = from + 1;
                while to < glyphs.len()
                    && stack.map_or_else(
                        || glyphs[to].bytes == first.bytes,
                        |r| r.start <= glyphs[to].scalars.start && glyphs[to].scalars.end <= r.end,
                    )
                {
                    to += 1;
                }
                // Marks and ligature components share the base cluster's
                // tangent; an inline fraction is one atomic curved group.
                let left = glyphs[from..to]
                    .iter()
                    .map(|g| g.pen)
                    .fold(f64::INFINITY, js_min);
                let right = glyphs[from..to]
                    .iter()
                    .map(|g| g.pen + g.advance)
                    .fold(f64::NEG_INFINITY, js_max);
                let middle = (left + right) * sx / 2.;
                let (p, tangent) = locate(start + middle, 0.);
                let turn = atan2(tangent.y, tangent.x);
                let local = [1., 0., t.lean, 1., -middle - up * t.lean, -up];
                let m = affine::compose(
                    &rotate,
                    &affine::compose(
                        &affine::translation(p.x, p.y),
                        &affine::compose(
                            &affine::rotation(turn, Vec2::new(0., 0.)),
                            &affine::compose(&local, &scale),
                        ),
                    ),
                );
                let baseline = at(&m, [left, 0.]);
                let center = at(&rotate, [p.x, p.y]);
                let direction = at(&rotate, [tangent.x, tangent.y]);
                out.clusters.push(crate::text::along::Letter {
                    at: Vec2::new(baseline[0], baseline[1]),
                    mid: Vec2::new(center[0], center[1]),
                    tangent: Vec2::new(direction[0], direction[1]),
                    turn: crate::text::along::degrees(atan2(direction[1], direction[0])),
                    advance: (right - left) * sx,
                });
                for p in rect(&m, [left, -0.23, right, 1.15]) {
                    include(&mut out.frame, at(&unrotate, p));
                }
                out.glyphs
                    .extend(glyphs[from..to].iter().cloned().map(|glyph| PlacedGlyph {
                        glyph,
                        transform: m,
                    }));
                active.retain(|r| r.rect[2] > left);
                while next_rule < rules.len() && rules[next_rule].rect[0] < right {
                    let rule = rules[next_rule];
                    if rule.rect[2] > left {
                        active.push(rule);
                    }
                    next_rule += 1;
                }
                for rule in &active {
                    let [a, b, c, d] = rule.rect;
                    let (a, c) = (js_max(a, left), js_min(c, right));
                    if c > a {
                        out.rules.push(PlacedRule {
                            corners: rect(&m, [a, b, c, d]),
                            run: rule.run,
                        });
                    }
                }
                from = to;
            }
        }
    } else {
        let along = t.align.map_or(0., TextAlign::along) * out.width;
        let up = crate::text::paragraph::rise(
            t.align.map_or(0., TextAlign::up),
            h,
            out.depth,
            out.pitch,
        );
        let offset = affine::translation(-along, -up);
        let lean = [1., 0., t.lean, 1., 0., 0.];
        let nominal = affine::compose(&offset, &lean);
        let below = out.depth.saturating_sub(1) as f64 * out.pitch;
        for p in rect(&nominal, [0., -0.23 * h - below, out.width, 1.15 * h]) {
            include(&mut out.frame, p);
        }
        for (index, piece) in flow.flow.pieces.iter().enumerate() {
            let m = affine::compose(
                &rotate,
                &affine::compose(
                    &offset,
                    &affine::compose(
                        &lean,
                        &affine::compose(&affine::translation(piece.at[0], piece.at[1]), &scale),
                    ),
                ),
            );
            out.glyphs
                .extend(piece.line.glyphs.iter().cloned().map(|glyph| PlacedGlyph {
                    glyph,
                    transform: m,
                }));
            out.rules
                .extend(rules_of(&flow, index).iter().map(|r| PlacedRule {
                    corners: rect(&m, r.rect),
                    run: r.run,
                }));
        }
    }
    out.measure(&unrotate)?;
    Ok(out)
}
