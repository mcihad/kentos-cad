//! Tarama uyumu (docs/adr/1500; `pointcloud.overlap`): two scans of the same place compared by
//! `cad.pointclouds.overlap`, read only. The two clouds on one line, the cell and the fewest points on the next, the
//! control layer, its reach and the scale on the third; the summary says RMSDz against the package's interswath limit,
//! the mean (a systematic shift), the largest difference, each cloud's KOH_Z at the control points, and the notes; a
//! choice shows the worst cells or the control points. CSV kaydet writes both.

use crate::map;
use std::path::PathBuf;

use iced::widget::{Column, container, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, Entity, PointCloudsOverlap, PointCloudsOverlapped};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, pointcloud_overlap};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Tarama uyumu";
const CLOSE: &str = "Kapat";
const SAVE: &str = "CSV kaydet…";
/// The control layer's choice when there is none.
const NONE: &str = "";

/// What the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Worst,
    Control,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Worst => "En kötü hücreler",
            View::Control => "Kontrol noktaları",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The drawing's point clouds: id and name.
    pub clouds: Vec<(String, String)>,
    /// The drawing's layers with points: id and name ("" for none first).
    pub layers: Vec<(String, String)>,
    pub a: String,
    pub b: String,
    pub cell: f64,
    pub least: f64,
    pub control: String,
    pub radius: f64,
    pub scale: f64,
    pub view: View,
    pub answer: Option<Result<PointCloudsOverlapped, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    A(String),
    B(String),
    Cell(f64),
    Least(f64),
    Control(String),
    Radius(f64),
    Scale(f64),
    View(View),
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Map(map::Event::Overlap(e))
}

fn cm(v: f64) -> String {
    format!("{:.1}", v * 100.0)
}

/// The answer as CSV: the summary, the worst cells and the control points (metres as computed).
pub fn csv(r: &PointCloudsOverlapped) -> String {
    let mut out = format!(
        "Kural paketi;{};Hücre (m);{};Örtüşen hücre;{};RMSDz (m);{};Ortalama (m);{};En büyük (m);{};Sınır (m);{}\n\nHücre X;Hücre Y;dz (m)\n",
        r.rules, r.cell, r.cells, r.rmsd_z, r.mean, r.max, r.limit
    );
    for c in &r.worst {
        out.push_str(&format!("{};{};{}\n", c.x, c.y, c.dz));
    }
    out.push_str("\nKontrol noktası;Kot (m);Birinci − kontrol (m);İkinci − kontrol (m)\n");
    let o = |v: Option<f64>| v.map(|x| x.to_string()).unwrap_or_default();
    for c in &r.control {
        out.push_str(&format!("{};{};{};{}\n", c.name, c.z, o(c.dz_a), o(c.dz_b)));
    }
    out
}

impl App {
    fn overlap_run(&mut self) {
        let s = &self.map.overlap;
        if s.a.is_empty() || s.b.is_empty() {
            self.map.overlap.answer = Some(Err("İki nokta bulutu seçin.".into()));
            return;
        }
        let input = PointCloudsOverlap {
            uids: vec![s.a.clone(), s.b.clone()],
            cell: Some(s.cell),
            least: Some(s.least.max(0.0) as u32),
            classes: None,
            control: (!s.control.is_empty()).then(|| s.control.clone()),
            field: None,
            radius: Some(s.radius),
            scale: Some(s.scale.max(1.0) as u32),
            rules: None,
            drawing: self.document.as_ref().and_then(|d| d.path.as_ref()).map(|p| p.to_string_lossy().into_owned()),
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        self.map.overlap.answer = Some(
            match pointcloud_overlap::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Karşılaştırılamadı.".to_owned()),
            },
        );
    }

    /// Tarama uyumu (`pointcloud.overlap`): the two selected clouds, else the drawing's first two; a control layer
    /// guessed from its name.
    pub(crate) fn open_overlap(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let clouds: Vec<(String, String)> = doc
            .model
            .entities_with_uids()
            .filter_map(|(u, e)| match e {
                Entity::PointCloud(c) => {
                    let name = c.cloud.sources.first().and_then(|s| s.file.as_deref()).map_or_else(
                        || format!("Bulut #{}", e.base().id),
                        |f| f.rsplit(['/', '\\']).next().unwrap_or(f).to_owned(),
                    );
                    Some((u.to_string(), name))
                }
                _ => None,
            })
            .collect();
        if clouds.len() < 2 {
            self.warn("Tarama uyumu: çizimde karşılaştırılacak iki nokta bulutu yok.");
            return Task::none();
        }
        let selected: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(doc.model.get(**s), Some(Entity::PointCloud(_))))
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .collect();
        let (a, b) = match selected.as_slice() {
            [a, b, ..] => (a.clone(), b.clone()),
            _ => (clouds[0].0.clone(), clouds[1].0.clone()),
        };
        let mut layers = vec![(NONE.to_owned(), "Kontrol yok".to_owned())];
        layers.extend(
            doc.model
                .layers()
                .leaves()
                .iter()
                .filter(|n| doc.model.entities().any(|e| e.base().layer_id == n.id && matches!(e, Entity::Point(_))))
                .map(|n| (n.id.clone(), n.name.clone())),
        );
        let control = layers
            .iter()
            .find(|(id, n)| !id.is_empty() && ["gnss", "kontrol"].iter().any(|w| n.to_lowercase().contains(w)))
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        self.map.overlap = State {
            clouds,
            layers,
            a,
            b,
            cell: 2.0,
            least: 4.0,
            control,
            radius: 2.0,
            scale: doc.model.settings().plot_scale.round().max(1.0),
            view: View::Worst,
            answer: None,
        };
        self.overlap_run();
        self.dialog = Some(Dialog::Map(map::MapDialogKind::Overlap));
        Task::none()
    }

    /// The answer written to `path` as CSV (a test or a trace gives it here, the picker in the app).
    pub(crate) fn overlap_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.map.overlap.answer else {
            return;
        };
        match std::fs::write(&path, csv(r)) {
            Ok(()) => self.say(Level::Success, format!("{TITLE}: sonuç {} dosyasına yazıldı.", path.display())),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}.", path.display())),
        }
    }

    pub(crate) fn overlap_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.map.overlap;
        match event {
            Event::A(v) => s.a = v,
            Event::B(v) => s.b = v,
            Event::Cell(v) => s.cell = v,
            Event::Least(v) => s.least = v,
            Event::Control(v) => s.control = v,
            Event::Radius(v) => s.radius = v,
            Event::Scale(v) => s.scale = v,
            Event::View(v) => {
                s.view = v;
                return Task::none();
            }
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("tarama-uyumu.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => {
                self.overlap_save(path);
                return Task::none();
            }
            Event::Saved(None) => return Task::none(),
            Event::Close => {
                self.map.overlap = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.overlap_run();
        Task::none()
    }

    pub(crate) fn overlap_view(&self) -> Element<'_, Message> {
        let s = &self.map.overlap;
        let pick = |v: &str, on: fn(String) -> Event| container(fields::select_owned(s.clouds.clone(), v, move |x| msg(on(x)))).width(230);
        let form = Form::new()
            .label_width(120.0)
            .field("Birinci tarama", row![pick(&s.a, Event::A), label::caption("İkinci"), pick(&s.b, Event::B)].spacing(10).align_y(Center))
            .field(
                "Hücre (m)",
                row![
                    NumberInput::new(s.cell, |v| msg(Event::Cell(v))).step(0.5).decimals(2).point().width(100),
                    label::caption("En az nokta"),
                    NumberInput::new(s.least, |v| msg(Event::Least(v))).step(1.0).decimals(0).width(80)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Kontrol",
                row![
                    container(fields::select_owned(s.layers.clone(), &s.control, |v| msg(Event::Control(v)))).width(230),
                    label::caption("Çevre (m)"),
                    NumberInput::new(s.radius, |v| msg(Event::Radius(v))).step(0.5).decimals(2).point().width(80),
                    label::caption("Ölçek 1/"),
                    NumberInput::new(s.scale, |v| msg(Event::Scale(v))).step(500.0).decimals(0).width(90)
                ]
                .spacing(10)
                .align_y(Center),
            );
        let mut lines = Vec::new();
        let mut table: Element<'_, Message> = words::empty("Sonuç yok.");
        match &s.answer {
            Some(Ok(r)) => {
                lines.push(match r.passed {
                    Some(ok) => words::text_line(
                        if ok { Kind::Ok } else { Kind::Error },
                        format!(
                            "{} örtüşen hücre ({} m²): RMSDz {} cm, sınır {} cm ({}): {}.",
                            r.cells,
                            r.area,
                            cm(r.rmsd_z),
                            cm(r.limit),
                            r.rules,
                            if ok { "sınırın içinde" } else { "sınırın dışında" }
                        ),
                    ),
                    None => words::text_line(Kind::Warn, "Örtüşen hücre yok: iki bulut aynı yeri yeterli noktayla kaplamıyor."),
                });
                if r.cells > 0 {
                    lines.push(words::text_line(
                        Kind::Info,
                        format!(
                            "Ortalama fark {} cm (sistematik kayma), standart sapma {} cm, en büyük {} cm; okunan {} + {} nokta.",
                            cm(r.mean),
                            cm(r.std),
                            cm(r.max),
                            r.read[0],
                            r.read[1]
                        ),
                    ));
                }
                for (which, st) in [("Birinci", &r.control_a), ("İkinci", &r.control_b)] {
                    if let Some(st) = st {
                        lines.push(match st.passed {
                            Some(ok) => words::text_line(
                                if ok { Kind::Ok } else { Kind::Error },
                                format!(
                                    "{which} tarama, {} kontrol noktası: KOH_Z {} cm (ortalama {} cm), sınır {} cm (BÖHHBÜY m. 91).",
                                    st.n,
                                    cm(st.koh_z),
                                    cm(st.mean),
                                    cm(r.limit_z)
                                ),
                            ),
                            None => words::text_line(Kind::Warn, format!("{which} tarama hiçbir kontrol noktasına ulaşmıyor.")),
                        });
                    }
                }
                for n in r.notes.iter().skip(1) {
                    lines.push(words::text_line(Kind::Info, n.clone()));
                }
                table = match s.view {
                    View::Worst => sheet(
                        vec![num("Hücre Y (doğu)", 150.0), num("Hücre X (kuzey)", 150.0), num("dz (cm)", 100.0)],
                        r.worst.iter().map(|c| vec![format!("{:.2}", c.x), format!("{:.2}", c.y), cm(c.dz)]).collect(),
                        "Örtüşen hücre yok.",
                        240.0,
                    ),
                    View::Control => sheet(
                        vec![col("Nokta", 110.0), num("Kot (m)", 110.0), num("Birinci (cm)", 110.0), num("İkinci (cm)", 110.0)],
                        r.control
                            .iter()
                            .map(|c| vec![c.name.clone(), format!("{:.3}", c.z), c.dz_a.map(cm).unwrap_or_default(), c.dz_b.map(cm).unwrap_or_default()])
                            .collect(),
                        "Kontrol noktası yok.",
                        240.0,
                    ),
                };
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let body = Column::new()
            .spacing(12)
            .push(form)
            .push(words::summary(lines))
            .push(Segmented::new([View::Worst, View::Control], s.view, |v| msg(Event::View(v))))
            .push(table);
        let ready = matches!(s.answer, Some(Ok(_)));
        Frame::new(TITLE)
            .push(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(860.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn overlap_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
