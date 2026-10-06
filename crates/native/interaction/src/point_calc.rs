//! Nokta hesapla (the web's `tools/pointCalc.ts`, docs/adr/0083, 0188):
//! Netcad's “Koordinat hesap makinası”. While a command waits for a point,
//! one of eleven constructions runs over it without ending it: the session suspends the
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
//! - Esc leaves it: the command goes on where it was (Km's Başlangıç asked
//!   first goes back to the value).
//! - `#ad` gives a point reference as a click would (docs/adr/0188 §3).

use kentos_contracts::AngleUnit;
use kentos_geometry_core::geom::survey::{
    along_line, clockwise_angle, distance_intersection, line_intersection, side_offsets, side_point,
};
use kentos_geometry_core::tools::point_calc::{bisector, bisector_nearest, slope};
use kentos_geometry_core::tools::point_input::{along_ratio, calc_polar, midpoint, nearest_of};
use kentos_geometry_core::tools::point_text::{is_js_space, js_trim, parse_number};

use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, fold_tr, upper_tr};
use crate::tool::{
    Context, Flow, Label, Marker, MarkerShape, Pointer, Preview, Stroke, Tag, Tone, Tool,
};
use crate::{Vec2, dist};

mod route;

use route::{Route, km, km_and_offset, number};

/// The eleven constructions.
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
    /// Obje üzerinde nokta: a distance along an object and an offset (docs/adr/0188 §1).
    Object,
    /// Km ve sapma: a route's km and an offset (§2).
    Km,
    /// Nokta adından: a named point's place (§3).
    Name,
    /// Mesafe ve eğim: a slope distance's horizontal from A towards B (§4).
    Slope,
    /// Açıortay: a distance along an angle's bisector (§5).
    Bisector,
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
pub const CALC_KINDS: [CalcDef; 11] = [
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
    CalcDef {
        kind: CalcKind::Object,
        label: "Obje üzerinde nokta",
        alias: "OBJE",
        icon: "calcObject",
        description: "Bir nesnenin yolunda, başlangıçtan uzaklık ve dik sapmayla (sağa artı) nokta. Çizgiye, yaya, daireye, elipse, eğriye ya da alana tıklayın (yakın uç başlangıçtır), “12.5” ya da “12.5,2” yazın.",
    },
    CalcDef {
        kind: CalcKind::Km,
        label: "Km ve sapma",
        alias: "KM",
        icon: "calcKm",
        description: "Güzergâhın km’siyle ve dik sapmasıyla (sağa artı) nokta; km ilk köşede Başlangıç’tır (B). Güzergâha tıklayın, “0+125.5” ya da “0+125.5,-3” yazın.",
    },
    CalcDef {
        kind: CalcKind::Name,
        label: "Nokta adından",
        alias: "NAD",
        icon: "calcName",
        description: "Adı bilinen noktanın yeri. Adını yazın: “P12” ya da “#P12”.",
    },
    CalcDef {
        kind: CalcKind::Slope,
        label: "Mesafe ve eğim",
        alias: "EGIM",
        icon: "calcSlope",
        description: "Eğik ölçülmüş mesafenin yatayıyla A’dan B’ye doğru nokta. A ve B’ye tıklayın, “eğik mesafe,eğim” yazın (eğim yüzde, ör. 25,8).",
    },
    CalcDef {
        kind: CalcKind::Bisector,
        label: "Açıortay",
        alias: "AO",
        icon: "calcBisector",
        description: "Bir açının ortayında, köşeden uzaklıkla nokta. Köşeye (K), sonra iki kolun birer noktasına (A, B) tıklayın; uzaklığı yazın ya da açıortayda tıklayın.",
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

    /// The construction a typed alias names (YAN, kkes: Turkish upper case;
    /// then without Turkish marks, so “eğim” is EGIM).
    pub fn by_alias(text: &str) -> Option<&'static CalcDef> {
        let text = upper_tr(js_trim(text));
        CALC_KINDS.iter().find(|d| d.alias == text).or_else(|| {
            CALC_KINDS
                .iter()
                .find(|d| fold_tr(d.alias) == fold_tr(&text))
        })
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
            CalcKind::Slope => &["başlangıç noktası (A)", "doğrultu noktası (B)"],
            CalcKind::Bisector => &[
                "açının köşesi (K)",
                "birinci kolun noktası (A)",
                "ikinci kolun noktası (B)",
            ],
            // An object (Obje, Km) or nothing (Nokta adından): no point to show.
            CalcKind::Object | CalcKind::Km | CalcKind::Name => &[],
        }
    }

    /// Obje üzerinde nokta and Km walk an object picked first.
    fn walks(self) -> bool {
        matches!(self, CalcKind::Object | CalcKind::Km)
    }

    /// What marks each reference on the drawing.
    fn letters(self) -> &'static [&'static str] {
        match self {
            CalcKind::Lines => &["A", "B", "C", "D"],
            CalcKind::Polar => &["S", "R"],
            CalcKind::Mid => &["1", "2"],
            CalcKind::Bisector => &["K", "A", "B"],
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
pub(crate) fn decimal(s: &str, signed: bool) -> Option<(f64, &str)> {
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

/// Why Açıortay has no bisector.
const BISECTOR_CORNER: &str = "Köşe bir kolun noktasıyla çakışıyor; açıortay yok. Kolların köşeden ayrı noktalarını gösterin.";

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
    /// Obje üzerinde nokta and Km: the object walked, once picked.
    route: Option<Route>,
    /// Km's Başlangıç (B) asked: the next text typed is the route's first km.
    asking_start: bool,
    /// Km's first km (the session's memory): the prompt and the tags say it.
    km_start: f64,
    /// The project's units and decimals as the last event saw them.
    format: Format,
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
            route: None,
            asking_start: false,
            km_start: 0.0,
            format: Format::default(),
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
    fn expected(&self, cx: &Context<'_>) -> String {
        let refs = self.kind.refs();
        if self.asking_start {
            return "Güzergâhın başındaki km’yi yazın: k+mmm.mmm ya da metre (ör. 0+000)."
                .to_owned();
        }
        if self.kind.walks() && self.route.is_none() {
            return "Nesneyi çizimde gösterin: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da alan.".to_owned();
        }
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
            CalcKind::Object => {
                "Başlangıçtan uzaklığı yazın, sapmayla da: uzaklık,sapma (ör. 12.5,2).".to_owned()
            }
            CalcKind::Km => format!(
                "Km’yi yazın, sapmayla da: km,sapma (ör. {},2).",
                km(cx.memory.calc_km_start + 12.5, &cx.format())
            ),
            CalcKind::Name => "Noktanın adını yazın (ör. P12 ya da #P12).".to_owned(),
            CalcKind::Slope => {
                "Eğik mesafeyi ve yüzde eğimi yazın: mesafe,eğim (ör. 25,8).".to_owned()
            }
            CalcKind::Bisector => {
                "Köşeden açıortay boyunca uzaklığı yazın ya da açıortayda tıklayın.".to_owned()
            }
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
            CalcKind::Slope => {
                // A % after the slope is read too.
                let t = js_trim(t.strip_suffix('%').unwrap_or(t));
                let Some((s, e)) = pair(t) else {
                    return false;
                };
                let horizontal = slope(m(s), e);
                let said = format!(
                    "Yatay uzaklık {}, yükseklik farkı {}.",
                    f.length(horizontal.horizontal),
                    f.length(horizontal.rise)
                );
                cx.say(Level::Info, said);
                self.finish(along_line(a, b, horizontal.horizontal), cx);
                true
            }
            CalcKind::Bisector => {
                let Some(d) = number(t) else {
                    return false;
                };
                match bisector(a, b, self.pts[2], m(d)) {
                    Some(p) => self.finish(Some(p), cx),
                    None => cx.say(Level::Warn, BISECTOR_CORNER),
                }
                true
            }
            CalcKind::Lines | CalcKind::Mid | CalcKind::Object | CalcKind::Km | CalcKind::Name => {
                false
            }
        }
    }

    /// A value typed for Obje üzerinde nokta, Km ve sapma or Nokta adından:
    /// false when the kind cannot read it.
    fn take_walk(&mut self, t: &str, cx: &mut Context<'_>) -> bool {
        let f = cx.format();
        if self.kind == CalcKind::Name {
            let name = js_trim(t.strip_prefix('#').unwrap_or(t));
            if name.is_empty() {
                return false;
            }
            match crate::session::named_point_at(cx.doc, name) {
                Ok(p) => self.finish(Some(p), cx),
                Err(line) => cx.say(Level::Warn, line),
            }
            return true;
        }
        let Some(route) = self.route.clone() else {
            return false;
        };
        let start = cx.memory.calc_km_start;
        let (s, offset) = match self.kind {
            CalcKind::Object => match pair(t).or_else(|| number(t).map(|s| (s, 0.0))) {
                Some((s, o)) => (f.to_metres(s), f.to_metres(o)),
                None => return false,
            },
            _ => match km_and_offset(t) {
                // The km is metres whatever the project's unit (§2); the offset is in it.
                Some((k, o)) => (k - start, f.to_metres(o)),
                None => return false,
            },
        };
        let at = route.at(s, offset, &f);
        match at.point {
            Some(p) => self.finish(Some(p), cx),
            None if self.kind == CalcKind::Km => {
                let line = format!(
                    "Km {} ile {} arasında olmalı.",
                    km(start, &f),
                    km(start + at.length, &f)
                );
                cx.say(Level::Warn, line);
            }
            None => {
                let line = format!(
                    "Uzaklık 0 ile yolun uzunluğu {} arasında olmalı.",
                    f.length(at.length)
                );
                cx.say(Level::Warn, line);
            }
        }
        true
    }

    /// A click (or a point given as one, `#ad`) once the references are
    /// shown: Kenar kesişimi's solution, a place on Hat üzerinde's line, on
    /// the walked object or on the bisector.
    fn click(&mut self, at: Vec2, cx: &mut Context<'_>) {
        if !self.candidates.is_empty() {
            let chosen = nearest_of(&self.candidates, at);
            self.finish(chosen, cx);
            return;
        }
        let need = self.kind.refs().len();
        if self.pts.len() < need {
            if self.pts.last().is_some_and(|last| dist(*last, at) < 1e-9) {
                return;
            }
            self.pts.push(at);
            if self.pts.len() == need {
                self.all_picked(cx);
            }
            return;
        }
        match self.kind {
            // Hat üzerinde: a click on the line places the point at its projection.
            CalcKind::Along => {
                if let Some(o) = side_offsets(self.pts[0], self.pts[1], at) {
                    self.finish(along_line(self.pts[0], self.pts[1], o.absis), cx);
                }
            }
            CalcKind::Object | CalcKind::Km => {
                let Some(route) = self.route.clone() else {
                    return;
                };
                // The object's point nearest the click, no offset.
                if let Some((s, _)) = route.read(at) {
                    let f = cx.format();
                    self.finish(route.at(s, 0.0, &f).point, cx);
                }
            }
            CalcKind::Bisector => {
                match bisector_nearest(self.pts[0], self.pts[1], self.pts[2], at) {
                    Some(p) => self.finish(Some(p), cx),
                    None => cx.say(Level::Warn, BISECTOR_CORNER),
                }
            }
            _ => {}
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
        if self.asking_start {
            return Prompt::new(
                label,
                format!(
                    "güzergâhın başındaki km’yi yazın (şimdi {})",
                    km(self.km_start, &self.format)
                ),
            );
        }
        let step = if self.kind.walks() && self.route.is_none() {
            match self.kind {
                CalcKind::Km => "güzergâha tıklayın (km’si ilk köşesinde başlar)".to_owned(),
                _ => "nesneye tıklayın (çizgi, yay, daire, elips, eğri ya da alan; yakın uç başlangıçtır)"
                    .to_owned(),
            }
        } else if self.pts.len() < refs.len() {
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
                CalcKind::Object => {
                    "başlangıçtan uzaklığı yazın: uzaklık ya da uzaklık,sapma (sapma sağa artı); ya da nesnede tıklayın"
                        .to_owned()
                }
                CalcKind::Km => {
                    "km’yi yazın: km ya da km,sapma (sapma sağa artı); ya da güzergâhta tıklayın"
                        .to_owned()
                }
                CalcKind::Name => "noktanın adını yazın (P12 ya da #P12)".to_owned(),
                CalcKind::Slope => "eğik mesafeyi ve yüzde eğimi yazın: mesafe,eğim".to_owned(),
                CalcKind::Bisector => {
                    "köşeden uzaklığı yazın; ya da açıortayda tıklayın".to_owned()
                }
                _ => {
                    let unit = if self.degrees { "derece" } else { "grad" };
                    format!("açı ({unit}, saat yönünde) ve mesafeyi yazın: açı,mesafe")
                }
            }
        };
        let prompt = Prompt::new(label, step);
        if self.kind == CalcKind::Km {
            prompt.option_with("Başlangıç", "B", km(self.km_start, &self.format))
        } else {
            prompt
        }
    }

    /// References picked so far.
    fn point_count(&self) -> usize {
        self.pts.len() + usize::from(self.route.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.degrees = cx.doc.settings().angle_unit == AngleUnit::Deg;
        self.km_start = cx.memory.calc_km_start;
        self.format = cx.format();
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.format = cx.format();
        self.hover = Some(p.world);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.asking_start {
            return;
        }
        if self.kind.walks() && self.route.is_none() {
            // The object under the cursor, not the snap's point (it may be a neighbour's).
            self.route = Route::pick(p.raw, self.kind == CalcKind::Object, cx);
            return;
        }
        self.click(p.world, cx);
    }

    /// Whatever is typed while the calculator runs is its own: what it
    /// cannot take is refused, saying what the step waits for.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        if self.asking_start {
            match kentos_geometry_core::tools::point_calc::km_value(t) {
                Some(v) => {
                    cx.memory.calc_km_start = v;
                    self.km_start = v;
                    self.asking_start = false;
                }
                None => {
                    let refusal = format!("“{t}” anlaşılamadı. {}", self.expected(cx));
                    cx.say(Level::Warn, refusal);
                }
            }
            return true;
        }
        // Km's Başlangıç, at any step.
        if self.kind == CalcKind::Km && upper_tr(t) == "B" {
            self.asking_start = true;
            return true;
        }
        let taken = if self.kind.walks() || self.kind == CalcKind::Name {
            (self.kind == CalcKind::Name || self.route.is_some()) && self.take_walk(t, cx)
        } else {
            self.pts.len() >= self.kind.refs().len() && self.take(t, cx)
        };
        if !taken {
            let refusal = format!("“{t}” anlaşılamadı. {}", self.expected(cx));
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

    /// Esc while Km's Başlangıç is asked goes back to the value; else it
    /// leaves the calculator (the session brings the command back).
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        std::mem::take(&mut self.asking_start)
    }

    /// Ctrl+Z: newest first, the choice between two solutions (back to typing
    /// the values), else the last reference picked (the object walked last).
    /// True even with nothing to take back: the drawing is never undone
    /// under the suspended command.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.asking_start {
            self.asking_start = false;
        } else if !self.candidates.is_empty() {
            self.candidates.clear();
        } else if self.pts.pop().is_none() {
            self.route = None;
        }
        true
    }

    /// A point given as if clicked (`#ad`, docs/adr/0188 §3): a reference,
    /// the place clicked once they are shown, or Nokta adından's point; not
    /// where an object is asked for.
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.result.is_some() || self.asking_start || (self.kind.walks() && self.route.is_none())
        {
            return false;
        }
        if self.kind == CalcKind::Name {
            self.finish(Some(p), cx);
            return true;
        }
        self.click(p, cx);
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
        let mut mark = |p: Vec2, text: String| {
            out.markers.push(Marker {
                at: p,
                shape: MarkerShape::Ring(4.0),
                tone: Tone::Snap,
            });
            out.labels.push(Label {
                at: p,
                text,
                offset: [7.0, -6.0],
                tone: Tone::Snap,
            });
        };
        for (i, p) in self.pts.iter().enumerate() {
            mark(*p, letters.get(i).copied().unwrap_or("").to_owned());
        }
        for q in &self.candidates {
            mark(*q, "?".to_owned());
        }
        // The walked object's start: A, or Km's first km.
        if let Some(route) = &self.route {
            let name = match self.kind {
                CalcKind::Km => km(self.km_start, format),
                _ => "A".to_owned(),
            };
            mark(route.start, name);
        }
        let (a, b, c) = (
            self.pts.first().copied(),
            self.pts.get(1).copied(),
            self.pts.get(2).copied(),
        );
        if let (Some(a), Some(b)) = (a, b)
            && !matches!(self.kind, CalcKind::Mid | CalcKind::Bisector)
        {
            let line = match self.kind {
                CalcKind::Lines | CalcKind::Side | CalcKind::Along | CalcKind::Slope => {
                    self.far_line(a, b)
                }
                _ => vec![a, b],
            };
            out.strokes.push(dashed(line, [4.0, 4.0]));
        }
        // Açıortay: its arms as shown, the bisector once both are.
        if self.kind == CalcKind::Bisector
            && let Some(k) = a
        {
            for arm in [b, c].into_iter().flatten() {
                out.strokes.push(dashed(vec![k, arm], [4.0, 4.0]));
            }
            if let (Some(b), Some(c)) = (b, c)
                && let Some(far) = bisector(k, b, c, self.far)
            {
                out.strokes.push(dashed(vec![k, far], [2.0, 3.0]));
            }
        }
        let Some(h) = self.hover else {
            return out;
        };
        if self.asking_start {
            return out;
        }
        if let Some(route) = &self.route {
            // The object's point under the cursor and the offset to it.
            if let Some((s, offset)) = route.read(h) {
                let on = route.at(s, 0.0, format).point;
                if let Some(on) = on {
                    out.markers.push(Marker {
                        at: on,
                        shape: MarkerShape::Ring(3.0),
                        tone: Tone::Snap,
                    });
                    out.strokes.push(dashed(vec![on, h], [2.0, 3.0]));
                }
                let first = match self.kind {
                    CalcKind::Km => format!("Km {}", km(self.km_start + s, format)),
                    _ => format!("Başlangıçtan {}", format.length(s)),
                };
                out.tag = Some(Tag {
                    at: h,
                    lines: vec![first, format!("Sapma {}", format.length(offset))],
                });
                out.tag_tone = Tone::Snap;
            }
            return out;
        }
        if let Some(c) = c
            && self.kind == CalcKind::Lines
        {
            out.strokes.push(dashed(self.far_line(c, h), [4.0, 4.0]));
        }
        if self.kind == CalcKind::Bisector {
            match (a, b, c) {
                (Some(k), Some(b), Some(c)) => {
                    if let Some(on) = bisector_nearest(k, b, c, h) {
                        out.markers.push(Marker {
                            at: on,
                            shape: MarkerShape::Ring(3.0),
                            tone: Tone::Snap,
                        });
                        out.tag = Some(Tag {
                            at: h,
                            lines: vec![format!("K’dan {}", format.length(dist(k, on)))],
                        });
                        out.tag_tone = Tone::Snap;
                    }
                }
                (Some(k), _, _) => out.strokes.push(dashed(vec![k, h], [2.0, 3.0])),
                _ => {}
            }
            return out;
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
            CalcKind::Along | CalcKind::Slope => {
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
            _ => {}
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
        // Without Turkish marks: “eğim” and “egim” are EGIM (docs/adr/0188).
        for typed in ["eğim", "egim", "EĞİM", "EGIM"] {
            assert_eq!(
                CalcDef::by_alias(typed).map(|d| d.kind),
                Some(CalcKind::Slope),
                "{typed}"
            );
        }
        assert_eq!(
            CalcDef::by_alias("obje").map(|d| d.kind),
            Some(CalcKind::Object)
        );
    }
}
