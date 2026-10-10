//! Raster stili (docs/adr/0204 §4, §8; the web's `ui/raster/RasterStyleDialog.ts`):
//! a raster's look edited and seen on the drawing as it changes: how it is
//! drawn (its colours, grey, its palette, a ramp, a shaded relief or a ramp
//! lit by its relief), its bands, the stretch with the bands' statistics,
//! the ramp among wide samples of each, the light of the relief, nodata,
//! the transparency and the sampling. The drawing shows the look while the
//! window is open, inside an undo group that is let go (nothing recorded);
//! Uygula writes it through `cad.entities.edit`'s `rasterStyle` as one
//! undo step (Raster stili), Vazgeç leaves the raster as it was. A NetCDF
//! dataset's raster has its Veri seti section (docs/adr/0243 §11): each
//! slice dimension's value, Zaman sürgüsünü izle, a mesh's lines and colour.

use iced::widget::{Column, Row, button, column, container, row, space, text_input};
use iced::{Center, Color, Element, Fill, Length, Task};
use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEdit, Entity, EntityEdit, EntityGeometry, RASTER_RAMPS,
    RasterDataset, RasterFields, RasterRender, RasterResampling, RasterStretch, RasterStyle,
};
use kentos_domain::{Group, Slot};
use kentos_formats::raster::stats::Stats;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, edit};
use kentos_ui::label;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};

use super::Event as Rasters;
use crate::app::{App, Dialog as Asking, Message};
use crate::exchange::words::{self, Kind as Line};
use crate::properties::rows::raster::{render_name, stretch_name};

pub const TITLE: &str = "Raster stili";

/// The window's numbers as typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typed {
    Min,
    Max,
    Azimuth,
    Altitude,
    ZFactor,
    Nodata,
    Clear,
}

pub struct State {
    pub slot: Slot,
    /// The raster as the window found it.
    original: RasterFields,
    /// The look being edited.
    pub style: RasterStyle,
    /// What is typed, by field.
    texts: [String; 7],
    /// The bands' statistics, once worked out; why not.
    stats: Option<Result<Stats, String>>,
    has_palette: bool,
    /// The dataset being edited (a NetCDF raster's): its slice and following.
    pub dataset: Option<RasterDataset>,
    /// A mesh's lines' colour as typed.
    edges_text: String,
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
    Render(RasterRender),
    /// The k-th band of the look (0-based) is band `b` (0: none, for the alpha band).
    Band(usize, u32),
    Stretch(RasterStretch),
    Ramp(&'static str),
    Invert,
    Type(Typed, String),
    Resampling(RasterResampling),
    /// Slice dimension `k` shows its `i`-th value.
    Slice(usize, u32),
    Follow,
    Edges,
    EdgesColour(String),
    Stats(Result<Stats, String>),
    Apply,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Rasters(Rasters::Look(e))
}

const RENDERS: [RasterRender; 6] = [
    RasterRender::Rgb,
    RasterRender::Gray,
    RasterRender::Palette,
    RasterRender::Ramp,
    RasterRender::Hillshade,
    RasterRender::RampShade,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stretch(RasterStretch);

impl std::fmt::Display for Stretch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            RasterStretch::None => "Yok",
            RasterStretch::MinMax => "En küçük – en büyük",
            RasterStretch::Percent => "%2 – %98",
            RasterStretch::Manual => "Elle",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sampling(RasterResampling);

impl std::fmt::Display for Sampling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            RasterResampling::Bilinear => "Çift doğrusal",
            RasterResampling::Nearest => "En yakın",
        })
    }
}

/// A statistic as the window writes it: a whole number whole, else two decimals.
fn stat_number(v: f64) -> String {
    if v.fract() == 0.0 {
        crate::crs::js_number(v)
    } else {
        kentos_interaction::fixed(v, 2)
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

/// The ramp's colours from left to right, `n` of them.
pub fn ramp_colours(name: &str, invert: bool, n: usize) -> Vec<Color> {
    let stops = kentos_formats::raster::style::ramp_stops(name);
    (0..n)
        .map(|k| {
            let t = if n > 1 {
                k as f64 / (n - 1) as f64
            } else {
                0.0
            };
            let [r, g, b] =
                kentos_formats::raster::style::ramp_at(stops, if invert { 1.0 - t } else { t });
            Color::from_rgb8(r, g, b)
        })
        .collect()
}

/// A ramp's wide sample: its colours side by side.
pub fn ramp_bar<'a, M: 'a>(name: &str, invert: bool, width: f32, height: f32) -> Element<'a, M> {
    const STEPS: usize = 32;
    let cells: Vec<Element<'a, M>> = ramp_colours(name, invert, STEPS)
        .into_iter()
        .map(|c| {
            container(space())
                .width(Length::Fixed(width / STEPS as f32))
                .height(height)
                .style(move |_| container::Style {
                    background: Some(c.into()),
                    ..container::Style::default()
                })
                .into()
        })
        .collect();
    container(Row::with_children(cells))
        .style(kentos_ui::style::container::bordered)
        .padding(1)
        .into()
}

impl App {
    /// `raster.style`: the window for the selected raster (or the one under the pointer's menu).
    pub(crate) fn raster_look_command(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let found = self
            .selection
            .ids()
            .iter()
            .find_map(|&s| match doc.model.get(s) {
                Some(Entity::Raster(r)) => Some((s, r.raster.clone(), r.base.layer_id.clone())),
                _ => None,
            });
        let Some((slot, fields, layer)) = found else {
            self.warn("Raster stili için önce bir raster seçin.");
            return Task::none();
        };
        if doc.model.layers().is_locked(&layer) {
            self.warn(format!(
                "“{}” katmanı kilitli; rasterin görünüşü değiştirilemez.",
                self.layer_name(&layer)
            ));
            return Task::none();
        }
        let key = super::key_of(&fields);
        let folder = doc
            .path
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf);
        let origin = super::origin_of(&key, &doc.model, folder.as_deref());
        let has_palette = fields.style.render == RasterRender::Palette;
        let s = &fields.style;
        let texts = [
            number(s.min),
            number(s.max),
            number(s.azimuth),
            number(s.altitude),
            number(s.z_factor),
            number(s.nodata),
            kentos_interaction::fixed((1.0 - fields.opacity.unwrap_or(1.0)) * 100.0, 0),
        ];
        self.rasters.look = Some(State {
            slot,
            style: fields.style.clone(),
            dataset: fields.dataset.clone(),
            edges_text: fields
                .style
                .edges
                .clone()
                .unwrap_or_else(|| kentos_formats::raster::style::MESH_EDGES.to_owned()),
            original: fields,
            texts,
            stats: None,
            has_palette,
            preview: None,
        });
        self.dialog = Some(Asking::RasterStyle);
        let Some(origin) = origin else {
            return Task::none();
        };
        super::off_thread(move || msg(Event::Stats(super::tiles::stats_of(&key, origin))))
    }

    pub(crate) fn raster_look_event(&mut self, e: Event) -> Task<Message> {
        let Some(s) = &mut self.rasters.look else {
            return Task::none();
        };
        match e {
            Event::Stats(stats) => {
                s.stats = Some(stats);
                return Task::none();
            }
            Event::Apply => {
                self.raster_look_apply();
                return Task::none();
            }
            Event::Close => {
                self.raster_look_close();
                return Task::none();
            }
            Event::Render(r) => {
                let st = &mut s.style;
                st.render = r;
                st.bands = match r {
                    RasterRender::Rgb => {
                        let n = s.original.bands;
                        if n >= 4 && s.original.sample == kentos_contracts::RasterSample::U8 {
                            vec![1, 2, 3, 4]
                        } else {
                            vec![1, 2, 3.min(n)]
                        }
                    }
                    _ => vec![st.bands.first().copied().unwrap_or(1)],
                };
                if matches!(r, RasterRender::Ramp | RasterRender::RampShade) && st.ramp.is_none() {
                    st.ramp = Some("Arazi".to_owned());
                }
            }
            Event::Band(k, b) => {
                let bands = &mut s.style.bands;
                if b == 0 {
                    bands.truncate(k.min(3));
                } else if k < bands.len() {
                    bands[k] = b;
                } else if k == bands.len() {
                    bands.push(b);
                }
            }
            Event::Stretch(st) => s.style.stretch = st,
            Event::Ramp(name) => s.style.ramp = Some(name.to_owned()),
            Event::Invert => s.style.invert = !s.style.invert,
            Event::Resampling(r) => s.style.resampling = r,
            Event::Slice(k, i) => {
                if let Some(d) = s.dataset.as_mut().and_then(|d| d.dims.get_mut(k)) {
                    d.index = i;
                }
            }
            Event::Follow => {
                if let Some(d) = s.dataset.as_mut() {
                    d.follow_time = !d.follow_time;
                }
            }
            Event::Edges => {
                s.style.edges = match s.style.edges {
                    Some(_) => None,
                    None => Some(s.edges_text.trim().to_owned()),
                };
            }
            Event::EdgesColour(text) => {
                if s.style.edges.is_some() {
                    s.style.edges = Some(text.trim().to_owned());
                }
                s.edges_text = text;
            }
            Event::Type(field, text) => {
                let v = read(&text);
                match field {
                    Typed::Min => s.style.min = v,
                    Typed::Max => s.style.max = v,
                    Typed::Azimuth => s.style.azimuth = v,
                    Typed::Altitude => s.style.altitude = v,
                    Typed::ZFactor => s.style.z_factor = v,
                    Typed::Nodata => s.style.nodata = v,
                    Typed::Clear => {}
                }
                s.texts[field as usize] = text;
            }
        }
        self.raster_look_preview();
        Task::none()
    }

    /// The raster as the window says, when that is a raster: its look and opacity.
    fn raster_look_fields(&self) -> Option<RasterFields> {
        let s = self.rasters.look.as_ref()?;
        let clear = read(&s.texts[Typed::Clear as usize]).unwrap_or(0.0);
        let opacity =
            (clear.is_finite() && (0.0..=90.0).contains(&clear)).then_some(1.0 - clear / 100.0)?;
        let mut f = s.original.clone();
        f.style = s.style.clone();
        f.dataset = s.dataset.clone();
        f.opacity = (opacity < 1.0).then_some(opacity);
        (f.problem().is_none() && edit::raster_finite(&f)).then_some(f)
    }

    /// Why Uygula cannot run, when it cannot.
    fn raster_look_problem(&self) -> Option<String> {
        let s = self.rasters.look.as_ref()?;
        let clear = read(&s.texts[Typed::Clear as usize]).unwrap_or(0.0);
        if !(clear.is_finite() && (0.0..=90.0).contains(&clear)) {
            return Some("Saydamlık %0 ile %90 arasında olmalı.".to_owned());
        }
        if let Some(i) = [
            Typed::Min,
            Typed::Max,
            Typed::Azimuth,
            Typed::Altitude,
            Typed::ZFactor,
            Typed::Nodata,
        ]
        .iter()
        .find(|&&t| read(&s.texts[t as usize]).is_some_and(f64::is_nan))
        {
            let _ = i;
            return Some("Sayı olmayan bir değer var; düzeltin ya da boş bırakın.".to_owned());
        }
        let mut f = s.original.clone();
        f.style = s.style.clone();
        f.dataset = s.dataset.clone();
        s.style.problem(s.original.bands).or_else(|| f.problem())
    }

    /// The drawing shows the window's look: the last preview let go, the new one written in its group.
    fn raster_look_preview(&mut self) {
        let fields = self.raster_look_fields();
        let (Some(doc), Some(s)) = (&mut self.document, &mut self.rasters.look) else {
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
        let mut raster = match doc.model.get(s.slot) {
            Some(Entity::Raster(r)) => r.clone(),
            _ => {
                doc.model.cancel_group(group);
                return;
            }
        };
        raster.raster = fields;
        let _ = kentos_interaction::properties::set_geometry(
            &mut doc.model,
            s.slot,
            &Entity::Raster(raster),
        );
        s.preview = Some(group);
    }

    /// Uygula: the preview let go, the look written as one step.
    fn raster_look_apply(&mut self) {
        let Some(fields) = self.raster_look_fields() else {
            return;
        };
        let (Some(doc), Some(s)) = (&mut self.document, &mut self.rasters.look) else {
            return;
        };
        if let Some(g) = s.preview.take() {
            doc.model.cancel_group(g);
        }
        let slot = s.slot;
        let unchanged = fields == s.original;
        self.rasters.look = None;
        self.dialog = None;
        if unchanged {
            return;
        }
        let Some(uid) = doc.model.uid(slot) else {
            return;
        };
        let input = EntitiesEdit {
            operation: EditOperation::RasterStyle,
            changes: vec![EntityEdit::Update {
                uid: uid.to_string(),
                geometry: EntityGeometry::Raster(fields),
            }],
            expected_revision: None,
        };
        match edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { warnings, .. } => {
                super::tiles::service().forget_tiles();
                self.say(
                    Level::Success,
                    "Rasterin görünüşü değişti. Ctrl+Z geri alır.",
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

    /// Vazgeç: the raster as it was.
    pub(crate) fn raster_look_close(&mut self) {
        if let (Some(doc), Some(s)) = (&mut self.document, &mut self.rasters.look)
            && let Some(g) = s.preview.take()
        {
            doc.model.cancel_group(g);
        }
        self.rasters.look = None;
        if self.dialog == Some(Asking::RasterStyle) {
            self.dialog = None;
        }
    }

    pub(crate) fn raster_look_view(&self) -> Element<'_, Message> {
        let Some(s) = &self.rasters.look else {
            return iced::widget::text("").into();
        };
        let bands = s.original.bands;
        let st = &s.style;
        let allowed = |r: RasterRender| match r {
            RasterRender::Rgb => bands >= 3,
            RasterRender::Palette => s.has_palette,
            _ => true,
        };
        let renders: Vec<RasterRender> = RENDERS.into_iter().filter(|&r| allowed(r)).collect();
        let render = Select::new(
            renders
                .iter()
                .map(|&r| Choice::new(render_name(r)))
                .collect::<Vec<_>>(),
            renders.iter().position(|&r| r == st.render),
            {
                let renders = renders.clone();
                move |i| {
                    msg(Event::Render(
                        renders.get(i).copied().unwrap_or(RasterRender::Gray),
                    ))
                }
            },
        );
        let band_choice = |k: usize, none: bool| {
            let mut choices: Vec<Choice> = Vec::new();
            if none {
                choices.push(Choice::new("Yok"));
            }
            choices.extend((1..=bands).map(|b| Choice::new(format!("{b}. bant"))));
            let at = st
                .bands
                .get(k)
                .map(|&b| b as usize - usize::from(!none))
                .or(none.then_some(0));
            Select::new(choices, at, move |i| {
                msg(Event::Band(k, if none { i as u32 } else { i as u32 + 1 }))
            })
        };
        let band_row: Element<'_, Message> = if st.render == RasterRender::Rgb {
            row![
                words::field("Kırmızı", band_choice(0, false), None),
                words::field("Yeşil", band_choice(1, false), None),
                words::field("Mavi", band_choice(2, false), None),
                words::field("Alfa", band_choice(3, true), None),
            ]
            .spacing(12)
            .into()
        } else if st.render == RasterRender::Palette {
            label::caption("Paletli raster kendi renkleriyle çizilir.").into()
        } else {
            words::field("Bant", band_choice(0, false), None)
        };
        let stretches = [
            RasterStretch::None,
            RasterStretch::MinMax,
            RasterStretch::Percent,
            RasterStretch::Manual,
        ];
        let stretch = Segmented::new(stretches.map(Stretch), Stretch(st.stretch), |v| {
            msg(Event::Stretch(v.0))
        });
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
        if let Some(d) = &s.dataset {
            body = body.push(self.raster_look_dataset(s, d));
        }
        body = body.push(row![words::field("Görünüş", render, None), band_row].spacing(16));
        if !matches!(st.render, RasterRender::Hillshade | RasterRender::Palette) {
            let mut part = column![words::field("Gerdirme", stretch, None)].spacing(6);
            if st.stretch == RasterStretch::Manual {
                part = part.push(
                    row![
                        words::field("En küçük", field_input(Typed::Min, "en küçük", 120.0), None),
                        words::field("En büyük", field_input(Typed::Max, "en büyük", 120.0), None),
                    ]
                    .spacing(12),
                );
            }
            part = part.push(self.raster_look_stats(s));
            body = body.push(part);
        }
        if matches!(st.render, RasterRender::Ramp | RasterRender::RampShade) {
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
        }
        if matches!(st.render, RasterRender::Hillshade | RasterRender::RampShade) {
            body = body.push(words::field(
                "Gölgeli kabartma",
                row![
                    words::field("Işığın doğrultusu (°)", field_input(Typed::Azimuth, "315", 110.0), None),
                    words::field("Yüksekliği (°)", field_input(Typed::Altitude, "45", 110.0), None),
                    words::field("Yükseklik çarpanı", field_input(Typed::ZFactor, "1", 110.0), None),
                ]
                .spacing(12),
                Some("Işık kuzeyden saat yönünde ölçülür (315: kuzeybatı); kabartma Horn yöntemiyle.".to_owned()),
            ));
        }
        let sampling = Segmented::new(
            [
                Sampling(RasterResampling::Bilinear),
                Sampling(RasterResampling::Nearest),
            ],
            Sampling(st.resampling),
            |v| msg(Event::Resampling(v.0)),
        );
        body = body.push(
            row![
                words::field(
                    "Nodata",
                    field_input(Typed::Nodata, "dosyanınki", 120.0),
                    None
                ),
                words::field("Saydamlık (%)", field_input(Typed::Clear, "0", 90.0), None),
                words::field("Örnekleme", sampling, None),
            ]
            .spacing(16),
        );
        let problem = self.raster_look_problem();
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
                .width(620.0)
                .max_height(820.0),
        )
    }

    /// Veri seti (docs/adr/0243 §11): each slice dimension's value, Zaman sürgüsünü izle, a mesh's lines.
    fn raster_look_dataset<'a>(&self, s: &'a State, d: &'a RasterDataset) -> Element<'a, Message> {
        let mut name = d.variable.clone();
        if let Some(v) = &d.vector {
            name = format!("{name} / {v} (vektör)");
        }
        let mut part = Column::new().spacing(8).push(label::body(name));
        if !d.dims.is_empty() {
            let mut line = row![].spacing(12);
            for (k, dim) in d.dims.iter().enumerate() {
                let labels = kentos_formats::multidim::cube::dim_labels(
                    &dim.values,
                    dim.time,
                    dim.units.as_deref(),
                );
                let choice = Select::new(
                    labels.into_iter().map(Choice::new).collect::<Vec<_>>(),
                    Some(dim.index as usize),
                    move |i| msg(Event::Slice(k, i as u32)),
                );
                let title = if dim.time { "Zaman" } else { dim.name.as_str() };
                line = line.push(container(words::field(title, choice, None)).width(Fill));
            }
            part = part.push(line);
        }
        if d.dims.iter().any(|x| x.time) {
            part = part.push(words::check(
                d.follow_time,
                "Zaman sürgüsünü izle",
                Some(msg(Event::Follow)),
            ));
        }
        if d.mesh.is_some() {
            let colour = kentos_ui::widget::focus_ring(
                text_input("#2B3440", &s.edges_text)
                    .on_input(|v| msg(Event::EdgesColour(v)))
                    .padding([5, 8])
                    .width(110.0)
                    .size(kentos_ui::theme::typography::body())
                    .style(kentos_ui::style::field::input),
            );
            part = part.push(
                row![
                    words::check(
                        s.style.edges.is_some(),
                        "Ağ çizgileri",
                        Some(msg(Event::Edges))
                    ),
                    words::field("Renk", colour, None),
                ]
                .spacing(16)
                .align_y(Center),
            );
        }
        words::field(
            "Veri seti",
            part,
            Some("Dilimin değerleri ve zaman sürgüsünü izleme çizimde hemen görünür; ağ çizgileri yüzlerin kenarlarıdır.".to_owned()),
        )
    }

    /// The bands' statistics, as the stretch reads them.
    fn raster_look_stats<'a>(&self, s: &'a State) -> Element<'a, Message> {
        let text = match &s.stats {
            None => "Bantların istatistikleri hesaplanıyor…".to_owned(),
            Some(Err(why)) => format!("İstatistik yok: {why}"),
            Some(Ok(stats)) => {
                let bands: Vec<u32> = if s.style.render == RasterRender::Rgb {
                    s.style.bands.iter().copied().take(3).collect()
                } else {
                    s.style.bands.clone()
                };
                bands
                    .iter()
                    .filter_map(|&b| {
                        let i = b as usize - 1;
                        let (min, max) = stats.minmax(i)?;
                        let (lo, hi) = stats.percent(i)?;
                        let n = stat_number;
                        Some(format!(
                            "{b}. bant: en küçük {}, en büyük {}; %2 {}, %98 {}",
                            n(min),
                            n(max),
                            n(lo),
                            n(hi)
                        ))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        };
        let _ = stretch_name;
        label::caption(text).into()
    }
}
