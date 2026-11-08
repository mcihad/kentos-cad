//! Yeniden izdüşür's preview (plan/map-cad.md MAP-02, docs/adr/2500): a layer's Koordinat sistemi ▸ Yeniden
//! izdüşür ▸, or the command line's Yeniden izdüşür from the active layer's record, opens it on
//! `cad.layers.reproject`'s plan — the way and its accuracy, the chain's verdict and its reasons, every point's shift
//! summed up, the objects whose points fall outside an EPSG area of use, and each object's shift in the kit's table
//! (those with a note first). Uygula writes the move as one undo step; Vazgeç writes nothing. A layer some of whose
//! objects do not move says why, each reason counted, and Nesneleri seç selects them.

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{ChainStatus, CommandResult, CommandWarning, LayersReproject, LayersReprojected};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_native_application::layers_crs::reproject;
use kentos_ui::widget::Dialog as Frame;

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "Yeniden izdüşür";
const APPLY: &str = "Uygula";
const CANCEL: &str = "Vazgeç";
const SELECT: &str = "Nesneleri seç";

/// The window: the layer, the system its objects are in (none: its record's), and the plan or why it is refused.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub layer: String,
    pub name: String,
    pub from: Option<u32>,
    pub plan: Option<Result<(LayersReprojected, Vec<CommandWarning>), Refused>>,
}

/// Why the layer does not move, and the objects that do not.
#[derive(Clone, Debug, PartialEq)]
pub struct Refused {
    pub message: String,
    pub uids: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Apply,
    Select,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Reproject(e)
}

/// A note as the table's Kullanım alanı dışı column says it: whose area of use the object's points leave.
fn note_word(note: &str) -> &'static str {
    match note {
        "outside_source_area" => "kaynak",
        "outside_operation_area" => "dönüşüm",
        _ => "proje",
    }
}

impl App {
    fn reproject_input(&self) -> LayersReproject {
        LayersReproject {
            layer: self.reproject.layer.clone(),
            from: self.reproject.from,
            expected_revision: None,
        }
    }

    /// Yeniden izdüşür of a layer from `from` (none: its record's), previewed: the command's plan, nothing written.
    pub(crate) fn open_reproject(&mut self, layer: String, from: Option<u32>) -> Task<Message> {
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let name = doc
            .model
            .layers()
            .get(&layer)
            .map_or_else(|| layer.clone(), |n| n.name.clone());
        self.reproject = State {
            layer,
            name,
            from,
            plan: None,
        };
        let input = self.reproject_input();
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        self.reproject.plan = Some(
            match reproject::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(Refused {
                    message: error.message,
                    uids: error.uids,
                }),
                _ => Err(Refused {
                    message: "Önizleme alınamadı.".to_owned(),
                    uids: Vec::new(),
                }),
            },
        );
        self.dialog = Some(Dialog::Reproject);
        Task::none()
    }

    pub(crate) fn reproject_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Apply => {
                let input = self.reproject_input();
                let name = self.reproject.name.clone();
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                match reproject::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    // The warnings were the preview's; the window says them before the move.
                    CommandResult::Completed { output: out, .. } => {
                        self.say(
                            Level::Success,
                            format!(
                                "“{name}” katmanının {} nesnesi EPSG:{}'den projenin sistemine (EPSG:{}) çevrildi{} (tek adımda geri alınır).",
                                out.moved,
                                out.from,
                                out.to,
                                if out.via.is_empty() { String::new() } else { format!(": {}", out.via) }
                            ),
                        );
                        self.reproject = State::default();
                        self.dialog = None;
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message))
                    }
                    _ => {}
                }
            }
            Event::Select => {
                let uids = match &self.reproject.plan {
                    Some(Err(r)) => r.uids.clone(),
                    _ => Vec::new(),
                };
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                let slots: Vec<Slot> = uids
                    .iter()
                    .filter_map(|u| Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
                self.selection.set(slots);
                self.reproject = State::default();
                self.dialog = None;
                if !ids.is_empty() {
                    self.navigating(|app| app.zoom_to_objects(&ids));
                }
            }
            Event::Close => {
                self.reproject = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// The lines over the table and its rows.
    fn reproject_lines(&self) -> (Vec<Element<'_, Message>>, Vec<Vec<String>>) {
        let s = &self.reproject;
        let f = self.format();
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let (out, warnings) = match &s.plan {
            Some(Ok(p)) => p,
            Some(Err(r)) => {
                lines.push(words::text_line(Kind::Warn, r.message.clone()));
                if !r.uids.is_empty() {
                    lines.push(words::text_line(
                        Kind::Info,
                        "Katmanın Koordinat sistemi ▸ Yeniden izdüşür ▸ menüsünden başka bir sistem seçin ya da Nesneleri seç ile çevrilemeyenleri seçip başka bir katmana alın.",
                    ));
                }
                return (lines, rows);
            }
            None => {
                lines.push(words::text_line(Kind::Info, "Önizleme yok."));
                return (lines, rows);
            }
        };
        let project = kentos_project::crs::system(out.to)
            .map_or_else(|| format!("EPSG:{}", out.to), |p| format!("{}, EPSG:{}", p.name, out.to));
        lines.push(words::text_line(
            Kind::Info,
            format!(
                "“{}” katmanının nesneleri {} (EPSG:{}) sisteminden projenin sistemine ({project}) çevrilir. Önizleme çizime hiçbir şey yazmaz; Uygula tek geri alma adımıdır.",
                s.name, out.crs.name, out.from
            ),
        ));
        lines.push(words::text_line(
            Kind::Info,
            if out.via.is_empty() {
                "Katmanda çevrilecek nesne yok; yalnız kaynak sistemi kaydı yazılır.".to_owned()
            } else {
                format!("Yol: {}.", out.via)
            },
        ));
        // A limited chain says its reasons in its warning, below.
        if out.status == Some(ChainStatus::Complete) {
            lines.push(words::text_line(
                Kind::Info,
                "Dönüşüm zinciri tam: adımların doğruluğu söylenmiş, epok ve datum belirli.",
            ));
        }
        for w in warnings {
            lines.push(words::text_line(Kind::Warn, w.message.clone()));
        }
        if let (Some(mx), Some(my), Some(lo), Some(hi)) = (out.mean_dx, out.mean_dy, out.least, out.most) {
            lines.push(words::text_line(
                Kind::Info,
                format!(
                    "{} nesne, {} nokta kayar: ortalama kayma d{} {}, d{} {}; en küçüğü {}, en büyüğü {}.",
                    out.moved,
                    out.points,
                    f.east_label(),
                    f.coord(mx),
                    f.north_label(),
                    f.coord(my),
                    f.coord(lo),
                    f.coord(hi)
                ),
            ));
        }
        if out.shifts.len() < out.moved as usize {
            lines.push(words::text_line(
                Kind::Info,
                format!(
                    "Tabloda {} nesne var (notlular önce); hepsi sayıldı.",
                    out.shifts.len()
                ),
            ));
        }
        for r in &out.shifts {
            rows.push(vec![
                crate::selecting::kind_title(&r.kind).to_owned(),
                r.points.to_string(),
                f.coord(r.before.x),
                f.coord(r.before.y),
                f.coord(r.dx),
                f.coord(r.dy),
                f.coord(r.shift),
                f.coord(r.most),
                r.notes.iter().map(|n| note_word(n)).collect::<Vec<_>>().join(", "),
            ]);
        }
        (lines, rows)
    }

    pub(crate) fn reproject_view(&self) -> Element<'_, Message> {
        let s = &self.reproject;
        let f = self.format();
        let (lines, rows) = self.reproject_lines();
        let (east, north) = (f.east_label(), f.north_label());
        // The shift's columns by the axes' names (Y, X in a CBS project; X, Y in a CAD one).
        let shift = |axis: &str| if axis == "Y" { "dY" } else { "dX" };
        let refused = matches!(&s.plan, Some(Err(r)) if !r.uids.is_empty());
        let mut body = Column::new().spacing(12).push(words::summary(lines));
        // A refused layer has no shifts: its reasons and Nesneleri seç say what to do.
        if !matches!(s.plan, Some(Err(_))) {
            body = body.push(sheet(
                vec![
                    col("Nesne", 90.0),
                    num("Nokta", 45.0),
                    num(east, 100.0),
                    num(north, 105.0),
                    num(shift(east), 75.0),
                    num(shift(north), 75.0),
                    num("Kayma", 75.0),
                    num("En büyük", 75.0),
                    col("Kullanım alanı dışı", 150.0),
                ],
                rows,
                "Kayacak nesne yok.",
                260.0,
            ));
        }
        let ready = matches!(s.plan, Some(Ok(_)));
        let mut frame = Frame::new(TITLE).push(body);
        if refused {
            frame = frame.aside(words::secondary(SELECT, Some(msg(Event::Select))));
        }
        frame
            .aside(words::ghost(CANCEL, Some(msg(Event::Close))))
            .action(words::primary(APPLY, ready.then(|| msg(Event::Apply))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn reproject_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
