//! Buluttan arazi (plan/road.md Y15.3, docs/adr/0928 §3, `terrain.cloud`): a terrain's points taken from a point cloud.
//! The user picks the cloud, its LAS classes and returns, an area (the selected closed polyline or area), the thinning
//! (the grid's cell, a steep cell's height tolerance, the reach beside the surface's breaklines), the layer the points
//! go on and the surface they feed (a new one, or one of the project's); KentOS fills in none of the numbers. Önizle
//! runs `road.terrains.cloud`'s plan on a reading copy (the cloud is read, nothing written): the points read, kept and
//! why, the coverage and density of the area, and the vertical error of the points left out against the surface made
//! of the kept ones. Uygula writes them (one undo step “Buluttan arazi”).
//!
//! ```text
//! ┌ Buluttan arazi ───────────────────────────────────────────────────────────────────────────────┐
//! │ Bulut [▾]  Dönüş [▾]  Hücre [ ]  Kot toleransı [ ]  Kırık hat erimi [ ]                        │
//! │ Sınıflar ☐ 0 … ☑ 2 Zemin … ☐ 9 Su                                                             │
//! │ Alan: seçili alan (5 köşe) [Seçimden al] [Kaldır]   Yüzey [▾ Yeni yüzey]  Ad [ ]  Katman [▾]   │
//! │ [Önizle]   Okunan … · süzgeçten geçen … · kalan … · kapsama …% · yoğunluk … · hata …           │
//! │                                                                       [Kapat] [Uygula]        │
//! └──────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::{Column, container, row, space};
use iced::{Element, Fill, Task, Theme};
use kentos_commands::{CloudReturns, CommandResult, Entity, SurfaceSources, TerrainsFromCloud, TerrainsFromCloudWritten};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_terrain_cloud};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tree_view::Check;
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Buluttan arazi";
const PREVIEW: &str = "Önizle";
const APPLY: &str = "Uygula";
const CLOSE: &str = "Kapat";
const FROM_SELECTION: &str = "Seçimden al";
const CLEAR_AREA: &str = "Kaldır";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 4] = ["Hücre (m)", "Kot toleransı (m)", "Kırık hat erimi (m)", "Yeni yüzeyin adı"];
/// The classes offered (ASPRS's, LAS 1.4 R15 Table 17): the bridge deck and the overhead structure are never the
/// ground and are not offered.
pub(crate) const CLASSES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const RETURNS: [(CloudReturns, &str); 4] = [
    (CloudReturns::All, "Bütün dönüşler"),
    (CloudReturns::Last, "Son dönüş"),
    (CloudReturns::First, "İlk dönüş"),
    (CloudReturns::Single, "Tek dönüş"),
];

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) cloud: Option<String>,
    pub(crate) classes: Vec<u8>,
    pub(crate) returns: CloudReturns,
    /// The area's id and its corners' count.
    pub(crate) area: Option<(String, usize)>,
    /// A project's surface by its id; none: a new one named by the last field.
    pub(crate) surface: Option<String>,
    pub(crate) layer: Option<String>,
    pub(crate) fields: [String; 4],
    pub(crate) preview: Option<Result<TerrainsFromCloudWritten, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Cloud(String),
    Class(u8),
    Returns(CloudReturns),
    FromSelection,
    ClearArea,
    Surface(Option<String>),
    Layer(String),
    Field(usize, String),
    Preview,
    Apply,
    Close,
}

fn msg(event: Event) -> Message {
    Message::TerrainCloud(event)
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

impl Window {
    /// The command's input as chosen, or why it is not one yet.
    pub(crate) fn input(&self, drawing: Option<String>) -> Result<TerrainsFromCloud, String> {
        let cloud = self.cloud.clone().ok_or("Bir nokta bulutu seçin.")?;
        let cell = number(&self.fields[0], FIELDS[0])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[0]))?;
        let name = self.fields[3].trim();
        if self.surface.is_none() && name.is_empty() {
            return Err("Yeni yüzeyin adını yazın ya da projedeki bir yüzeyi seçin.".into());
        }
        Ok(TerrainsFromCloud {
            clouds: vec![cloud],
            classes: self.classes.clone(),
            returns: self.returns,
            area: self.area.as_ref().map(|a| a.0.clone()),
            cell,
            z_tolerance: number(&self.fields[1], FIELDS[1])?,
            near: number(&self.fields[2], FIELDS[2])?,
            layer_id: self.layer.clone().ok_or("Noktaların yazılacağı katmanı seçin.")?,
            surface: self.surface.clone(),
            name: self.surface.is_none().then(|| name.to_owned()),
            breaklines: SurfaceSources::default(),
            boundary: None,
            drawing,
            expected_revision: None,
        })
    }
}

/// The selected closed polyline's or area's id and corners' count.
fn chosen_area(app: &App) -> Option<(String, usize)> {
    let doc = app.document.as_ref()?;
    let [slot] = app.selection.ids() else { return None };
    let corners = match doc.model.get(*slot)? {
        Entity::Polygon(p) => p.pts.len(),
        Entity::Polyline(p) if p.closed => p.pts.len(),
        _ => return None,
    };
    Some((doc.model.uid(*slot)?.to_string(), corners))
}

impl App {
    /// Buluttan arazi: the window on the drawing's first point cloud.
    pub(crate) fn open_terrain_cloud(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if !kentos_native_application::road_terrains::holds_surfaces(doc.model.settings().cad_work) {
            self.warn("Buluttan arazi Harita ve Yol projelerinde olur.".to_owned());
            return;
        }
        let cloud = doc
            .model
            .entities_with_uids()
            .find_map(|(u, e)| matches!(e, Entity::PointCloud(_)).then(|| u.to_string()));
        let Some(cloud) = cloud else {
            self.warn("Çizimde nokta bulutu yok: önce Ekle › Nokta bulutu ile bir bulut ekleyin.".to_owned());
            return;
        };
        let n = doc.model.settings().road.as_ref().map_or(0, |r| r.surfaces.len()) + 1;
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let area = chosen_area(self);
        self.terrain_cloud_window = Some(Box::new(Window {
            cloud: Some(cloud),
            classes: Vec::new(),
            returns: CloudReturns::All,
            area,
            surface: None,
            layer,
            fields: [String::new(), String::new(), String::new(), format!("Buluttan yüzey {n}")],
            preview: None,
            warnings: Vec::new(),
        }));
        self.dialog = Some(Dialog::TerrainCloud);
    }

    fn preview_terrain_cloud(&mut self) {
        let drawing = self.drawing_path();
        let Some(doc) = &self.document else { return };
        let Some(w) = self.terrain_cloud_window.as_mut() else { return };
        let (preview, warnings) = match w.input(drawing) {
            Err(why) => (Err(why), Vec::new()),
            Ok(input) => match road_terrain_cloud::plan(&ExecutionContext::new(&mut doc.model.reading_copy()), &input) {
                CommandResult::Completed { output, warnings } => {
                    (Ok(output), warnings.into_iter().map(|x| x.message).collect())
                }
                CommandResult::Failed { error } | CommandResult::NeedsInput { error } | CommandResult::Conflict { error } => {
                    (Err(error.message), Vec::new())
                }
                _ => (Err("Önizlenemedi.".into()), Vec::new()),
            },
        };
        w.preview = Some(preview);
        w.warnings = warnings;
    }

    pub(crate) fn terrain_cloud_event(&mut self, event: Event) -> Task<Message> {
        let picked = chosen_area(self);
        let Some(w) = self.terrain_cloud_window.as_mut() else { return Task::none() };
        match event {
            Event::Cloud(c) => w.cloud = Some(c),
            Event::Class(c) => {
                match w.classes.iter().position(|x| *x == c) {
                    Some(i) => {
                        w.classes.remove(i);
                    }
                    None => w.classes.push(c),
                }
                w.classes.sort_unstable();
            }
            Event::Returns(r) => w.returns = r,
            Event::FromSelection => match picked {
                Some(a) => w.area = Some(a),
                None => {
                    self.warn("Alan için kapalı bir çoklu çizgi ya da alan seçin.".to_owned());
                    return Task::none();
                }
            },
            Event::ClearArea => w.area = None,
            Event::Surface(s) => w.surface = s,
            Event::Layer(l) => w.layer = Some(l),
            Event::Field(k, t) => w.fields[k.min(3)] = t,
            Event::Preview => {
                self.preview_terrain_cloud();
                return Task::none();
            }
            Event::Apply => return self.apply_terrain_cloud(),
            Event::Close => {
                self.terrain_cloud_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        // What was previewed was for the values then.
        if let Some(w) = self.terrain_cloud_window.as_mut() {
            w.preview = None;
            w.warnings.clear();
        }
        Task::none()
    }

    fn apply_terrain_cloud(&mut self) -> Task<Message> {
        let drawing = self.drawing_path();
        let Some(w) = self.terrain_cloud_window.as_ref() else { return Task::none() };
        let input = match w.input(drawing) {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Buluttan arazi yazılamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_terrain_cloud::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(
                Level::Success,
                format!(
                    "“{}” yüzeyine {} zemin noktası yazıldı (okunan {}, kapsama %{:.1}).",
                    out.surface.name,
                    out.cloud.kept,
                    out.cloud.read,
                    out.cloud.coverage * 100.0
                ),
            );
            self.terrain_cloud_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    pub(crate) fn terrain_cloud_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.terrain_cloud_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let clouds: Vec<(String, String)> = model.map_or_else(Vec::new, |m| {
            m.entities_with_uids()
                .filter_map(|(u, e)| match e {
                    Entity::PointCloud(c) => Some((
                        u.to_string(),
                        format!("{} · {} nokta", m.layers().get(&c.base.layer_id).map_or("—", |l| l.name.as_str()), c.cloud.count),
                    )),
                    _ => None,
                })
                .collect()
        });
        let cids: Vec<String> = clouds.iter().map(|c| c.0.clone()).collect();
        let on_cloud = Select::new(
            clouds.iter().map(|c| Choice::new(c.1.clone())),
            w.cloud.as_ref().and_then(|c| cids.iter().position(|i| i == c)),
            move |i| msg(Event::Cloud(cids[i].clone())),
        );
        let on_returns = Select::new(
            RETURNS.iter().map(|r| Choice::new(r.1)),
            RETURNS.iter().position(|r| r.0 == w.returns),
            |i| msg(Event::Returns(RETURNS[i].0)),
        );
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![
            words::field("Nokta bulutu", container(on_cloud).width(220), None),
            words::field("Dönüş", container(on_returns).width(150), None),
            field("m", 0, 100.0),
            field("yok", 1, 130.0),
            field("yok", 2, 130.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let check = |c: u8| {
            let on = if w.classes.contains(&c) { Check::Checked } else { Check::Unchecked };
            crate::style::fields::check(on, format!("{c} {}", kentos_pointcloud::classes::name(c)), Some(msg(Event::Class(c))))
        };
        let classes = Column::new()
            .spacing(2)
            .push(label::caption("Sınıflar (ASPRS)").style(muted))
            .push(row(CLASSES[..6].iter().map(|c| container(check(*c)).width(180).into())).spacing(4))
            .push(row(CLASSES[6..].iter().map(|c| container(check(*c)).width(180).into())).spacing(4));
        let area_words = match &w.area {
            Some((_, n)) => format!("Alan: seçili alan ({n} köşe)"),
            None => "Alan: bulutun tümü".to_owned(),
        };
        let surfaces: Vec<(Option<String>, String)> = std::iter::once((None, "Yeni yüzey".to_owned()))
            .chain(
                model
                    .and_then(|m| m.settings().road.as_ref())
                    .map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (Some(s.id.clone()), s.name.clone())).collect()),
            )
            .collect();
        let sids: Vec<Option<String>> = surfaces.iter().map(|s| s.0.clone()).collect();
        let on_surface = Select::new(
            surfaces.iter().map(|s| Choice::new(s.1.clone())),
            sids.iter().position(|s| *s == w.surface),
            move |i| msg(Event::Surface(sids[i].clone())),
        );
        let lids: Vec<String> = self.relation_layers().into_iter().map(|(id, _)| id).collect();
        let lnames: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let on_layer = Select::new(
            lnames.iter().map(|n| Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| lids.iter().position(|id| id == l)),
            move |i| msg(Event::Layer(lids[i].clone())),
        )
        .placeholder("Katman");
        let mut target = row![
            label::body(area_words),
            words::secondary(FROM_SELECTION, chosen_area(self).map(|_| msg(Event::FromSelection))),
            words::secondary(CLEAR_AREA, w.area.as_ref().map(|_| msg(Event::ClearArea))),
            space::horizontal().width(Fill),
            words::field("Yüzey", container(on_surface).width(170), None),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        if w.surface.is_none() {
            target = target.push(field("Ad", 3, 170.0));
        }
        target = target.push(words::field("Katman", container(on_layer).width(170), None));
        let mut body = Column::new().spacing(12).push(top).push(classes).push(target);
        match &w.preview {
            None => {
                body = body.push(
                    label::caption("Önizle bulutu okur ve sonucu yazmadan gösterir; değer değişince yeniden önizleyin.")
                        .style(muted),
                )
            }
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok(o)) => {
                let c = &o.cloud;
                let error = match (c.error_max, c.error_rms) {
                    (Some(m), Some(r)) => format!("en büyük {m:.3} m · karesel ortalama {r:.3} m"),
                    _ => "ölçülmedi".to_owned(),
                };
                let rows: Vec<(&str, String)> = vec![
                    ("Okunan nokta (bütün sınıflar)", o.scanned.to_string()),
                    ("Sınıf, dönüş ve alandan geçen", c.read.to_string()),
                    (
                        "Kalan",
                        format!("{} (hücre {}, dik hücre {}, kırık hat {})", c.kept, o.by_cell, o.by_slope, o.by_line),
                    ),
                    (
                        "Kapsama",
                        format!("%{:.1} · {} / {} hücre · {:.1} m²", c.coverage * 100.0, o.occupied, o.cells, o.area),
                    ),
                    ("Yoğunluk", format!("{:.2} nokta/m²", c.density)),
                    ("Seyreltme hatası", error),
                    ("Yüzeyin dışında kalan", c.outside.to_string()),
                    ("Yüzey", format!("{} · {} köşe", o.surface.name, o.surface.figures.as_ref().map_or(0, |f| f.vertices))),
                ];
                let mut table = Column::new().spacing(4);
                for (k, v) in rows {
                    table = table.push(row![label::caption(k).style(muted).width(230), label::body(v)].spacing(10));
                }
                body = body.push(container(table).padding([8, 10]).width(Fill).style(style::container::field_box));
                for wn in &w.warnings {
                    body = body.push(label::caption(wn.clone()).style(warn));
                }
            }
        }
        let ready = matches!(w.preview, Some(Ok(_)));
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer =
            row![
                words::secondary(PREVIEW, Some(msg(Event::Preview))),
                space::horizontal().width(Fill),
                words::secondary(CLOSE, Some(msg(Event::Close))),
                words::primary(APPLY, ready.then(|| msg(Event::Apply))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center);
        Frame::new(TITLE).scroll(body).footer(footer).width(1100.0).modal(msg(Event::Close))
    }

    pub(crate) fn terrain_cloud_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.terrain_cloud_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(FROM_SELECTION) => Some(msg(Event::FromSelection)),
            Control::Press(CLEAR_AREA) => Some(msg(Event::ClearArea)),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
