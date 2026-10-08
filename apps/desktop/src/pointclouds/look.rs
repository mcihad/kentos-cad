//! Nokta bulutu stili (docs/adr/0207 §5, §9; the web's
//! `ui/pointcloud/CloudStyleDialog.ts`): a cloud's look edited and seen on
//! the drawing as it changes: how its points are coloured (their colours,
//! classes, heights or intensities through a ramp, returns, the object's
//! colour), the ramp among wide samples and its range (Otomatik: the 2nd and
//! 98th percentiles of a sample, its files' root nodes), the classes shown,
//! the points' size, unit and shape, and the transparency. The drawing shows
//! the look while the window is open, inside an undo group that is let go
//! (nothing recorded); Uygula writes it through `cad.entities.edit`'s
//! `pointcloudStyle` as one undo step (Nokta bulutu stili), Vazgeç leaves the
//! cloud as it was. A new look only colours the nodes again; their points stay.

use iced::widget::{Column, Row, button, column, container, row, space, text_input};
use iced::{Center, Color, Element, Fill, Task};
use kentos_contracts::{
    CloudRender, CommandResult, EditOperation, EntitiesEdit, Entity, EntityEdit, EntityGeometry,
    MAX_POINT_SIZE, MIN_POINT_SIZE, PointCloudFields, PointCloudStyle, PointShape, PointSizeUnit,
    RASTER_RAMPS,
};
use kentos_domain::{Group, Slot};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, edit};
use kentos_pointcloud::look::Sample;
use kentos_ui::label;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};

use super::Event as Clouds;
use super::service::{State as Opened, service};
use crate::app::{App, Dialog as Asking, Message};
use crate::exchange::words::{self, Kind as Line};
use crate::properties::rows::cloud::render_name;
use crate::rasters::look::ramp_bar;

pub const TITLE: &str = "Nokta bulutu stili";

/// The window's numbers as typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typed {
    Min,
    Max,
    Size,
    Clear,
}

pub struct State {
    pub slot: Slot,
    /// The cloud as the window found it.
    original: PointCloudFields,
    /// The look being edited.
    pub style: PointCloudStyle,
    /// What is typed, by field.
    texts: [String; 4],
    /// The sample Otomatik reads, once read; why not.
    sample: Option<Result<Sample, String>>,
    /// The drawing's preview: an undo group let go when the window closes.
    preview: Option<Group>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State").field("slot", &self.slot).finish()
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Render(CloudRender),
    Ramp(&'static str),
    Invert,
    Rgb8,
    /// Otomatik: the range from the sample.
    Auto,
    /// A class shown or hidden.
    Class(u8),
    /// Every class shown.
    AllClasses,
    Unit(PointSizeUnit),
    Shape(PointShape),
    Type(Typed, String),
    Sample(Result<Sample, String>),
    Apply,
    Close,
}

fn msg(e: Event) -> Message {
    Message::PointClouds(Clouds::Look(e))
}

const RENDERS: [CloudRender; 6] = [
    CloudRender::Rgb,
    CloudRender::Classification,
    CloudRender::Elevation,
    CloudRender::Intensity,
    CloudRender::Returns,
    CloudRender::Single,
];

/// ASPRS's named classes (docs/adr/0207 §5.1).
const CLASSES: std::ops::RangeInclusive<u8> = 0..=22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Unit(PointSizeUnit);

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            PointSizeUnit::Px => "Piksel",
            PointSizeUnit::M => "Metre",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shape(PointShape);

impl std::fmt::Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            PointShape::Round => "Yuvarlak",
            PointShape::Square => "Kare",
        })
    }
}

/// A number as the window writes it.
fn number(v: Option<f64>) -> String {
    v.map_or_else(String::new, crate::crs::js_number)
}

/// A typed number: none when blank, NaN when not a number.
fn read(t: &str) -> Option<f64> {
    let t = t.trim();
    (!t.is_empty()).then(|| crate::calc::read::read_number(t).unwrap_or(f64::NAN))
}

/// The sample Otomatik reads: the root nodes of at most four of the cloud's opened files.
fn sample_of(key: &str) -> Result<Sample, String> {
    let entry = service()
        .entry(key)
        .ok_or("Bulut çizimde henüz açılmadı.")?;
    let mut sample = Sample::default();
    let mut read = 0;
    let mut records = Vec::new();
    for m in &entry.members {
        if read >= 4 {
            break;
        }
        let Opened::Ready(o) = m.state() else {
            continue;
        };
        let Some(run) = o.cloud.node_run(kentos_pointcloud::copc::Key::ROOT) else {
            continue;
        };
        let bytes = o.bytes.read(run.need.offset, run.need.len)?;
        records.clear();
        o.cloud
            .records(&run, &bytes, &mut records)
            .map_err(|e| e.0)?;
        let step = kentos_pointcloud::place::sample_step(std::slice::from_ref(&run));
        sample.take(
            &o.cloud.layout,
            &records,
            o.cloud.head.scale,
            o.cloud.head.offset,
            step,
        );
        read += 1;
    }
    if read == 0 {
        return Err("bulutun dizini hazır değil; hazır olunca yeniden deneyin.".into());
    }
    Ok(sample)
}

impl App {
    /// `pointcloud.style`: the window for the selected cloud.
    pub(crate) fn cloud_look_command(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let found = self
            .selection
            .ids()
            .iter()
            .find_map(|&s| match doc.model.get(s) {
                Some(Entity::PointCloud(c)) => Some((s, c.cloud.clone(), c.base.layer_id.clone())),
                _ => None,
            });
        let Some((slot, fields, layer)) = found else {
            self.warn("Nokta bulutu stili için önce bir nokta bulutu seçin.");
            return Task::none();
        };
        if doc.model.layers().is_locked(&layer) {
            self.warn(format!(
                "“{}” katmanı kilitli; bulutun görünüşü değiştirilemez.",
                self.layer_name(&layer)
            ));
            return Task::none();
        }
        let key = super::key_of(&fields);
        let s = &fields.style;
        let texts = [
            number(s.min),
            number(s.max),
            crate::crs::js_number(s.size),
            kentos_interaction::fixed((1.0 - fields.opacity.unwrap_or(1.0)) * 100.0, 0),
        ];
        self.clouds.look = Some(State {
            slot,
            style: fields.style.clone(),
            original: fields,
            texts,
            sample: None,
            preview: None,
        });
        self.dialog = Some(Asking::PointCloudStyle);
        crate::rasters::off_thread(move || msg(Event::Sample(sample_of(&key))))
    }

    pub(crate) fn cloud_look_event(&mut self, e: Event) -> Task<Message> {
        let Some(s) = &mut self.clouds.look else {
            return Task::none();
        };
        match e {
            Event::Sample(sample) => {
                s.sample = Some(sample);
                return Task::none();
            }
            Event::Apply => {
                self.cloud_look_apply();
                return Task::none();
            }
            Event::Close => {
                self.cloud_look_close();
                return Task::none();
            }
            Event::Render(r) => {
                let was = s.style.render;
                s.style.render = r;
                // A ramped look over another quantity takes the sample's range of it.
                if r.ramped() && was != r {
                    let range = s
                        .sample
                        .as_ref()
                        .and_then(|x| x.as_ref().ok())
                        .and_then(|x| x.range_for(r));
                    let (lo, hi) = match (range, r) {
                        (Some(v), _) => v,
                        (None, CloudRender::Intensity) => (0.0, 65535.0),
                        (None, _) => (
                            s.original.bounds[2],
                            s.original.bounds[5].max(s.original.bounds[2] + 1.0),
                        ),
                    };
                    s.style.min = Some(lo);
                    s.style.max = Some(hi);
                    s.texts[Typed::Min as usize] = crate::crs::js_number(lo);
                    s.texts[Typed::Max as usize] = crate::crs::js_number(hi);
                }
            }
            Event::Ramp(name) => s.style.ramp = (name != "Arazi").then(|| name.to_owned()),
            Event::Invert => s.style.invert = !s.style.invert,
            Event::Rgb8 => s.style.rgb8 = !s.style.rgb8,
            Event::Auto => {
                if let Some(Ok(sample)) = &s.sample
                    && let Some((lo, hi)) = sample.range_for(s.style.render)
                {
                    s.style.min = Some(lo);
                    s.style.max = Some(hi);
                    s.texts[Typed::Min as usize] = crate::crs::js_number(lo);
                    s.texts[Typed::Max as usize] = crate::crs::js_number(hi);
                }
            }
            Event::Class(c) => {
                let hidden = &mut s.style.hidden;
                match hidden.binary_search(&c) {
                    Ok(i) => {
                        hidden.remove(i);
                    }
                    Err(i) => hidden.insert(i, c),
                }
            }
            Event::AllClasses => s.style.hidden.clear(),
            Event::Unit(u) => s.style.size_unit = u,
            Event::Shape(sh) => s.style.shape = sh,
            Event::Type(field, text) => {
                let v = read(&text);
                match field {
                    Typed::Min => s.style.min = v,
                    Typed::Max => s.style.max = v,
                    Typed::Size => {
                        if let Some(v) = v {
                            s.style.size = v;
                        }
                    }
                    Typed::Clear => {}
                }
                s.texts[field as usize] = text;
            }
        }
        self.cloud_look_preview();
        Task::none()
    }

    /// The cloud as the window says, when that is a cloud: its look and opacity.
    fn cloud_look_fields(&self) -> Option<PointCloudFields> {
        let s = self.clouds.look.as_ref()?;
        let clear = read(&s.texts[Typed::Clear as usize]).unwrap_or(0.0);
        let opacity =
            (clear.is_finite() && (0.0..=90.0).contains(&clear)).then_some(1.0 - clear / 100.0)?;
        if read(&s.texts[Typed::Size as usize]).is_none_or(f64::is_nan) {
            return None;
        }
        let mut f = s.original.clone();
        f.style = s.style.clone();
        if !f.style.render.ramped() && read(&s.texts[Typed::Min as usize]).is_none() {
            f.style.min = None;
            f.style.max = None;
        }
        f.opacity = (opacity < 1.0).then_some(opacity);
        (f.problem().is_none() && edit::cloud_finite(&f)).then_some(f)
    }

    /// Why Uygula cannot run, when it cannot.
    fn cloud_look_problem(&self) -> Option<String> {
        let s = self.clouds.look.as_ref()?;
        let clear = read(&s.texts[Typed::Clear as usize]).unwrap_or(0.0);
        if !(clear.is_finite() && (0.0..=90.0).contains(&clear)) {
            return Some("Saydamlık %0 ile %90 arasında olmalı.".to_owned());
        }
        if [Typed::Min, Typed::Max, Typed::Size]
            .iter()
            .any(|&t| read(&s.texts[t as usize]).is_some_and(f64::is_nan))
        {
            return Some("Sayı olmayan bir değer var; düzeltin.".to_owned());
        }
        if read(&s.texts[Typed::Size as usize]).is_none() {
            return Some(format!(
                "Noktanın boyu {MIN_POINT_SIZE} ile {MAX_POINT_SIZE} arasında olmalı."
            ));
        }
        self.cloud_look_fields()
            .is_none()
            .then(|| {
                let mut f = s.original.clone();
                f.style = s.style.clone();
                f.style.problem()
            })
            .flatten()
    }

    /// The drawing shows the window's look: the last preview let go, the new one written in its group.
    fn cloud_look_preview(&mut self) {
        let fields = self.cloud_look_fields();
        let (Some(doc), Some(s)) = (&mut self.document, &mut self.clouds.look) else {
            return;
        };
        if let Some(g) = s.preview.take() {
            doc.model.cancel_group(g);
        }
        let Some(fields) = fields else {
            return;
        };
        if fields == s.original {
            return;
        }
        let group = doc.model.begin_group(TITLE);
        let mut cloud = match doc.model.get(s.slot) {
            Some(Entity::PointCloud(c)) => c.clone(),
            _ => {
                doc.model.cancel_group(group);
                return;
            }
        };
        cloud.cloud = fields;
        let _ = kentos_interaction::properties::set_geometry(
            &mut doc.model,
            s.slot,
            &Entity::PointCloud(cloud),
        );
        s.preview = Some(group);
    }

    /// Uygula: the preview let go, the look written as one step.
    fn cloud_look_apply(&mut self) {
        let Some(fields) = self.cloud_look_fields() else {
            return;
        };
        let (Some(doc), Some(s)) = (&mut self.document, &mut self.clouds.look) else {
            return;
        };
        if let Some(g) = s.preview.take() {
            doc.model.cancel_group(g);
        }
        let slot = s.slot;
        let unchanged = fields == s.original;
        self.clouds.look = None;
        self.dialog = None;
        if unchanged {
            return;
        }
        let Some(uid) = doc.model.uid(slot) else {
            return;
        };
        let input = EntitiesEdit {
            operation: EditOperation::PointCloudStyle,
            changes: vec![EntityEdit::Update {
                uid: uid.to_string(),
                geometry: EntityGeometry::PointCloud(fields),
            }],
            expected_revision: None,
        };
        match edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { warnings, .. } => {
                self.say(
                    Level::Success,
                    "Bulutun görünüşü değişti. Ctrl+Z geri alır.",
                );
                for w in warnings {
                    self.warn(w.message);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// Vazgeç: the cloud as it was.
    pub(crate) fn cloud_look_close(&mut self) {
        if let (Some(doc), Some(s)) = (&mut self.document, &mut self.clouds.look)
            && let Some(g) = s.preview.take()
        {
            doc.model.cancel_group(g);
        }
        self.clouds.look = None;
        if self.dialog == Some(Asking::PointCloudStyle) {
            self.dialog = None;
        }
    }

    pub(crate) fn cloud_look_view(&self) -> Element<'_, Message> {
        let Some(s) = &self.clouds.look else {
            return iced::widget::text("").into();
        };
        let st = &s.style;
        let render = Select::new(
            RENDERS
                .iter()
                .map(|&r| Choice::new(render_name(r)))
                .collect::<Vec<_>>(),
            RENDERS.iter().position(|&r| r == st.render),
            |i| {
                msg(Event::Render(
                    RENDERS.get(i).copied().unwrap_or(CloudRender::Elevation),
                ))
            },
        );
        let field_input = |t: Typed, placeholder: &str, width: f32| {
            kentos_ui::widget::focus_ring(
                text_input(placeholder, &s.texts[t as usize])
                    .on_input(move |v| msg(Event::Type(t, v)))
                    .padding([5, 8])
                    .width(width)
                    .size(kentos_ui::theme::typography::body())
                    .style(kentos_ui::style::field::input),
            )
        };
        let mut body = Column::new().spacing(14);
        let hint = match st.render {
            CloudRender::Rgb => "Noktalar dosyanın renkleriyle çizilir.",
            CloudRender::Classification => "Her sınıf ASPRS'nin rengiyle çizilir.",
            CloudRender::Elevation => "Noktaların kotu renk rampasıyla.",
            CloudRender::Intensity => "Yoğunluk griyle.",
            CloudRender::Returns => "Tek, ilk, ara ve son dönüşler dört renkle.",
            CloudRender::Single => "Bütün noktalar nesnenin rengiyle.",
        };
        body = body.push(words::field("Görünüş", render, Some(hint.to_owned())));
        if st.render == CloudRender::Rgb {
            body = body.push(words::check(
                st.rgb8,
                "Renkler 8 bitlik (dosya 0–255 yazıyor)",
                Some(msg(Event::Rgb8)),
            ));
        }
        if st.render.ramped() {
            let unit = if st.render == CloudRender::Intensity {
                "yoğunluk"
            } else {
                "kot, m"
            };
            let sample_line = match &s.sample {
                None => "Örnek noktalar okunuyor…".to_owned(),
                Some(Err(why)) => format!("Otomatik aralık yok: {why}"),
                Some(Ok(x)) => format!(
                    "Otomatik: {} örnek noktanın %2 ve %98'lik değerleri.",
                    crate::crs::grouped(x.z.len() as f64)
                ),
            };
            let auto = button(label::body("Otomatik"))
                .on_press_maybe(
                    s.sample
                        .as_ref()
                        .is_some_and(Result::is_ok)
                        .then(|| msg(Event::Auto)),
                )
                .padding([5, 12])
                .style(kentos_ui::style::button::secondary);
            body = body.push(words::field(
                "Aralık",
                column![
                    row![
                        words::field("En küçük", field_input(Typed::Min, unit, 120.0), None),
                        words::field("En büyük", field_input(Typed::Max, unit, 120.0), None),
                        column![space().height(18), auto],
                    ]
                    .spacing(12),
                    label::caption(sample_line),
                ]
                .spacing(6),
                None,
            ));
        }
        if st.render == CloudRender::Elevation {
            let mut ramps = Column::new().spacing(4);
            for name in RASTER_RAMPS {
                let chosen = st.ramp_name() == name;
                let face = row![ramp_bar(name, st.invert, 224.0, 14.0), label::body(name)]
                    .spacing(10)
                    .align_y(Center);
                ramps = ramps.push(
                    button(face)
                        .on_press(msg(Event::Ramp(name)))
                        .padding([3, 6])
                        .width(Fill)
                        .style(kentos_ui::style::button::list_item(chosen)),
                );
            }
            body = body.push(words::field(
                "Renk rampası",
                column![
                    ramps,
                    words::check(st.invert, "Ters çevir", Some(msg(Event::Invert)))
                ]
                .spacing(6),
                None,
            ));
        } else if st.render == CloudRender::Intensity {
            body = body.push(words::check(
                st.invert,
                "Ters çevir",
                Some(msg(Event::Invert)),
            ));
        }
        body = body.push(self.cloud_look_classes(st));
        let unit = Segmented::new(
            [Unit(PointSizeUnit::Px), Unit(PointSizeUnit::M)],
            Unit(st.size_unit),
            |u| msg(Event::Unit(u.0)),
        );
        let shape = Segmented::new(
            [Shape(PointShape::Round), Shape(PointShape::Square)],
            Shape(st.shape),
            |v| msg(Event::Shape(v.0)),
        );
        body = body.push(
            row![
                words::field("Boy", field_input(Typed::Size, "2", 80.0), None),
                words::field("Birim", unit, None),
                words::field("Biçim", shape, None),
                words::field("Saydamlık (%)", field_input(Typed::Clear, "0", 80.0), None),
            ]
            .spacing(16),
        );
        let problem = self.cloud_look_problem();
        if let Some(why) = &problem {
            body = body.push(words::text_line(Line::Error, why.clone()));
        } else {
            body = body.push(words::text_line(
                Line::Info,
                "Çizim pencerenin görünüşünü gösteriyor; Uygula tek adımda yazar, Vazgeç eski görünüşe döndürür.",
            ));
        }
        overlay::blocking(
            Dialog::new(TITLE)
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Uygula",
                    problem.is_none().then(|| msg(Event::Apply)),
                ))
                .width(640.0)
                .max_height(820.0),
        )
    }

    /// The classes shown, each with its colour; a click hides or shows it.
    fn cloud_look_classes<'a>(&self, st: &'a PointCloudStyle) -> Element<'a, Message> {
        let cell = |c: u8| -> Element<'a, Message> {
            let [r, g, b] = kentos_pointcloud::classes::colour(c);
            let swatch = container(space())
                .width(12)
                .height(12)
                .style(move |_| container::Style {
                    background: Some(Color::from_rgb8(r, g, b).into()),
                    border: iced::Border {
                        color: Color::from_rgba8(0, 0, 0, 0.35),
                        width: 1.0,
                        radius: 2.0.into(),
                    },
                    ..container::Style::default()
                });
            let shown = st.hidden.binary_search(&c).is_err();
            row![
                words::check(shown, "", Some(msg(Event::Class(c)))),
                swatch,
                label::body(format!("{c} {}", kentos_pointcloud::classes::name(c))),
            ]
            .spacing(6)
            .align_y(Center)
            .into()
        };
        let all: Vec<u8> = CLASSES.collect();
        let half = all.len().div_ceil(2);
        let col = |part: &[u8]| -> Element<'a, Message> {
            Column::with_children(part.iter().map(|&c| cell(c)))
                .spacing(3)
                .into()
        };
        let lists = Row::new()
            .push(container(col(&all[..half])).width(Fill))
            .push(container(col(&all[half..])).width(Fill))
            .spacing(12);
        let others = st.hidden.iter().filter(|&&c| c > 22).count();
        let mut part = column![lists].spacing(6);
        if others > 0 || !st.hidden.is_empty() {
            let mut line = Row::new().spacing(12).align_y(Center);
            if others > 0 {
                line = line.push(label::caption(format!("{others} başka sınıf da gizli.")));
            }
            line = line.push(
                button(label::body("Hepsini göster"))
                    .on_press(msg(Event::AllClasses))
                    .padding([3, 10])
                    .style(kentos_ui::style::button::secondary),
            );
            part = part.push(line);
        }
        words::field(
            "Gösterilen sınıflar",
            part,
            Some("İşareti kaldırılan sınıfın noktaları çizilmez.".to_owned()),
        )
    }
}
