//! Nokta hesapla (the web's `tools/pointCalc.ts`, docs/adr/0083): Netcad's
//! “Koordinat hesap makinası”. While a command waits for a point, one of six
//! constructions runs over it without ending it: the session suspends the
//! command (ADR 0018 “Askıda”, [`Session::nest`](crate::Session::nest)). The
//! calculator picks its references on the drawing, with snaps, takes the
//! typed values and hands the computed point back to the command as if
//! clicked ([`Tool::accept_point`]).
//!
//! - Whatever is typed while it runs is its own. What it cannot read is
//!   refused in its own words, saying what the step waits for, and the step
//!   stays (the web's d0a358c).
//! - Ctrl+Z takes back its newest step: the choice between two solutions,
//!   else the last reference. It never undoes the drawing under the
//!   suspended command.
//! - Esc leaves it: the command goes on where it was.

use kentos_contracts::AngleUnit;
use kentos_geometry_core::geom::survey::{
    along_line, clockwise_angle, distance_intersection, line_intersection, side_offsets, side_point,
};
use kentos_geometry_core::tools::point_input::{along_ratio, calc_polar, midpoint, nearest_of};
use kentos_geometry_core::tools::point_text::{is_js_space, js_trim, parse_number};

use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Label, Marker, MarkerShape, Pointer, Preview, Stroke, Tag, Tone, Tool,
};
use crate::{Vec2, dist};

/// The six constructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalcKind {
    /// Yan nokta: dik ayak and dik boy along a line.
    Side,
    /// Kenar kesişimi: distances from two points.
    Distances,
    /// Doğru kesişimi: where two lines cross.
    Lines,
    /// Hat üzerinde nokta: a distance or a ratio along a line.
    Along,
    /// Açı ve mesafe: an angle and a distance from a station.
    Polar,
    /// İki nokta ortası.
    Mid,
}

/// A construction as the menus and the command line offer it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalcDef {
    pub kind: CalcKind,
    pub label: &'static str,
    /// Typed on the command line while a point is expected.
    pub alias: &'static str,
    /// The web's icon for it.
    pub icon: &'static str,
    /// What it computes and where to click (a menu row's second line).
    pub description: &'static str,
}

/// The constructions in the web's order (`CALC_KINDS`).
pub const CALC_KINDS: [CalcDef; 6] = [
    CalcDef {
        kind: CalcKind::Side,
        label: "Yan nokta (dik ayak, dik boy)",
        alias: "YAN",
        icon: "calcSide",
        description: "Ölçü krokisindeki gibi: bir hat boyunca dik ayak, ona dik dik boy (sağa artı). A ve B’ye tıklayın, “12.5,3” yazın.",
    },
    CalcDef {
        kind: CalcKind::Distances,
        label: "Kenar kesişimi",
        alias: "KKES",
        icon: "calcDistances",
        description: "İki noktaya uzaklığı bilinen nokta (şeritle ölçülmüş köşe). A ve B’ye tıklayın, “d1,d2” yazın, iki çözümden birine tıklayın.",
    },
    CalcDef {
        kind: CalcKind::Lines,
        label: "Doğru kesişimi (4 nokta)",
        alias: "DKES",
        icon: "calcLines",
        description: "İki doğrunun, uzantıları dahil, kesiştiği nokta. Birinci doğrunun iki noktasına, sonra ikincinin iki noktasına tıklayın.",
    },
    CalcDef {
        kind: CalcKind::Along,
        label: "Hat üzerinde nokta",
        alias: "HAT",
        icon: "calcAlong",
        description: "A–B hattı üzerinde, A’dan uzaklıkla ya da oranla (1/3) nokta. A ve B’ye tıklayın, uzaklığı yazın ya da hatta tıklayın.",
    },
    CalcDef {
        kind: CalcKind::Polar,
        label: "Açı ve mesafe",
        alias: "AM",
        icon: "calcPolar",
        description: "Takeometre gibi: durulan noktadan (S), bakılan noktaya (R) göre saat yönünde açı ve mesafe. S ve R’ye tıklayın, “açı,mesafe” yazın.",
    },
    CalcDef {
        kind: CalcKind::Mid,
        label: "İki nokta ortası",
        alias: "ORTA",
        icon: "calcMid",
        description: "İki noktanın tam ortası. İki noktaya tıklayın.",
    },
];

/// The heading of the calculator's menus (the web's `calcMenuItems`).
pub const MENU_HEADER: &str = "Komut nokta beklerken ölçülerden nokta hesaplar";

/// Why the calculator does not start (the web's `startPointCalc`).
pub const NOT_NOW: &str = "Nokta hesabı, nokta bekleyen bir komut sırasında kullanılır.";

impl CalcDef {
    pub fn of(kind: CalcKind) -> &'static CalcDef {
        CALC_KINDS
            .iter()
            .find(|d| d.kind == kind)
            .unwrap_or(&CALC_KINDS[0])
    }

    /// The construction a typed alias names (YAN, kkes: Turkish upper case).
    pub fn by_alias(text: &str) -> Option<&'static CalcDef> {
        let text = upper_tr(js_trim(text));
        CALC_KINDS.iter().find(|d| d.alias == text)
    }

    /// The command line's entry when it starts (the web's `nest` label).
    pub fn started(&self) -> String {
        format!("Nokta hesabı: {}", self.label)
    }
}

impl CalcKind {
    /// The references it asks for, in order.
    fn refs(self) -> &'static [&'static str] {
        match self {
            CalcKind::Side => &[
                "hattın başlangıç noktası (A)",
                "hattın doğrultu noktası (B)",
            ],
            CalcKind::Distances => &["birinci nokta (A)", "ikinci nokta (B)"],
            CalcKind::Lines => &[
                "birinci doğrunun ilk noktası (A)",
                "birinci doğrunun ikinci noktası (B)",
                "ikinci doğrunun ilk noktası (C)",
                "ikinci doğrunun ikinci noktası (D)",
            ],
            CalcKind::Along => &["hattın başlangıcı (A)", "hattın sonu (B)"],
            CalcKind::Polar => &["durulan nokta (S)", "bakılan nokta (R)"],
            CalcKind::Mid => &["birinci nokta", "ikinci nokta"],
        }
    }

    /// What marks each reference on the drawing.
    fn letters(self) -> &'static [&'static str] {
        match self {
            CalcKind::Lines => &["A", "B", "C", "D"],
            CalcKind::Polar => &["S", "R"],
            CalcKind::Mid => &["1", "2"],
            _ => &["A", "B"],
        }
    }
}

/// The first letter in Turkish upper case (the web's `capitalize`).
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => upper_tr(&first.to_string()) + chars.as_str(),
        None => String::new(),
    }
}

/// `-?\d+(\.\d+)?` at the start of `s`: the number and what follows it.
fn decimal(s: &str, signed: bool) -> Option<(f64, &str)> {
    let bytes = s.as_bytes();
    let mut i = usize::from(signed && bytes.first() == Some(&b'-'));
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let whole = digits(i);
    if whole == 0 {
        return None;
    }
    i += whole;
    if bytes.get(i) == Some(&b'.') {
        let fraction = digits(i + 1);
        if fraction > 0 {
            i += 1 + fraction;
        }
    }
    s[..i].parse().ok().map(|v| (v, &s[i..]))
}

/// Skips JavaScript's `\s` at the start of `s`.
fn spaces(s: &str) -> &str {
    s.trim_start_matches(is_js_space)
}

/// The web's `^(-?\d+(?:\.\d+)?)\s*[,; ]\s*(-?\d+(?:\.\d+)?)$`.
fn pair(t: &str) -> Option<(f64, f64)> {
    let (a, rest) = decimal(t, true)?;
    let after = spaces(rest);
    let rest = match after.strip_prefix([',', ';']) {
        Some(rest) => spaces(rest),
        // A space alone parts them too.
        None if after.len() < rest.len() => after,
        None => return None,
    };
    let (b, rest) = decimal(rest, true)?;
    rest.is_empty().then_some((a, b))
}

/// The web's `^(\d+(?:\.\d+)?)\s*\/\s*(\d+(?:\.\d+)?)$`.
fn ratio(t: &str) -> Option<(f64, f64)> {
    let (a, rest) = decimal(t, false)?;
    let rest = spaces(spaces(rest).strip_prefix('/')?);
    let (b, rest) = decimal(rest, false)?;
    rest.is_empty().then_some((a, b))
}

/// The point calculator over a suspended command (see the module comment).
pub struct PointCalc {
    kind: CalcKind,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    /// Kenar kesişimi: both solutions, waiting for the user to pick one.
    candidates: Vec<Vec2>,
    /// The project's angles are degrees (else grads), read when it starts.
    degrees: bool,
    /// How far a reference line runs past its points: twice the view's diagonal.
    far: f64,
    /// The computed point, once found: the session hands it to the command.
    result: Option<Vec2>,
}

impl PointCalc {
    pub fn new(kind: CalcKind) -> Self {
        Self {
            kind,
            pts: Vec::new(),
            hover: None,
            candidates: Vec::new(),
            degrees: false,
            far: 1000.0,
            result: None,
        }
    }

    fn def(&self) -> &'static CalcDef {
        CalcDef::of(self.kind)
    }

    /// The view as the last event saw it: how far reference lines run.
    fn see(&mut self, cx: &Context<'_>) {
        let b = cx.view.visible();
        let diagonal = (b.max_x - b.min_x).hypot(b.max_y - b.min_y);
        if diagonal.is_finite() && diagonal > 0.0 {
            self.far = diagonal * 2.0;
        }
    }

    /// What the step waits for, as a refusal says it.
    fn expected(&self) -> String {
        let refs = self.kind.refs();
        if self.pts.len() < refs.len() {
            return format!("{} çizimde gösterin.", capitalize(refs[self.pts.len()]));
        }
        if !self.candidates.is_empty() {
            return "İki çözümden istediğinize tıklayın ya da Enter’la sağdakini alın.".to_owned();
        }
        match self.kind {
            CalcKind::Side => "Dik ayak ve dik boyu yazın: absis,ordinat (ör. 12.5,3).".to_owned(),
            CalcKind::Distances => {
                "A ve B noktalarına uzaklıkları yazın: d1,d2 (ör. 10,8).".to_owned()
            }
            CalcKind::Along => "A’dan uzaklığı yazın ya da a/b oranı (ör. 1/3).".to_owned(),
            // Açı ve mesafe (İki nokta ortası and Doğru kesişimi take no value:
            // they finish on their last reference).
            _ => {
                let (unit, example) = if self.degrees {
                    ("derece", "90")
                } else {
                    ("grad", "100")
                };
                format!(
                    "Açı ({unit}, saat yönünde) ve mesafeyi yazın: açı,mesafe (ör. {example},25)."
                )
            }
        }
    }

    /// Every reference is shown: Orta and Doğru kesişimi finish.
    fn all_picked(&mut self, cx: &mut Context<'_>) {
        let (a, b) = (self.pts[0], self.pts[1]);
        match self.kind {
            CalcKind::Mid => self.finish(Some(midpoint(a, b)), cx),
            CalcKind::Lines => match line_intersection(a, b, self.pts[2], self.pts[3]) {
                Some(x) => self.finish(Some(x), cx),
                None => {
                    cx.say(
                        Level::Warn,
                        "Doğrular paralel; kesişim yok. Başka iki nokta gösterin.",
                    );
                    self.pts.truncate(2);
                }
            },
            _ => {}
        }
    }

    /// Takes a value typed once every reference is shown; false when the kind cannot read it.
    fn take(&mut self, t: &str, cx: &mut Context<'_>) -> bool {
        let (a, b) = (self.pts[0], self.pts[1]);
        // Lengths are typed in the project's unit (docs/adr/0165 §2).
        let f = cx.format();
        let m = |typed: f64| f.to_metres(typed);
        match self.kind {
            CalcKind::Side => {
                let Some((absis, ordinat)) = pair(t) else {
                    return false;
                };
                self.finish(side_point(a, b, m(absis), m(ordinat)), cx);
                true
            }
            CalcKind::Distances => {
                let Some((d1, d2)) = pair(t) else {
                    return false;
                };
                let solutions = distance_intersection(a, b, m(d1), m(d2));
                match solutions.len() {
                    0 => cx.say(
                        Level::Warn,
                        "Bu uzaklıklarla kesişim yok: iki uzaklığın toplamı A–B aralığından küçük ya da farkı büyük.",
                    ),
                    1 => self.finish(Some(solutions[0]), cx),
                    _ => self.candidates = solutions,
                }
                true
            }
            CalcKind::Along => {
                if let Some((num, den)) = ratio(t)
                    && den > 0.0
                {
                    self.finish(along_ratio(a, b, num, den), cx);
                    return true;
                }
                let Some(n) = parse_number(t) else {
                    return false;
                };
                self.finish(along_line(a, b, m(n)), cx);
                true
            }
            CalcKind::Polar => {
                let Some((angle, distance)) = pair(t) else {
                    return false;
                };
                let unit = if self.degrees { "deg" } else { "grad" };
                self.finish(calc_polar(a, b, angle, unit, m(distance)), cx);
                true
            }
            CalcKind::Lines | CalcKind::Mid => false,
        }
    }

    /// The computed point goes to the command; none: the references coincide.
    fn finish(&mut self, p: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(p) = p else {
            cx.say(
                Level::Warn,
                "Nokta hesaplanamadı: referans noktaları çakışıyor.",
            );
            return;
        };
        let text = format!("Hesaplanan nokta: {}", cx.format().point(p));
        cx.say(Level::Info, text);
        self.result = Some(p);
    }

    /// The line through p and q, running well past both (lines are unbounded here).
    fn far_line(&self, p: Vec2, q: Vec2) -> Vec<Vec2> {
        let l = match dist(p, q) {
            l if l > 0.0 => l,
            _ => 1.0,
        };
        let (ux, uy) = ((q.x - p.x) / l * self.far, (q.y - p.y) / l * self.far);
        vec![Vec2::new(p.x - ux, p.y - uy), Vec2::new(p.x + ux, p.y + uy)]
    }
}

fn dashed(pts: Vec<Vec2>, dash: [f32; 2]) -> Stroke {
    Stroke::dashed(pts, false, dash).tone(Tone::Snap)
}

impl Tool for PointCalc {
    fn id(&self) -> &'static str {
        "pointCalc"
    }

    fn label(&self) -> &'static str {
        self.def().label
    }

    fn prompt(&self) -> Prompt {
        let label = self.def().label;
        let refs = self.kind.refs();
        if !self.candidates.is_empty() {
            return Prompt::new(label, "iki çözümden istediğinize tıklayın")
                .option("Sağdaki", "Enter");
        }
        let step = if self.pts.len() < refs.len() {
            format!("{} gösterin", refs[self.pts.len()])
        } else {
            match self.kind {
                CalcKind::Side => {
                    "dik ayak ve dik boyu yazın: absis,ordinat (ordinat sağa artı)".to_owned()
                }
                CalcKind::Distances => "A ve B noktalarına uzaklıkları yazın: d1,d2".to_owned(),
                CalcKind::Along => {
                    "A’dan uzaklığı yazın ya da a/b oranı (ör. 1/3); ya da hat üzerinde tıklayın"
                        .to_owned()
                }
                _ => {
                    let unit = if self.degrees { "derece" } else { "grad" };
                    format!("açı ({unit}, saat yönünde) ve mesafeyi yazın: açı,mesafe")
                }
            }
        };
        Prompt::new(label, step)
    }

    /// References picked so far.
    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.degrees = cx.doc.settings().angle_unit == AngleUnit::Deg;
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.hover = Some(p.world);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if !self.candidates.is_empty() {
            let chosen = nearest_of(&self.candidates, p.world);
            self.finish(chosen, cx);
            return;
        }
        let need = self.kind.refs().len();
        if self.pts.len() < need {
            if self
                .pts
                .last()
                .is_some_and(|last| dist(*last, p.world) < 1e-9)
            {
                return;
            }
            self.pts.push(p.world);
            if self.pts.len() == need {
                self.all_picked(cx);
            }
            return;
        }
        // Hat üzerinde: a click on the line places the point at its projection.
        if self.kind == CalcKind::Along
            && let Some(o) = side_offsets(self.pts[0], self.pts[1], p.world)
        {
            self.finish(along_line(self.pts[0], self.pts[1], o.absis), cx);
        }
    }

    /// Whatever is typed while the calculator runs is its own: what it
    /// cannot take is refused, saying what the step waits for.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        if self.pts.len() < self.kind.refs().len() || !self.take(t, cx) {
            let refusal = format!("“{t}” anlaşılamadı. {}", self.expected());
            cx.say(Level::Warn, refusal);
        }
        true
    }

    /// Enter: the solution on the right of A→B; with none shown, it leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if let Some(&first) = self.candidates.first() {
            self.finish(Some(first), cx);
        }
        Flow::Exit
    }

    /// Ctrl+Z: newest first, the choice between two solutions (back to typing
    /// the values), else the last reference picked. True even with nothing
    /// to take back: the drawing is never undone under the suspended command.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.candidates.is_empty() {
            self.pts.pop();
        } else {
            self.candidates.clear();
        }
        true
    }

    fn finished(&self) -> bool {
        self.result.is_some()
    }

    fn computed(&self) -> Option<Vec2> {
        self.result
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = Preview::default();
        let letters = self.kind.letters();
        let mut mark = |p: Vec2, text: &str| {
            out.markers.push(Marker {
                at: p,
                shape: MarkerShape::Ring(4.0),
                tone: Tone::Snap,
            });
            out.labels.push(Label {
                at: p,
                text: text.to_owned(),
                offset: [7.0, -6.0],
                tone: Tone::Snap,
            });
        };
        for (i, p) in self.pts.iter().enumerate() {
            mark(*p, letters.get(i).copied().unwrap_or(""));
        }
        for q in &self.candidates {
            mark(*q, "?");
        }
        let (a, b, c) = (
            self.pts.first().copied(),
            self.pts.get(1).copied(),
            self.pts.get(2).copied(),
        );
        if let (Some(a), Some(b)) = (a, b)
            && self.kind != CalcKind::Mid
        {
            let line = match self.kind {
                CalcKind::Lines | CalcKind::Side | CalcKind::Along => self.far_line(a, b),
                _ => vec![a, b],
            };
            out.strokes.push(dashed(line, [4.0, 4.0]));
        }
        let Some(h) = self.hover else {
            return out;
        };
        if let Some(c) = c
            && self.kind == CalcKind::Lines
        {
            out.strokes.push(dashed(self.far_line(c, h), [4.0, 4.0]));
        }
        let (Some(a), Some(b)) = (a, b) else {
            if let Some(a) = a {
                out.strokes.push(dashed(vec![a, h], [2.0, 3.0]));
            }
            return out;
        };
        if !self.candidates.is_empty() {
            return out;
        }
        // Live readings of the cursor, in the terms the values are typed.
        let mut lines = Vec::new();
        match self.kind {
            CalcKind::Side => {
                if let Some(o) = side_offsets(a, b, h) {
                    lines.push(format!("Dik ayak {}", format.length(o.absis)));
                    lines.push(format!("Dik boy {}", format.length(o.ordinat)));
                }
            }
            CalcKind::Along => {
                if let Some(o) = side_offsets(a, b, h) {
                    lines.push(format!("A’dan {}", format.length(o.absis)));
                }
            }
            CalcKind::Polar => {
                let grad = clockwise_angle(a, b, h) * 200.0 / std::f64::consts::PI;
                lines.push(format!("Açı {}", format.bearing(grad)));
                lines.push(format!("Mesafe {}", format.length(dist(a, h))));
                out.strokes.push(dashed(vec![a, h], [2.0, 3.0]));
            }
            CalcKind::Distances => {
                lines.push(format!("A’ya {}", format.length(dist(a, h))));
                lines.push(format!("B’ye {}", format.length(dist(b, h))));
            }
            CalcKind::Lines | CalcKind::Mid => {}
        }
        if !lines.is_empty() {
            out.tag = Some(Tag { at: h, lines });
            out.tag_tone = Tone::Snap;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{CalcDef, CalcKind, capitalize, pair, ratio};

    /// The web's patterns, character by character.
    #[test]
    fn typed_values_read_as_the_web_reads_them() {
        assert_eq!(pair("12.5,3"), Some((12.5, 3.0)));
        assert_eq!(pair("12.5 ; -3"), Some((12.5, -3.0)));
        assert_eq!(pair("100 6"), Some((100.0, 6.0)));
        assert_eq!(pair("10,10"), Some((10.0, 10.0)));
        for bad in [
            "abc", "12.5", "12.5,", ",3", "1.,2", ".5,2", "1,2,3", "1e3,2", "+1,2",
        ] {
            assert_eq!(pair(bad), None, "{bad}");
        }
        assert_eq!(ratio("1/4"), Some((1.0, 4.0)));
        assert_eq!(ratio("1 / 3.5"), Some((1.0, 3.5)));
        assert_eq!(ratio("-1/4"), None);
        assert_eq!(capitalize("ikinci nokta (B)"), "İkinci nokta (B)");
        assert_eq!(capitalize("hattın başlangıcı (A)"), "Hattın başlangıcı (A)");
        assert_eq!(
            CalcDef::by_alias(" kkes ").map(|d| d.kind),
            Some(CalcKind::Distances)
        );
        assert_eq!(
            CalcDef::by_alias("orta").map(|d| d.kind),
            Some(CalcKind::Mid)
        );
        assert!(CalcDef::by_alias("ORTAK").is_none());
    }
}
