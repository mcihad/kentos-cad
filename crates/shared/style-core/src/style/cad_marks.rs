//! CAD linetype marks keep the source phase, corners and R/A/U policy.
//! Templates are frozen bundled-font polygons; no backend shapes text.
use super::{
    compile::{Env, to_world},
    model::Unit,
    prim::{FillPaint, Sink, StrokeStyle},
};
use kentos_geometry_core::jsmath::{atan2, cos, js_hypot, js_max, sin};
use kentos_geometry_core::{Vec2, api::json::Json};

#[derive(Clone, Debug)]
pub struct Mark {
    at: f64,
    cap: f64,
    height: f64,
    x: f64,
    y: f64,
    rotation: f64,
    absolute: bool,
    upright: bool,
    polygons: Vec<Vec<Vec<Vec2>>>,
    cost: usize,
}
#[derive(Clone, Debug)]
pub struct Marks {
    period: f64,
    marks: Vec<Mark>,
    name: String,
}
const MAX_POINTS: usize = 500_000;
const MAX_MARKS: usize = 2000;
fn number(v: &Json) -> Result<f64, String> {
    match v {
        Json::Num(n) if n.is_finite() => Ok(*n),
        _ => Err("Çizgi tipi işaretinde geçersiz sayı.".into()),
    }
}
fn array(v: &Json) -> Result<&[Json], String> {
    match v {
        Json::Arr(a) => Ok(a),
        _ => Err("Çizgi tipi işaretinin konturu geçersiz.".into()),
    }
}
impl Marks {
    pub fn read(v: &Json) -> Option<Result<Self, String>> {
        if matches!(v, Json::Null) {
            return None;
        }
        Some(Self::parse(v))
    }
    fn parse(v: &Json) -> Result<Self, String> {
        let period = number(v.get("period"))?;
        if period <= 0. {
            return Err("Çizgi tipinin tekrar uzunluğu pozitif olmalı.".into());
        }
        let mut marks = Vec::new();
        let mut cost = 0;
        let list = array(v.get("marks"))?;
        if list.len() > 12 {
            return Err("Çizgi tipi işaret sayısı sınırı aşıldı.".into());
        }
        for m in list {
            let before = cost;
            let mut polygons = Vec::new();
            for p in array(m.get("polygons"))? {
                let mut rings = Vec::new();
                for r in array(p)? {
                    let mut points = Vec::new();
                    for p in array(r)? {
                        let pair = array(p)?;
                        if pair.len() != 2 {
                            return Err("Çizgi tipi noktası geçersiz.".into());
                        }
                        points.push(Vec2::new(number(&pair[0])?, number(&pair[1])?));
                        cost += 1;
                        if cost > 32_768 {
                            return Err("Çizgi tipi kaynak konturu sınırı aşıldı.".into());
                        }
                    }
                    if points.len() < 3 {
                        return Err("Çizgi tipi konturunda en az üç nokta gerekli.".into());
                    }
                    rings.push(points);
                }
                if rings.is_empty() {
                    return Err("Çizgi tipi konturu boş.".into());
                }
                polygons.push(rings);
            }
            let mark = Mark {
                at: number(m.get("at"))?,
                cap: number(m.get("cap"))?,
                height: number(m.get("height"))?,
                x: number(m.get("x"))?,
                y: number(m.get("y"))?,
                rotation: number(m.get("rotation"))?.to_radians(),
                absolute: matches!(m.get("absolute"), Json::Bool(true)),
                upright: matches!(m.get("upright"), Json::Bool(true)),
                polygons,
                cost: cost - before,
            };
            if mark.cap <= 0. || mark.height <= 0. || mark.at < 0. || mark.at > period {
                return Err("Çizgi tipi işaretinin boyu veya fazı geçersiz.".into());
            }
            marks.push(mark);
        }
        Ok(Self {
            period,
            marks,
            name: match v.get("name") {
                Json::Str(s) => s.clone(),
                _ => "Çizgi tipi".into(),
            },
        })
    }
    pub fn source(&self) -> &str {
        &self.name
    }
    /// Validate the whole path before painting. A limit never returns a
    /// misleading partially drawn prefix, and never loops through hidden cycles.
    pub fn placements<'a>(
        &'a self,
        path: &[Vec2],
        closed: bool,
        phase: f64,
        unit: Unit,
        env: &Env,
    ) -> Result<Vec<Placement<'a>>, String> {
        if path.len() < 2 {
            return Ok(Vec::new());
        }
        if !phase.is_finite() {
            return Err("Çizgi tipi başlangıç fazı geçersiz.".into());
        }
        let period = to_world(self.period, unit, env);
        if !period.is_finite() || period <= 0. {
            return Err("Çizgi tipi tekrar uzunluğu çizim ölçeğinde geçersiz.".into());
        }
        let mut segments = Vec::new();
        let mut total = 0.;
        for (&a, &b) in path
            .iter()
            .zip(path.iter().skip(1).chain(closed.then(|| &path[0])))
        {
            let length = js_hypot(b.x - a.x, b.y - a.y);
            if !length.is_finite() {
                return Err("Çizgi tipi yolu geçersiz.".into());
            }
            if length > 0. {
                segments.push((a, b, total, length));
                total += length;
            }
        }
        if !total.is_finite() {
            return Err("Çizgi tipi yol uzunluğu geçersiz.".into());
        }
        let mut placed = Vec::new();
        let mut cost = 0usize;
        for m in &self.marks {
            let at = to_world(m.at, unit, env);
            let mut distance = at + phase.rem_euclid(period);
            if distance >= period && at < period {
                distance -= period;
            }
            let count = js_max(((total - distance) / period).ceil(), 0.);
            if count > MAX_MARKS as f64
                || placed.len().saturating_add(count as usize) > MAX_MARKS
                || cost.saturating_add((count as usize).saturating_mul(m.cost)) > MAX_POINTS
            {
                return Err("Çizgi tipi işaretleri çizim karmaşıklığı sınırını aşıyor; eksik çıktı üretilmedi.".into());
            }
            let mut segment = 0;
            for _ in 0..count as usize {
                if total - distance <= period * 1e-10 {
                    break;
                }
                while segment + 1 < segments.len()
                    && segments[segment].2 + segments[segment].3 <= distance
                {
                    segment += 1;
                }
                let Some(&(a, b, start, len)) = segments.get(segment) else {
                    break;
                };
                let t = ((distance - start) / len).clamp(0., 1.);
                let tangent = atan2(b.y - a.y, b.x - a.x);
                placed.push(Placement {
                    mark: m,
                    at: Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t),
                    tangent,
                });
                cost = cost.saturating_add(m.cost);
                distance += period;
            }
        }
        Ok(placed)
    }
}
pub struct Placement<'a> {
    mark: &'a Mark,
    pub at: Vec2,
    pub tangent: f64,
}
impl Placement<'_> {
    pub fn cost(&self) -> usize {
        self.mark.cost
    }
    pub fn paint(&self, style: &StrokeStyle, unit: Unit, env: &Env, sink: &mut dyn Sink) {
        let m = self.mark;
        let height = to_world(m.height, unit, env);
        let (ts, tc) = (sin(self.tangent), cos(self.tangent));
        let x = to_world(m.x, unit, env);
        let y = to_world(m.y, unit, env);
        let origin = Vec2::new(self.at.x + tc * x - ts * y, self.at.y + ts * x + tc * y);
        let base = if m.absolute { 0. } else { self.tangent };
        let mut angle = m.rotation + base;
        if m.upright && cos(angle) < 0. {
            angle += std::f64::consts::PI;
        }
        let (s, c) = (sin(angle), cos(angle));
        let paint = FillPaint::Solid {
            color: style.color.clone(),
            opacity: style.opacity,
            level: style.level,
        };
        for polygon in &m.polygons {
            let rings: Vec<Vec<Vec2>> = polygon
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|p| {
                            // Rotation pivot is midway between baseline and cap height.
                            let x = p.x * height;
                            let y = (p.y - m.cap * 0.5) * height;
                            Vec2::new(
                                origin.x - sin(base) * m.cap * 0.5 * height + c * x - s * y,
                                origin.y + cos(base) * m.cap * 0.5 * height + s * x + c * y,
                            )
                        })
                        .collect()
                })
                .collect();
            sink.cad_mark(&paint, &rings);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::prim::{PrimUnit, PrimitiveList};
    use super::*;
    use std::collections::HashMap;
    fn marks(rotation: f64, absolute: bool, upright: bool) -> Marks {
        let s = serde_json::json!({"period":4.,"marks":[{"at":3.,"cap":1.,"height":2.,"x":0.5,"y":-0.25,
            "rotation":rotation,"absolute":absolute,"upright":upright,"polygons":[[[[0.,0.],[1.,0.],[0.,1.]]]]}]});
        Marks::read(&Json::parse(&s.to_string()).unwrap())
            .unwrap()
            .unwrap()
    }
    fn env<'a>(aspects: &'a HashMap<String, f64>) -> Env<'a> {
        Env {
            plot_scale: 50.,
            aspects,
            screen: false,
            moment: None,
        }
    }
    #[test]
    fn corner_and_closed_seam_keep_distance_without_dropping_marks() {
        let aspects = HashMap::new();
        let env = env(&aspects);
        let m = marks(0., false, false);
        let pts = [
            Vec2::new(0., 0.),
            Vec2::new(2., 0.),
            Vec2::new(2., 2.),
            Vec2::new(0., 2.),
        ];
        let p = m.placements(&pts, true, 0., Unit::M, &env).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].at, Vec2::new(2., 1.));
        assert_eq!(p[1].at, Vec2::new(0., 1.));
        assert!((p[0].tangent - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((p[1].tangent + std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!(m.placements(&pts, true, f64::NAN, Unit::M, &env).is_err());
        assert!(
            m.placements(
                &[Vec2::new(0., 0.), Vec2::new(1e12, 0.)],
                false,
                0.,
                Unit::M,
                &env
            )
            .is_err()
        );
    }
    #[test]
    fn relative_absolute_and_upright_rotate_about_half_cap_without_scaling_offsets_by_s() {
        let aspects = HashMap::new();
        let env = env(&aspects);
        let style = StrokeStyle {
            color: "ink".into(),
            opacity: 0.5,
            width: 0.1,
            unit: PrimUnit::World,
            dash: None,
            dash_offset: 0.,
            line_type_loss: None,
            cap: "butt".into(),
            join: "miter".into(),
            blur: 0.,
            level: 0.,
            ground: false,
        };
        let path = [Vec2::new(10., 0.), Vec2::new(0., 0.)];
        // Anchor at (7,0), source offset rotated with tangent: (6.5,.25).
        for (absolute, upright, first) in [
            (false, false, [6.5, 0.25]),
            (true, false, [6.5, 0.25]),
            (false, true, [6.5, -1.75]),
        ] {
            let m = marks(0., absolute, upright);
            let p = m.placements(&path, false, 0., Unit::M, &env).unwrap();
            let mut out = PrimitiveList::default();
            p[0].paint(&style, Unit::M, &env, &mut out);
            let all: serde_json::Value = serde_json::from_str(&out.json()).unwrap();
            let value = &all["fills"][0];
            let xy = &value["rings"][0][0];
            for (i, e) in first.into_iter().enumerate() {
                assert!(
                    (xy[["x", "y"][i]].as_f64().unwrap() - e).abs() < 1e-12,
                    "{value}"
                );
            }
            assert_eq!(value["paint"]["opacity"], 0.5);
        }
    }
    #[test]
    fn line_marks_follow_stroke_visibility_and_keep_the_fill_filter_after_drawing() {
        use super::super::batch::BatchSink;
        let paint = FillPaint::Solid {
            color: "ink".into(),
            opacity: 1.,
            level: 0.,
        };
        let rings = vec![vec![
            Vec2::new(0., 0.),
            Vec2::new(2., 0.),
            Vec2::new(0., 2.),
        ]];
        for (fills, strokes, has_ink) in [
            (true, false, true),
            (false, true, false),
            (true, true, false),
        ] {
            let mut sink = BatchSink::new(Vec2::new(0., 0.));
            sink.hide(fills, strokes);
            sink.cad_mark(&paint, &rings);
            if fills {
                sink.fill(&paint, &rings);
            }
            let made = sink.finish();
            assert_eq!(!made.data.is_empty(), has_ink);
            if has_ink {
                assert_eq!(made.data.len(), 6, "ordinary area stayed hidden");
            }
        }
    }
    #[test]
    fn aggregate_glyph_workload_is_reserved_atomically_across_paths() {
        use super::super::batch::BatchSink;
        let mut sink = BatchSink::new(Vec2::new(0., 0.));
        assert!(sink.reserve_cad_points(1_900_000));
        assert!(!sink.reserve_cad_points(200_000));
        assert!(
            sink.reserve_cad_points(100_000),
            "a refused path consumed no budget"
        );
        assert!(!sink.reserve_cad_points(1));
        assert!(sink.reserve_cad_points(0));
    }
}
