//! Dash a geometric-width CAD path before filling it. Split distances use
//! analytic arc length; widths interpolate on the original edge, not on the
//! tessellated outline. The source shape is never rewritten.
use super::{
    compile::{Env, Values, dd_bool, emit_line_layer_on, stroke_style, to_world},
    model::{Layer, Symbol},
    prim::{FillPaint, MarkerStyle, Sink, StrokeStyle},
};
use kentos_geometry_core::{
    Vec2,
    entity::Shape,
    geom::{
        band,
        bulge::{BulgeArc, bulge_arc, bulge_at, bulge_of_sweep},
    },
    jsmath::{cos, js_hypot, js_min, sin},
};

const MAX_PIECES: usize = 20_000;
const MAX_POINTS: usize = 500_000;

#[derive(Clone, Debug, Default)]
struct Piece {
    pts: Vec<Vec2>,
    bulges: Vec<f64>,
    widths: Vec<[f64; 2]>,
    closed: bool,
}
#[derive(Clone, Copy)]
struct Edge {
    a: Vec2,
    b: Vec2,
    arc: Option<BulgeArc>,
    length: f64,
    width: [f64; 2],
}
impl Edge {
    fn point(self, f: f64) -> Vec2 {
        if f <= 0. {
            return self.a;
        }
        if f >= 1. {
            return self.b;
        }
        match self.arc {
            Some(a) => Vec2::new(
                a.c.x + a.r * cos(a.a0 + a.sweep * f),
                a.c.y + a.r * sin(a.a0 + a.sweep * f),
            ),
            None => Vec2::new(
                self.a.x + (self.b.x - self.a.x) * f,
                self.a.y + (self.b.y - self.a.y) * f,
            ),
        }
    }
    fn append(self, start: f64, end: f64, piece: &mut Piece) {
        if piece.pts.is_empty() {
            piece.pts.push(self.point(start));
        }
        piece.pts.push(self.point(end));
        piece.bulges.push(
            self.arc
                .map_or(0., |a| bulge_of_sweep(a.sweep * (end - start))),
        );
        let width = |f| self.width[0] + (self.width[1] - self.width[0]) * f;
        piece.widths.push([width(start), width(end)]);
    }
}

fn split(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    widths: &[[f64; 2]],
    dash: &[f64],
    phase: f64,
) -> Result<Vec<Piece>, String> {
    let fail =
        || "Geniş çizginin desen karmaşıklığı sınırı aşıldı; eksik çıktı üretilmedi.".to_owned();
    if dash.is_empty() || dash.iter().any(|d| !d.is_finite() || *d < 0.) || !phase.is_finite() {
        return Err("Geniş çizginin deseni geçersiz.".into());
    }
    if dash.iter().step_by(2).any(|d| *d == 0.) {
        return Err("Nokta öğeli geniş çizgi deseni henüz desteklenmiyor.".into());
    }
    let mut dash = dash.to_vec();
    if dash.len() % 2 == 1 {
        dash.extend_from_within(..);
    }
    let period = dash.iter().sum::<f64>();
    if !period.is_finite() || period <= 0. {
        return Err("Geniş çizginin tekrar uzunluğu geçersiz.".into());
    }
    let mut edges = Vec::new();
    let mut total = 0.;
    for i in 0..if closed {
        pts.len()
    } else {
        pts.len().saturating_sub(1)
    } {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let arc = bulge_arc(a, b, bulge_at(bulges, i));
        let length = arc.map_or_else(|| js_hypot(b.x - a.x, b.y - a.y), |a| a.r * a.sweep.abs());
        if !length.is_finite() {
            return Err("Geniş çizgi yolu geçersiz.".into());
        }
        if length == 0. {
            continue;
        }
        edges.push(Edge {
            a,
            b,
            arc,
            length,
            width: widths.get(i).copied().unwrap_or([0., 0.]),
        });
        total += length;
    }
    // Bound the entire traversal before allocating a prefix. Each cycle has
    // dash.len() transitions; edge boundaries add at most edges.len().
    if !total.is_finite()
        || (total / period + 2.) * dash.len() as f64 + edges.len() as f64 > MAX_PIECES as f64
    {
        return Err(fail());
    }
    let mut offset = (-phase).rem_euclid(period);
    let mut di = 0;
    while di + 1 < dash.len() && offset >= dash[di] {
        offset -= dash[di];
        di += 1;
    }
    let mut left = dash[di] - offset;
    let mut out = Vec::new();
    let mut piece = Piece::default();
    for e in edges {
        let mut at = 0.;
        while at < e.length {
            if left <= 0. {
                di = (di + 1) % dash.len();
                left = dash[di];
                continue;
            }
            let amount = js_min(left, e.length - at);
            let end = at + amount;
            if end <= at {
                return Err("Geniş çizgi deseni bu koordinatta ayırt edilemiyor.".into());
            }
            if di % 2 == 0 {
                e.append(at / e.length, end / e.length, &mut piece);
            }
            at = end;
            left -= amount;
            if left == 0. {
                let was_on = di % 2 == 0;
                di = (di + 1) % dash.len();
                left = dash[di];
                // A zero gap joins adjacent ink runs instead of making an
                // artificial cap at a vertex.
                while left == 0. {
                    di = (di + 1) % dash.len();
                    left = dash[di];
                }
                if was_on && di % 2 == 1 && !piece.pts.is_empty() {
                    out.push(std::mem::take(&mut piece));
                }
            }
        }
    }
    if !piece.pts.is_empty() {
        out.push(piece);
    }
    if closed
        && out.len() > 1
        && out[0].pts.first() == pts.first()
        && out.last().and_then(|p| p.pts.last()) == pts.first()
    {
        let first = out.remove(0);
        if let Some(last) = out.last_mut() {
            last.pts.extend(first.pts.into_iter().skip(1));
            last.bulges.extend(first.bulges);
            last.widths.extend(first.widths);
        }
    }
    if closed && out.len() == 1 && out[0].pts.first() == out[0].pts.last() {
        let p = &mut out[0];
        p.pts.pop();
        p.closed = true;
    }
    Ok(out)
}

/// Carry the existing mark compiler's visibility, budget and diagnostics;
/// suppress only its centre stroke, since the bands supply geometric ink.
struct MarksOnly<'a>(&'a mut dyn Sink);
impl Sink for MarksOnly<'_> {
    fn cad_shape(&mut self, s: &StrokeStyle, p: &[Vec2]) {
        self.0.cad_shape(s, p);
    }
    fn stroke(&mut self, s: &StrokeStyle, p: &[Vec2], c: bool) {
        if s.line_type_loss.is_some() {
            self.0.stroke(s, p, c);
        }
    }
    fn fill(&mut self, p: &FillPaint, r: &[Vec<Vec2>]) {
        self.0.fill(p, r);
    }
    fn marker(&mut self, s: &MarkerStyle, p: Vec2, a: f64) {
        self.0.marker(s, p, a);
    }
    fn reserve_cad_points(&mut self, n: usize) -> bool {
        self.0.reserve_cad_points(n)
    }
    fn cad_mark(&mut self, p: &FillPaint, r: &[Vec<Vec2>]) {
        self.0.cad_mark(p, r);
    }
}

/// Native intrinsic wide paths have one plain line layer. Preserve the old
/// solid-band path byte-for-byte for continuous lines and all other symbols.
pub(super) fn compile(
    shape: &Shape,
    symbol: &Symbol,
    values: &dyn Values,
    env: &Env,
    sink: &mut dyn Sink,
    level: f64,
) -> bool {
    let Shape::Polyline {
        pts,
        bulges,
        closed,
        widths: Some(widths),
        ..
    } = shape
    else {
        return false;
    };
    if !widths.iter().any(|[a, b]| *a > 0. || *b > 0.) {
        return false;
    }
    let [Layer::SimpleLine(l)] = symbol.layers.as_slice() else {
        return false;
    };
    if l.dash.as_ref().is_none_or(Vec::is_empty)
        && l.cad_marks.is_none()
        && l.line_type_loss.is_none()
    {
        return false;
    }
    if l.offset.is_some() || l.shift.is_some() || l.wave.is_some() || l.base.effects.is_some() {
        return false;
    }
    if !dd_bool(&l.base.enabled, values) {
        return true;
    }
    let Some(mut style) = stroke_style(l, level, values, env) else {
        return true;
    };
    let dash: Vec<f64> = l
        .dash
        .as_ref()
        .map(|d| d.iter().map(|v| to_world(*v, l.base.unit, env)).collect())
        .unwrap_or_else(|| vec![1., 0.]);
    let generated = (|| {
        let parts = if l.dash.as_ref().is_none_or(Vec::is_empty) {
            vec![Piece {
                pts: pts.clone(),
                bulges: bulges.clone().unwrap_or_else(|| vec![0.; pts.len()]),
                widths: widths.clone(),
                closed: *closed,
            }]
        } else {
            split(
                pts,
                bulges.as_deref(),
                *closed,
                widths,
                &dash,
                to_world(l.dash_offset.unwrap_or(0.), l.base.unit, env),
            )?
        };
        let mut bands = Vec::with_capacity(parts.len());
        let mut cost = 0usize;
        for p in parts {
            let b = band::band(&p.pts, Some(&p.bulges), p.closed, &p.widths);
            cost = cost.saturating_add(
                b.rings.iter().map(Vec::len).sum::<usize>()
                    + b.thin.iter().map(Vec::len).sum::<usize>(),
            );
            if cost > MAX_POINTS {
                return Err("Geniş çizginin kontur sınırı aşıldı; eksik çıktı üretilmedi.".into());
            }
            bands.push(b);
        }
        if !sink.reserve_cad_points(cost) {
            return Err("Katmanın toplam geniş çizgi kontur sınırı aşıldı.".into());
        }
        Ok(bands)
    })();
    match generated {
        Ok(bands) => {
            let paint = FillPaint::Solid {
                color: style.color.clone(),
                opacity: style.opacity,
                level,
            };
            let mut thin = style.clone();
            thin.dash = None;
            thin.dash_offset = 0.;
            for b in bands {
                for r in b.rings {
                    sink.cad_mark(&paint, &[r]);
                }
                for p in b.thin {
                    sink.stroke(&thin, &p, false);
                }
            }
        }
        Err(error) => {
            style.line_type_loss = Some(error);
            sink.stroke(&style, pts, *closed);
        }
    }
    if l.cad_marks.is_some() || l.line_type_loss.is_some() {
        match super::cad_marks::Route::polyline(pts, bulges.as_deref(), *closed) {
            Ok(route) => emit_line_layer_on(
                &Layer::SimpleLine(l.clone()),
                (pts, *closed),
                level,
                values,
                env,
                &mut MarksOnly(sink),
                Some(&route),
            ),
            Err(error) => {
                style.line_type_loss = Some(error);
                sink.stroke(&style, pts, *closed);
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }
    #[test]
    fn cuts_interpolate_widths_and_keep_phase_over_vertices() {
        let got = split(
            &[p(0., 0.), p(2., 0.), p(2., 3.)],
            None,
            false,
            &[[1., 3.], [3., 6.]],
            &[3., 1.],
            0.,
        )
        .unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].pts, vec![p(0., 0.), p(2., 0.), p(2., 1.)]);
        assert_eq!(got[0].widths, vec![[1., 3.], [3., 4.]]);
        assert_eq!(got[1].pts, vec![p(2., 2.), p(2., 3.)]);
        assert_eq!(got[1].widths, vec![[5., 6.]]);
    }
    #[test]
    fn arc_cuts_use_arc_length_instead_of_chords() {
        let got = split(
            &[p(1., 0.), p(-1., 0.)],
            Some(&[1.]),
            false,
            &[[2., 4.]],
            &[std::f64::consts::FRAC_PI_2, 10.],
            0.,
        )
        .unwrap();
        assert_eq!(got.len(), 1);
        assert!((got[0].pts[1].x).abs() < 1e-12);
        assert!((got[0].pts[1].y - 1.).abs() < 1e-12);
        assert!((got[0].bulges[0] - (std::f64::consts::PI / 8.).tan()).abs() < 1e-12);
        assert_eq!(got[0].widths, vec![[2., 3.]]);
    }
    #[test]
    fn closed_seam_joins_ink_and_zero_gaps_do_not_add_caps() {
        let got = split(
            &[p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.)],
            None,
            true,
            &[[1., 1.]; 4],
            &[3., 1.],
            -1.,
        )
        .unwrap();
        assert_eq!(got.len(), 2);
        assert!(got.iter().any(|r| r.pts.windows(3).any(|p| p
            == [
                super::tests::p(0., 1.),
                super::tests::p(0., 0.),
                super::tests::p(2., 0.)
            ])));
        let got = split(
            &[p(0., 0.), p(5., 0.)],
            None,
            false,
            &[[1., 1.]],
            &[1., 0.],
            0.,
        )
        .unwrap();
        assert_eq!(got.len(), 1);
    }
    #[test]
    fn whole_path_work_is_bounded_and_invalid_patterns_are_explicit() {
        assert!(
            split(
                &[p(0., 0.), p(1e12, 0.)],
                None,
                false,
                &[[1., 1.]],
                &[1., 1.],
                0.
            )
            .is_err()
        );
        for d in [vec![0., 1.], vec![f64::NAN, 1.], vec![1., -1.]] {
            assert!(split(&[p(0., 0.), p(1., 0.)], None, false, &[[1., 1.]], &d, 0.).is_err());
        }
    }
}
