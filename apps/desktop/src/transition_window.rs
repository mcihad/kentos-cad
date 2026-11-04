//! Geçiş önerisi (plan/road.md Y34.3, docs/adr/0940 §4): opened from Dever on the chosen schedule as typed, it reads
//! `road.superelevations.transitions`: each curve's runoff by the relative gradient, the clothoid by comfort and by its
//! shift, the runout and the length at full superelevation, and whether consecutive curves' transitions fit the tangent
//! between them. The picture is the outside's cross slope along the alignment, now (dashed) and proposed (solid), the
//! tangents too short in red. Çizelgeye yaz puts the runoffs and shares into Dever's cells and goes back; Dever's Kaydet
//! writes them. The limits are the user's: KentOS fills in none (the template, speed and relative gradient are taken
//! from Dever's Denetim).
//!
//! ```text
//! ┌ Geçiş önerisi ──────────────────────────────────────────────────────────────────────── × ┐
//! │ Sınırlar ────────────────      ┌ outside cross slope along the alignment ─────────────┐ │
//! │  Tip kesit, En dik göreli      │  now dashed, proposed solid, short tangents red       │ │
//! │  eğim (1:n), Proje hızı,       └───────────────────────────────────────────────────────┘ │
//! │  Konfor C, Kaydırma en az/çok  [kurp] [en uzun geçiş] [en uzun klotoid] [bulgu]           │
//! │ Öneri ─ Teğet payı, Yuvarlama  PI · R · e · G · geçiş · klotoid · öneri · pay · tam dever │
//! │                                                                  [Geri] [■ Çizelgeye yaz] │
//! └────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Theme, mouse};
use kentos_commands::{SuperelevationsTransitioned, SuperelevationsTransitions};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form};

use crate::app::{App, Message};
use crate::road_dialog::{self, RoadDialog, col, num};
use crate::superelevation_window::{Event as Dever, comma_text, number};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Geçiş önerisi";
pub(crate) const APPLY: &str = "Çizelgeye yaz";
pub(crate) const BACK: &str = "Geri";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 7] = [
    "En dik göreli eğim (1:n)",
    "Proje hızı (km/sa)",
    "Konfor C (m/s³)",
    "En az kaydırma (m)",
    "En çok kaydırma (m)",
    "Teğet payı",
    "Yuvarlama (m)",
];

/// The window's state, kept on Dever's while it is open.
#[derive(Debug, Clone, Default)]
pub struct Transition {
    pub(crate) fields: [String; 7],
    pub(crate) template: Option<String>,
    pub(crate) result: Option<Result<SuperelevationsTransitioned, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Field(usize, String),
    Template(String),
    Apply,
    Back,
}

fn msg(e: Event) -> Message {
    Message::Superelevations(Dever::Transition(e))
}

impl Transition {
    /// The query from the typed fields; why not, in words.
    pub(crate) fn input(&self, superelevation: &str) -> Result<SuperelevationsTransitions, String> {
        let read = |k: usize| number(&self.fields[k]).map_err(|()| format!("{} bir sayı olmalı.", FIELDS[k]));
        let relative = read(0)?;
        if relative.is_some_and(|n| n <= 1.0) {
            return Err(format!("{}: 1'den büyük bir n yazın (1:200 için 200).", FIELDS[0]));
        }
        Ok(SuperelevationsTransitions {
            superelevation: superelevation.to_owned(),
            template: self.template.clone().ok_or("Bir tip kesit seçin: göreli eğim şeritlerin kenarında okunur.")?,
            most_relative: relative.map(|n| 1.0 / n),
            speed: read(1)?,
            comfort: read(2)?,
            shift: read(3)?,
            most_shift: read(4)?,
            on_tangent: read(5)?,
            step: read(6)?,
        })
    }
}

impl App {
    /// Geçiş öner… from Dever: the window on the chosen schedule, the template, the speed and the relative gradient
    /// from Dever's Denetim.
    pub(crate) fn open_transition(&mut self) {
        let Some(w) = self.superelevation_window.as_mut() else { return };
        let mut x = Transition { template: w.check_template.clone(), ..Transition::default() };
        x.fields[0] = w.check[2].clone();
        x.fields[1] = w.check[0].clone();
        w.transition = Some(Box::new(x));
        self.refresh_transition();
    }

    /// The proposal read again on the chosen schedule as typed.
    pub(crate) fn refresh_transition(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.superelevation_window.as_mut() else { return };
        let def = w.chosen.and_then(|i| w.rows.get(i)).map(|r| r.def());
        let Some(x) = w.transition.as_mut() else { return };
        x.warnings.clear();
        x.result = Some(match def {
            None => Err("Dever penceresinde bir çizelge seçin.".into()),
            Some(Err(e)) => Err(e),
            Some(Ok(def)) => x.input(&def.id).and_then(|q| {
                kentos_native_application::road_superelevations_transitions::preview(&doc.model, &def, &q).map(|(out, warnings)| {
                    x.warnings = warnings.into_iter().map(|w| w.message).collect();
                    out
                })
            }),
        });
    }

    /// An event of the window.
    pub(crate) fn transition_event(&mut self, event: Event) {
        let Some(w) = self.superelevation_window.as_mut() else { return };
        let Some(x) = w.transition.as_mut() else { return };
        match event {
            Event::Field(k, t) => {
                if let Some(f) = x.fields.get_mut(k) {
                    *f = t;
                }
            }
            Event::Template(t) => x.template = Some(t),
            Event::Back => {
                w.transition = None;
                return;
            }
            Event::Apply => {
                let Some(Ok(out)) = &x.result else { return };
                let proposed: Vec<(String, Option<f64>, Option<f64>)> =
                    out.rows.iter().map(|r| (r.pi.clone(), r.runoff, r.on_tangent)).collect();
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.set_transitions(&proposed);
                    w.transition = None;
                    self.refresh_superelevation();
                }
                return;
            }
        }
        self.refresh_transition();
    }

    /// The window over Dever's.
    pub(crate) fn transition_view<'a>(&'a self, x: &'a Transition) -> Element<'a, Message> {
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
        let form = Form::new()
            .label_width(170.0)
            .section("Geçiş boyu")
            .field("Tip kesit", pick)
            .help("Göreli eğim şeritlerin kenarında, çizelgenin dönme ekseniyle okunur.")
            .field(FIELDS[0], field(0, "yok"))
            .help("En kısa geçiş L·G/Δ: kurbun kendi geçişindeki en dik kenar eğimiyle.")
            .section("Klotoid")
            .field(FIELDS[1], field(1, "yok"))
            .field(FIELDS[2], field(2, "yok"))
            .help("Shortt: v·(v²/R − g·e)/C; karayolunda 1 ile 3 m/s³ arası değerler kullanılır.")
            .field(FIELDS[3], field(3, "yok"))
            .field(FIELDS[4], field(4, "yok"))
            .help("Kaydırma p ≈ L²/24R: en az ve en çok klotoid √(24·p·R).")
            .section("Öneri")
            .field(FIELDS[5], field(5, "şimdiki"))
            .help("Klotoidsiz kurpta geçişin kurptan önceki payı (0–1); klotoidli kurpta geçiş klotoidin üstündedir.")
            .field(FIELDS[6], field(6, "yok"))
            .help("Sınırlar standardınızındır; KentOS sayı vermez.");
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Back)).form_width(420.0).width(1200.0).close_caption(BACK);
        let ok = x.result.as_ref().and_then(|r| r.as_ref().ok());
        let normal = self
            .superelevation_window
            .as_ref()
            .and_then(|w| w.chosen.and_then(|i| w.rows.get(i)))
            .and_then(|r| r.def().ok())
            .map_or((-2.5, -2.5), |d| (d.normal_left, d.normal_right));
        d = d
            .picture(iced::widget::canvas(Ramps { out: ok, normal }).width(Fill).height(Fill), 220.0)
            .legend("Dış yanın enine eğimi (%) km boyunca: kesikli şimdiki, düz önerilen; kırmızı aralık geçişlerin sığmadığı teğet");
        match &x.result {
            Some(Err(e)) => d = d.note(Banner::info(e.clone())),
            Some(Ok(out)) => {
                d = d.figure("Kurp", out.rows.len().to_string());
                let longest = out.rows.iter().filter_map(|r| r.runoff).fold(0.0, f64::max);
                d = d.figure("En uzun geçiş", format!("{} m", comma_text(longest, 1)));
                let spiral = out.rows.iter().filter_map(|r| r.spiral).fold(0.0, f64::max);
                d = d.figure("En uzun klotoid", if spiral > 0.0 { format!("{} m", comma_text(spiral, 1)) } else { "—".into() });
                d = d.figure("Bulgu", out.findings.len().to_string());
                for warning in &x.warnings {
                    d = d.note(Banner::warning(warning.clone()));
                }
                for f in out.findings.iter().take(6) {
                    d = d.note(if f.kind == "full" || f.kind == "tangent" {
                        Banner::warning(f.message.clone())
                    } else {
                        Banner::info(f.message.clone())
                    });
                }
                let o = |v: Option<f64>, d: usize| v.map_or_else(|| "—".to_owned(), |v| comma_text(v, d));
                let rows = out
                    .rows
                    .iter()
                    .map(|r| {
                        vec![
                            r.pi.clone(),
                            comma_text(r.radius, 1),
                            comma_text(r.rate, 2),
                            r.gradient_now.map_or_else(|| "—".into(), |g| format!("1:{:.0}", 1.0 / g)),
                            o(r.runoff_gradient, 1),
                            o(r.spiral, 1),
                            format!("{} / {}", comma_text(r.spiral_in, 1), comma_text(r.spiral_out, 1)),
                            format!("{} → {}", comma_text(r.runoff_now, 1), o(r.runoff, 1)),
                            o(r.on_tangent, 2),
                            o(r.runout, 1),
                            o(r.full, 1),
                        ]
                    })
                    .collect();
                d = d.detail(road_dialog::sheet(
                    vec![
                        col("PI", 40.0),
                        num("R (m)", 60.0),
                        num("e (%)", 48.0),
                        num("G şimdi", 64.0),
                        num("Geçiş G'den", 84.0),
                        num("Klotoid en az", 92.0),
                        num("Klotoid (m)", 84.0),
                        num("Geçiş (m)", 96.0),
                        num("Pay", 44.0),
                        num("Runout", 60.0),
                        num("Tam dever", 72.0),
                    ],
                    rows,
                    "Çizelgede kurp yok",
                    160.0,
                ));
            }
            None => {}
        }
        let can = ok.is_some_and(|o| o.rows.iter().any(|r| r.runoff.is_some() || r.on_tangent.is_some()));
        d.primary(APPLY, can.then(|| msg(Event::Apply))).view()
    }
}

/// The outside's cross slope along the alignment, now and proposed, the tangents too short marked.
struct Ramps<'a> {
    out: Option<&'a SuperelevationsTransitioned>,
    normal: (f64, f64),
}

/// A curve's outside slope as break points (station, percent): normal crown → level crown → full and back.
fn ramp(ts: f64, st: f64, runoff: f64, share: f64, n: f64, e: f64) -> Vec<[f64; 2]> {
    let runout = runoff * n.abs() / e;
    let lc_in = ts - share * runoff;
    let fs_in = ts + (1.0 - share) * runoff;
    let (fs_out, lc_out) = (st - (1.0 - share) * runoff, st + share * runoff);
    vec![[lc_in - runout, n], [lc_in, 0.0], [fs_in, e], [fs_out, e], [lc_out, 0.0], [lc_out + runout, n]]
}

impl canvas::Program<Message> for Ramps<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let text = |frame: &mut canvas::Frame, s: String, at: Point, color: Color| {
            frame.fill_text(Text { content: s, position: at, color, size: Pixels(typography::caption()), font: typography::ui(), ..Text::default() })
        };
        let Some(out) = self.out.filter(|o| !o.rows.is_empty()) else {
            text(&mut frame, "Çizelgenin kurpları ve geçişleri burada çizilir.".into(), Point::new(16.0, 20.0), t.muted);
            return vec![frame.into_geometry()];
        };
        let n = self.normal.0.min(self.normal.1);
        let ramps: Vec<(Vec<[f64; 2]>, Vec<[f64; 2]>)> = out
            .rows
            .iter()
            .map(|r| {
                let now = ramp(r.from, r.to, r.runoff_now, r.on_tangent_now, n, r.rate);
                let new = ramp(r.from, r.to, r.runoff.unwrap_or(r.runoff_now), r.on_tangent.unwrap_or(r.on_tangent_now), n, r.rate);
                (now, new)
            })
            .collect();
        let (mut s0, mut s1) = (f64::INFINITY, f64::NEG_INFINITY);
        for (a, b) in &ramps {
            for p in a.iter().chain(b.iter()) {
                s0 = s0.min(p[0]);
                s1 = s1.max(p[0]);
            }
        }
        let top = out.rows.iter().map(|r| r.rate).fold(0.0, f64::max);
        let (z0, z1) = (n.min(0.0) - 0.5, top + 0.5);
        let (left, right, up, down) = (48.0, 12.0, 12.0, 24.0);
        let (w, h) = (bounds.width - left - right, bounds.height - up - down);
        let at = |s: f64, z: f64| {
            Point::new(left + ((s - s0) / (s1 - s0).max(1e-9)) as f32 * w, up + ((z1 - z) / (z1 - z0)) as f32 * h)
        };
        // Zero, the normal slope and the rates, as a grid.
        for z in [0.0, n] {
            let line = Path::line(at(s0, z), at(s1, z));
            frame.stroke(&line, Stroke::default().with_color(Color { a: 0.4, ..t.muted }).with_width(0.6));
            text(&mut frame, format!("%{}", comma_text(z, 1)), Point::new(6.0, at(s0, z).y - 7.0), t.muted);
        }
        // The stations, every round step that leaves 70 px between labels.
        let every = crate::picture::step(70.0 * (s1 - s0) / f64::from(w.max(1.0)));
        let mut s = (s0 / every).ceil() * every;
        while s <= s1 + 1e-9 {
            let p = at(s, z0);
            frame.stroke(&Path::line(Point::new(p.x, up + h), Point::new(p.x, up + h + 4.0)), Stroke::default().with_color(t.muted).with_width(0.8));
            text(&mut frame, comma_text(s, crate::picture::decimals(every)), Point::new(p.x - 12.0, up + h + 6.0), t.muted);
            s += every;
        }
        // The curves' extents.
        for r in &out.rows {
            let (a, b) = (at(r.from, z1), at(r.to, z0));
            frame.fill_rectangle(Point::new(a.x, up), iced::Size::new((b.x - a.x).max(1.0), h), Color { a: 0.08, ..t.accent });
            text(&mut frame, r.pi.clone(), Point::new(a.x + 3.0, up + 2.0), t.muted);
        }
        // The tangents too short: between the curves the finding names.
        let danger = Color { a: 0.18, ..t.danger };
        for f in out.findings.iter().filter(|f| f.kind == "tangent") {
            let mut ends = f.pi.split('–');
            let (Some(a), Some(b)) = (ends.next(), ends.next()) else { continue };
            let (Some(ra), Some(rb)) = (out.rows.iter().find(|r| r.pi == a), out.rows.iter().find(|r| r.pi == b)) else { continue };
            let (p, q) = (at(ra.to, z1), at(rb.from, z0));
            frame.fill_rectangle(Point::new(p.x, up), iced::Size::new((q.x - p.x).max(1.0), h), danger);
        }
        let stroke = |pts: &[[f64; 2]]| Path::new(|b| {
            for (i, p) in pts.iter().enumerate() {
                let q = at(p[0], p[1]);
                if i == 0 { b.move_to(q) } else { b.line_to(q) }
            }
        });
        for (now, new) in &ramps {
            frame.stroke(
                &stroke(now),
                Stroke { line_dash: LineDash { segments: &[5.0, 4.0], offset: 0 }, ..Stroke::default().with_color(t.muted).with_width(1.2) },
            );
            frame.stroke(&stroke(new), Stroke::default().with_color(t.accent).with_width(2.0));
        }
        vec![frame.into_geometry()]
    }
}
