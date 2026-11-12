//! Bulutu konumla (docs/adr/5100; `pointcloud.place`): a point cloud given a place in the drawing without writing its
//! files, by `cad.pointclouds.place`. The cloud is the selected one (at first the selection's first cloud, else the
//! drawing's first); the map is a matrix, an axis and an angle, or Euler angles — Sapma about Z, then Yunuslama about
//! the turned Y, then Yatış about the twice turned X (R = Rz·Ry·Rx) — about a centre (empty: the middle of the cloud's
//! box) and then a move. The summary says where the cloud is now and what the map does (the command's answer, nothing
//! written); the table gives each file's place after it. Konumla writes it, Konumu kaldır takes the places away; each
//! is one undo step.
//!
//! ```text
//! ┌ Bulutu konumla ─────────────────────────────────────────────────────────────┐
//! │ Bulut            [ag-b.xyz ▾]                                                │
//! │ Dönüşüm          [Matris | Eksen ve açı | Euler açıları]                      │
//! │ Sapma (Z) [0,5]° Yunuslama (Y) [0]° Yatış (X) [0]°                           │
//! │ Merkez           [boş: bulutun kutusunun ortası]                             │
//! │ Öteleme          [0 0 0]                                                     │
//! │ ┌ summary ─────────────────────────────────────────────────────────────────┐│
//! │                                           [Konumu kaldır]  [Konumla]         │
//! │ ┌ Dosya | Dönme (°) | Ortanın kayması (m) ─────────────────────────────────┐│
//! │                                                                     [Kapat] │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::map;
use iced::widget::{container, row, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CloudPlaceBy, CommandResult, Entity, PointCloudsPlace, PointCloudsPlaced};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, pointcloud_place};
use kentos_ui::label;
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Bulutu konumla";
const CLOSE: &str = "Kapat";
pub const PLACE: &str = "Konumla";
pub const REMOVE: &str = "Konumu kaldır";

/// How the map is given, in the window's words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum By {
    Matrix,
    AxisAngle,
    #[default]
    Euler,
}

impl std::fmt::Display for By {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            By::Matrix => "Matris",
            By::AxisAngle => "Eksen ve açı",
            By::Euler => "Euler açıları",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The drawing's clouds (id, name), in order.
    pub clouds: Vec<(String, String)>,
    pub cloud: String,
    pub by: By,
    pub matrix: String,
    pub axis: String,
    pub angle: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
    pub center: String,
    pub translation: String,
    /// The command's answer for the map as it stands, nothing written; why not.
    pub preview: Option<Result<PointCloudsPlaced, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Cloud(String),
    By(By),
    Matrix(String),
    Axis(String),
    Angle(f64),
    Yaw(f64),
    Pitch(f64),
    Roll(f64),
    Center(String),
    Translation(String),
    Place,
    Remove,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Map(map::Event::CloudPlace(e))
}

/// Numbers from text, apart by spaces, commas or semicolons; a point for the decimal.
fn numbers(text: &str, what: &str, n: usize) -> Result<Vec<f64>, String> {
    let words: Vec<&str> =
        text.split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '[' | ']')).filter(|w| !w.is_empty()).collect();
    if words.len() != n {
        return Err(format!("{what} {n} sayıdır; {} sayı yazıldı.", words.len()));
    }
    words.iter().map(|w| w.parse::<f64>().map_err(|_| format!("{what}: “{w}” bir sayı değil; ondalık için nokta kullanın."))).collect()
}

/// Three numbers, or none when the text is empty.
fn triple(text: &str, what: &str) -> Result<Option<[f64; 3]>, String> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let v = numbers(text, what, 3)?;
    Ok(Some([v[0], v[1], v[2]]))
}

/// A number with `d` decimals, a comma for the point.
fn comma(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

/// The rotation angle (degrees) of a place and how far it moves the middle of `box_` (metres).
fn turn_and_shift(m: &[f64], b: &[f64; 6]) -> (f64, f64) {
    let (ax, ay, az) = (m[9] - m[6], m[2] - m[8], m[4] - m[1]);
    let angle = (ax * ax + ay * ay + az * az).sqrt().atan2(m[0] + m[5] + m[10] - 1.0).to_degrees();
    let c = [(b[0] + b[3]) / 2.0, (b[1] + b[4]) / 2.0, (b[2] + b[5]) / 2.0];
    let q: [f64; 3] = std::array::from_fn(|i| m[4 * i] * c[0] + m[4 * i + 1] * c[1] + m[4 * i + 2] * c[2] + m[4 * i + 3]);
    (angle, ((q[0] - c[0]).powi(2) + (q[1] - c[1]).powi(2) + (q[2] - c[2]).powi(2)).sqrt())
}

impl App {
    /// The drawing's point clouds (id, name).
    fn cloud_place_clouds(&self) -> Vec<(String, String)> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        doc.model
            .entities_with_uids()
            .filter_map(|(u, e)| {
                let Entity::PointCloud(c) = e else { return None };
                let name = c.cloud.sources.first().and_then(|s| s.file.as_deref()).map_or_else(
                    || format!("Bulut #{}", e.base().id),
                    |f| f.rsplit(['/', '\\']).next().unwrap_or(f).to_owned(),
                );
                Some((u.to_string(), name))
            })
            .collect()
    }

    /// The command's input for the map as the window has it.
    fn cloud_place_input(&self, remove: bool) -> Result<PointCloudsPlace, String> {
        let s = &self.map.cloud_place;
        let place = if remove {
            CloudPlaceBy::Remove
        } else {
            let center = triple(&s.center, "Merkez")?;
            let translation = triple(&s.translation, "Öteleme")?;
            match s.by {
                By::Matrix => CloudPlaceBy::Matrix { matrix: numbers(&s.matrix, "Matris", 16)? },
                By::AxisAngle => {
                    let a = numbers(&s.axis, "Eksen", 3)?;
                    CloudPlaceBy::AxisAngle { axis: [a[0], a[1], a[2]], angle: s.angle, center, translation }
                }
                By::Euler => CloudPlaceBy::Euler { yaw: s.yaw, pitch: s.pitch, roll: s.roll, center, translation },
            }
        };
        Ok(PointCloudsPlace { uid: s.cloud.clone(), place, expected_revision: None })
    }

    fn cloud_place_run(&mut self) {
        let input = self.cloud_place_input(false);
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        self.map.cloud_place.preview =
            Some(input.and_then(|i| pointcloud_place::answer_in(&doc.model, &i)));
    }

    /// Bulutu konumla (`pointcloud.place`): the selection's first cloud, else the drawing's first.
    pub(crate) fn open_cloud_place(&mut self) -> Task<Message> {
        let clouds = self.cloud_place_clouds();
        if clouds.is_empty() {
            self.warn("Bulutu konumla: çizimde nokta bulutu yok.");
            return Task::none();
        }
        let chosen = self
            .document
            .as_ref()
            .and_then(|d| self.selection.ids().iter().find_map(|s| d.model.uid(*s).map(|u| u.to_string())))
            .filter(|u| clouds.iter().any(|(c, _)| c == u))
            .unwrap_or_else(|| clouds[0].0.clone());
        self.map.cloud_place = State {
            clouds,
            cloud: chosen,
            matrix: "1 0 0 0; 0 1 0 0; 0 0 1 0; 0 0 0 1".to_owned(),
            axis: "0 0 1".to_owned(),
            ..Default::default()
        };
        self.cloud_place_run();
        self.dialog = Some(Dialog::Map(map::MapDialogKind::CloudPlace));
        Task::none()
    }

    /// Konumla or Konumu kaldır: the command writes it, one undo step.
    fn cloud_place_write(&mut self, remove: bool) {
        let input = match self.cloud_place_input(remove) {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("{TITLE}: {why}"));
                return;
            }
        };
        let name = self
            .map
            .cloud_place
            .clouds
            .iter()
            .find(|(u, _)| *u == input.uid)
            .map(|(_, n)| n.clone())
            .unwrap_or_default();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        match pointcloud_place::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } if !output.changed => {
                self.say(Level::Info, format!("{TITLE}: “{name}” bulutunun nesnede konumu yok; hiçbir şey yazılmadı."));
            }
            CommandResult::Completed { output, .. } => {
                let words = if remove {
                    format!("“{name}” dosyasının kendi yerine döndü.")
                } else {
                    format!(
                        "“{name}” konumlandı: dönme {}°, ortanın kayması {} m; dosya değişmedi.",
                        comma(output.angle, 4),
                        comma(output.shift, 3)
                    )
                };
                self.say(Level::Success, format!("{TITLE}: {words} Tek adımda geri alınır."));
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.warn(format!("{TITLE}: {}", error.message));
            }
            _ => {}
        }
        self.cloud_place_run();
    }

    pub(crate) fn cloud_place_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.map.cloud_place;
        match event {
            Event::Cloud(v) => s.cloud = v,
            Event::By(v) => s.by = v,
            Event::Matrix(v) => s.matrix = v,
            Event::Axis(v) => s.axis = v,
            Event::Angle(v) => s.angle = v,
            Event::Yaw(v) => s.yaw = v,
            Event::Pitch(v) => s.pitch = v,
            Event::Roll(v) => s.roll = v,
            Event::Center(v) => s.center = v,
            Event::Translation(v) => s.translation = v,
            Event::Place => {
                self.cloud_place_write(false);
                return Task::none();
            }
            Event::Remove => {
                self.cloud_place_write(true);
                return Task::none();
            }
            Event::Close => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.cloud_place_run();
        Task::none()
    }

    /// The summary's lines and the files' rows: where the cloud is now, and each file's place after the map.
    pub(crate) fn cloud_place_lines(&self) -> (Vec<Element<'_, Message>>, Vec<Vec<String>>) {
        let s = &self.map.cloud_place;
        let mut lines: Vec<Element<'_, Message>> = Vec::new();
        let mut rows = Vec::new();
        let cloud = self.document.as_ref().and_then(|d| {
            let u = kentos_domain::Uuid::parse_str(&s.cloud).ok()?;
            match d.model.get(d.model.slot_of(u)?)? {
                Entity::PointCloud(c) => Some(c.cloud.clone()),
                _ => None,
            }
        });
        if let Some(c) = &cloud {
            let now = if c.placed() {
                let first = c.sources.iter().find_map(|x| x.pose).unwrap_or_default();
                let (a, d) = turn_and_shift(&first, &c.sources[0].bounds);
                format!("Şimdi: konumu nesnede (dönme {}°, ortanın kayması {} m); dosyası değişmedi.", comma(a, 4), comma(d, 3))
            } else {
                "Şimdi: dosyanın kendi yerinde.".to_owned()
            };
            lines.push(words::text_line(Kind::Info, now));
        }
        match &s.preview {
            Some(Ok(p)) => {
                lines.push(words::text_line(
                    Kind::Ok,
                    format!(
                        "Konumla: dönme {}°, bulutun ortası {} m kayar; noktalar çizimde, sorguda ve ölçüde yeni yerinde, dosya yazılmaz.",
                        comma(p.angle, 4),
                        comma(p.shift, 3)
                    ),
                ));
                if let Some(c) = &cloud {
                    for (src, pose) in c.sources.iter().zip(&p.poses) {
                        let name = src.file.as_deref().or(src.url.as_deref()).or(src.asset.as_deref()).unwrap_or("");
                        let name = name.rsplit(['/', '\\']).next().unwrap_or(name).to_owned();
                        let (a, d) = pose.as_deref().map_or((0.0, 0.0), |m| turn_and_shift(m, &src.bounds));
                        rows.push(vec![name, comma(a, 4), comma(d, 3)]);
                    }
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        (lines, rows)
    }

    pub(crate) fn cloud_place_view(&self) -> Element<'_, Message> {
        let s = &self.map.cloud_place;
        let by = Segmented::new([By::Matrix, By::AxisAngle, By::Euler], s.by, |v| msg(Event::By(v)));
        let mut form = Form::new()
            .label_width(130.0)
            .field("Bulut", container(fields::select_owned(s.clouds.clone(), &s.cloud, |v| msg(Event::Cloud(v)))).width(280))
            .field("Dönüşüm", by);
        form = match s.by {
            By::Matrix => form.field(
                "Matris",
                kentos_ui::widget::text_field("1 0 0 0; 0 1 0 0; 0 0 1 0; 0 0 0 1", &s.matrix).on_input(|t| msg(Event::Matrix(t))),
            ),
            By::AxisAngle => form.field(
                "Eksen",
                row![
                    kentos_ui::widget::text_field("0 0 1", &s.axis).on_input(|t| msg(Event::Axis(t))).width(160),
                    label::caption("Açı (°)"),
                    NumberInput::new(s.angle, |v| msg(Event::Angle(v))).step(0.1).decimals(4).point().width(110),
                ]
                .spacing(10)
                .align_y(Center),
            ),
            By::Euler => form.field(
                "Sapma Z (°)",
                row![
                    NumberInput::new(s.yaw, |v| msg(Event::Yaw(v))).step(0.1).decimals(4).point().width(100),
                    label::caption("Yunuslama Y (°)"),
                    NumberInput::new(s.pitch, |v| msg(Event::Pitch(v))).step(0.1).decimals(4).point().width(100),
                    label::caption("Yatış X (°)"),
                    NumberInput::new(s.roll, |v| msg(Event::Roll(v))).step(0.1).decimals(4).point().width(100),
                ]
                .spacing(10)
                .align_y(Center),
            ),
        };
        if s.by != By::Matrix {
            form = form
                .field(
                    "Merkez",
                    kentos_ui::widget::text_field("boş: bulutun kutusunun ortası", &s.center).on_input(|t| msg(Event::Center(t))),
                )
                .field("Öteleme (m)", kentos_ui::widget::text_field("0 0 0", &s.translation).on_input(|t| msg(Event::Translation(t))));
        }
        let form = form.help(match s.by {
            By::Euler => "X doğu, Y kuzey, Z yukarı; önce Z çevresinde Sapma, sonra dönmüş Y çevresinde Yunuslama, en son iki kez dönmüş X çevresinde Yatış (sağ el kuralı): R = Rz·Ry·Rx. Bulutun dosyası yazılmaz.",
            By::AxisAngle => "Eksenin ucundan bakınca saatin tersine dönme (sağ el kuralı). Bulutun dosyası yazılmaz.",
            By::Matrix => "Satır satır 16 sayı; yalnız dönme ve öteleme (Tarama kaydı'nın ve Tarama ağı'nın matrisi). Bulutun dosyası yazılmaz.",
        });
        let (lines, rows) = self.cloud_place_lines();
        let placed = self
            .document
            .as_ref()
            .and_then(|d| {
                let u = kentos_domain::Uuid::parse_str(&s.cloud).ok()?;
                match d.model.get(d.model.slot_of(u)?)? {
                    Entity::PointCloud(c) => Some(c.cloud.placed()),
                    _ => None,
                }
            })
            .unwrap_or(false);
        let ready = matches!(&s.preview, Some(Ok(_)));
        let actions = row![
            space::horizontal().width(Fill),
            words::secondary(REMOVE, placed.then(|| msg(Event::Remove))),
            words::primary(PLACE, ready.then(|| msg(Event::Place))),
        ]
        .spacing(10)
        .align_y(Center);
        let table = sheet(vec![col("Dosya", 260.0), num("Dönme (°)", 120.0), num("Ortanın kayması (m)", 170.0)], rows, "Dosya yok.", 140.0);
        let body = Grows::new().spacing(12.0).push(form).push(words::summary(lines)).push(actions).push_grow(table, 140.0);
        Frame::new(TITLE).scroll(body).action(words::secondary(CLOSE, Some(msg(Event::Close)))).width(820.0).modal(msg(Event::Close))
    }

    pub(crate) fn cloud_place_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PLACE) => Some(msg(Event::Place)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Fill("Matris", t) => Some(msg(Event::Matrix(t.to_owned()))),
            Control::Fill("Merkez", t) => Some(msg(Event::Center(t.to_owned()))),
            Control::Fill("Öteleme (m)", t) => Some(msg(Event::Translation(t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
