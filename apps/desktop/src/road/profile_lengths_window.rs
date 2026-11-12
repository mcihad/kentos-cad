//! Düşey kurp boyu (plan/road.md Y31.3, docs/adr/0935 §3): opened from Boykesit on the chosen profile as typed, it
//! reads `road.profiles.lengths` with the user's values: each PVI's least length by the stopping and passing sight, the
//! headlights, comfort and the bisector, against its length now. Kısaları uzat sets the short curves to the suggestion
//! in Boykesit's PVI table (a kink gets a parabola, a circle the radius of that K, an asymmetric curve both arms in
//! their ratio) and goes back; Boykesit's Kaydet writes them. The values are kept while the program runs.
//!
//! ```text
//! ┌ Düşey kurp boyu ──────────────────────────────────────────────────────────────────── × ┐
//! │ Hız ─ Proje hızı                │ ┌ each PVI: its length now (bar) and the length asked ┐ │
//! │ Durma görüşü ─ süre, yavaşlama   │ └──────────────────────────────────────────────────────┘ │
//! │ Tepe ─ göz, engel, geçme         │ [kurp] [kısa] [DGM] [en uzun gereken]                   │
//! │ Çukur ─ far, ışın, ivme          │ PVI · tür · A · şimdi · belirleyen · gereken · öneri    │
//! │ Öneri ─ bisektris, yuvarlama     │                                                          │
//! │                                                               [Geri] [■ Kısaları uzat]  │
//! └──────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Theme, mouse};
use kentos_commands::{RoadProfileLengths, RoadProfilesLengths};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Banner, Form, Switch};

use crate::app::{App, Message};
use crate::profile_window::Event as Profile;
use crate::road_dialog::{self, RoadDialog, col, num};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Düşey kurp boyu";
pub(crate) const APPLY: &str = "Kısaları uzat";
pub(crate) const BACK: &str = "Geri";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 11] = [
    "Proje hızı (km/sa)",
    "Tepki süresi (s)",
    "Yavaşlama (m/s²)",
    "Göz yüksekliği (m)",
    "Engel yüksekliği (m)",
    "Geçme görüşü (m)",
    "Far yüksekliği (m)",
    "Işın açısı (°)",
    "Düşey ivme (m/s²)",
    "Bisektris (m)",
    "Yuvarlama (m)",
];

/// The window's state, held by Boykesit's.
#[derive(Debug, Clone, Default)]
pub struct Lengths {
    pub(crate) fields: [String; 11],
    pub(crate) grades: bool,
    pub(crate) result: Option<Result<RoadProfileLengths, String>>,
    pub(crate) warnings: Vec<String>,
    /// The short curves Kısaları uzat cannot bring to the suggestion: there is no room between their PVIs.
    pub(crate) cut: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Field(usize, String),
    Grades(bool),
    Apply,
    Back,
}

fn msg(e: Event) -> Message {
    Message::Profiles(Profile::Lengths(e))
}

/// A number the user typed, with a comma or a point; empty: none.
fn number(text: &str) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| ())
}

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

impl Lengths {
    /// The query from the typed fields; why not, in words.
    pub(crate) fn input(&self, profile: &str) -> Result<RoadProfilesLengths, String> {
        let read = |k: usize| number(&self.fields[k]).map_err(|()| format!("{} bir sayı olmalı.", FIELDS[k]));
        // Empty: the alignment's design speed (docs/adr/0986); the command says so when it has none.
        let speed = read(0)?;
        Ok(RoadProfilesLengths {
            profile: profile.to_owned(),
            speed,
            time: read(1)?,
            decel: read(2)?,
            grades: self.grades,
            eye: read(3)?,
            object: read(4)?,
            passing: read(5)?,
            headlight: read(6)?,
            beam: read(7)?,
            accel: read(8)?,
            external: read(9)?,
            step: read(10)?,
        })
    }
}

impl App {
    /// Kurp boyu öner… from Boykesit: the window on the chosen profile, the values typed before kept.
    pub(crate) fn open_profile_lengths(&mut self) {
        let Some(w) = self.profile_window.as_mut() else { return };
        let kept = w.lengths_kept.clone().unwrap_or_default();
        w.lengths = Some(Box::new(Lengths { fields: kept.0, grades: kept.1, ..Lengths::default() }));
        self.refresh_profile_lengths();
    }

    /// The suggestion read again on the chosen profile as typed.
    pub(crate) fn refresh_profile_lengths(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.profile_window.as_mut() else { return };
        let def = w.chosen_def();
        // Kısaları uzat tried on a copy, to say before it is pressed which suggestions do not fit.
        let mut trial = w.clone();
        trial.lengths = None;
        let Some(x) = w.lengths.as_mut() else { return };
        x.warnings.clear();
        x.cut.clear();
        x.result = Some(match def {
            None => Err("Boykesit penceresinde bir boykesit seçin.".into()),
            Some(Err(e)) => Err(e),
            Some(Ok(def)) => x.input(&def.id).and_then(|q| {
                kentos_native_application::road_profiles_lengths::preview(&doc.model, &def, &q).map(|(out, warnings)| {
                    x.warnings = warnings.into_iter().map(|w| w.message).collect();
                    out
                })
            }),
        });
        if let Some(Ok(out)) = &x.result {
            let short: Vec<(String, f64, f64)> =
                out.curves.iter().filter(|c| c.short).map(|c| (c.pvi.clone(), c.suggested, c.a)).collect();
            x.cut = trial.lengthen(&short).1;
        }
    }

    /// An event of the window.
    pub(crate) fn profile_lengths_event(&mut self, event: Event) {
        let Some(w) = self.profile_window.as_mut() else { return };
        let Some(x) = w.lengths.as_mut() else { return };
        match event {
            Event::Field(k, t) => {
                if let Some(f) = x.fields.get_mut(k) {
                    *f = t;
                }
            }
            Event::Grades(on) => x.grades = on,
            Event::Back => {
                w.lengths_kept = Some((x.fields.clone(), x.grades));
                w.lengths = None;
                return;
            }
            Event::Apply => {
                let Some(Ok(out)) = &x.result else { return };
                let short: Vec<(String, f64, f64)> =
                    out.curves.iter().filter(|c| c.short).map(|c| (c.pvi.clone(), c.suggested, c.a)).collect();
                w.lengths_kept = Some((x.fields.clone(), x.grades));
                w.lengths = None;
                let (n, cut) = w.lengthen(&short);
                if cut.is_empty() {
                    self.output(format!("{n} kurp önerilen boya uzatıldı; Kaydet ile yazılır."));
                } else {
                    self.warn(format!(
                        "{n} kurp uzatıldı; {} PVI'lar arasına sığdığı kadar uzatıldı, öneriye varmadı: PVI'ları uzaklaştırın ya da hızı düşürün. Kaydet ile yazılır.",
                        cut.join(", ")
                    ));
                }
                return;
            }
        }
        self.refresh_profile_lengths();
    }

    /// The window over Boykesit's.
    pub(crate) fn profile_lengths_view<'a>(&'a self, x: &'a Lengths) -> Element<'a, Message> {
        let field = |k: usize, hint: &'a str| kentos_ui::widget::text_field(hint, &x.fields[k]).on_input(move |t| msg(Event::Field(k, t)));
        let form = Form::new()
            .label_width(140.0)
            .section("Hız")
            .field(FIELDS[0], field(0, ""))
            .section("Durma görüşü")
            .field(FIELDS[1], field(1, "yok"))
            .field(FIELDS[2], field(2, "yok"))
            .help("Algılama-tepki süresi ve frenleme yavaşlaması; standartlar bunları sürücülerin %90'ının yüzdeliğinden alır.")
            .field("Eğim", Switch::new(x.grades, |on| msg(Event::Grades(on))).label("Yaklaşma eğimleriyle"))
            .help("Açıkken her kurpta iki yönün yaklaşma eğimi hesaba girer, uzun olan alınır; kapalıyken düz yol.")
            .section("Tepe kurbu")
            .field(FIELDS[3], field(3, "yok"))
            .field(FIELDS[4], field(4, "yok"))
            .field(FIELDS[5], field(5, "yok"))
            .help("Geçme görüşünde karşıdan gelen araç göz yüksekliğindedir.")
            .section("Çukur kurbu")
            .field(FIELDS[6], field(6, "yok"))
            .field(FIELDS[7], field(7, "yok"))
            .field(FIELDS[8], field(8, "yok"))
            .help("Far görüşü durma mesafesi kadar ileriyi aydınlatmalı; düşey ivme konforun sınırıdır.")
            .section("Öneri")
            .field(FIELDS[9], field(9, "yok"))
            .field(FIELDS[10], field(10, "yok"))
            .help("Değerler standardınızındır; KentOS sayı vermez.");
        let ok = x.result.as_ref().and_then(|r| r.as_ref().ok());
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Back))
            .form_width(360.0)
            .width(1180.0)
            .close_caption(BACK)
            .picture(iced::widget::canvas(Bars { out: ok }).width(Fill).height(Fill), 200.0)
            .legend("Her PVI'de kurbun şimdiki boyu (çubuk) ve ölçütlerin istediği boy (çizgi); kısa olanlar kırmızı");
        match &x.result {
            Some(Err(e)) => d = d.note(Banner::info(e.clone())),
            Some(Ok(out)) => {
                let short = out.curves.iter().filter(|c| c.short).count();
                d = d
                    .figure("Kurp", out.curves.len().to_string())
                    .figure("Kısa", short.to_string())
                    .figure("Durma görüşü", out.stopping.map_or_else(|| "—".into(), |s| format!("{} m", comma(s, 1))))
                    .figure(
                        "En uzun gereken",
                        format!("{} m", comma(out.curves.iter().map(|c| c.required).fold(0.0, f64::max), 1)),
                    );
                for w in &x.warnings {
                    d = d.note(Banner::warning(w.clone()));
                }
                if !x.cut.is_empty() {
                    d = d.note(Banner::warning(format!(
                        "{} için önerilen boy komşu PVI'lara kadar sığmıyor: Kısaları uzat sığdığı kadar uzatır. Öneriye varmak için PVI'ları uzaklaştırın.",
                        x.cut.join(", ")
                    )));
                }
                let kind = |k: &str| match k {
                    "stopping" => "durma görüşü",
                    "passing" => "geçme görüşü",
                    "headlight" => "far",
                    "comfort" => "konfor",
                    _ => "bisektris",
                };
                let rows = out
                    .curves
                    .iter()
                    .map(|c| {
                        vec![
                            c.pvi.clone(),
                            if c.crest { "tepe".into() } else { "çukur".into() },
                            comma(c.a, 2),
                            c.length.map_or_else(|| "kırık".into(), |l| comma(l, 1)),
                            c.governing.as_deref().map_or_else(|| "—".into(), |k| kind(k).to_owned()),
                            comma(c.required, 1),
                            if x.cut.contains(&c.pvi) { format!("{} sığmaz", comma(c.suggested, 1)) } else { comma(c.suggested, 1) },
                        ]
                    })
                    .collect();
                d = d.detail(road_dialog::table(
                    vec![
                        col("PVI", 40.0),
                        col("Tür", 50.0),
                        num("A (%)", 56.0),
                        num("Şimdi (m)", 70.0),
                        col("Belirleyen", 96.0),
                        num("Gereken (m)", 84.0),
                        num("Öneri (m)", 112.0),
                    ],
                    rows,
                    "Boykesitte kırılan PVI yok",
                ));
            }
            None => {}
        }
        let can = ok.is_some_and(|o| o.curves.iter().any(|c| c.short));
        d.primary(APPLY, can.then(|| msg(Event::Apply))).view()
    }
}

/// Each PVI's length now (a bar) and the length its criteria ask (a line across it); short ones in the danger colour.
struct Bars<'a> {
    out: Option<&'a RoadProfileLengths>,
}

impl canvas::Program<Message> for Bars<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let word = |frame: &mut canvas::Frame, s: String, at: Point, color: Color| {
            frame.fill_text(Text { content: s, position: at, color, size: Pixels(typography::caption()), font: typography::ui(), ..Text::default() });
        };
        let Some(out) = self.out.filter(|o| !o.curves.is_empty()) else {
            word(&mut frame, "Hızı ve en az bir ölçütü yazınca kurpların boyları burada karşılaştırılır.".into(), Point::new(16.0, 20.0), t.muted);
            return vec![frame.into_geometry()];
        };
        let top = out.curves.iter().map(|c| c.required.max(c.length.unwrap_or(0.0))).fold(1.0, f64::max) * 1.1;
        let plot = Rectangle { x: 48.0, y: 12.0, width: bounds.width - 60.0, height: bounds.height - 34.0 };
        let y = |v: f64| plot.y + plot.height * (1.0 - (v / top) as f32);
        let step = [10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0].into_iter().find(|s| top / s <= 6.0).unwrap_or(2000.0);
        let mut v = 0.0;
        while v <= top {
            frame.stroke(&Path::line(Point::new(plot.x, y(v)), Point::new(plot.x + plot.width, y(v))), Stroke::default().with_color(t.border).with_width(1.0));
            word(&mut frame, format!("{v:.0}"), Point::new(4.0, y(v) - 7.0), t.muted);
            v += step;
        }
        let n = out.curves.len() as f32;
        let slot = plot.width / n;
        for (i, c) in out.curves.iter().enumerate() {
            let cx = plot.x + slot * (i as f32 + 0.5);
            let w = (slot * 0.4).min(48.0);
            let now = c.length.unwrap_or(0.0);
            let color = if c.short { t.danger } else { t.success };
            frame.fill_rectangle(Point::new(cx - w / 2.0, y(now)), iced::Size::new(w, y(0.0) - y(now)), color.scale_alpha(0.5));
            let need = y(c.required);
            frame.stroke(&Path::line(Point::new(cx - w * 0.8, need), Point::new(cx + w * 0.8, need)), Stroke::default().with_color(t.text).with_width(2.0));
            // The length asked right of its line.
            word(&mut frame, format!("{} m", comma(c.required, 1)), Point::new(cx + w * 0.8 + 4.0, need - 7.0), t.text);
            // The length now on the bar's left, set right: the asked one is on the right, the two never meet.
            let now_words = c.length.map_or_else(|| "kırık".to_owned(), |l| format!("{} m", comma(l, 1)));
            frame.fill_text(Text {
                content: now_words,
                position: Point::new(cx - w * 0.8 - 4.0, (y(now) - 7.0).min(y(0.0) - 14.0)),
                color: t.muted,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                align_x: iced::widget::text::Alignment::Right,
                ..Text::default()
            });
            let kind = if c.crest { "tepe" } else { "çukur" };
            word(&mut frame, format!("{} · {kind}", c.pvi), Point::new(cx - 18.0, plot.y + plot.height + 4.0), t.text);
        }
        vec![frame.into_geometry()]
    }
}
