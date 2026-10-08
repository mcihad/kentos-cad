//! Raster oturt (docs/adr/0204 §6; the web's `ui/raster/RasterGeorefDialog.ts`):
//! a raster fitted to the drawing by control points. Each row is a pixel
//! of the raster (column and row, shown on the raster where it is drawn)
//! and where it is to go (Y and X, shown on the drawing, snapped, or
//! typed); Kullan leaves a row out of the solution. The transform is the
//! geometry core's (`ops::georef`): Helmert, affine, projective, polynomials
//! of order 2 and 3, thin plate; every row's residual (metres) and m0,
//! solved again at every change. Uygula: Helmert and affine set the
//! raster's affine (one step, Raster oturt), with a world file beside a
//! linked raster when asked; the others resample the raster on a thread of
//! its own (`rasters::jobs`) into a GeoTIFF beside it (`<ad>-oturtulmus.tif`)
//! or, embedded, into the project's library, and the raster shows that.

use iced::widget::tooltip::Position;
use iced::widget::{button, column, container, row};
use iced::{Element, Fill, Task};
use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEdit, Entity, EntityEdit, EntityGeometry, RasterFields,
    RasterRender, RasterSample, RasterStretch,
};
use kentos_domain::{Document as Model, Slot};
use kentos_geometry_core::ops::georef::{Gcp, Georef, GeorefError, Method, solve};
use kentos_interaction::{Level, Vec2, fixed};
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Tip, tip};

use super::grid::{Col, Mark, Table};
use super::read::read_number;
use super::{Event as Calc, MAX_HEIGHT, Window, event, footer_button, grid, summary};
use crate::app::{App, Message};
use crate::exchange::words::{self, Kind as Line};
use crate::rasters::jobs::{self, Output, Warped};

pub const TITLE: &str = "Raster oturt";

pub const USE: usize = 0;
pub const COLUMN: usize = 1;
pub const ROW: usize = 2;
pub const TARGET_Y: usize = 3;
pub const TARGET_X: usize = 4;
pub const RES_Y: usize = 5;
pub const RES_X: usize = 6;
pub const RES: usize = 7;

const COLUMNS: [Col; 8] = [
    col("Kullan", None, false),
    col("Sütun", Some("piksel"), true),
    col("Satır", Some("piksel"), true),
    col("Hedef Y", None, true),
    col("Hedef X", None, true),
    col("vY", Some("m"), true),
    col("vX", Some("m"), true),
    col("v", Some("m"), true),
];

const fn col(label: &'static str, unit: Option<&'static str>, numeric: bool) -> Col {
    Col {
        label,
        unit,
        numeric,
    }
}

/// The transforms in the window's order.
const METHODS: [Named; 6] = [
    Named(Method::Helmert),
    Named(Method::Affine),
    Named(Method::Projective),
    Named(Method::Poly2),
    Named(Method::Poly3),
    Named(Method::ThinPlate),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Named(pub Method);

impl std::fmt::Display for Named {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            Method::Helmert => "Helmert",
            Method::Affine => "Afin",
            Method::Projective => "Projektif",
            Method::Poly2 => "Polinom 2",
            Method::Poly3 => "Polinom 3",
            Method::ThinPlate => "İnce plaka",
        })
    }
}

/// What a transform does, under the choice.
fn method_hint(m: Method) -> &'static str {
    match m {
        Method::Helmert => {
            "Öteler, döndürür, ölçekler; en az 2 nokta. Rasterin dönüşümü değişir, yeniden örneklenmez."
        }
        Method::Affine => {
            "İki yönde ayrı ölçek ve kayma da; en az 3 nokta. Rasterin dönüşümü değişir, yeniden örneklenmez."
        }
        Method::Projective => {
            "Eğik çekilmiş fotoğraf için; en az 4 nokta. Raster yeniden örneklenir."
        }
        Method::Poly2 => "Hafif bükülmeler için; en az 6 nokta. Raster yeniden örneklenir.",
        Method::Poly3 => "Daha güçlü bükülmeler için; en az 10 nokta. Raster yeniden örneklenir.",
        Method::ThinPlate => {
            "Noktalardan tam geçer (artık yok); en az 3 nokta. Raster yeniden örneklenir."
        }
    }
}

/// Which end of a row a pick is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Source,
    Target,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// The raster fitted, by its slot (from the list).
    Raster(u32),
    /// The raster shown on the drawing.
    PickRaster,
    Method(Method),
    Pick(usize, Side),
    Pixel(String),
    Nearest(bool),
    World(bool),
    Apply,
    Warped(Result<Warped, String>),
}

pub fn raster_fit_event(e: Event) -> Message {
    event(Calc::RasterFit(e))
}

/// What is typed, kept while the app runs; four empty rows at first.
#[derive(Clone, Debug)]
pub struct Form {
    pub raster: Option<Slot>,
    pub method: Method,
    pub rows: Vec<[String; 8]>,
    /// The output's pixel as typed (empty: the transform's mean scale).
    pub pixel: String,
    pub nearest: bool,
    pub world: bool,
    pub status: Option<String>,
    pub(super) picking: Option<(usize, Side)>,
    pub(super) picking_raster: bool,
    solution: Option<Result<Georef, GeorefError>>,
    pair_rows: Vec<usize>,
    /// A resampling runs: Uygula waits.
    pub(super) warping: bool,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            raster: None,
            method: Method::Affine,
            rows: vec![Default::default(); 4],
            pixel: String::new(),
            nearest: false,
            world: false,
            status: None,
            picking: None,
            picking_raster: false,
            solution: None,
            pair_rows: Vec::new(),
            warping: false,
        }
    }
}

impl Table for Form {
    fn columns(&self) -> usize {
        COLUMNS.len()
    }

    fn rows(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, row: usize, col: usize) -> &str {
        self.rows.get(row).map_or("", |r| r[col].as_str())
    }

    fn set(&mut self, row: usize, col: usize, text: String) {
        if let Some(r) = self.rows.get_mut(row) {
            r[col] = text;
        }
    }

    fn readonly(&self, _row: usize, col: usize) -> bool {
        col >= RES_Y
    }

    fn check(&self, col: usize) -> bool {
        col == USE
    }

    fn mark(&self, row: usize) -> Option<Mark> {
        if self.rows.get(row).is_some_and(|r| r[USE] == "0") {
            return Some(Mark::Off);
        }
        let g = self.solution.as_ref()?.as_ref().ok()?;
        g.m0?;
        let worst = self
            .pair_rows
            .iter()
            .zip(&g.residuals)
            .filter(|(r, _)| self.rows.get(**r).is_some_and(|row| row[USE] != "0"))
            .max_by(|a, b| a.1[2].total_cmp(&b.1[2]))?;
        (*worst.0 == row).then_some(Mark::Worst)
    }

    fn insert_after(&mut self, row: usize) {
        let at = (row + 1).min(self.rows.len());
        self.rows.insert(at, Default::default());
    }

    fn can_remove(&self, _row: usize) -> bool {
        self.rows.len() > 1
    }

    fn remove(&mut self, row: usize) {
        if self.rows.len() > 1 && row < self.rows.len() {
            self.rows.remove(row);
        }
    }
}

/// A row's control point, when its four numbers are.
fn gcp_of(row: &[String; 8]) -> Option<Gcp> {
    let n = |c: usize| read_number(&row[c]).filter(|v| v.is_finite());
    Some(Gcp {
        pixel: Vec2::new(n(COLUMN)?, n(ROW)?),
        target: Vec2::new(n(TARGET_Y)?, n(TARGET_X)?),
        used: row[USE] != "0",
    })
}

/// The rasters of the drawing: slot and the name their layer gives them.
fn rasters(model: &Model) -> Vec<(Slot, String)> {
    let layers = model.layers();
    model
        .entities()
        .filter_map(|e| match e {
            Entity::Raster(r) => Some((Slot(r.base.id), layers.path(&r.base.layer_id))),
            _ => None,
        })
        .collect()
}

impl Form {
    /// The raster fitted, when the drawing has it.
    fn fields(&self, model: &Model) -> Option<RasterFields> {
        match model.get(self.raster?) {
            Some(Entity::Raster(r)) => Some(r.raster.clone()),
            _ => None,
        }
    }

    /// What the drawing offers when the window opens: the selected raster, else the one the window had, else the first.
    pub fn sync(&mut self, model: &Model, selected: &[Slot]) {
        let all = rasters(model);
        let chosen = selected
            .iter()
            .find(|s| all.iter().any(|(r, _)| r == *s))
            .copied();
        if chosen.is_some() {
            self.raster = chosen;
        } else if self
            .raster
            .is_none_or(|r| !all.iter().any(|(s, _)| *s == r))
        {
            self.raster = all.first().map(|(s, _)| *s);
        }
        self.status = None;
        self.solve();
    }

    /// The rows solved again, the residuals into their cells.
    pub fn solve(&mut self) {
        let mut points = Vec::new();
        self.pair_rows.clear();
        for (r, row) in self.rows.iter_mut().enumerate() {
            for c in [RES_Y, RES_X, RES] {
                row[c].clear();
            }
            if let Some(p) = gcp_of(row) {
                points.push(p);
                self.pair_rows.push(r);
            }
        }
        self.solution = (!points.is_empty()).then(|| solve(&points, self.method));
        if let Some(Ok(g)) = &self.solution
            && self.method != Method::ThinPlate
        {
            for (&r, [vx, vy, v]) in self.pair_rows.iter().zip(&g.residuals) {
                let row = &mut self.rows[r];
                row[RES_Y] = if vx.is_finite() {
                    fixed(*vx, 3)
                } else {
                    "—".to_owned()
                };
                row[RES_X] = if vy.is_finite() {
                    fixed(*vy, 3)
                } else {
                    "—".to_owned()
                };
                row[RES] = if v.is_finite() {
                    fixed(*v, 3)
                } else {
                    "—".to_owned()
                };
            }
        }
    }

    /// The used points, as the core takes them.
    pub fn points(&self) -> Vec<Gcp> {
        self.rows.iter().filter_map(gcp_of).collect()
    }

    /// A pick's point: the pixel under it on the raster, or the target.
    pub fn picked(&mut self, model: &Model, row: usize, side: Side, p: Vec2) {
        let Some(r) = self.rows.get_mut(row) else {
            return;
        };
        match side {
            Side::Source => {
                let Some(f) = (match self.raster.and_then(|s| model.get(s)) {
                    Some(Entity::Raster(x)) => Some(x.raster.clone()),
                    _ => None,
                }) else {
                    return;
                };
                if let Some(q) = kentos_geometry_core::geom::raster::pixel_of(f.affine, p) {
                    r[COLUMN] = fixed(q.x, 2);
                    r[ROW] = fixed(q.y, 2);
                }
            }
            Side::Target => {
                r[TARGET_Y] = fixed(p.x, 3);
                r[TARGET_X] = fixed(p.y, 3);
            }
        }
        if r[USE].is_empty() {
            r[USE] = "1".to_owned();
        }
    }

    /// The summary: the solution's state, m0 and what Uygula does.
    fn summary_lines(&self, model: &Model) -> Vec<(Line, String)> {
        let mut out = Vec::new();
        let Some(fields) = self.fields(model) else {
            out.push((
                Line::Warn,
                "Çizimde raster yok ya da seçili değil: önce bir raster seçin.".to_owned(),
            ));
            return out;
        };
        match &self.solution {
            None => out.push((
                Line::Info,
                format!(
                    "Kontrol noktalarını girin: Sütun ve Satır rasterin üstünde, Hedef çizimde gösterilir. {} en az {} nokta ister.",
                    Named(self.method),
                    self.method.need()
                ),
            )),
            Some(Err(GeorefError::TooFew(n))) => out.push((
                Line::Warn,
                format!("{} en az {n} kullanılan nokta ister.", Named(self.method)),
            )),
            Some(Err(GeorefError::Collinear)) => out.push((
                Line::Warn,
                "Noktaların pikselleri bir doğru üzerinde; rasterin iki yönüne yayılmış noktalar seçin.".to_owned(),
            )),
            Some(Err(GeorefError::Duplicate)) => out.push((
                Line::Warn,
                "İki kullanılan nokta aynı pikselde; birini düzeltin ya da Kullan'dan çıkarın.".to_owned(),
            )),
            Some(Err(GeorefError::Singular)) => out.push((
                Line::Warn,
                "Bu noktalarla dönüşüm çözülemiyor; noktaları rasterin geneline yayın.".to_owned(),
            )),
            Some(Ok(g)) => {
                let m0 = match g.m0 {
                    Some(m0) => format!("m0 = ±{} m.", fixed(m0, 3)),
                    None if self.method == Method::ThinPlate => "İnce plaka noktalardan tam geçer; artık yok.".to_owned(),
                    None => "Fazla nokta yok: m0 hesaplanmaz.".to_owned(),
                };
                out.push((Line::Ok, format!("{} çözüldü. {m0}", Named(self.method))));
                let how = if self.method.affine() {
                    "Uygula rasterin dönüşümünü değiştirir (tek adım)."
                } else if fields.file.is_some() {
                    "Uygula rasteri yeniden örnekleyip yanına “-oturtulmus.tif” olarak yazar ve raster onu gösterir (tek adım)."
                } else {
                    "Uygula rasteri yeniden örnekleyip projenin kitaplığına gömer ve raster onu gösterir (tek adım)."
                };
                out.push((Line::Info, how.to_owned()));
            }
        }
        out
    }

    pub fn view<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        let all = rasters(model);
        let ids: Vec<u32> = all.iter().map(|(s, _)| s.0).collect();
        let selected = self
            .raster
            .and_then(|r| all.iter().position(|(s, _)| *s == r));
        let list = Select::new(
            all.iter()
                .map(|(_, name)| Choice::new(name.clone()))
                .collect::<Vec<_>>(),
            selected,
            move |i| raster_fit_event(Event::Raster(ids.get(i).copied().unwrap_or(0))),
        )
        .searchable(false);
        let raster = words::field(
            "Raster",
            row![
                container(list).width(Fill),
                pick_button(
                    Event::PickRaster,
                    "Rasteri çizimden seç",
                    "Rasterin üstüne tıklayın."
                )
            ]
            .spacing(6),
            None,
        );
        let method = words::field(
            "Dönüşüm",
            Segmented::new(METHODS, Named(self.method), |m| {
                raster_fit_event(Event::Method(m.0))
            }),
            Some(method_hint(self.method).to_owned()),
        );
        let table = grid::view_with(
            Window::RasterFit,
            &COLUMNS,
            self,
            |_, _| String::new(),
            2,
            move |r| {
                vec![
                    pick_button(
                        Event::Pick(r, Side::Source),
                        &format!("{}. noktanın pikselini rasterde göster", r + 1),
                        "Rasterin üstünde, noktanın göründüğü yere tıklayın.",
                    ),
                    pick_button_pin(
                        Event::Pick(r, Side::Target),
                        &format!("{}. noktanın hedefini çizimde göster", r + 1),
                        "Noktanın doğru yerine tıklayın; kenet çalışır.",
                    ),
                ]
            },
        );
        let mut body = column![row![container(raster).width(Fill)].spacing(16), method].spacing(16);
        body = body.push(column![label::strong("Kontrol noktaları"), table].spacing(8));
        if let Some(s) = summary(self.summary_lines(model)) {
            body = body.push(s);
        }
        let linked = self.fields(model).is_some_and(|f| f.file.is_some());
        let options: Element<'a, Message> = if self.method.affine() {
            words::field(
                "Dünya dosyası",
                words::check(
                    self.world && linked,
                    "Rasterin yanına dünya dosyası da yaz",
                    linked.then(|| raster_fit_event(Event::World(!self.world))),
                ),
                (!linked).then(|| "Gömülü rasterin dosyası yok.".to_owned()),
            )
        } else {
            row![
                super::number_field(
                    "Çıktının piksel boyu (m)",
                    &self.pixel,
                    "dönüşümden",
                    |t| { raster_fit_event(Event::Pixel(t)) }
                ),
                words::field(
                    "Örnekleme",
                    words::check(
                        self.nearest,
                        "En yakın piksel (sınıflı rasterler için)",
                        Some(raster_fit_event(Event::Nearest(!self.nearest)))
                    ),
                    None
                ),
            ]
            .spacing(18)
            .into()
        };
        body = body.push(options);
        let can =
            !self.warping && matches!(self.solution, Some(Ok(_))) && self.fields(model).is_some();
        let mut dialog = Dialog::new(TITLE)
            .scroll(body)
            .action(footer_button("Kapat", Some(event(Calc::Close)), false))
            .action(footer_button(
                "Uygula",
                can.then(|| raster_fit_event(Event::Apply)),
                true,
            ))
            .max_height(MAX_HEIGHT)
            .width(900.0);
        if let Some(status) = &self.status {
            dialog = dialog.aside(label::caption(status.clone()).style(style::text::danger));
        }
        dialog.into()
    }
}

fn pick_button<'a>(on: Event, title: &str, body: &'static str) -> Element<'a, Message> {
    tip(
        button(icon(crate::icons::from_web(Some("target"))).size(14.0))
            .on_press(raster_fit_event(on))
            .padding(6)
            .style(style::button::ghost),
        Tip::new(title.to_owned()).body(body),
        Position::Top,
    )
}

fn pick_button_pin<'a>(on: Event, title: &str, body: &'static str) -> Element<'a, Message> {
    tip(
        button(icon(crate::icons::from_web(Some("pin"))).size(14.0))
            .on_press(raster_fit_event(on))
            .padding(6)
            .style(style::button::ghost),
        Tip::new(title.to_owned()).body(body),
        Position::Top,
    )
}

/// The look of a raster after its resampling: an RGBA one shows its colours.
fn look_after(fields: &RasterFields, w: &Warped) -> kentos_contracts::RasterStyle {
    let mut style = fields.style.clone();
    if w.alpha {
        style.render = RasterRender::Rgb;
        style.bands = vec![1, 2, 3, 4];
        style.stretch = RasterStretch::None;
        style.ramp = None;
    }
    style
}

impl App {
    pub(crate) fn raster_fit_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Raster(slot) => self.calc.raster_fit.raster = Some(Slot(slot)),
            Event::PickRaster => {
                self.calc.raster_fit.picking_raster = true;
                self.calc_pick(TITLE, "rasterin üstüne tıklayın".to_owned());
            }
            Event::Method(m) => self.calc.raster_fit.method = m,
            Event::Pick(row, side) => {
                let label = format!(
                    "{}. noktanın {}",
                    row + 1,
                    match side {
                        Side::Source => "rasterdeki yeri",
                        Side::Target => "hedefi",
                    }
                );
                self.calc.raster_fit.picking = Some((row, side));
                self.calc_pick(TITLE, label);
            }
            Event::Pixel(t) => self.calc.raster_fit.pixel = t,
            Event::Nearest(on) => self.calc.raster_fit.nearest = on,
            Event::World(on) => self.calc.raster_fit.world = on,
            Event::Apply => return self.raster_fit_apply(),
            Event::Warped(result) => {
                self.calc.raster_fit.warping = false;
                self.raster_fit_warped(result);
            }
        }
        Task::none()
    }

    /// A pick's answer for Raster oturt: the raster under the point, or a row's end.
    pub(super) fn raster_fit_picked(&mut self, p: Option<Vec2>) -> bool {
        let form = &mut self.calc.raster_fit;
        if form.picking_raster {
            form.picking_raster = false;
            if let (Some(p), Some(doc)) = (p, &self.document) {
                let under =
                    rasters(&doc.model)
                        .into_iter()
                        .rev()
                        .find(|(s, _)| match doc.model.get(*s) {
                            Some(Entity::Raster(r)) => {
                                let x = &r.raster;
                                kentos_geometry_core::geom::raster::pixel_of(x.affine, p)
                                    .is_some_and(|q| {
                                        q.x >= 0.0
                                            && q.y >= 0.0
                                            && q.x < f64::from(x.width)
                                            && q.y < f64::from(x.height)
                                    })
                            }
                            _ => false,
                        });
                if let Some((slot, _)) = under {
                    self.calc.raster_fit.raster = Some(slot);
                }
            }
            return true;
        }
        let Some((row, side)) = form.picking.take() else {
            return false;
        };
        if let (Some(p), Some(doc)) = (p, &self.document) {
            self.calc.raster_fit.picked(&doc.model, row, side, p);
        }
        true
    }

    /// Uygula: an affine set in one step, or the resampling started.
    fn raster_fit_apply(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let form = &self.calc.raster_fit;
        let (Some(slot), Some(Ok(g))) = (form.raster, &form.solution) else {
            return Task::none();
        };
        let Some(fields) = form.fields(&doc.model) else {
            return Task::none();
        };
        let srid = doc.settings().srid;
        if let Some(affine) = g.affine {
            let mut f = fields.clone();
            f.affine = affine;
            f.srid = srid;
            let world = form.world;
            if self.raster_fit_write(slot, f, "dönüşümü değişti") && world {
                self.raster_fit_world(&fields, &affine);
            }
            return Task::none();
        }
        // Resampled: beside a linked file, or into memory to embed.
        let folder = doc
            .path
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf);
        let key = crate::rasters::key_of(&fields);
        let Some(origin) = crate::rasters::origin_of(&key, &doc.model, folder.as_deref()) else {
            self.calc.raster_fit.status = Some("Rasterin dosyası bulunamadı.".to_owned());
            return Task::none();
        };
        let (output, name) = match &fields.file {
            Some(file) => {
                let path = crate::pictures::resolve(file, folder.as_deref());
                let stem = path
                    .file_stem()
                    .map_or_else(|| "raster".to_owned(), |s| s.to_string_lossy().into_owned());
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                (
                    Output::File(path.with_file_name(format!("{stem}-oturtulmus.tif"))),
                    name,
                )
            }
            None => (
                Output::Memory,
                self.layer_name(&match doc.model.get(slot) {
                    Some(Entity::Raster(r)) => r.base.layer_id.clone(),
                    _ => String::new(),
                }),
            ),
        };
        let points = form.points();
        let method = form.method;
        let pixel = read_number(&form.pixel).filter(|v| v.is_finite() && *v > 0.0);
        let nearest = form.nearest;
        self.calc.raster_fit.warping = true;
        self.calc.raster_fit.status = None;
        self.calc.open = None;
        self.dialog = None;
        self.say(
            Level::Info,
            format!(
                "{TITLE}: “{name}” yeniden örnekleniyor; sağ alttaki panel ilerlemeyi gösterir."
            ),
        );
        crate::rasters::off_thread(move || {
            let result = jobs::warp(name, origin, output, move |reader| {
                let g = solve(&points, method).map_err(|e| e.code().to_owned())?;
                let forward = |p: kentos_geometry_core::vec2::Vec2| g.forward(p);
                let used: Vec<kentos_geometry_core::vec2::Vec2> =
                    points.iter().filter(|p| p.used).map(|p| p.pixel).collect();
                let size = pixel
                    .or_else(|| kentos_formats::raster::warp::default_pixel(&forward, &used))
                    .ok_or_else(|| "Çıktının piksel boyu bulunamadı.".to_owned())?;
                let i = &reader.info;
                let edge: Vec<_> = kentos_formats::raster::warp::border(i.width, i.height, 64)
                    .into_iter()
                    .filter_map(forward)
                    .collect();
                let grid =
                    kentos_formats::raster::warp::Grid::covering(&edge, size).ok_or_else(|| {
                        "Çıktının ızgarası kurulamadı: dönüşüm rasteri çok büyütüyor ya da bozuyor."
                            .to_owned()
                    })?;
                let (w, h, bands, sample) = (i.width, i.height, i.bands, i.sample);
                let inverse = Box::new(move |p| g.inverse(p));
                kentos_formats::raster::warp::Job::new(
                    grid,
                    inverse,
                    w,
                    h,
                    bands,
                    sample,
                    reader.nodata,
                    reader.palette.clone(),
                    nearest,
                    (srid > 0).then_some(srid),
                    false,
                )
                .map_err(|e| e.0)
            });
            raster_fit_event(Event::Warped(result))
        })
    }

    /// The resampled raster in place of the old: its grid, kind and file, one step.
    fn raster_fit_warped(&mut self, result: Result<Warped, String>) {
        let w = match result {
            Ok(w) => w,
            Err(why) if why == "durduruldu" => {
                self.say(
                    Level::Info,
                    format!("{TITLE}: durduruldu; raster değişmedi."),
                );
                return;
            }
            Err(why) => {
                self.warn(format!("{TITLE}: {why}"));
                return;
            }
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let form = &self.calc.raster_fit;
        let (Some(slot), Some(fields)) = (form.raster, form.fields(&doc.model)) else {
            return;
        };
        let mut f = fields.clone();
        f.affine = w.affine;
        f.width = w.width;
        f.height = w.height;
        f.bands = w.bands;
        f.sample = if w.alpha { RasterSample::U8 } else { w.sample };
        f.srid = doc.settings().srid;
        f.style = look_after(&fields, &w);
        match (&w.path, &w.bytes) {
            (Some(path), _) => {
                f.file = Some(path.to_string_lossy().into_owned());
                f.asset = None;
            }
            (None, Some(bytes)) => {
                // An embedded raster's own name is its layer's.
                let layer = match doc.model.get(slot) {
                    Some(Entity::Raster(r)) => doc
                        .model
                        .layers()
                        .get(&r.base.layer_id)
                        .map_or_else(|| "raster".to_owned(), |l| l.name.clone()),
                    _ => "raster".to_owned(),
                };
                let name = format!("{layer}-oturtulmus.tif");
                match crate::rasters::library_item(&name, bytes, w.width, w.height) {
                    Ok((id, item)) => {
                        crate::rasters::keep_in_library(&mut doc.model, &id, item);
                        f.asset = Some(id);
                        f.file = None;
                    }
                    Err(why) => {
                        self.warn(format!("{TITLE}: {why}"));
                        return;
                    }
                }
            }
            (None, None) => return,
        }
        self.raster_fit_write(slot, f, "yeniden örneklendi");
    }

    /// The raster's new fields through `cad.entities.edit`'s `rasterGeoref`, one step; whether written.
    fn raster_fit_write(&mut self, slot: Slot, f: RasterFields, done: &str) -> bool {
        let Some(doc) = &mut self.document else {
            return false;
        };
        let Some(uid) = doc.model.uid(slot) else {
            return false;
        };
        let input = EntitiesEdit {
            operation: EditOperation::RasterGeoref,
            changes: vec![EntityEdit::Update {
                uid: uid.to_string(),
                geometry: EntityGeometry::Raster(f),
            }],
            expected_revision: None,
        };
        match kentos_native_application::edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            input,
        ) {
            CommandResult::Completed { warnings, .. } => {
                crate::rasters::tiles::service().forget_tiles();
                self.calc.open = None;
                self.dialog = None;
                self.say(
                    Level::Success,
                    format!("{TITLE}: rasterin {done}. Ctrl+Z geri alır."),
                );
                for w in warnings {
                    self.warn(w.message);
                }
                self.zoom_to(&[slot]);
                true
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.calc.raster_fit.status = Some(error.message);
                false
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => false,
        }
    }

    /// The world file beside a linked raster, for its new affine.
    fn raster_fit_world(&mut self, fields: &RasterFields, affine: &[f64; 6]) {
        let Some(file) = &fields.file else {
            return;
        };
        let folder = self
            .document
            .as_ref()
            .and_then(|d| d.path.as_deref())
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf);
        let path = crate::pictures::resolve(file, folder.as_deref());
        let ext = path
            .extension()
            .map_or_else(|| "tif".to_owned(), |e| e.to_string_lossy().into_owned());
        let target = path.with_extension(kentos_formats::raster::world::extension_for(&ext));
        match std::fs::write(&target, kentos_formats::raster::world::write(affine)) {
            Ok(()) => self.say(
                Level::Info,
                format!(
                    "{TITLE}: dünya dosyası “{}” yazıldı.",
                    target
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                ),
            ),
            Err(e) => self.warn(format!("{TITLE}: dünya dosyası yazılamadı: {e}.")),
        }
    }
}
