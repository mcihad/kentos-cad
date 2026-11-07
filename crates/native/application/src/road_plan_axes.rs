//! A road project's alignments as a plan map on paper draws them (plan/road.md Y76, docs/adr/0982): the model holds no
//! object for an alignment (the screen draws it as a layer of its own, `alignment_layer.rs`), so a sheet's plan map
//! drew the survey points and nothing of the road. A map now draws a reading copy of the drawing with the alignments
//! added as objects on a layer of their own, sized for the map's scale; the drawing itself is not touched.
//!
//! What is drawn follows KGM's sample sheets of June 2024 (Etüt, Proje ve Çevre Dairesi Başkanlığı, *Örnek
//! Paftalar*, the newest the KGM publishes; docs/adr/0984), the clothoid curves of the 2019 şartname's EK-10–13,
//! Büyük Ölçekli Harita Yönetmeliği md. 80 (the axis's points “aliyman üstü noktalar, some noktası, To, TF”) and the
//! Turkish practice (İTÜ, MEGEP *Yol Projesi*, Netcad's codes):
//!
//! - the axis, red and dash-dot as the 2024 sheets draw it, a parallel axis thin;
//! - a tick across the axis at every hectometre, the full km (`6+000`) along the axis at a round km and the
//!   hectometre's digit elsewhere;
//! - each curve's main points (TO and TF; TS, SC, CS and ST with clothoids) as a line across the axis with the code,
//!   the curve's number and the km to two decimals along it (`TO1=0+100.42`);
//! - each curve's PI as a triangle with its name boxed (`S - 1`), and its curve data in a framed box outside the
//!   curve (see [`curve_data`]);
//! - the PI line, thin and dashed, and PROJE BAŞI and PROJE SONU with their km and coordinates.
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
/// The widest a character of the drawing font is, in text heights (Barlow's figures measured 0.465 h on the PDF).
const WIDE: f64 = 0.5;

/// The part of the drawing a map shows (docs/adr/0984): its centre on the ground and half its width and height along
/// the paper as the map's content is turned, in metres. A curve's data box is kept inside it: below a PI near the map's
/// edge the box ran off the map and the map cut its last lines (Y77's and Y78's first pictures lost Y and S-X, S-Y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Room {
    pub center: [f64; 2],
    pub half: [f64; 2],
}

impl Room {
    /// A map's room: its centre and its content rectangle (paper micrometres) at 1:`scale`.
    pub fn of_map(center: [f64; 2], width_um: i32, height_um: i32, scale: f64) -> Room {
        let ground = |um: i32| f64::from(um) / 1e6 * scale;
        Room { center, half: [0.5 * ground(width_um), 0.5 * ground(height_um)] }
    }

    /// Whether the map shows the ground point `p`, its content turned `turn` degrees.
    pub fn holds(&self, p: [f64; 2], turn: f64) -> bool {
        let r = turn.to_radians();
        let d = [p[0] - self.center[0], p[1] - self.center[1]];
        (d[0] * r.cos() + d[1] * r.sin()).abs() <= self.half[0] && (-d[0] * r.sin() + d[1] * r.cos()).abs() <= self.half[1]
    }
}

/// Whether the drawing has an alignment a map would draw.
pub fn has_axes(doc: &Document) -> bool {
    made_alignments(doc).iter().any(|(_, b)| b.is_ok())
}

/// The drawing as a plan map at 1:`scale` turned `turn` degrees (the map's content turn: a road sheet turns its map
/// so the axis runs left to right) draws it: a reading copy with the alignments on [`LAYER`]; none when it has no
/// alignment. The texts read left to right on that paper.
pub fn with_axes(doc: &Document, scale: f64, turn: f64) -> Option<Document> {
    with_axes_on(doc.reading_copy(), scale, turn, None)
}

/// [`with_axes`] on a copy already made (a map's copy at its scale), the curves' data kept inside `room` when given.
pub fn with_axes_on(mut copy: Document, scale: f64, turn: f64, room: Option<Room>) -> Option<Document> {
    let objects = objects(&copy, scale, turn, room);
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

/// A coordinate as KGM's 2024 sheets write it: four decimals, the whole metres in groups of three (`4 349 283.7346`).
pub fn grouped(v: f64) -> String {
    let s = format!("{:.4}", v.abs());
    let (whole, part) = s.split_once('.').unwrap_or((&s, ""));
    let mut out = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    format!("{}{out}.{part}", if v < 0.0 { "-" } else { "" })
}

/// The superelevation schedule a plan map reads a curve's S, Gn and Lr, Lt, La from: the alignment's only schedule,
/// built, with its normal slopes. None without one, with more than one (the map cannot tell which the sheet means;
/// docs/adr/0983 §Açık) or when it does not build.
fn schedule_for(
    doc: &Document,
    alignment: &str,
    b: &Built,
) -> Option<(kentos_road::superelevation::Schedule, kentos_road::superelevation::Crossfall)> {
    let mut on = doc.road_superelevations().iter().filter(|d| d.alignment == alignment);
    let def = on.next()?;
    if on.next().is_some() {
        return None;
    }
    let (s, _) = crate::road_superelevations::schedule(def, b).ok()?;
    Some((s, crate::road_superelevations::normal(def)))
}

/// A curve's PI data as KGM's June 2024 sample sheets write it (docs/adr/0984; KGM 2019 şartname s. 5):
///
/// ```text
/// S - 1 (Sol)
/// V = 80               the curve's design speed, km/h: its PI's, else the alignment's (docs/adr/0985)
/// D = 19.0937          the deflection in grad
/// R = 1500.00
/// L = 449.88           R·D
/// T = 226.64           R·tan(D/2)
/// Se = % 2.20          from the schedule: the slope toward the centre, negative in normal crown
/// Lr = 22.00 - 22.00   in and out
/// Lt = 20.00 - 20.00
/// La = 36.42 - 36.70
/// Gn = 0.00
/// X = 4 349 283.7346   north
/// Y = 542 198.1906     east
/// ```
///
/// With clothoids D, L and T are the circular part's and X, Y the point where the tangents at SC and CS meet, then
/// `S-D` the whole deflection and `S-X`, `S-Y` the PI: KGM's 2019 EK-10 writes `A=27.45958` and `S-A=35.60832` for
/// R 600 and clothoids of 76.8 m (35.60832 − 27.45958 = 76.8/600 rad), EK-13 the same; the 2024 sheets name the
/// deflection D, so the whole one is `S-D` here (docs/adr/0984: the name is ours, the values KGM's). V is written when
/// the curve has a design speed. `inner` is the SC–CS tangents' meeting point.
pub fn curve_data(
    number: usize,
    c: &kentos_road::alignment::CurveReport,
    speed: Option<f64>,
    pi: [f64; 2],
    inner: Option<[f64; 2]>,
    schedule: Option<&(kentos_road::superelevation::Schedule, kentos_road::superelevation::Crossfall)>,
) -> Vec<String> {
    let side = if c.delta > 0.0 { "Sol" } else { "Sağ" };
    let clothoid = c.spiral_in > 0.0 || c.spiral_out > 0.0;
    let dc = if clothoid { (c.arc / c.radius).max(0.0) } else { c.delta.abs() };
    let mut lines = vec![format!("S - {number} ({side})")];
    if let Some(v) = speed {
        lines.push(format!("V = {}", kmh(v)));
    }
    lines.extend([
        format!("D = {:.4}", grad(dc)),
        format!("R = {}", n2(c.radius)),
        format!("L = {}", n2(c.radius * dc)),
        format!("T = {}", n2(c.radius * (0.5 * dc).tan())),
    ]);
    if let Some((s, normal)) = schedule {
        let got = kentos_road::superelevation_pi::curve_super(s, *normal, c);
        lines.push(format!("Se = % {}", n2(got.rate * 100.0)));
        if let (Some(a), Some(b)) = (got.entry, got.exit) {
            lines.push(format!("Lr = {} - {}", n2(a.runoff), n2(b.runoff)));
            lines.push(format!("Lt = {} - {}", n2(a.runout), n2(b.runout)));
            lines.push(format!("La = {} - {}", n2(a.on_tangent), n2(b.on_tangent)));
        }
        lines.push(format!("Gn = {}", n2(got.widening)));
    }
    let at = if clothoid { inner.unwrap_or(pi) } else { pi };
    lines.push(format!("X = {}", grouped(at[1])));
    lines.push(format!("Y = {}", grouped(at[0])));
    if clothoid {
        lines.push(format!("S-D = {:.4}", grad(c.delta)));
        lines.push(format!("S-X = {}", grouped(pi[1])));
        lines.push(format!("S-Y = {}", grouped(pi[0])));
    }
    lines
}

/// A speed as KGM writes it: whole km/h without decimals (`V = 80`), else to one decimal.
fn kmh(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 { format!("{v:.0}") } else { format!("{v:.1}") }
}

/// Where the lines through `a` and `b` in directions `ta` and `tb` (radians) meet; none when they are parallel.
fn meet(a: [f64; 2], ta: f64, b: [f64; 2], tb: f64) -> Option<[f64; 2]> {
    let (da, db) = ([ta.cos(), ta.sin()], [tb.cos(), tb.sin()]);
    let cross = da[0] * db[1] - da[1] * db[0];
    if cross.abs() < 1e-12 {
        return None;
    }
    let t = ((b[0] - a[0]) * db[1] - (b[1] - a[1]) * db[0]) / cross;
    Some([a[0] + t * da[0], a[1] + t * da[1]])
}

/// The objects every alignment of the drawing draws at 1:`scale` on a paper turned `turn` degrees, the curves' data
/// inside `room`.
pub fn objects(doc: &Document, scale: f64, turn: f64, room: Option<Room>) -> Vec<NewObject> {
    let m = scale.max(1.0) / 1000.0;
    let h = TEXT_MM * m;
    let mut out = Vec::new();
    let made = made_alignments(doc);
    // Every axis sampled every half text height: a curve's data box keeps clear of them.
    let mut roads: Vec<[f64; 2]> = Vec::new();
    for b in made.iter().filter_map(|(_, b)| b.as_ref().ok()) {
        let n = ((b.length() / (0.5 * h)).ceil() as usize).clamp(1, 200_000);
        roads.extend((0..=n).filter_map(|i| b.place(b.length() * i as f64 / n as f64)).map(|p| [p.x, p.y]));
        // The main points' lines and their texts (`TS1=1+194.71`, 13 characters or so) stand out from the axis.
        for c in b.core.curves.iter().filter(|c| c.radius > 0.0 && c.radius.is_finite()) {
            for s in c.stations {
                let Some(p) = b.place(s) else { continue };
                let (_, nrm) = frame(p.direction);
                let reach = MAIN_MM * m + 0.5 * h + 13.0 * WIDE * h;
                let k = (2.0 * reach / (0.5 * h)).ceil() as usize;
                roads.extend((0..=k).map(|i| {
                    let d = -MAIN_MM * m + (reach + MAIN_MM * m) * i as f64 / k as f64;
                    [p.x + nrm[0] * d, p.y + nrm[1] * d]
                }));
            }
        }
    }
    for (def, built) in made {
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
        let schedule = schedule_for(doc, &def.id, &b);
        curves(&mut out, &def, &b, schedule.as_ref(), room, &roads, Paper { m, h, turn });
        ends(&mut out, &b, m, h, turn, room);
        if let Some(room) = room {
            places(&mut out, &def, &b, m, h, turn, room);
        }
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
        // KGM's 2024 sheets draw the axis red and dash-dot.
        out.push(path(&pts, Some(RED), weight, Some(LineType::Dashdot)));
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

/// The paper a map draws on: metres a paper millimetre stands for, the text height (m) and the content's turn (°).
#[derive(Clone, Copy)]
struct Paper {
    m: f64,
    h: f64,
    turn: f64,
}

fn curves(
    out: &mut Vec<NewObject>,
    def: &kentos_commands::AlignmentDef,
    b: &Built,
    schedule: Option<&(kentos_road::superelevation::Schedule, kentos_road::superelevation::Crossfall)>,
    room: Option<Room>,
    roads: &[[f64; 2]],
    paper: Paper,
) {
    let Paper { m, h, turn } = paper;
    let mut named = 0usize;
    for c in &b.core.curves {
        if c.radius <= 0.0 || !c.radius.is_finite() {
            continue;
        }
        named += 1;
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
            out.push(path(&[c0, c1], None, 0.25, None));
            // Along the line, beyond its left end: `TO1=0+100.42` (KGM 2024).
            let at = [p.x + n[0] * (len + 0.5 * h), p.y + n[1] * (len + 0.5 * h)];
            let words = format!("{}{named}={}", codes[i], km_text(p.km, 2));
            out.push(text(
                at,
                words,
                h,
                (p.direction + std::f64::consts::FRAC_PI_2).to_degrees(),
                turn,
                TextAlign::MiddleLeft,
                None,
            ));
        }
        // The PI and its curve data, outside the curve (away from the centre).
        let Some(pi) = def.pis.iter().find(|p| p.id == c.pi) else {
            continue;
        };
        let pt = [pi.x, pi.y];
        let away = [pt[0] - c.centre[0], pt[1] - c.centre[1]];
        let len = away[0].hypot(away[1]);
        let u = if len > 1e-9 {
            [away[0] / len, away[1] / len]
        } else {
            [0.0, 1.0]
        };
        let inner = match (b.place(c.stations[1]), b.place(c.stations[2])) {
            (Some(a), Some(z)) => meet([a.x, a.y], a.direction, [z.x, z.y], z.direction),
            _ => None,
        };
        let lines = curve_data(named, c, pi.speed.or(def.design_speed), pt, inner, schedule);
        let r = turn.to_radians();
        let (ex, ey) = ([r.cos(), r.sin()], [-r.sin(), r.cos()]);
        // Upright on the paper: a point `x` along and `y` up from `o`.
        let on = |o: [f64; 2], x: f64, y: f64| [o[0] + ex[0] * x + ey[0] * y, o[1] + ex[1] * x + ey[1] * y];
        // The PI as the 2024 sheets mark it: a triangle, 2.5 mm a side, and its name boxed on the curve's side.
        let side = 2.5 * m;
        let rise = side * 0.75_f64.sqrt();
        out.push(path(
            &[
                on(pt, 0.0, 2.0 * rise / 3.0),
                on(pt, -0.5 * side, -rise / 3.0),
                on(pt, 0.5 * side, -rise / 3.0),
                on(pt, 0.0, 2.0 * rise / 3.0),
            ],
            None,
            0.25,
            None,
        ));
        // The name beside the triangle, on the side the data box does not take: between the PI and the curve it sat on
        // the parallel axis and the PI lines (Y78's first picture).
        let name = format!("S - {named}");
        let half = [0.5 * (name.chars().count() as f64 * WIDE + 0.6) * h, 0.7 * h];
        let beside = if u[0] * ex[0] + u[1] * ex[1] >= 0.0 { -1.0 } else { 1.0 };
        let name_at = on(pt, beside * (0.5 * side + 1.0 * m + half[0]), 0.0);
        out.push(path(
            &[
                on(name_at, -half[0], -half[1]),
                on(name_at, half[0], -half[1]),
                on(name_at, half[0], half[1]),
                on(name_at, -half[0], half[1]),
                on(name_at, -half[0], -half[1]),
            ],
            None,
            0.18,
            None,
        ));
        out.push(text(name_at, name, h, turn, turn, TextAlign::MiddleCenter, None));
        // The data box, 4 mm beyond the PI away from the curve: above it when the outside is up, below it otherwise;
        // to the right when the outside is to the right. KGM 2024's frame: a box, a double rule along the left, a rule
        // under the name and one over X, Y. A line is at most WIDE h a character (Barlow's figures measured 0.465 h).
        let corner = [pt[0] + u[0] * 4.0 * m, pt[1] + u[1] * 4.0 * m];
        // Only a map that shows the PI has its box: another map of the drawing (the profile's, on the same sheet)
        // pulled a box it does not show into its own frame (Y78's picture); left by its PI, the box still hung into
        // the profile map's rows under the plan (Y85's picture, docs/adr/0992). KGM's sheets: the box is on the sheet
        // that holds its PI.
        if room.is_some_and(|r| !r.holds(pt, turn)) {
            continue;
        }
        let pitch = 1.6 * h;
        let height = pitch * lines.len() as f64 + 0.4 * h;
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f64 * WIDE * h + 2.2 * h;
        let mut left = if u[0] * ex[0] + u[1] * ex[1] >= 0.0 { 0.0 } else { -width };
        let up = u[0] * ey[0] + u[1] * ey[1] >= 0.0;
        let mut bottom = if up { 0.0 } else { -height };
        // Where it goes (docs/adr/0984): outside the PI as above when that is inside the map and 6 mm clear of every
        // axis and main point's line and text; else across the road, 6 mm beyond the axes over the box's width; else moved inside the map, 2 mm from
        // its edge (a box taller or wider than the map keeps its top and left). Below a PI near the map's edge the box
        // ran off the map; moved straight in, it sat on the road (Y78's pictures).
        let paper = |p: &[f64; 2]| {
            let d = [p[0] - corner[0], p[1] - corner[1]];
            [d[0] * ex[0] + d[1] * ex[1], d[0] * ey[0] + d[1] * ey[1]]
        };
        let near: Vec<[f64; 2]> = roads.iter().map(paper).collect();
        let clear = 6.0 * m;
        let free = |l: f64, b: f64| {
            !near.iter().any(|q| q[0] > l - clear && q[0] < l + width + clear && q[1] > b - clear && q[1] < b + height + clear)
        };
        let fits = |l: f64, b: f64| {
            room.is_none_or(|r| {
                let d = [corner[0] - r.center[0], corner[1] - r.center[1]];
                let c = [d[0] * ex[0] + d[1] * ex[1], d[0] * ey[0] + d[1] * ey[1]];
                let (lo, hi) = ([-r.half[0] + 2.0 * m, -r.half[1] + 2.0 * m], [r.half[0] - 2.0 * m, r.half[1] - 2.0 * m]);
                c[0] + l >= lo[0] && c[0] + l + width <= hi[0] && c[1] + b >= lo[1] && c[1] + b + height <= hi[1]
            })
        };
        // Across this curve's own axis first (another axis crossing the box's width far off must not set it), then
        // past any axis the box still meets, a few times.
        let beyond = |pts: &mut dyn Iterator<Item = [f64; 2]>, l: f64| {
            let ys = pts.filter(|q| q[0] > l - clear && q[0] < l + width + clear).map(|q| q[1]);
            if up {
                ys.fold(f64::INFINITY, f64::min) - clear - height
            } else {
                ys.fold(f64::NEG_INFINITY, f64::max) + clear
            }
        };
        let step = 0.5 * h;
        let n = ((b.length() / step).ceil() as usize).clamp(1, 200_000);
        let mut own = (0..=n).filter_map(|i| b.place(b.length() * i as f64 / n as f64)).map(|p| paper(&[p.x, p.y]));
        let mut across = beyond(&mut own, left);
        for _ in 0..6 {
            if !across.is_finite() || free(left, across) {
                break;
            }
            let mut met = near.iter().copied().filter(|q| q[1] > across - clear && q[1] < across + height + clear);
            across = beyond(&mut met, left);
        }
        let placed = if fits(left, bottom) && free(left, bottom) {
            true
        } else if across.is_finite() && fits(left, across) && free(left, across) {
            bottom = across;
            true
        } else {
            false
        };
        if let (false, Some(room)) = (placed, room) {
            let d = [corner[0] - room.center[0], corner[1] - room.center[1]];
            let c = [d[0] * ex[0] + d[1] * ex[1], d[0] * ey[0] + d[1] * ey[1]];
            let keep = |from: f64, size: f64, half: f64, high_first: bool| {
                let (lo, hi) = (-half + 2.0 * m, half - 2.0 * m);
                if size > hi - lo {
                    if high_first { hi - size - from } else { lo - from }
                } else if from < lo {
                    lo - from
                } else if from + size > hi {
                    hi - (from + size)
                } else {
                    0.0
                }
            };
            left += keep(c[0] + left, width, room.half[0], false);
            bottom += keep(c[1] + bottom, height, room.half[1], true);
        }
        let top = bottom + height;
        let at = |x: f64, y: f64| on(corner, x, y);
        let ix = lines.iter().position(|l| l.starts_with("X ="));
        for (i, l) in lines.into_iter().enumerate() {
            let y = top - 0.2 * h - pitch * (i as f64 + 0.5);
            let mut line = text(
                at(left + 1.2 * h, y),
                l,
                h,
                turn,
                turn,
                TextAlign::MiddleLeft,
                if i == 0 { Some(RED) } else { None },
            );
            // Masked: a survey point under the box read into its text ("S○1", Y79's picture). The mask is the text's
            // own: a mask the box's width (a paragraph's `box_width`) covered the rules too, as a map draws texts over
            // lines whatever their order. A point beside a short line still shows (docs/adr/0985 §Açık).
            if let EntityGeometry::Text { mask, .. } = &mut line.geometry {
                *mask = true;
            }
            out.push(line);
        }
        out.push(path(
            &[at(left, bottom), at(left + width, bottom), at(left + width, top), at(left, top), at(left, bottom)],
            None,
            0.25,
            None,
        ));
        out.push(path(&[at(left + 0.45 * h, bottom), at(left + 0.45 * h, top)], None, 0.18, None));
        out.push(path(&[at(left + 0.75 * h, bottom), at(left + 0.75 * h, top)], None, 0.5, None));
        let rule = |y: f64| path(&[at(left, y), at(left + width, y)], None, 0.35, None);
        out.push(rule(top - 0.2 * h - pitch));
        if let Some(ix) = ix {
            out.push(rule(top - 0.2 * h - pitch * ix as f64));
        }
    }
}

/// The places the road leads to beyond its ends (docs/adr/1028; KGM's June 2024 sample sheets: "← KAMAN" and
/// "ANKARA-NİĞDE OTOYOLU →" in red at the plan's ends, on every sheet): at the first and the last of the alignment the map
/// shows, 4 mm in from its edge, a red arrow 15 mm along the road pointing out, 4 mm beside it, and the name over it.
fn places(out: &mut Vec<NewObject>, def: &kentos_commands::AlignmentDef, b: &Built, m: f64, h: f64, turn: f64, room: Room) {
    if def.start_place.is_none() && def.end_place.is_none() {
        return;
    }
    let inset = Room { center: room.center, half: [room.half[0] - 4.0 * m, room.half[1] - 4.0 * m] };
    if inset.half[0] <= 0.0 || inset.half[1] <= 0.0 {
        return;
    }
    let n = ((b.length() / (0.5 * h)).ceil() as usize).clamp(2, 200_000);
    let shown: Vec<f64> = (0..=n)
        .map(|i| b.length() * i as f64 / n as f64)
        .filter(|s| b.place(*s).is_some_and(|p| inset.holds([p.x, p.y], turn)))
        .collect();
    let (Some(&first), Some(&last)) = (shown.first(), shown.last()) else { return };
    let (len, head, beside) = (15.0 * m, 3.0 * m, 4.0 * m);
    for (place, s, out_dir) in [(&def.start_place, first, -1.0), (&def.end_place, last, 1.0)] {
        let Some(name) = place else { continue };
        let Some(p) = b.place(s) else { continue };
        let (t, nrm) = frame(p.direction);
        // Pointing out of the map along the road: back at the start, ahead at the end.
        let dir = [t[0] * out_dir, t[1] * out_dir];
        let base = [p.x + nrm[0] * beside, p.y + nrm[1] * beside];
        let tip = base;
        let tail = [tip[0] - dir[0] * len, tip[1] - dir[1] * len];
        let neck = [tip[0] - dir[0] * head, tip[1] - dir[1] * head];
        out.push(path(&[tail, neck], Some(RED), 0.35, None));
        let wing = 0.4 * head;
        let arrow = [
            [tip[0], tip[1]],
            [neck[0] + nrm[0] * wing, neck[1] + nrm[1] * wing],
            [neck[0] - nrm[0] * wing, neck[1] - nrm[1] * wing],
            [tip[0], tip[1]],
        ];
        out.push(object(
            EntityGeometry::Hatch {
                ring: arrow.iter().map(|q| Vec2 { x: q[0], y: q[1] }).collect(),
                holes: None,
                pattern: kentos_commands::HatchPattern::user(kentos_commands::HatchPatternType::Solid, 0.0, 1.0),
                assoc: None,
                boundary: None,
                annotative: None,
            },
            Some(RED),
            None,
            None,
        ));
        let mid = [0.5 * (tail[0] + tip[0]) + nrm[0] * 0.9 * h, 0.5 * (tail[1] + tip[1]) + nrm[1] * 0.9 * h];
        let degrees = p.direction.to_degrees();
        let mut words = text(mid, name.clone(), h, degrees, turn, TextAlign::MiddleCenter, Some(RED));
        if let EntityGeometry::Text { mask, .. } = &mut words.geometry {
            *mask = true;
        }
        out.push(words);
    }
}

fn ends(out: &mut Vec<NewObject>, b: &Built, m: f64, h: f64, turn: f64, room: Option<Room>) {
    for (s, words) in [(0.0, "PROJE BAŞI"), (b.length(), "PROJE SONU")] {
        let Some(p) = b.place(s) else { continue };
        // Only a map that shows the end has its words (docs/adr/0992): beside an end off the map they ran into the
        // profile map under the plan (Y85's picture).
        if room.is_some_and(|r| !r.holds([p.x, p.y], turn)) {
            continue;
        }
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
        // KGM 2024: the words, `KM : 0+000.00` and the point's X and Y (the 2019 şartname: “Proje başı ve sonunda kot
        // ve koordinat değerleri yazılacaktır”; the elevation is the profile's). The lines run down the paper as the
        // reader sees them, whichever way the text was turned to read.
        let degrees = (p.direction - std::f64::consts::FRAC_PI_2).to_degrees();
        let (on_paper, _) = readable(degrees - turn, TextAlign::MiddleLeft);
        let down = (on_paper + turn - 90.0).to_radians();
        let at = [p.x - n[0] * (len + 0.5 * h), p.y - n[1] * (len + 0.5 * h)];
        let lines = [
            format!("{words} KM : {}", km_text(p.km, 2)),
            format!("X = {}", grouped(p.y)),
            format!("Y = {}", grouped(p.x)),
        ];
        for (i, l) in lines.into_iter().enumerate() {
            let k = 1.6 * h * i as f64;
            out.push(text(
                [at[0] + down.cos() * k, at[1] + down.sin() * k],
                l,
                h,
                degrees,
                turn,
                TextAlign::MiddleLeft,
                None,
            ));
        }
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
    fn coordinates_in_groups_of_three() {
        // KGM 2024: `X =4 349 283.7346`, `Y =   542 198.1906`.
        assert_eq!(grouped(4_349_283.734_6), "4 349 283.7346");
        assert_eq!(grouped(542_198.190_6), "542 198.1906");
        assert_eq!(grouped(12.5), "12.5000");
        assert_eq!(grouped(-1234.0), "-1 234.0000");
    }

    #[test]
    fn deflection_in_grad() {
        // A right angle is 100 grad; KGM's EK-11 writes 36.56916 for 0.574420… rad.
        assert!((grad(std::f64::consts::FRAC_PI_2) - 100.0).abs() < 1e-12);
        assert!((grad(-std::f64::consts::PI) - 200.0).abs() < 1e-12);
    }
}
