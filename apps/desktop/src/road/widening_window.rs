//! Genişletme önerisi (plan/road.md Y28.3, docs/adr/0932 §4): opened from Dever on the chosen schedule as typed, it
//! reads `road.superelevations.widen` for a design vehicle: the AASHTO widening of each curve, the curve of widening
//! against radius for that vehicle with the alignment's curves on it, and with Sür the off-tracking the vehicle reaches
//! driving each curve. Çizelgeye yaz puts the suggestions into Dever's Genişleme cells and goes back; Dever's Kaydet
//! writes them. The vehicle's sizes, the clearance, the speed, the rounding and the least widening are the user's:
//! KentOS fills in none (the vehicle is taken from Araç izi when that window holds one).
//!
//! ```text
//! ┌ Genişletme önerisi ───────────────────────────────────────────────────────────────────── × ┐
//! │ Taşıt ───────────────────      ┌ widening against radius, the curves on it ─────────────┐ │
//! │  Dingil aralığı, Ön çıkıntı    └────────────────────────────────────────────────────────┘ │
//! │  Genişlik, İz genişliği        [kurp] [en büyük öneri] [kararlı iz] [sürüş / kararlı]     │
//! │  Römork [Yok|Var] …            PI · R · iz · U · F_A · Z · W_c · genişletme · öneri · şimdi│
//! │ Yol ─ Tip kesit, Açıklık, Hız                                                               │
//! │ Öneri ─ Yöntem, Yuvarlama, En az, Sür                                                       │
//! │                                                                  [Geri] [■ Çizelgeye yaz] │
//! └────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::road;
use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Theme, mouse};
use kentos_commands::{SuperelevationsWiden, SuperelevationsWidened, SweptTrailerDef, SweptVehicleDef, WideningMethod};
use kentos_road::widening::{self as wid, WideningRoad, WideningVehicle};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form, Segmented, Switch};

use crate::app::{App, Message};
use crate::road_dialog::{self, RoadDialog, col, num};
use crate::superelevation_window::{Event as Dever, comma_text, number};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Genişletme önerisi";
pub(crate) const APPLY: &str = "Çizelgeye yaz";
pub(crate) const BACK: &str = "Geri";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 10] = [
    "Dingil aralığı (m)",
    "Ön çıkıntı (m)",
    "Genişlik (m)",
    "İz genişliği (m)",
    "Mafsal (m)",
    "Römork dingili (m)",
    "Yanal açıklık (m)",
    "Proje hızı (km/sa)",
    "Yuvarlama (m)",
    "En az genişletme (m)",
];

/// AASHTO's two forms, as the window names them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Standard,
    Simple,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Method::Standard => "Standart",
            Method::Simple => "Basit",
        })
    }
}

/// With or without a trailer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Towed {
    No,
    Yes,
}

impl std::fmt::Display for Towed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Towed::No => "Yok",
            Towed::Yes => "Var",
        })
    }
}

/// The window's state, held by Dever's.
#[derive(Debug, Clone, Default)]
pub struct Widen {
    pub(crate) fields: [String; 10],
    pub(crate) trailer: bool,
    pub(crate) method: Method,
    pub(crate) drive: bool,
    pub(crate) template: Option<String>,
    pub(crate) result: Option<Result<SuperelevationsWidened, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Field(usize, String),
    Trailer(bool),
    Method(Method),
    Drive(bool),
    Template(String),
    Apply,
    Back,
}

fn msg(e: Event) -> Message {
    Message::Road(road::Event::Superelevations(Dever::Widen(e)))
}

impl Widen {
    /// The query from the typed fields; why not, in words.
    pub(crate) fn input(&self, superelevation: &str) -> Result<SuperelevationsWiden, String> {
        let read = |k: usize| number(&self.fields[k]).map_err(|()| format!("{} bir sayı olmalı.", FIELDS[k]));
        let need = |k: usize| read(k)?.ok_or_else(|| format!("{} yazın.", FIELDS[k]));
        // In the form's order, so the first empty field is the one named.
        let (wheelbase, front, width) = (need(0)?, need(1)?, need(2)?);
        let trailer = if self.trailer {
            Some(SweptTrailerDef { kingpin: need(4)?, wheelbase: need(5)?, front: 0.0, rear: 0.0, width })
        } else {
            None
        };
        Ok(SuperelevationsWiden {
            superelevation: superelevation.to_owned(),
            template: self.template.clone().ok_or("Bir tip kesit seçin: şerit sayısı ve genişliği ondan okunur.")?,
            vehicle: SweptVehicleDef { name: None, wheelbase, front, rear: 0.0, width, trailer },
            track: read(3)?,
            clearance: need(6)?,
            speed: read(7)?,
            method: Some(match self.method {
                Method::Standard => WideningMethod::Standard,
                Method::Simple => WideningMethod::Simple,
            }),
            step: read(8)?,
            least: read(9)?,
            drive: self.drive,
        })
    }
}

impl App {
    /// Genişletme öner… from Dever: the window on the chosen schedule, the vehicle from Araç izi if it holds one, the
    /// template and speed from Dever's Denetim.
    pub(crate) fn open_widening(&mut self) {
        let swept: Option<([String; 5], bool)> = self.road.swept_window.as_ref().map(|s| {
            ([s.fields[0].clone(), s.fields[1].clone(), s.fields[3].clone(), s.fields[4].clone(), s.fields[5].clone()], s.trailer)
        });
        let Some(w) = self.road.superelevation_window.as_mut() else {
            return;
        };
        let mut x = Widen {
            template: w.check_template.clone(),
            ..Widen::default()
        };
        x.fields[7] = w.check[0].clone();
        if let Some((f, trailer)) = swept {
            [x.fields[0], x.fields[1], x.fields[2], x.fields[4], x.fields[5]] = f;
            x.trailer = trailer;
        }
        w.widen = Some(Box::new(x));
        self.refresh_widening();
    }

    /// The suggestion read again on the chosen schedule as typed.
    pub(crate) fn refresh_widening(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.road.superelevation_window.as_mut() else {
            return;
        };
        let def = w.chosen.and_then(|i| w.rows.get(i)).map(|r| r.def());
        let Some(x) = w.widen.as_mut() else { return };
        x.result = Some(match def {
            None => Err("Dever penceresinde bir çizelge seçin.".into()),
            Some(Err(e)) => Err(e),
            Some(Ok(def)) => x.input(&def.id).and_then(|q| {
                kentos_native_application::road_superelevations_widen::preview(&doc.model, &def, &q).map(|(out, warnings)| {
                    x.warnings = warnings.into_iter().map(|w| w.message).collect();
                    out
                })
            }),
        });
        if matches!(x.result, Some(Err(_))) {
            x.warnings.clear();
        }
    }

    /// An event of the window.
    pub(crate) fn widening_event(&mut self, event: Event) {
        let Some(w) = self.road.superelevation_window.as_mut() else {
            return;
        };
        let Some(x) = w.widen.as_mut() else { return };
        match event {
            Event::Field(k, t) => {
                if let Some(f) = x.fields.get_mut(k) {
                    *f = t;
                }
            }
            Event::Trailer(on) => x.trailer = on,
            Event::Method(m) => x.method = m,
            Event::Drive(on) => x.drive = on,
            Event::Template(t) => x.template = Some(t),
            Event::Back => {
                w.widen = None;
                return;
            }
            Event::Apply => {
                let Some(Ok(out)) = &x.result else { return };
                let suggested: Vec<(String, f64)> = out.curves.iter().map(|c| (c.pi.clone(), c.suggested)).collect();
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    let missing = r.set_widening(&suggested);
                    w.widen = None;
                    if !missing.is_empty() {
                        self.warn(format!(
                            "{} kurbu dever çizelgesinde yok; genişletmesi yazılmadı (önce kurbu çizelgeye ekleyin).",
                            missing.join(", ")
                        ));
                    }
                    self.refresh_superelevation();
                }
                return;
            }
        }
        self.refresh_widening();
    }

    /// The window over Dever's.
    pub(crate) fn widening_view<'a>(&'a self, w: &'a crate::superelevation_window::Window, x: &'a Widen) -> Element<'a, Message> {
        let templates: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_templates().iter().map(|t| (t.id.clone(), t.name.clone())).collect());
        let ids: Vec<String> = templates.iter().map(|t| t.0.clone()).collect();
        let pick = Select::new(
            templates.iter().map(|t| Choice::new(t.1.clone())),
            x.template.as_ref().and_then(|t| ids.iter().position(|i| i == t)),
            move |i| msg(Event::Template(ids[i].clone())),
        )
        .placeholder("Tip kesit");
        let field = |k: usize, hint: &'a str| kentos_ui::widget::text_field(hint, &x.fields[k]).on_input(move |t| msg(Event::Field(k, t)));
        let towed = Segmented::new([Towed::No, Towed::Yes], if x.trailer { Towed::Yes } else { Towed::No }, |t| {
            msg(Event::Trailer(t == Towed::Yes))
        });
        let mut form = Form::new()
            .label_width(150.0)
            .section("Taşıt")
            .field(FIELDS[0], field(0, ""))
            .field(FIELDS[1], field(1, ""))
            .help("Ön dingilin önünde kalan gövde.")
            .field(FIELDS[2], field(2, ""))
            .field(FIELDS[3], field(3, "genişlik"))
            .help("Lastiklerin dıştan dışa; boşsa taşıtın genişliği alınır.")
            .field("Römork", towed);
        if x.trailer {
            form = form
                .field(FIELDS[4], field(4, ""))
                .help("Mafsalın arka dingile uzaklığı: arkasında artı, önünde eksi.")
                .field(FIELDS[5], field(5, ""));
        }
        let form = form
            .section("Yol")
            .field("Tip kesit", pick)
            .help("Şerit sayısı ve teğetteki genişliği ondan okunur.")
            .field(FIELDS[6], field(6, ""))
            .help("Şerit başına (C); standardınızınki.")
            .field(FIELDS[7], field(7, "yok"))
            .help("Boşsa sürüş güçlüğü payı (Z) eklenmez.")
            .section("Öneri")
            .field("Yöntem", Segmented::new([Method::Standard, Method::Simple], x.method, |m| msg(Event::Method(m))))
            .help("Standart: N(U + C) + (N − 1)·F_A + Z. Basit: N(U + C).")
            .field(FIELDS[8], field(8, "yok"))
            .field(FIELDS[9], field(9, "yok"))
            .help("Bunun altındaki genişletme yazılmaz.")
            .field("Sürüş", Switch::new(x.drive, |on| msg(Event::Drive(on))).label("Taşıtı her kurpta sür"));
        let curves_method = w.chosen.and_then(|i| w.rows.get(i)).is_some_and(|r| r.method == 0);
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Back)).form_width(400.0).width(1200.0).close_caption(BACK);
        let ok = x.result.as_ref().and_then(|r| r.as_ref().ok());
        let vehicle = self.widening_vehicle(x);
        d = d
            .picture(
                iced::widget::canvas(Curve { out: ok, vehicle, least: number(&x.fields[9]).ok().flatten() }).width(Fill).height(Fill),
                220.0,
            )
            .legend("Genişletme (m) yarıçapa (m, logaritmik) göre; noktalar aksın kurpları, kesikli çizgi en az genişletme. Tabloda iz, F_A ve Z metredir");
        match &x.result {
            Some(Err(e)) => d = d.note(Banner::info(e.clone())),
            Some(Ok(out)) => {
                d = d.figure("Kurp", out.curves.len().to_string());
                let most = out.curves.iter().map(|c| c.suggested).fold(0.0, f64::max);
                d = d.figure("En büyük öneri", format!("{} m", comma_text(most, 2)));
                let off = out.curves.iter().map(|c| c.offtracking).fold(0.0, f64::max);
                d = d.figure("Kararlı iz (en çok)", format!("{} m", comma_text(off, 3)));
                let ratio = out
                    .curves
                    .iter()
                    .filter_map(|c| c.driven.map(|v| v / c.offtracking))
                    .reduce(f64::min)
                    .map_or_else(|| "sürülmedi".to_owned(), |r| format!("%{:.0}", 100.0 * r));
                d = d.figure("Sürüşte iz / kararlı", ratio);
                for warning in &x.warnings {
                    d = d.note(Banner::warning(warning.clone()));
                }
                if !curves_method {
                    d = d.note(Banner::info("Km satırlı çizelgeye genişletme yazılmaz; öneri yalnız okunur."));
                }
                // The terms the widening is made of, then what is suggested and what the schedule has (U and W_c follow
                // from them; at 1100 px eleven columns cut the suggestion off).
                let mut cols = vec![
                    col("PI", 40.0),
                    num("R (m)", 56.0),
                    num("İz", 52.0),
                    num("F_A", 50.0),
                    num("Z", 50.0),
                    num("Genişletme", 78.0),
                    num("Öneri", 52.0),
                    num("Çizelgede", 70.0),
                ];
                if x.drive {
                    cols.push(num("Sürüşte iz", 70.0));
                }
                let rows = out
                    .curves
                    .iter()
                    .map(|c| {
                        let mut r = vec![
                            c.pi.clone(),
                            comma_text(c.radius, 1),
                            comma_text(c.offtracking, 3),
                            comma_text(c.overhang, 3),
                            comma_text(c.allowance, 3),
                            comma_text(c.widening, 3),
                            comma_text(c.suggested, 2),
                            comma_text(c.current, 2),
                        ];
                        if x.drive {
                            r.push(c.driven.map_or_else(|| "—".into(), |v| comma_text(v, 3)));
                        }
                        r
                    })
                    .collect();
                d = d.detail(road_dialog::table(cols, rows, "Aksın kurbu yok"));
            }
            None => {}
        }
        let can = curves_method && ok.is_some_and(|o| !o.curves.is_empty());
        d.primary(APPLY, can.then(|| msg(Event::Apply))).view()
    }

    /// The vehicle and road the curve is drawn for, when the fields read.
    fn widening_vehicle(&self, x: &Widen) -> Option<(WideningVehicle, WideningRoad)> {
        let out = x.result.as_ref()?.as_ref().ok()?;
        let q = x.input(&out.superelevation).ok()?;
        let vehicle = WideningVehicle {
            track: q.track.unwrap_or(q.vehicle.width),
            front: q.vehicle.front,
            wheelbase: q.vehicle.wheelbase,
            trailer: q.vehicle.trailer.as_ref().map(|t| (t.kingpin, t.wheelbase)),
        };
        let road = WideningRoad {
            lanes: out.lanes,
            tangent: out.tangent,
            clearance: q.clearance,
            speed: q.speed,
            method: match x.method {
                Method::Standard => wid::WideningMethod::Standard,
                Method::Simple => wid::WideningMethod::Simple,
            },
        };
        Some((vehicle, road))
    }
}

/// The widening against radius for the vehicle (a log axis from just above where it cannot turn to 2000 m), the
/// alignment's curves on it, zero and the least widening.
struct Curve<'a> {
    out: Option<&'a SuperelevationsWidened>,
    vehicle: Option<(WideningVehicle, WideningRoad)>,
    least: Option<f64>,
}

impl canvas::Program<Message> for Curve<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let word = |frame: &mut canvas::Frame, s: String, at: Point, color: Color| {
            frame.fill_text(Text {
                content: s,
                position: at,
                color,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                ..Text::default()
            });
        };
        let (Some(out), Some((vehicle, road))) = (self.out, self.vehicle) else {
            word(&mut frame, "Taşıtın ölçülerini ve açıklığı yazınca genişletme burada çizilir.".into(), Point::new(16.0, 20.0), t.muted);
            return vec![frame.into_geometry()];
        };
        // The radii round the alignment's curves (a third of the least to four times the most, at least 1000 m), not
        // down to where the vehicle cannot turn: there the widening runs to tens of metres and flattens the curves'.
        let least_r = out.curves.iter().map(|c| c.radius).fold(f64::INFINITY, f64::min);
        let most_r = out.curves.iter().map(|c| c.radius).fold(0.0, f64::max);
        let low = (vehicle.sum_of_squares().max(0.0).sqrt() * 1.05).max(if least_r.is_finite() { least_r / 3.0 } else { 30.0 });
        let high = (4.0 * most_r).max(1000.0);
        let (r0, r1) = (low.ln(), high.ln());
        let samples: Vec<(f64, f64)> = (0..=160)
            .filter_map(|i| {
                let r = (r0 + (r1 - r0) * f64::from(i) / 160.0).exp();
                wid::widening(r, &vehicle, &road).ok().map(|w| (r, w.widening))
            })
            .collect();
        let top = samples.iter().map(|s| s.1).chain(out.curves.iter().map(|c| c.widening)).fold(0.5, f64::max) * 1.05;
        let bottom = samples.iter().map(|s| s.1).fold(0.0, f64::min).min(0.0);
        let plot = Rectangle { x: 48.0, y: 12.0, width: bounds.width - 60.0, height: bounds.height - 34.0 };
        let x = |r: f64| plot.x + plot.width * ((r.ln() - r0) / (r1 - r0)) as f32;
        let y = |v: f64| plot.y + plot.height * ((top - v) / (top - bottom)) as f32;
        let thin = |c: Color| Stroke::default().with_color(c).with_width(1.0);
        // The grid: radii 20, 50, 100, 200, 500, 1000, 2000 m; widening every half metre or metre.
        for r in [20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0] {
            if r < low || r > high {
                continue;
            }
            frame.stroke(&Path::line(Point::new(x(r), plot.y), Point::new(x(r), plot.y + plot.height)), thin(t.border));
            // The last name kept inside the picture.
            let at = (x(r) - 8.0).min(plot.x + plot.width - 28.0);
            word(&mut frame, format!("{r:.0}"), Point::new(at, plot.y + plot.height + 4.0), t.muted);
        }
        // At most six lines across the widening.
        let step = [0.25, 0.5, 1.0, 2.0, 5.0, 10.0].into_iter().find(|s| (top - bottom) / s <= 6.0).unwrap_or(20.0);
        let mut v = (bottom / step).ceil() * step;
        while v <= top {
            frame.stroke(&Path::line(Point::new(plot.x, y(v)), Point::new(plot.x + plot.width, y(v))), thin(if v == 0.0 { t.muted } else { t.border }));
            word(&mut frame, comma_text(v, 1), Point::new(4.0, y(v) - 7.0), t.muted);
            v += step;
        }
        if let Some(l) = self.least.filter(|l| *l > bottom && *l < top) {
            frame.stroke(
                &Path::line(Point::new(plot.x, y(l)), Point::new(plot.x + plot.width, y(l))),
                Stroke { line_dash: LineDash { segments: &[4.0, 4.0], offset: 0 }, ..thin(t.warning) },
            );
        }
        let line = Path::new(|p| {
            for (i, (r, w)) in samples.iter().enumerate() {
                let at = Point::new(x(*r), y(*w));
                if i == 0 { p.move_to(at) } else { p.line_to(at) }
            }
        });
        frame.stroke(&line, Stroke::default().with_color(t.accent).with_width(2.0));
        for c in &out.curves {
            if c.radius < low || c.radius > 2000.0 {
                continue;
            }
            let at = Point::new(x(c.radius), y(c.widening));
            frame.fill(&Path::circle(at, 4.0), t.danger);
            word(&mut frame, c.pi.clone(), Point::new(at.x + 6.0, at.y - 14.0), t.text);
        }
        vec![frame.into_geometry()]
    }
}
