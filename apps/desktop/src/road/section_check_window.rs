//! Kesit kontrolü (docs/adr/5120; `road.terrains.sectioncheck`, BÖHHBÜY m. 93(1)), opened from Pafta eğrileri. The
//! selected points with their own heights make the section: the two farthest apart are its ends (the control points of
//! known height), the others its points. The contour layer is chosen (Pafta eğrileri's layer first) and, if wanted, how
//! far off the line a point may lie. The summary says the RMS against 1.5 · KOH_Z and the gaps past 20 m; the table
//! lists the points along the section. Read only.

use crate::road;
use iced::widget::container;
use iced::{Element, Length, Task};
use kentos_commands::{CommandResult, Entity, TerrainsSectionCheck, TerrainsSectionChecked};
use kentos_native_application::{ExecutionContext, road_terrains_section};
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "Kesit kontrolü";
/// Pafta eğrileri's button that opens it.
pub const OPEN: &str = "Kesit kontrolü…";
const CLOSE: &str = "Kapat";
pub const CORRIDOR: &str = "Kesitten uzaklık (m)";

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The section's ends (the two selected height points farthest apart) and its points, by lasting id.
    pub from: String,
    pub to: String,
    pub points: Vec<String>,
    /// The drawing's layers: id and name; the contour layer chosen.
    pub layers: Vec<(String, String)>,
    pub contours: String,
    /// Empty: no limit, the distance only said.
    pub corridor: String,
    pub answer: Option<Result<(TerrainsSectionChecked, Vec<String>), String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Contours(String),
    Corridor(String),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Road(road::Event::SectionCheck(e))
}

/// A point's state in words.
pub fn state_word(state: &str) -> &'static str {
    match state {
        "compared" => "Karşılaştırıldı",
        "noContour" => "Bir yanında eğri yok",
        "outside" => "Kesitin dışında",
        "offLine" => "Kesitten uzak",
        _ => "—",
    }
}

impl App {
    fn section_check_run(&mut self) {
        let s = &self.road.section_check;
        let corridor = match s.corridor.trim() {
            "" => None,
            t => match t.replace(',', ".").parse::<f64>() {
                Ok(v) => Some(v),
                Err(_) => {
                    let why = format!("Kesitten uzaklık “{t}” bir sayı değil.");
                    self.road.section_check.answer = Some(Err(why));
                    return;
                }
            },
        };
        let input = TerrainsSectionCheck {
            from: s.from.clone(),
            to: s.to.clone(),
            points: s.points.clone(),
            contours: vec![s.contours.clone()],
            scale: None,
            corridor,
            rules: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        self.road.section_check.answer = Some(
            match road_terrains_section::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, warnings } => {
                    Ok((output, warnings.into_iter().map(|w| w.message).collect()))
                }
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Kesit kontrolü yapılamadı.".to_owned()),
            },
        );
    }

    /// Kesit kontrolü from the selection: its points with their own heights, the two farthest apart the ends.
    pub(crate) fn open_section_check(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let picked: Vec<(String, [f64; 2])> = self
            .selection
            .ids()
            .iter()
            .filter_map(|slot| {
                let Some(Entity::Point(p)) = doc.model.get(*slot) else { return None };
                p.z?;
                Some((doc.model.uid(*slot)?.to_string(), [p.p.x, p.p.y]))
            })
            .collect();
        if picked.len() < 3 {
            self.warn(
                "Kesit kontrolü: kendi kotu olan en az üç nokta seçin; en uzak ikisi kesitin uçları (yüksekliği bilinen \
                 kontrol noktaları), öbürleri kesit noktaları olur.",
            );
            return Task::none();
        }
        let (mut a, mut b, mut far) = (0, 1, -1.0);
        for i in 0..picked.len() {
            for j in i + 1..picked.len() {
                let d = (picked[i].1[0] - picked[j].1[0]).hypot(picked[i].1[1] - picked[j].1[1]);
                if d > far {
                    (a, b, far) = (i, j, d);
                }
            }
        }
        let layers: Vec<(String, String)> =
            doc.model.layers().leaves().iter().map(|n| (n.id.clone(), n.name.clone())).collect();
        // Pafta eğrileri's layer, else one named for contours, else the first.
        let named = layers.iter().find(|(_, n)| {
            let n = n.to_lowercase();
            ["eğri", "egri", "yükselti", "yukselti", "kontur"].iter().any(|w| n.contains(w))
        });
        let contours = Some(self.road.sheet_contours.layer.clone())
            .filter(|l| layers.iter().any(|(id, _)| id == l))
            .or_else(|| named.map(|l| l.0.clone()))
            .or_else(|| layers.first().map(|l| l.0.clone()))
            .unwrap_or_default();
        self.road.section_check = State {
            from: picked[a].0.clone(),
            to: picked[b].0.clone(),
            points: picked.iter().enumerate().filter(|(k, _)| *k != a && *k != b).map(|(_, p)| p.0.clone()).collect(),
            layers,
            contours,
            ..State::default()
        };
        self.section_check_run();
        self.dialog = Some(Dialog::Road(road::RoadDialogKind::SectionCheck));
        Task::none()
    }

    pub(crate) fn section_check_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => return self.open_section_check(),
            Event::Contours(v) => {
                self.road.section_check.contours = v;
                self.section_check_run();
            }
            Event::Corridor(v) => {
                self.road.section_check.corridor = v;
                self.section_check_run();
            }
            Event::Close => {
                self.road.section_check = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn section_check_view(&self) -> Element<'_, Message> {
        let s = &self.road.section_check;
        let f = self.format();
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        match &s.answer {
            Some(Ok((r, warnings))) => {
                lines.push(words::text_line(
                    Kind::Info,
                    format!(
                        "Kesit {} (uçların kotları {} ve {}); {} eğri kesişimi; 1/{}.",
                        f.length(r.length),
                        f.coord(r.ends[0]),
                        f.coord(r.ends[1]),
                        r.crossings.len(),
                        r.scale
                    ),
                ));
                match (r.rms, r.within) {
                    (Some(rms), Some(within)) => lines.push(words::text_line(
                        if within { Kind::Ok } else { Kind::Warn },
                        format!(
                            "Ölçülen − eğrilerden: {} noktada KOH {}, m. 93(1)(c) sınırı (1,5 · KOH_Z) {}: {}.",
                            r.compared,
                            f.length(rms),
                            f.length(r.limit),
                            if within { "altında" } else { "aşıyor" }
                        ),
                    )),
                    _ => lines.push(words::text_line(Kind::Warn, "Eğrilerden kotu okunan nokta yok.")),
                }
                if let Some(most) = r.spacing {
                    lines.push(if r.gaps.is_empty() {
                        words::text_line(Kind::Ok, format!("Komşu noktalar en çok {most} m arayla (m. 93(1)(b))."))
                    } else {
                        words::text_line(Kind::Warn, warnings.iter().find(|w| w.contains("komşu")).cloned().unwrap_or_default())
                    });
                }
                if r.unlevelled > 0 {
                    lines.push(words::text_line(
                        Kind::Info,
                        format!("Eğri katmanında her köşesi aynı kotta olmayan {} çizgi okunmadı.", r.unlevelled),
                    ));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Teknik denetimdir; uygunluk ya da kurum kabulü yerine geçmez. Noktaların kotu nivelmanla taşınmış olmalı (m. 93(1)(b)).",
                ));
                let maybe = |v: Option<f64>| v.map_or_else(|| "—".to_owned(), |x| f.coord(x));
                rows = r
                    .points
                    .iter()
                    .enumerate()
                    .map(|(k, p)| {
                        vec![
                            (k + 1).to_string(),
                            f.coord(p.along),
                            f.coord(p.offset),
                            f.coord(p.measured),
                            maybe(p.contour),
                            maybe(p.difference),
                            p.between.map_or_else(|| "—".to_owned(), |b| format!("{} – {}", f.coord(b[0]), f.coord(b[1]))),
                            state_word(&p.state).to_owned(),
                        ]
                    })
                    .collect();
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let text = |placeholder: &'static str, value: &str, on: fn(String) -> Event| {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |v| msg(on(v))).width(Length::Fixed(160.0))
        };
        let form = Form::new()
            .label_width(170.0)
            .field(
                "Kesit",
                words::text_line(
                    Kind::Info,
                    format!("Seçili noktaların en uzak ikisi uçlar; {} kesit noktası.", s.points.len()),
                ),
            )
            .field(
                "Eğri katmanı",
                container(crate::style::fields::select_owned(s.layers.clone(), &s.contours, |id| msg(Event::Contours(id)))).width(300),
            )
            .field(CORRIDOR, text("boş: sınır yok", &s.corridor, Event::Corridor))
            .help("Noktanın eğrilerden kotu, kesitin önünde ve ardında kestiği en yakın iki eğrinin kotları arasında doğrusal enterpolasyondur.");
        let table = sheet(
            vec![
                num("No", 32.0),
                num("Uzaklık", 90.0),
                num("Kesitten", 80.0),
                num("Ölçülen", 90.0),
                num("Eğrilerden", 90.0),
                num("Fark", 80.0),
                col("Arasındaki eğriler", 150.0),
                col("Durum", 150.0),
            ],
            rows,
            "Kesit noktası yok.",
            260.0,
        );
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(kentos_ui::label::caption("Eş yükselti eğrilerini iki kontrol noktası arasındaki kesitte ölçülen kotlarla denetler."))
            .push(form)
            .push(words::summary(lines))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        Frame::new(TITLE)
            .scroll(body)
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(900.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn section_check_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Fill(CORRIDOR, v) => Some(msg(Event::Corridor(v.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
