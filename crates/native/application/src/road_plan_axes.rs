//! A road project's alignments as a plan map on paper draws them (plan/road.md Y76, docs/adr/0982): the model holds no
//! object for an alignment (the screen draws it as a layer of its own, `alignment_layer.rs`), so a sheet's plan map
//! drew the survey points and nothing of the road. A map now draws a reading copy of the drawing with the alignments
//! added as objects on a layer of their own, sized for the map's scale; the drawing itself is not touched.
//!
//! What is drawn follows KGM's sample plan–profile sheets (KGM 2019 şartname EK-10–13), Büyük Ölçekli Harita
//! Yönetmeliği md. 80 (the axis's points “aliyman üstü noktalar, some noktası, To, TF”) and the Turkish practice
//! (İTÜ, MEGEP *Yol Projesi*, Netcad's codes):
//!
//! - the axis, red (İTÜ: “Geçki ekseni: Kırmızı, kalın”), a parallel axis thin;
//! - a tick across the axis at every hectometre, the full km (`6+000`) along the axis at a round km and the
//!   hectometre's digit elsewhere;
//! - each curve's main points (TO and TF; TS, SC, CS and ST with clothoids) as a line across the axis with the code,
//!   the curve's name and the km to three decimals along it (`TF S1 0+412.118`);
//! - each curve's PI as a dot named S1, S2 … and its curve data outside the curve: the turn (SAĞ, SOL), the
//!   deflection A in grad to five decimals (KGM writes the deflection as A), R, T, L (the circular part), the
//!   clothoids' lengths Lr, and the PI's Y (east) and X (north);
//! - the PI line, thin and dashed, and PROJE BAŞI and PROJE SONU with their km.
//!
//! Sizes are paper millimetres times the map's scale: texts 2.5 mm (the band texts of docs/adr/0980), ticks 3 mm.

use kentos_commands::{
    EntitiesCreate, EntityGeometry, LineType, NewObject, TextAlign, TextFace, Vec2,
};
use kentos_domain::Document;
use kentos_road::alignment::{Kind, km_text};

use crate::road_alignments::{Built, made_alignments};

/// The layer the alignments are drawn on in a map's copy of the drawing.
pub const LAYER: &str = "yol-aks-pafta";
/// Its name, as a map's layer list shows it.
pub const LAYER_NAME: &str = "Aks (pafta)";
const RED: &str = "#d62828";
/// Paper millimetres of the texts and of a tick's half.
const TEXT_MM: f64 = 2.5;
const TICK_MM: f64 = 1.5;
/// A main point's line, each side of the axis.
const MAIN_MM: f64 = 6.0;

/// Whether the drawing has an alignment a map would draw.
pub fn has_axes(doc: &Document) -> bool {
    made_alignments(doc).iter().any(|(_, b)| b.is_ok())
}

/// The drawing as a plan map at 1:`scale` turned `turn` degrees (the map's content turn: a road sheet turns its map
/// so the axis runs left to right) draws it: a reading copy with the alignments on [`LAYER`]; none when it has no
/// alignment. The texts read left to right on that paper.
pub fn with_axes(doc: &Document, scale: f64, turn: f64) -> Option<Document> {
    with_axes_on(doc.reading_copy(), scale, turn)
}

/// [`with_axes`] on a copy already made (a map's copy at its scale).
pub fn with_axes_on(mut copy: Document, scale: f64, turn: f64) -> Option<Document> {
    let objects = objects(&copy, scale, turn);
    if objects.is_empty() {
        return None;
    }
    if copy.layers().get(LAYER).is_none() {
        let layer = kentos_domain::NewLayer {
            id: Some(LAYER.to_owned()),
            ..kentos_domain::NewLayer::layer(LAYER_NAME.to_owned())
        };
        copy.add_layer(layer, None, false).ok()?;
    }
    let create = EntitiesCreate {
        layer_id: LAYER.to_owned(),
        objects,
        operation: None,
        expected_revision: None,
    };
    match crate::create::write_in(&mut crate::ExecutionContext::new(&mut copy), create) {
        kentos_commands::CommandResult::Completed { .. } => Some(copy),
        _ => None,
    }
}

fn object(
    geometry: EntityGeometry,
    color: Option<&str>,
    weight: Option<f64>,
    line_type: Option<LineType>,
) -> NewObject {
    NewObject {
        geometry,
        color: color.map(str::to_owned),
        line_weight: weight,
        line_type,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

fn path(
    pts: &[[f64; 2]],
    color: Option<&str>,
    weight: f64,
    line_type: Option<LineType>,
) -> NewObject {
    object(
        EntityGeometry::Polyline {
            pts: pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
            bulges: None,
            zs: None,
            ms: None,
            parts: None,
            closed: false,
            widths: None,
        },
        color,
        Some(weight),
        line_type,
    )
}

/// A text at `at` turned `degrees` counterclockwise, turned half round when it would read upside down on a paper turned
/// `turn` degrees.
fn text(
    at: [f64; 2],
    words: String,
    height: f64,
    degrees: f64,
    turn: f64,
    align: TextAlign,
    color: Option<&str>,
) -> NewObject {
    let (on_paper, align) = readable(degrees - turn, align);
    let rotation = on_paper + turn;
    object(
        EntityGeometry::Text {
            layout: None,
            annotative: None,
            p: Vec2 { x: at[0], y: at[1] },
            text: words,
            height,
            rotation,
            align: Some(align),
            width_factor: None,
            mask: false,
            box_width: None,
            line_spacing: None,
            runs: Vec::new(),
            face: TextFace::default(),
            path: None,
        },
        color,
        None,
        None,
    )
}

/// A direction in degrees read left to right: in (−90°, 90°]; a text turned half round is anchored at its other end.
pub fn readable(degrees: f64, align: TextAlign) -> (f64, TextAlign) {
    let mut d = degrees.rem_euclid(360.0);
    if d > 180.0 {
        d -= 360.0;
    }
    if d > 90.0 + 1e-9 || d <= -90.0 + 1e-9 {
        // Half round: left and right swap, top and bottom too.
        let flipped = match align {
            TextAlign::MiddleLeft => TextAlign::MiddleRight,
            TextAlign::MiddleRight => TextAlign::MiddleLeft,
            TextAlign::BottomLeft => TextAlign::TopRight,
            TextAlign::BottomRight => TextAlign::TopLeft,
            TextAlign::TopLeft => TextAlign::BottomRight,
            TextAlign::TopRight => TextAlign::BottomLeft,
            TextAlign::BottomCenter => TextAlign::TopCenter,
            TextAlign::TopCenter => TextAlign::BottomCenter,
            other => other,
        };
        let d = if d > 0.0 { d - 180.0 } else { d + 180.0 };
        (d, flipped)
    } else {
        (d, align)
    }
}

/// The deflection in grad, as KGM writes it (five decimals): |Δ|·200/π.
pub fn grad(delta: f64) -> f64 {
    delta.abs() * 200.0 / std::f64::consts::PI
}

/// The main points' codes: TO and TF for a circular curve (BÖHHBÜY md. 80, Netcad), TS, SC, CS, ST with clothoids.
pub fn main_codes(spiral_in: f64, spiral_out: f64) -> [&'static str; 4] {
    if spiral_in > 0.0 || spiral_out > 0.0 {
        ["TS", "SC", "CS", "ST"]
    } else {
        ["TO", "TO", "TF", "TF"]
    }
}

fn n2(v: f64) -> String {
    format!("{v:.2}")
}

/// The objects every alignment of the drawing draws at 1:`scale` on a paper turned `turn` degrees.
pub fn objects(doc: &Document, scale: f64, turn: f64) -> Vec<NewObject> {
    let m = scale.max(1.0) / 1000.0;
    let h = TEXT_MM * m;
    let mut out = Vec::new();
    for (def, built) in made_alignments(doc) {
        let Ok(b) = built else { continue };
        let parallel = def.offset_of.is_some();
        axis(&mut out, &b, parallel, m);
        if parallel {
            continue;
        }
        if def.pis.len() > 1 {
            let pis: Vec<[f64; 2]> = def.pis.iter().map(|p| [p.x, p.y]).collect();
            out.push(path(&pis, None, 0.13, Some(LineType::Dashed)));
        }
        hectometres(&mut out, &b, m, h, turn);
        curves(&mut out, &def, &b, m, h, turn);
        ends(&mut out, &b, m, h, turn);
    }
    out
}

fn axis(out: &mut Vec<NewObject>, b: &Built, parallel: bool, _m: f64) {
    let weight = if parallel { 0.25 } else { 0.5 };
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for e in &b.core.elements {
        let turn = e.k0.abs().max(e.k1.abs()) * e.length;
        let n = if e.kind == Kind::Tangent {
            1
        } else {
            ((turn / 0.01).ceil() as usize).clamp(4, 2000)
        };
        for i in 0..=n {
            if let Some(p) = b.place(e.station + e.length * i as f64 / n as f64) {
                let q = [p.x, p.y];
                if pts
                    .last()
                    .is_none_or(|l| (l[0] - q[0]).hypot(l[1] - q[1]) > 1e-9)
                {
                    pts.push(q);
                }
            }
        }
    }
    if pts.len() > 1 {
        out.push(path(&pts, Some(RED), weight, None));
    }
}

/// The tangent and the left normal of a direction θ (counterclockwise from the x axis): (cos θ, sin θ), (−sin θ, cos θ).
fn frame(dir: f64) -> ([f64; 2], [f64; 2]) {
    ([dir.cos(), dir.sin()], [-dir.sin(), dir.cos()])
}

fn hectometres(out: &mut Vec<NewObject>, b: &Built, m: f64, h: f64, turn: f64) {
    let step = 100.0;
    for r in &b.regions {
        let (a, z) = (r.ahead, r.ahead + r.dir * (r.until - r.at));
        let (lo, hi) = (a.min(z), a.max(z));
        let mut k = (lo / step - 1e-9).ceil() * step;
        while k <= hi + 1e-9 {
            let s = r.at + r.dir * (k - r.ahead);
            if (r.at - 1e-9..=r.until + 1e-9).contains(&s)
                && let Some(p) = b.place(s.clamp(0.0, b.length()))
            {
                let (_, n) = frame(p.direction);
                let c = [p.x, p.y];
                let half = TICK_MM * m;
                out.push(path(
                    &[
                        [c[0] - n[0] * half, c[1] - n[1] * half],
                        [c[0] + n[0] * half, c[1] + n[1] * half],
                    ],
                    None,
                    0.25,
                    None,
                ));
                let round = (k / 1000.0).round() * 1000.0;
                let at = [
                    c[0] + n[0] * (half + 0.8 * h),
                    c[1] + n[1] * (half + 0.8 * h),
                ];
                let degrees = p.direction.to_degrees();
                if (k - round).abs() < 1e-6 {
                    // The full km along the axis, centred on its tick.
                    out.push(text(
                        at,
                        km_text(k, 0),
                        h,
                        degrees,
                        turn,
                        TextAlign::MiddleCenter,
                        None,
                    ));
                } else {
                    let digit = ((k.rem_euclid(1000.0)) / 100.0).round() as i64 % 10;
                    out.push(text(
                        at,
                        digit.to_string(),
                        0.8 * h,
                        degrees,
                        turn,
                        TextAlign::MiddleCenter,
                        None,
                    ));
                }
            }
            k += step;
        }
    }
}

fn curves(
    out: &mut Vec<NewObject>,
    def: &kentos_commands::AlignmentDef,
    b: &Built,
    m: f64,
    h: f64,
    turn: f64,
) {
    let mut named = 0usize;
    for c in &b.core.curves {
        if c.radius <= 0.0 || !c.radius.is_finite() {
            continue;
        }
        named += 1;
        let name = format!("S{named}");
        let codes = main_codes(c.spiral_in, c.spiral_out);
        let mut seen: Vec<f64> = Vec::new();
        for (i, s) in c.stations.iter().enumerate() {
            if seen.iter().any(|q| (q - s).abs() < 1e-6) {
                continue;
            }
            seen.push(*s);
            let Some(p) = b.place(*s) else { continue };
            let (_, n) = frame(p.direction);
            let len = MAIN_MM * m;
            let c0 = [p.x - n[0] * len, p.y - n[1] * len];
            let c1 = [p.x + n[0] * len, p.y + n[1] * len];
            out.push(path(&[c0, c1], Some(RED), 0.25, None));
            // Along the line, beyond its left end.
            let at = [p.x + n[0] * (len + 0.5 * h), p.y + n[1] * (len + 0.5 * h)];
            let words = format!("{} {name} {}", codes[i], km_text(p.km, 3));
            out.push(text(
                at,
                words,
                h,
                (p.direction + std::f64::consts::FRAC_PI_2).to_degrees(),
                turn,
                TextAlign::MiddleLeft,
                Some(RED),
            ));
        }
        // The PI and its curve data, outside the curve (away from the centre).
        let Some(pi) = def.pis.iter().find(|p| p.id == c.pi) else {
            continue;
        };
        let pt = [pi.x, pi.y];
        out.push(object(
            EntityGeometry::Circle {
                c: Vec2 { x: pt[0], y: pt[1] },
                r: 0.75 * m,
            },
            None,
            Some(0.25),
            None,
        ));
        let away = [pt[0] - c.centre[0], pt[1] - c.centre[1]];
        let len = away[0].hypot(away[1]);
        let u = if len > 1e-9 {
            [away[0] / len, away[1] / len]
        } else {
            [0.0, 1.0]
        };
        let side = if c.delta > 0.0 { "SOL" } else { "SAĞ" };
        let mut lines = vec![
            format!("{name} ({side})"),
            format!("A={:.5}", grad(c.delta)),
            format!("R={}", n2(c.radius)),
            if (c.t_in - c.t_out).abs() < 5e-4 {
                format!("T={}", n2(c.t_in))
            } else {
                format!("T={}-{}", n2(c.t_in), n2(c.t_out))
            },
            format!("L={}", n2(c.arc)),
        ];
        if c.spiral_in > 0.0 || c.spiral_out > 0.0 {
            lines.push(format!("Lr={}-{}", n2(c.spiral_in), n2(c.spiral_out)));
        }
        lines.push(format!("Y={:.3}", pi.x));
        lines.push(format!("X={:.3}", pi.y));
        // Upright on the paper, 4 mm beyond the PI away from the curve: above it when the outside is up (the block
        // grows up from there), below it otherwise; left aligned when the outside is to the right.
        let r = turn.to_radians();
        let (ex, ey) = ([r.cos(), r.sin()], [-r.sin(), r.cos()]);
        let corner = [pt[0] + u[0] * 4.0 * m, pt[1] + u[1] * 4.0 * m];
        let pitch = 1.6 * h;
        let up = u[0] * ey[0] + u[1] * ey[1] >= 0.0;
        let first = if up {
            pitch * (lines.len() as f64 - 1.0)
        } else {
            -h
        };
        let align = if u[0] * ex[0] + u[1] * ex[1] >= 0.0 {
            TextAlign::MiddleLeft
        } else {
            TextAlign::MiddleRight
        };
        for (i, l) in lines.into_iter().enumerate() {
            let y = first - pitch * i as f64;
            let at = [corner[0] + ey[0] * y, corner[1] + ey[1] * y];
            out.push(text(
                at,
                l,
                h,
                turn,
                turn,
                align,
                if i == 0 { Some(RED) } else { None },
            ));
        }
        let name_at = [pt[0] - u[0] * 2.0 * m, pt[1] - u[1] * 2.0 * m];
        out.push(text(
            name_at,
            name,
            h,
            turn,
            turn,
            TextAlign::MiddleCenter,
            Some(RED),
        ));
    }
}

fn ends(out: &mut Vec<NewObject>, b: &Built, m: f64, h: f64, turn: f64) {
    for (s, words) in [(0.0, "PROJE BAŞI"), (b.length(), "PROJE SONU")] {
        let Some(p) = b.place(s) else { continue };
        let (_, n) = frame(p.direction);
        let len = MAIN_MM * m;
        out.push(path(
            &[
                [p.x - n[0] * len, p.y - n[1] * len],
                [p.x + n[0] * len, p.y + n[1] * len],
            ],
            None,
            0.35,
            None,
        ));
        let at = [p.x - n[0] * (len + 0.5 * h), p.y - n[1] * (len + 0.5 * h)];
        out.push(text(
            at,
            format!("{words} KM: {}", km_text(p.km, 3)),
            h,
            (p.direction - std::f64::consts::FRAC_PI_2).to_degrees(),
            turn,
            TextAlign::MiddleLeft,
            None,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_read_left_to_right() {
        assert_eq!(
            readable(30.0, TextAlign::MiddleLeft),
            (30.0, TextAlign::MiddleLeft)
        );
        assert_eq!(
            readable(90.0, TextAlign::MiddleLeft),
            (90.0, TextAlign::MiddleLeft)
        );
        assert_eq!(
            readable(-90.0, TextAlign::MiddleLeft),
            (90.0, TextAlign::MiddleRight)
        );
        assert_eq!(
            readable(150.0, TextAlign::MiddleLeft),
            (-30.0, TextAlign::MiddleRight)
        );
        assert_eq!(
            readable(-150.0, TextAlign::MiddleRight),
            (30.0, TextAlign::MiddleLeft)
        );
        assert_eq!(
            readable(200.0, TextAlign::MiddleCenter),
            (20.0, TextAlign::MiddleCenter)
        );
    }

    #[test]
    fn deflection_in_grad() {
        // A right angle is 100 grad; KGM's EK-11 writes 36.56916 for 0.574420… rad.
        assert!((grad(std::f64::consts::FRAC_PI_2) - 100.0).abs() < 1e-12);
        assert!((grad(-std::f64::consts::PI) - 200.0).abs() < 1e-12);
    }
}
