//! Sınır grafiği (docs/adr/2000; `cad.parcels.boundaries`): the parcels' boundaries as a graph, read only, opened from
//! Parsel alan denetimi on the same parcels (its scope: the selected areas, else the parcel layer's). The summary
//! counts the corners and the boundaries, shared and outer, and the problems by kind; a choice shows the boundaries
//! (their corners, length, the parcel on each side), the corners (their place and parcels) or the problems (where and
//! which parcels); Sorunluları seç selects the parcels a problem names; Çizimde göster closes the window and leaves the
//! graph's marks on the drawing (parcel_boundaries_marks.rs, docs/adr/2220). A row pressed is chosen: Çizimde göster
//! then goes to it and the marks bring it forward (docs/adr/3020). Nothing is written.

use iced::{Element, Task};
use kentos_commands::{ParcelsBoundaries, ParcelsBounded};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::parcels_boundaries;
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet_chosen};
use crate::traces::Control;

pub const TITLE: &str = "Sınır grafiği";
/// Parsel alan denetimi's button that opens it.
pub const OPEN: &str = "Sınır grafiği…";
const CLOSE: &str = "Kapat";
const SELECT: &str = "Sorunluları seç";
pub const SHOW: &str = "Çizimde göster";

/// Which rows the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    Boundaries,
    Corners,
    Problems,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::Boundaries => "Sınırlar",
            Show::Corners => "Köşeler",
            Show::Problems => "Sorunlar",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub uids: Option<Vec<String>>,
    pub show: Show,
    pub answer: Option<Result<ParcelsBounded, String>>,
    /// The row pressed, in the table shown (docs/adr/3020).
    pub chosen: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Show(Show),
    /// A row of the table pressed: Çizimde göster goes to it (docs/adr/3020).
    Choose(usize),
    Select,
    /// Çizimde göster (docs/adr/2220).
    Draw,
    Close,
}

fn msg(e: Event) -> Message {
    Message::ParcelBoundaries(e)
}

/// A problem's kind in words.
pub fn problem_word(kind: &str) -> &'static str {
    match kind {
        "missingVertex" => "Komşuda eksik köşe",
        "splitCorner" => "Ayrık köşe (1 µm'den yakın)",
        "overlap" => "Çakışan kenar",
        // docs/adr/2620: one side an arc, the other its chord (or another arc).
        "arcChord" => "Yay ile kiriş uyuşmuyor",
        _ => "Sorun",
    }
}

/// A problem's kind in few words, as the drawing writes it beside its ring (docs/adr/2220).
pub fn problem_short(kind: &str) -> &'static str {
    match kind {
        "missingVertex" => "Komşuda eksik köşe",
        "splitCorner" => "Ayrık köşe",
        "overlap" => "Çakışan kenar",
        "arcChord" => "Yay ile kiriş",
        _ => "Sorun",
    }
}

/// A problem's size (docs/adr/2620): an arc and chord mismatch's boundaries, its sliver's area and how far apart the
/// two curves are at their middles; none for the others.
pub fn problem_size(p: &kentos_commands::BoundaryProblem, f: &kentos_interaction::Format) -> String {
    match (p.area, p.gap) {
        (Some(area), Some(gap)) => format!("{} · {}, ortada {}", p.boundaries.join(" ile "), f.area(area), f.length(gap)),
        _ => "—".into(),
    }
}

/// A parcel by its identity, else the first characters of its id.
fn named(g: &ParcelsBounded, uid: &str) -> String {
    g.parcels
        .iter()
        .find(|p| p.uid == uid)
        .map(|p| if p.name.is_empty() { format!("#{}", &uid[uid.len().saturating_sub(4)..]) } else { p.name.clone() })
        .unwrap_or_else(|| uid.to_owned())
}

/// The table's rows for `show`.
pub fn rows(g: &ParcelsBounded, show: Show, f: &kentos_interaction::Format) -> Vec<Vec<String>> {
    let names = |list: &[String]| list.iter().map(|u| named(g, u)).collect::<Vec<_>>().join(", ");
    match show {
        Show::Boundaries => g
            .boundaries
            .iter()
            .map(|b| {
                vec![
                    b.id.clone(),
                    b.from.clone(),
                    b.to.clone(),
                    f.length(b.length),
                    if b.bulge == 0.0 { "doğru".into() } else { "yay".into() },
                    b.left.as_deref().map_or_else(|| "—".to_owned(), |u| named(g, u)),
                    b.right.as_deref().map_or_else(|| "—".to_owned(), |u| named(g, u)),
                ]
            })
            .collect(),
        Show::Corners => {
            g.corners.iter().map(|c| vec![c.id.clone(), f.coord(c.at[0]), f.coord(c.at[1]), names(&c.parcels)]).collect()
        }
        Show::Problems => g
            .problems
            .iter()
            .map(|p| {
                vec![
                    problem_word(&p.kind).to_owned(),
                    f.coord(p.at[0]),
                    f.coord(p.at[1]),
                    names(&p.parcels),
                    problem_size(p, f),
                ]
            })
            .collect(),
    }
}

impl App {
    fn parcel_boundaries_run(&mut self) {
        let input = ParcelsBoundaries { uids: self.parcel_boundaries.uids.clone() };
        let Some(doc) = self.document.as_ref() else { return };
        self.parcel_boundaries.answer = Some(parcels_boundaries::preview(&doc.model, &input));
    }

    /// Sınır grafiği from Parsel alan denetimi: its parcels.
    pub(crate) fn open_parcel_boundaries(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.parcel_boundaries = State { uids: self.parcels_check.uids.clone(), ..State::default() };
        self.parcel_boundaries_run();
        self.dialog = Some(Dialog::ParcelBoundaries);
        Task::none()
    }

    pub(crate) fn parcel_boundaries_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => return self.open_parcel_boundaries(),
            Event::Show(v) => {
                self.parcel_boundaries.show = v;
                self.parcel_boundaries.chosen = None;
            }
            Event::Choose(i) => self.parcel_boundaries.chosen = Some(i),
            Event::Select => {
                let Some(Ok(g)) = &self.parcel_boundaries.answer else { return Task::none() };
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let mut slots: Vec<Slot> = Vec::new();
                for u in g.problems.iter().flat_map(|p| p.parcels.iter()) {
                    if let Some(s) = Uuid::parse_str(u).ok().and_then(|u| doc.model.slot_of(u))
                        && !slots.contains(&s)
                    {
                        slots.push(s);
                    }
                }
                let n = slots.len();
                self.selection.set(slots);
                self.parcel_boundaries = State::default();
                self.dialog = None;
                self.say(Level::Info, format!("{TITLE}: sorunlu {n} parsel seçildi."));
            }
            Event::Draw => {
                let Some(Ok(g)) = &self.parcel_boundaries.answer else { return Task::none() };
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let (corners, problems) = (g.corners.len(), g.problems.len());
                // The view on the corners, a tenth wider each way; on the chosen row's place when one is (docs/adr/3020).
                let focus = self.parcel_boundaries.chosen.and_then(|i| crate::parcel_boundaries_marks::Focus::of(g, self.parcel_boundaries.show, i));
                let all = g.corners.iter().fold(None::<[f64; 4]>, |b, c| {
                    let [x, y] = c.at;
                    Some(b.map_or([x, y, x, y], |b| [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]))
                });
                let bounds = focus.as_ref().and_then(|f| f.bounds(g, all)).or(all);
                let mut shown = crate::parcel_boundaries_marks::Shown::new(doc.session, self.parcel_boundaries.uids.clone());
                shown.focus = focus;
                self.parcel_graph = Some(shown);
                self.parcel_boundaries = State::default();
                self.dialog = None;
                if let Some([x0, y0, x1, y1]) = bounds {
                    let (mx, my) = ((x1 - x0).max(1.0) * 0.1, (y1 - y0).max(1.0) * 0.1);
                    let b = kentos_commands::Bounds { min_x: x0 - mx, min_y: y0 - my, max_x: x1 + mx, max_y: y1 + my };
                    self.navigating(|app| app.zoom_to_bounds(&b));
                }
                self.say(
                    Level::Info,
                    format!("{TITLE} çizimde: {corners} köşe, {problems} sorun. İşareti kaldır işaretleri siler."),
                );
            }
            Event::Close => {
                self.parcel_boundaries = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn parcel_boundaries_view(&self) -> Element<'_, Message> {
        let s = &self.parcel_boundaries;
        let f = self.format();
        let mut lines = Vec::new();
        let mut table_rows = Vec::new();
        let mut problems = 0;
        let mut drawable = false;
        match &s.answer {
            Some(Ok(g)) => {
                problems = g.problems.len();
                drawable = !g.corners.is_empty();
                lines.push(words::text_line(
                    if problems == 0 { Kind::Ok } else { Kind::Warn },
                    format!(
                        "{} parsel, {} köşe, {} sınır: {} ortak, {} dış.",
                        g.parcels.len(),
                        g.corners.len(),
                        g.boundaries.len(),
                        g.shared,
                        g.outer
                    ),
                ));
                if problems > 0 {
                    let by = |k: &str| g.problems.iter().filter(|p| p.kind == k).count();
                    lines.push(words::text_line(
                        Kind::Error,
                        format!(
                            "{problems} sorun: {} komşuda eksik köşe, {} ayrık köşe, {} çakışan kenar, {} yay ile kiriş uyuşmazlığı. Ortak sınır yalnız koordinatların çakışmasıyla değil, iki parselin aynı sınırıyla kurulur.",
                            by("missingVertex"),
                            by("splitCorner"),
                            by("overlap"),
                            by("arcChord")
                        ),
                    ));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Kimlikler geometriden türetilir (K doğu sonra kuzey sırasıyla, S köşelerine göre); aynı geometri aynı kimlikleri verir. Çizime bir şey yazılmaz; Çizimde göster numaraları ve sorunları yalnız ekranda işaretler, tabloda seçilen satıra gidip onu öne çıkarır.",
                ));
                table_rows = rows(g, s.show, &f);
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let cols = match s.show {
            Show::Boundaries => vec![
                col("Sınır", 60.0),
                col("Başı", 60.0),
                col("Sonu", 60.0),
                num("Boy", 90.0),
                col("Biçim", 60.0),
                col("Solda", 220.0),
                col("Sağda", 220.0),
            ],
            Show::Corners => vec![col("Köşe", 60.0), num("Y", 130.0), num("X", 130.0), col("Parseller", 420.0)],
            Show::Problems => vec![
                col("Sorun", 200.0),
                num("Y", 120.0),
                num("X", 120.0),
                col("Parseller", 230.0),
                col("Fark", 230.0),
            ],
        };
        let table = sheet_chosen(cols, table_rows, "Gösterilecek satır yok.", 300.0, s.chosen, |i| msg(Event::Choose(i)));
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(words::summary(lines))
            .push(Segmented::new([Show::Boundaries, Show::Corners, Show::Problems], s.show, |v| msg(Event::Show(v))))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(SELECT, (problems > 0).then(|| msg(Event::Select))))
            .aside(words::secondary(SHOW, drawable.then(|| msg(Event::Draw))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn parcel_boundaries_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            Control::Press(SHOW) => Some(msg(Event::Draw)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
