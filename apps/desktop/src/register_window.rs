//! Tarama kaydı (docs/adr/1800; `pointcloud.register`): the second scan fitted to the first by
//! `cad.pointclouds.register`, read only. The two clouds on one line, the voxel, the reach and the iterations on the
//! next, the control layer, its reach and the scale on the third; the summary says the steps and the RMS before and
//! after, the transform (east, north, height and the three small angles in the project's unit), how firmly the
//! geometry holds it (a flat ground: the plan shift unknown), the reliable coverage and each set's KOH_Z at the
//! control points; a choice shows the worst cells or the control points. CSV kaydet writes the answer and the matrix.
//! The weights row (docs/adr/2300) asks for Huber's weights (its k, 1.345 at first) or the trimmed ICP (the share of
//! pairs kept, 1 at first: nothing trimmed until the user says how much of the scans overlap).

use std::path::PathBuf;

use iced::widget::{Column, container, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, Entity, PointCloudsRegister, PointCloudsRegistered, RegisterRobust};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, pointcloud_register};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::calc::traverse::small_angle_text;
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Tarama kaydı";
const CLOSE: &str = "Kapat";
const SAVE: &str = "CSV kaydet…";
/// The control layer's choice when there is none.
const NONE: &str = "";

/// The robust weights of each step (docs/adr/2300).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Weights {
    #[default]
    Plain,
    Huber,
    Trimmed,
}

impl std::fmt::Display for Weights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Weights::Plain => "Eşit",
            Weights::Huber => "Huber",
            Weights::Trimmed => "Kırpılmış",
        })
    }
}

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
    pub fixed: String,
    pub moving: String,
    pub spacing: f64,
    pub reach: f64,
    pub iterations: f64,
    pub control: String,
    pub radius: f64,
    pub scale: f64,
    /// docs/adr/2300: the weights, Huber's k and the trimmed share.
    pub weights: Weights,
    pub k: f64,
    pub ratio: f64,
    pub view: View,
    pub answer: Option<Result<PointCloudsRegistered, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Fixed(String),
    Moving(String),
    Spacing(f64),
    Reach(f64),
    Iterations(f64),
    Control(String),
    Radius(f64),
    Scale(f64),
    Weights(Weights),
    K(f64),
    Ratio(f64),
    View(View),
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Register(e)
}

fn cm(v: f64) -> String {
    format!("{:.1}", v * 100.0)
}

/// The answer as CSV: the summary, the matrix, the worst cells and the control points (metres as computed).
pub fn csv(r: &PointCloudsRegistered) -> String {
    let mut out = format!(
        "Kural paketi;{};Adım;{};Yakınsadı;{};Eşleşme;{};Örtüşme;{};RMS önce (m);{};RMS sonra (m);{};Güç;{};Yatay güç;{}\n",
        r.rules,
        r.iterations,
        if r.converged { "Evet" } else { "Hayır" },
        r.pairs,
        r.overlap,
        r.rms_before,
        r.rms_after,
        r.strength,
        r.horizontal
    );
    out.push_str(&format!(
        "Merkez;{};{};{}\nÖteleme (m);{};{};{}\nDönüklük ω φ κ (rad);{};{};{}\n\nMatris (hareketli → sabit)\n",
        r.centre[0], r.centre[1], r.centre[2], r.translation[0], r.translation[1], r.translation[2], r.rotation[0], r.rotation[1], r.rotation[2]
    ));
    for row in r.matrix.chunks(4) {
        out.push_str(&format!("{};{};{};{}\n", row[0], row[1], row[2], row[3]));
    }
    out.push_str("\nHücre X;Hücre Y;Eşleşme;RMS (m)\n");
    for c in &r.worst {
        out.push_str(&format!("{};{};{};{}\n", c.x, c.y, c.n, c.rms));
    }
    out.push_str("\nKontrol noktası;Kot (m);Sabit − kontrol (m);Önce − kontrol (m);Sonra − kontrol (m)\n");
    let o = |v: Option<f64>| v.map(|x| x.to_string()).unwrap_or_default();
    for c in &r.control {
        out.push_str(&format!("{};{};{};{};{}\n", c.name, c.z, o(c.dz_fixed), o(c.dz_before), o(c.dz_after)));
    }
    out
}

impl App {
    fn register_run(&mut self) {
        let s = &self.register;
        if s.fixed.is_empty() || s.moving.is_empty() {
            self.register.answer = Some(Err("İki nokta bulutu seçin.".into()));
            return;
        }
        let input = PointCloudsRegister {
            uids: vec![s.fixed.clone(), s.moving.clone()],
            spacing: Some(s.spacing),
            reach: Some(s.reach),
            iterations: Some(s.iterations.max(0.0) as u32),
            control: (!s.control.is_empty()).then(|| s.control.clone()),
            radius: Some(s.radius),
            scale: Some(s.scale.max(1.0) as u32),
            drawing: self.document.as_ref().and_then(|d| d.path.as_ref()).map(|p| p.to_string_lossy().into_owned()),
            robust: match s.weights {
                Weights::Plain => None,
                Weights::Huber => Some(RegisterRobust { kind: "huber".into(), k: Some(s.k), ratio: None }),
                Weights::Trimmed => Some(RegisterRobust { kind: "trimmed".into(), k: None, ratio: Some(s.ratio) }),
            },
            ..Default::default()
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.register.answer = Some(match pointcloud_register::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Kayıt denetlenemedi.".to_owned()),
        });
    }

    /// Tarama kaydı (`pointcloud.register`): the two selected clouds (the first fixed), else the drawing's first two;
    /// a control layer guessed from its name.
    pub(crate) fn open_register(&mut self) -> Task<Message> {
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
            self.warn("Tarama kaydı: çizimde oturtulacak iki nokta bulutu yok.");
            return Task::none();
        }
        let selected: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(doc.model.get(**s), Some(Entity::PointCloud(_))))
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .collect();
        let (fixed, moving) = match selected.as_slice() {
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
        self.register = State {
            clouds,
            layers,
            fixed,
            moving,
            spacing: 0.5,
            reach: 1.0,
            iterations: 30.0,
            control,
            radius: 2.0,
            scale: doc.model.settings().plot_scale.round().max(1.0),
            weights: Weights::Plain,
            k: pointcloud_register::HUBER_K,
            ratio: 1.0,
            view: View::Worst,
            answer: None,
        };
        self.register_run();
        self.dialog = Some(Dialog::Register);
        Task::none()
    }

    /// The answer written to `path` as CSV (a test or a trace gives it here, the picker in the app).
    pub(crate) fn register_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.register.answer else { return };
        match std::fs::write(&path, csv(r)) {
            Ok(()) => self.say(Level::Success, format!("{TITLE}: sonuç {} dosyasına yazıldı.", path.display())),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}.", path.display())),
        }
    }

    pub(crate) fn register_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.register;
        match event {
            Event::Fixed(v) => s.fixed = v,
            Event::Moving(v) => s.moving = v,
            Event::Spacing(v) => s.spacing = v,
            Event::Reach(v) => s.reach = v,
            Event::Iterations(v) => s.iterations = v,
            Event::Control(v) => s.control = v,
            Event::Radius(v) => s.radius = v,
            Event::Scale(v) => s.scale = v,
            Event::Weights(v) => s.weights = v,
            Event::K(v) => s.k = v,
            Event::Ratio(v) => s.ratio = v,
            Event::View(v) => {
                s.view = v;
                return Task::none();
            }
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("tarama-kaydi.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => {
                self.register_save(path);
                return Task::none();
            }
            Event::Saved(None) => return Task::none(),
            Event::Close => {
                self.register = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.register_run();
        Task::none()
    }

    fn register_lines(&self, r: &PointCloudsRegistered) -> Vec<Element<'_, Message>> {
        let format = self.format();
        let degrees = format.angle_unit_label() == "°";
        let angle = |rad: f64| small_angle_text(&format, if degrees { rad.to_degrees() } else { rad * 200.0 / std::f64::consts::PI });
        let mut lines = Vec::new();
        if r.pairs == 0 {
            lines.push(words::text_line(Kind::Warn, "Eşleşen nokta yok: bulutlar erişim içinde örtüşmüyor; erişimi büyütün."));
        } else {
            lines.push(words::text_line(
                if r.converged { Kind::Ok } else { Kind::Warn },
                format!(
                    "{} adımda {}: nokta-düzlem RMS {} cm → {} cm; {} eşleşme (hareketli taramanın %{:.0}'i).",
                    r.iterations,
                    if r.converged { "yakınsadı" } else { "toleransa inmedi" },
                    cm(r.rms_before),
                    cm(r.rms_after),
                    r.pairs,
                    r.overlap * 100.0
                ),
            ));
            lines.push(words::text_line(
                Kind::Info,
                format!(
                    "Dönüşüm: öteleme doğu {} cm, kuzey {} cm, kot {} cm; dönüklük ω {}, φ {}, κ {}.",
                    cm(r.translation[0]),
                    cm(r.translation[1]),
                    cm(r.translation[2]),
                    angle(r.rotation[0]),
                    angle(r.rotation[1]),
                    angle(r.rotation[2])
                ),
            ));
            lines.push(words::text_line(
                if r.horizontal < 0.1 { Kind::Warn } else { Kind::Ok },
                format!(
                    "Geometri gücü {:.2}, yatay öteleme gücü {:.2}{}.",
                    r.strength,
                    r.horizontal,
                    if r.horizontal < 0.1 { ": düz zemin, yatay kayma bilinmez" } else { "" }
                ),
            ));
            lines.push(words::text_line(
                if r.reliable >= r.covered { Kind::Ok } else { Kind::Warn },
                format!(
                    "Güvenilir kapsam {:.0} m² / {:.0} m² (hücre RMS ≤ {} cm, {}).",
                    r.reliable,
                    r.covered,
                    cm(r.limit),
                    r.rules
                ),
            ));
        }
        for (which, st) in [("Sabit tarama", &r.control_fixed), ("Oturtulan, önce", &r.control_before), ("Oturtulan, sonra", &r.control_after)] {
            if let Some(st) = st {
                lines.push(match st.passed {
                    Some(ok) => words::text_line(
                        if ok { Kind::Ok } else { Kind::Error },
                        format!(
                            "{which}, {} kontrol noktası: KOH_Z {} cm (ortalama {} cm), sınır {} cm (BÖHHBÜY m. 91).",
                            st.n,
                            cm(st.koh_z),
                            cm(st.mean),
                            cm(r.limit_z)
                        ),
                    ),
                    None => words::text_line(Kind::Warn, format!("{which}: hiçbir kontrol noktasına ulaşmıyor.")),
                });
            }
        }
        // docs/adr/2300: what the weights did at the last step.
        if let Some(w) = &r.robust {
            let said = match (w.kind.as_str(), w.k, w.ratio) {
                ("huber", Some(k), _) => format!(
                    "Ağırlıklar Huber (k {}, artıkların ölçeği {} cm): son adımda {} eşleşme hafifletildi; RMS bütün eşleşmelerden.",
                    format!("{k}").replace('.', ","),
                    cm(w.scale.unwrap_or(0.0)),
                    w.lighter
                ),
                (_, _, Some(ratio)) => format!(
                    "Kırpılmış ICP (eşleşmelerin en iyi %{:.0}'i): son adımda {} eşleşme dışarıda bırakıldı; RMS bütün eşleşmelerden.",
                    ratio * 100.0,
                    w.lighter
                ),
                _ => String::new(),
            };
            lines.push(words::text_line(Kind::Info, said));
        }
        lines.push(words::text_line(Kind::Info, r.notes[0].clone()));
        for n in r.notes.iter().skip(3) {
            lines.push(words::text_line(Kind::Info, n.clone()));
        }
        lines
    }

    pub(crate) fn register_view(&self) -> Element<'_, Message> {
        let s = &self.register;
        let pick = |v: &str, on: fn(String) -> Event| container(fields::select_owned(s.clouds.clone(), v, move |x| msg(on(x)))).width(230);
        let form = Form::new()
            .label_width(120.0)
            .field("Sabit tarama", row![pick(&s.fixed, Event::Fixed), label::caption("Oturtulacak"), pick(&s.moving, Event::Moving)].spacing(10).align_y(Center))
            .field(
                "Voksel (m)",
                row![
                    NumberInput::new(s.spacing, |v| msg(Event::Spacing(v))).step(0.25).decimals(2).point().width(100),
                    label::caption("Erişim (m)"),
                    NumberInput::new(s.reach, |v| msg(Event::Reach(v))).step(0.5).decimals(2).point().width(80),
                    label::caption("En çok adım"),
                    NumberInput::new(s.iterations, |v| msg(Event::Iterations(v))).step(5.0).decimals(0).width(80)
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
            )
            .field("Ağırlık", {
                let choice = Segmented::new([Weights::Plain, Weights::Huber, Weights::Trimmed], s.weights, |v| msg(Event::Weights(v)));
                match s.weights {
                    Weights::Plain => row![choice],
                    Weights::Huber => row![
                        choice,
                        label::caption("k"),
                        NumberInput::new(s.k, |v| msg(Event::K(v))).step(0.1).decimals(3).point().width(90)
                    ],
                    Weights::Trimmed => row![
                        choice,
                        label::caption("Tutulan pay"),
                        NumberInput::new(s.ratio, |v| msg(Event::Ratio(v))).step(0.05).decimals(2).point().width(90)
                    ],
                }
                .spacing(10)
                .align_y(Center)
            });
        let mut lines = Vec::new();
        let mut table: Element<'_, Message> = words::empty("Sonuç yok.");
        match &s.answer {
            Some(Ok(r)) => {
                lines = self.register_lines(r);
                table = match s.view {
                    View::Worst => sheet(
                        vec![num("Hücre Y (doğu)", 150.0), num("Hücre X (kuzey)", 150.0), num("Eşleşme", 90.0), num("RMS (cm)", 100.0)],
                        r.worst.iter().map(|c| vec![format!("{:.2}", c.x), format!("{:.2}", c.y), c.n.to_string(), cm(c.rms)]).collect(),
                        "Kapsam hücresi yok.",
                        220.0,
                    ),
                    View::Control => sheet(
                        vec![
                            col("Nokta", 100.0),
                            num("Kot (m)", 110.0),
                            num("Sabit (cm)", 100.0),
                            num("Önce (cm)", 100.0),
                            num("Sonra (cm)", 100.0),
                        ],
                        r.control
                            .iter()
                            .map(|c| {
                                vec![
                                    c.name.clone(),
                                    format!("{:.3}", c.z),
                                    c.dz_fixed.map(cm).unwrap_or_default(),
                                    c.dz_before.map(cm).unwrap_or_default(),
                                    c.dz_after.map(cm).unwrap_or_default(),
                                ]
                            })
                            .collect(),
                        "Kontrol noktası yok.",
                        220.0,
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
            .width(880.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn register_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
