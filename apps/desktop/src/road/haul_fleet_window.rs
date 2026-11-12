//! Kamyon filosu (plan/road.md Y32.3, docs/adr/0936 §3): opened from Taşıma planı on the diagram and objective chosen
//! there, it reads `road.hauls.trucks` with the user's truck: each haul's run loaded and back empty on the road's own
//! heights, its cycle, trips and balance number; with a fleet the loader's production deterministically and as the
//! finite-source queue, the working hours, the fuel and the carbon. The chosen haul's speeds are drawn. Every number is
//! the user's; KentOS fills in none. The values are kept while the program runs.
//!
//! ```text
//! ┌ Kamyon filosu ─────────────────────────────────────────────────────────────────────────── × ┐
//! │ Kamyon ─ kütleler, güç, verim, hızlar   ┌ the chosen haul: speed loaded →, empty ← ───────┐ │
//! │ Yol ─ [Proje kotu|Arazi], yuvarlanma     └─────────────────────────────────────────────────┘ │
//! │ Çevrim ─ kasa, yükleme, boşaltma, manevra [sefer] [çevrim] [üretim] [süre]                 │
//! │ Filo ─ kamyon, saatte çalışılan           (Taşımalar | Filo) table                          │
//! │ Yakıt ─ özgül tüketim, rölanti, CO₂                                                         │
//! │                                                                                   [Geri]   │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke};
use iced::{Element, Fill, Point, Rectangle, Renderer, Theme, mouse};
use kentos_commands::{HaulObjective, HaulTrack, HaulTruckDef, HaulTrucked, HaulsTrucks};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form, Segmented};

use crate::app::{App, Message};
use crate::haul_plan_window::{Aim, Event as Plan};
use crate::road_dialog::{self, RoadDialog, col, num};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Kamyon filosu";
pub(crate) const BACK: &str = "Geri";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 19] = [
    "Boş kütle (t)",
    "Yüklü kütle (t)",
    "Motor gücü (kW)",
    "Aktarma verimi",
    "En yüksek hız (km/sa)",
    "İniş hızı (km/sa)",
    "Çekiş katsayısı",
    "Tahrik payı",
    "Fren (m/s²)",
    "Yuvarlanma (%)",
    "Kasa (m³)",
    "Yükleme (dk)",
    "Boşaltma (dk)",
    "Manevra (dk)",
    "Kamyon sayısı",
    "Saatte çalışılan (dk)",
    "Özgül yakıt (L/kWh)",
    "Rölanti (L/sa)",
    "CO₂ (kg/L)",
];

/// Where the trucks drive, as the window names it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Road {
    #[default]
    Profile,
    Ground,
}

impl std::fmt::Display for Road {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Road::Profile => "Proje kotu",
            Road::Ground => "Arazi",
        })
    }
}

/// The detail shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Listed {
    #[default]
    Hauls,
    Fleet,
}

impl std::fmt::Display for Listed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Listed::Hauls => "Taşımalar",
            Listed::Fleet => "Filo",
        })
    }
}

/// The window's state, held by Taşıma planı's.
#[derive(Debug, Clone, Default)]
pub struct Fleet {
    pub(crate) fields: [String; 19],
    pub(crate) road: Road,
    /// The haul whose speeds are drawn.
    pub(crate) trace: usize,
    pub(crate) list: Listed,
    pub(crate) result: Option<Result<HaulTrucked, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Field(usize, String),
    Road(Road),
    Trace(usize),
    List(Listed),
    Back,
}

fn msg(e: Event) -> Message {
    Message::HaulPlan(Plan::Fleet(e))
}

/// A number the user typed, with a comma or a point; empty: none.
fn number(text: &str) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or(())
}

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

/// Thousands parted by a thin space.
fn grouped(v: f64, places: usize) -> String {
    let text = format!("{:.places$}", v.abs());
    let (whole, part) = text.split_once('.').map_or((text.as_str(), None), |(a, b)| (a, Some(b)));
    let mut out = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if let Some(p) = part {
        out.push(',');
        out.push_str(p);
    }
    if v < 0.0 && out.chars().any(|c| c.is_ascii_digit() && c != '0') {
        out.insert(0, '−');
    }
    out
}

impl Fleet {
    /// The query from the typed fields; why not, in words.
    pub(crate) fn input(&self, haul: &str, objective: HaulObjective) -> Result<HaulsTrucks, String> {
        let read = |k: usize| number(&self.fields[k]).map_err(|()| format!("{} bir sayı olmalı.", FIELDS[k]));
        let need = |k: usize| read(k)?.ok_or_else(|| format!("{} yazın.", FIELDS[k]));
        let truck = HaulTruckDef {
            empty: need(0)?,
            gross: need(1)?,
            power: need(2)?,
            efficiency: need(3)?,
            top: need(4)?,
            downhill: read(5)?,
            traction: need(6)?,
            drive: need(7)?,
            brake: need(8)?,
        };
        let rolling = need(9)?;
        let load = need(10)?;
        let loading = need(11)?;
        let dumping = need(12)?;
        let trucks = match read(14)? {
            None => None,
            Some(n) if n.fract() == 0.0 && n >= 1.0 => Some(n as u32),
            Some(_) => return Err("Kamyon sayısı 1 ya da daha büyük bir tamsayı olmalı.".into()),
        };
        Ok(HaulsTrucks {
            haul: haul.to_owned(),
            objective: Some(objective),
            track: match self.road {
                Road::Profile => HaulTrack::Profile,
                Road::Ground => HaulTrack::Ground,
            },
            truck,
            rolling,
            load,
            loading,
            dumping,
            spotting: read(13)?,
            trucks,
            hour: read(15)?,
            fuel: read(16)?,
            idle: read(17)?,
            co2: read(18)?,
            trace: Some(self.trace as u32),
        })
    }
}

impl App {
    /// Kamyon filosu… from Taşıma planı: the window on its diagram and objective, the values typed before kept.
    pub(crate) fn open_haul_fleet(&mut self) {
        let Some(w) = self.haul_plan_window.as_mut() else { return };
        let mut fleet = w.fleet_kept.clone().unwrap_or_default();
        fleet.result = None;
        fleet.warnings.clear();
        w.fleet = Some(Box::new(fleet));
        self.refresh_haul_fleet();
    }

    /// The fleet read again with the values as typed.
    pub(crate) fn refresh_haul_fleet(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.haul_plan_window.as_mut() else { return };
        let haul = w.haul.clone();
        let objective = if w.aim == Aim::Cost { HaulObjective::Cost } else { HaulObjective::Moment };
        let Some(x) = w.fleet.as_mut() else { return };
        let read = |x: &Fleet| -> Result<(HaulTrucked, Vec<String>), String> {
            let haul = haul.clone().ok_or("Taşıma planı penceresinde bir Brükner seçin.")?;
            let q = x.input(&haul, objective)?;
            let (out, warnings) = kentos_native_application::road_hauls_trucks::preview(&doc.model, &q)?;
            Ok((out, warnings.into_iter().map(|w| w.message).collect()))
        };
        let mut result = read(x);
        // The traced haul past the end (another diagram, another objective): the first.
        if matches!(&result, Err(e) if e.contains("sıralı taşıma yok")) {
            x.trace = 0;
            result = read(x);
        }
        x.warnings.clear();
        x.result = Some(match result {
            Ok((out, warnings)) => {
                x.warnings = warnings;
                Ok(out)
            }
            Err(e) => Err(e),
        });
    }

    /// An event of the window.
    pub(crate) fn haul_fleet_event(&mut self, event: Event) {
        let Some(w) = self.haul_plan_window.as_mut() else { return };
        let Some(x) = w.fleet.as_mut() else { return };
        match event {
            Event::Field(k, t) => {
                if let Some(f) = x.fields.get_mut(k) {
                    *f = t;
                }
            }
            Event::Road(r) => x.road = r,
            Event::Trace(i) => x.trace = i,
            Event::List(l) => {
                x.list = l;
                return;
            }
            Event::Back => {
                let mut kept = (**x).clone();
                kept.result = None;
                w.fleet_kept = Some(kept);
                w.fleet = None;
                return;
            }
        }
        self.refresh_haul_fleet();
    }

    /// The window over Taşıma planı's.
    pub(crate) fn haul_fleet_view<'a>(&'a self, x: &'a Fleet) -> Element<'a, Message> {
        let field = |k: usize, hint: &'a str| kentos_ui::widget::text_field(hint, &x.fields[k]).on_input(move |t| msg(Event::Field(k, t)));
        let ok = x.result.as_ref().and_then(|r| r.as_ref().ok());
        let mut form = Form::new()
            .label_width(150.0)
            .section("Kamyon")
            .field(FIELDS[0], field(0, ""))
            .field(FIELDS[1], field(1, ""))
            .help("Yüklü kütle boş kütle ile yükün toplamıdır.")
            .field(FIELDS[2], field(2, ""))
            .field(FIELDS[3], field(3, ""))
            .help("Motor gücünün tekerleğe varan payı (0 ile 1 arası).")
            .field(FIELDS[4], field(4, ""))
            .field(FIELDS[5], field(5, "yok"))
            .help("Yokuş aşağı itilen kamyonun hızı; boşsa en yüksek hız.")
            .field(FIELDS[6], field(6, ""))
            .field(FIELDS[7], field(7, ""))
            .help("Tutunma, çekiş katsayısı ile tahrik dingilindeki ağırlık payının çarpımıdır; eğimin direncinden azsa kamyon çıkamaz.")
            .field(FIELDS[8], field(8, ""))
            .section("Yol")
            .field("Yol", Segmented::new([Road::Profile, Road::Ground], x.road, |r| msg(Event::Road(r))))
            .help("Proje kotu koridorun boykesitidir; arazi kazıdan önceki yüzeydir.")
            .field(FIELDS[9], field(9, ""))
            .section("Çevrim")
            .field(FIELDS[10], field(10, ""))
            .help("Bir seferde taşınan hacim, Brüknerin hacmi gibi.")
            .field(FIELDS[11], field(11, ""))
            .field(FIELDS[12], field(12, ""))
            .field(FIELDS[13], field(13, "yok"))
            .section("Filo")
            .field(FIELDS[14], field(14, "yok"))
            .help("Verilince yükleyicideki üretim belirlenimci ve kuyruklu hesaplanır.")
            .field(FIELDS[15], field(15, "60"))
            .section("Yakıt")
            .field(FIELDS[16], field(16, "yok"))
            .field(FIELDS[17], field(17, "yok"))
            .field(FIELDS[18], field(18, "yok"))
            .help("Değerler kamyonun ve şantiyenin; KentOS sayı vermez.");
        if let Some(out) = ok
            && !out.hauls.is_empty()
        {
            let names: Vec<String> =
                out.hauls.iter().enumerate().map(|(i, h)| format!("{}: {} → {}", i + 1, h.from_text, h.to_text)).collect();
            let pick = Select::new(names.iter().map(|n| Choice::new(n.clone())), Some(x.trace.min(names.len() - 1)), |i| {
                msg(Event::Trace(i))
            });
            form = form.section("Grafik").field("İzlenen taşıma", pick);
        }
        let name = ok
            .and_then(|o| o.hauls.get(x.trace))
            .map_or_else(|| TITLE.to_owned(), |h| format!("Kamyon hızı — {} → {}", h.from_text, h.to_text));
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Back))
            .form_width(400.0)
            .width(1200.0)
            .close_caption(BACK)
            .picture(iced::widget::canvas(Speeds { out: ok }).width(Fill).height(Fill), 220.0)
            .picture_name(name)
            .legend("İzlenen taşımada kamyonun hızı (km/sa) km'ye göre: kırmızı yüklü gidiş, mavi boş dönüş");
        match &x.result {
            None => {}
            Some(Err(e)) => d = d.note(Banner::info(e.clone())),
            Some(Ok(out)) => {
                d = d.figure("Sefer", grouped(out.trips, 0));
                d = d.figure(
                    "Ortalama çevrim",
                    out.cycle.map_or_else(|| "—".into(), |c| format!("{} dk", comma(c / 60.0, 2))),
                );
                match &out.fleet {
                    Some(f) => {
                        d = d
                            .figure("Üretim", format!("{} m³/sa", grouped(f.queued, 0)))
                            .figure("Süre", format!("{} sa", comma(f.hours_queued, 1)))
                            .note(Banner::info(format!(
                                "{} kamyon: uyum katsayısı {}, yükleyici %{} dolu, kamyon yükleyicide çevrim başına {} dk bekler. Üretim belirlenimci {} m³/sa, kuyrukla {} m³/sa; gerçek süreler ikisinin arasındadır.",
                                f.trucks,
                                comma(f.match_factor, 2),
                                (f.busy * 100.0).round(),
                                comma(f.wait / 60.0, 1),
                                grouped(f.steady, 0),
                                grouped(f.queued, 0),
                            )));
                    }
                    None => {
                        d = d
                            .figure("Denge sayısı", out.balance.map_or_else(|| "—".into(), |b| comma(b, 2)))
                            .figure("Motor enerjisi", format!("{} kWh", grouped(out.energy, 0)));
                    }
                }
                for w in &x.warnings {
                    d = d.note(Banner::warning(w.clone()));
                }
                let hauls = road_dialog::table(
                    vec![
                        num("#", 26.0),
                        col("Baş km", 80.0),
                        col("Son km", 80.0),
                        num("Hacim m³", 74.0),
                        num("Uzaklık m", 70.0),
                        num("Kot farkı m", 78.0),
                        num("Yüklü s", 58.0),
                        num("Boş s", 52.0),
                        num("Çevrim dk", 70.0),
                        num("Sefer", 50.0),
                        num("Denge", 50.0),
                    ],
                    out.hauls
                        .iter()
                        .enumerate()
                        .map(|(i, h)| {
                            vec![
                                (i + 1).to_string(),
                                h.from_text.clone(),
                                h.to_text.clone(),
                                grouped(h.volume, 1),
                                grouped(h.distance, 1),
                                comma(h.rise, 2),
                                comma(h.loaded, 1),
                                comma(h.empty, 1),
                                comma(h.cycle / 60.0, 2),
                                grouped(h.trips, 0),
                                comma(h.balance, 2),
                            ]
                        })
                        .collect(),
                    "Planda taşıma yok: plan kurulamıyor ya da taşınacak bir şey yok",
                );
                let mut rows = Vec::new();
                if let Some(f) = &out.fleet {
                    rows.push(vec!["Kamyon".to_owned(), f.trucks.to_string()]);
                    rows.push(vec!["Uyum katsayısı".to_owned(), comma(f.match_factor, 2)]);
                    rows.push(vec!["Üretim, belirlenimci".to_owned(), format!("{} m³/sa", grouped(f.steady, 1))]);
                    rows.push(vec!["Üretim, kuyrukla".to_owned(), format!("{} m³/sa", grouped(f.queued, 1))]);
                    rows.push(vec!["Yükleyicinin doluluğu".to_owned(), format!("%{}", comma(f.busy * 100.0, 1))]);
                    rows.push(vec!["Yükleyicide bekleme".to_owned(), format!("{} dk/çevrim", comma(f.wait / 60.0, 2))]);
                    rows.push(vec!["Süre, belirlenimci".to_owned(), format!("{} sa", comma(f.hours_steady, 1))]);
                    rows.push(vec!["Süre, kuyrukla".to_owned(), format!("{} sa", comma(f.hours_queued, 1))]);
                }
                rows.push(vec!["Denge sayısı".to_owned(), out.balance.map_or_else(|| "—".into(), |b| comma(b, 2))]);
                rows.push(vec!["Motor enerjisi".to_owned(), format!("{} kWh", grouped(out.energy, 0))]);
                if let Some(fuel) = out.fuel {
                    rows.push(vec!["Yakıt".to_owned(), format!("{} L", grouped(fuel, 0))]);
                }
                if let Some(co2) = out.co2 {
                    rows.push(vec!["CO₂".to_owned(), format!("{} t", comma(co2 / 1000.0, 2))]);
                }
                let fleet = road_dialog::table(vec![col("Kalem", 220.0), num("Değer", 180.0)], rows, "");
                let shown = match x.list {
                    Listed::Hauls => hauls,
                    Listed::Fleet => fleet,
                };
                d = d.detail(road_dialog::detail_choice([Listed::Hauls, Listed::Fleet], x.list, |l| msg(Event::List(l)), shown));
            }
        }
        d.view()
    }
}

/// The chosen haul's speeds along the road: loaded from its start to its end, empty back.
struct Speeds<'a> {
    out: Option<&'a HaulTrucked>,
}

impl canvas::Program<Message> for Speeds<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let Some(trace) = self.out.and_then(|o| o.trace.as_ref()).filter(|tr| tr.loaded.len() > 1) else {
            crate::picture::label(
                &mut frame,
                "Kamyonun değerlerini yazınca izlenen taşımanın hızı burada çizilir.".into(),
                Point::new(16.0, 16.0),
                t.muted,
                false,
                None,
            );
            return vec![frame.into_geometry()];
        };
        let all = trace.loaded.iter().chain(&trace.empty);
        let (mut s0, mut s1, mut top) = (f64::INFINITY, f64::NEG_INFINITY, 1.0f64);
        for p in all {
            s0 = s0.min(p[0]);
            s1 = s1.max(p[0]);
            top = top.max(p[1]);
        }
        let top = top * 1.12;
        // Room on the right for the last km's label, centred on the plot's edge.
        let plot = Rectangle { x: 48.0, y: 12.0, width: bounds.width - 84.0, height: bounds.height - 36.0 };
        let x = |s: f64| plot.x + ((s - s0) / (s1 - s0).max(1e-9) * f64::from(plot.width)) as f32;
        let y = |v: f64| plot.y + plot.height - (v / top * f64::from(plot.height)) as f32;
        let shown = crate::picture::Shown::of(bounds.size());
        let thin = |c: iced::Color| Stroke::default().with_color(c).with_width(1.0);
        // The grid from the pixels (docs/adr/0937 §3): speeds at least 36 px apart, stations 80 px.
        let vstep = crate::picture::step(36.0 * top / f64::from(plot.height));
        let sstep = crate::picture::step(80.0 * (s1 - s0).max(1e-9) / f64::from(plot.width));
        let mut v = 0.0;
        let mut speeds = Vec::new();
        while v <= top {
            if shown.has_y(y(v)) {
                frame.stroke(&Path::line(Point::new(plot.x, y(v)), Point::new(plot.x + plot.width, y(v))), thin(t.border));
                speeds.push(v);
            }
            v += vstep;
        }
        let mut stations = Vec::new();
        let mut s = (s0 / sstep).ceil() * sstep;
        while s <= s1 + 1e-9 {
            if shown.has_x(x(s)) {
                frame.stroke(&Path::line(Point::new(x(s), plot.y), Point::new(x(s), plot.y + plot.height)), thin(t.border));
                stations.push(s);
            }
            s += sstep;
        }
        for (points, color) in [(&trace.loaded, t.danger), (&trace.empty, t.accent)] {
            let line = Path::new(|b| {
                for (i, p) in points.iter().enumerate() {
                    let at = Point::new(x(p[0]), y(p[1]));
                    if i == 0 { b.move_to(at) } else { b.line_to(at) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(color).with_width(2.0));
        }
        let left = shown.left(4.0);
        let base = shown.bottom(plot.y + plot.height + 6.0, kentos_ui::theme::typography::caption());
        let ground = t.field.scale_alpha(0.9);
        // Not into the stations' row at the bottom (a corner shared by both rows).
        for v in speeds.into_iter().filter(|v| y(*v) < base - 8.0) {
            crate::picture::label(&mut frame, comma(v, crate::picture::decimals(vstep)), Point::new(left, y(v) - 7.0), t.muted, false, (left > 4.5).then_some(ground));
        }
        for s in stations {
            crate::picture::label(&mut frame, format!("{} m", comma(s, crate::picture::decimals(sstep))), Point::new(x(s), base), t.muted, true, (base < plot.y + plot.height + 5.5).then_some(ground));
        }
        vec![frame.into_geometry()]
    }
}
