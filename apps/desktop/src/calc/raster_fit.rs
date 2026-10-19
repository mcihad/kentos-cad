//! Raster oturt (docs/adr/0204 §6, docs/adr/0518; the web's `ui/raster/RasterGeorefDialog.ts`): a raster fitted to the
//! drawing by control points, as a record the raster keeps. Each row is a pixel of the raster's source (column and row,
//! shown on the raster where it is drawn) and where it is to go (Y and X, shown on the drawing, snapped, or typed, in the
//! project's system or in “Hedeflerin sistemi”); Kullan leaves a row out, Denetim makes it a check point (not fitted to,
//! its error measures the fit). The window solves again at every change by `cad.rasters.georef`'s own fit
//! (`rasters_georef::fit`): every row's residual or error, m0, the check points' errors, a resampling's grid. Uygula
//! writes by the command: Helmert and the affine at once; the others resample the source on a thread of its own
//! (`rasters::jobs`) by the command's plan (its grid, its inverse), then hand the output to the command (`made`). A
//! raster with a record opens with its points; Noktaları al… and Noktaları kaydet… read and write QGIS's `.points`.

use iced::widget::tooltip::Position;
use iced::widget::{button, column, container, row};
use iced::{Element, Fill, Task};
use kentos_contracts::{
    CommandError, CommandResult, Entity, GeorefMade, GeorefMethod, GeorefPoint, RasterFields, RasterGeoref,
    RasterResampling, RastersGeoref,
};
use kentos_domain::{Document as Model, Slot};
use kentos_formats::raster::gcp;
use kentos_interaction::{Level, Vec2, fixed};
use kentos_native_application::rasters_georef::{self, Fitted};
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Tip, tip};

use super::grid::{Col, Mark, Table};
use super::read::read_number;
use super::{Event as Calc, MAX_HEIGHT, Window, event, footer_button, grid, summary};
use crate::app::{App, Message, Picker};
use crate::exchange::words::{self, Kind as Line};
use crate::rasters::jobs::{self, Output, Warped};

pub const TITLE: &str = "Raster oturt";

pub const USE: usize = 0;
pub const CHECK: usize = 1;
pub const NAME: usize = 2;
pub const COLUMN: usize = 3;
pub const ROW: usize = 4;
pub const TARGET_Y: usize = 5;
pub const TARGET_X: usize = 6;
pub const RES_Y: usize = 7;
pub const RES_X: usize = 8;
pub const RES: usize = 9;

/// A row of the points' table.
pub type Row = [String; 10];

const COLUMNS: [Col; 10] = [
    col("Kullan", None, false),
    col("Denetim", None, false),
    col("Ad", None, false),
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
    Named(GeorefMethod::Helmert),
    Named(GeorefMethod::Affine),
    Named(GeorefMethod::Projective),
    Named(GeorefMethod::Poly2),
    Named(GeorefMethod::Poly3),
    Named(GeorefMethod::ThinPlate),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Named(pub GeorefMethod);

impl std::fmt::Display for Named {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.title())
    }
}

/// What a transform does, under the choice.
fn method_hint(m: GeorefMethod) -> &'static str {
    match m {
        GeorefMethod::Helmert => {
            "Öteler, döndürür, ölçekler; en az 2 kontrol noktası. Rasterin dönüşümü değişir, yeniden örneklenmez."
        }
        GeorefMethod::Affine => {
            "İki yönde ayrı ölçek ve kayma da; en az 3 kontrol noktası. Rasterin dönüşümü değişir, yeniden örneklenmez."
        }
        GeorefMethod::Projective => "Eğik çekilmiş fotoğraf için; en az 4 kontrol noktası. Raster yeniden örneklenir.",
        GeorefMethod::Poly2 => "Hafif bükülmeler için; en az 6 kontrol noktası. Raster yeniden örneklenir.",
        GeorefMethod::Poly3 => "Daha güçlü bükülmeler için; en az 10 kontrol noktası. Raster yeniden örneklenir.",
        GeorefMethod::ThinPlate => {
            "Kontrol noktalarından tam geçer (artık yok; doğruluğu denetim noktaları söyler); en az 3. Raster yeniden örneklenir."
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
    Method(GeorefMethod),
    Pick(usize, Side),
    Srid(String),
    Pixel(String),
    Nearest(bool),
    World(bool),
    /// Noktaları al…: the file chosen (none: the dialog was closed).
    Import,
    Imported(Option<std::path::PathBuf>),
    /// Noktaları kaydet….
    Export,
    Exported(Option<std::path::PathBuf>),
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
    pub method: GeorefMethod,
    pub rows: Vec<Row>,
    /// The targets' system, an EPSG code (empty: the project's).
    pub srid: String,
    /// The output's pixel as typed (empty: the transform's mean scale).
    pub pixel: String,
    pub nearest: bool,
    pub world: bool,
    pub status: Option<String>,
    pub(super) picking: Option<(usize, Side)>,
    pub(super) picking_raster: bool,
    solution: Option<Result<Fitted, Box<CommandError>>>,
    pair_rows: Vec<usize>,
    /// A resampling runs: Uygula waits; the command's input it will be written with.
    pub(super) warping: bool,
    pending: Option<RastersGeoref>,
    /// The raster whose record the rows were last opened from.
    opened: Option<Slot>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            raster: None,
            method: GeorefMethod::Affine,
            rows: vec![Default::default(); 4],
            srid: String::new(),
            pixel: String::new(),
            nearest: false,
            world: false,
            status: None,
            picking: None,
            picking_raster: false,
            solution: None,
            pair_rows: Vec::new(),
            warping: false,
            pending: None,
            opened: None,
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
        match self.rows.get(row) {
            // Denetim is off until it is ticked.
            Some(r) if col == CHECK && r[CHECK].is_empty() => "0",
            Some(r) => r[col].as_str(),
            None => "",
        }
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
        col == USE || col == CHECK
    }

    fn mark(&self, row: usize) -> Option<Mark> {
        if self.rows.get(row).is_some_and(|r| r[USE] == "0") {
            return Some(Mark::Off);
        }
        let f = self.solution.as_ref()?.as_ref().ok()?;
        f.m0?;
        let worst = self
            .pair_rows
            .iter()
            .zip(&f.residuals)
            .filter(|(r, _)| self.rows.get(**r).is_some_and(|row| row[USE] != "0" && row[CHECK] != "1"))
            .filter_map(|(r, v)| Some((r, v.d?)))
            .max_by(|a, b| a.1.total_cmp(&b.1))?;
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

/// A row's point, when its four numbers are; Denetim on a row left out is not a check point.
fn point_of(row: &Row) -> Option<GeorefPoint> {
    let n = |c: usize| read_number(&row[c]).filter(|v| v.is_finite());
    let off = row[USE] == "0";
    let name = row[NAME].trim();
    Some(GeorefPoint {
        name: (!name.is_empty()).then(|| name.to_owned()),
        pixel: [n(COLUMN)?, n(ROW)?],
        target: [n(TARGET_Y)?, n(TARGET_X)?],
        check: row[CHECK] == "1" && !off,
        off,
    })
}

/// A point as a row.
fn row_of(p: &GeorefPoint) -> Row {
    let mut r: Row = Default::default();
    r[USE] = if p.off { "0" } else { "1" }.to_owned();
    r[CHECK] = if p.check { "1" } else { "0" }.to_owned();
    r[NAME] = p.name.clone().unwrap_or_default();
    r[COLUMN] = number(p.pixel[0]);
    r[ROW] = number(p.pixel[1]);
    r[TARGET_Y] = number(p.target[0]);
    r[TARGET_X] = number(p.target[1]);
    r
}

/// A number as it was given (the shortest text that reads back).
fn number(v: f64) -> String {
    let t = format!("{v}");
    if t == "-0" { "0".to_owned() } else { t }
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

/// A record's command input, to solve it again (the source pixel a pick on a resampled raster comes from).
fn input_of_record(uid: String, g: &RasterGeoref) -> RastersGeoref {
    RastersGeoref {
        uid,
        method: g.method,
        points: g.points.clone(),
        srid: g.srid,
        pixel: g.output.as_ref().map(|o| o.pixel),
        resampling: g.output.as_ref().map_or(RasterResampling::Bilinear, |o| o.resampling),
        output: None,
        made: None,
        drawing: None,
        expected_revision: None,
    }
}

impl Form {
    /// The raster fitted, when the drawing has it.
    fn fields(&self, model: &Model) -> Option<RasterFields> {
        match model.get(self.raster?) {
            Some(Entity::Raster(r)) => Some(r.raster.clone()),
            _ => None,
        }
    }

    /// The command's input from the rows (the rows lacking a number left out).
    pub fn input(&self, model: &Model, drawing: Option<String>) -> Option<RastersGeoref> {
        let uid = model.uid(self.raster?)?.to_string();
        Some(RastersGeoref {
            uid,
            method: self.method,
            points: self.points(),
            srid: self.srid.trim().parse::<u32>().ok().filter(|s| *s > 0),
            pixel: read_number(&self.pixel),
            resampling: if self.nearest { RasterResampling::Nearest } else { RasterResampling::Bilinear },
            output: None,
            made: None,
            drawing,
            expected_revision: None,
        })
    }

    /// What the drawing offers when the window opens: the selected raster, else the one the window had, else the
    /// first; its record's points when it has one.
    pub fn sync(&mut self, model: &Model, selected: &[Slot]) {
        let all = rasters(model);
        let chosen = selected.iter().find(|s| all.iter().any(|(r, _)| r == *s)).copied();
        if chosen.is_some() {
            self.raster = chosen;
        } else if self.raster.is_none_or(|r| !all.iter().any(|(s, _)| *s == r)) {
            self.raster = all.first().map(|(s, _)| *s);
        }
        self.status = None;
        self.follow(model);
        self.solve(model);
    }

    /// A raster newly chosen, or the rows empty: its record opened in the rows; the rows a record filled emptied when
    /// the next raster has none.
    fn follow(&mut self, model: &Model) {
        let empty = self.points().is_empty();
        if self.opened == self.raster && !empty {
            return;
        }
        let record = self.fields(model).and_then(|f| f.georef);
        match record {
            Some(g) => self.load(&g),
            None if self.opened.is_some() && self.opened != self.raster => {
                self.rows = vec![Default::default(); 4];
                self.srid.clear();
                self.pixel.clear();
            }
            None => {}
        }
        self.opened = self.raster;
    }

    /// The rows, method and options of a record.
    pub fn load(&mut self, g: &RasterGeoref) {
        self.method = g.method;
        self.rows = g.points.iter().map(row_of).collect();
        self.srid = g.srid.map(|s| s.to_string()).unwrap_or_default();
        self.pixel = g.output.as_ref().map(|o| number(o.pixel)).unwrap_or_default();
        self.nearest = g.output.as_ref().is_some_and(|o| o.resampling == RasterResampling::Nearest);
    }

    /// The rows solved again, the residuals and errors into their cells.
    pub fn solve(&mut self, model: &Model) {
        self.pair_rows.clear();
        for (r, row) in self.rows.iter_mut().enumerate() {
            for c in [RES_Y, RES_X, RES] {
                row[c].clear();
            }
            if point_of(row).is_some() {
                self.pair_rows.push(r);
            }
        }
        self.solution = match self.input(model, None) {
            Some(input) if !input.points.is_empty() => Some(rasters_georef::fit(model, &input)),
            _ => None,
        };
        if let Some(Ok(f)) = &self.solution {
            let thin = self.method == GeorefMethod::ThinPlate;
            for (&r, v) in self.pair_rows.iter().zip(&f.residuals) {
                let row = &mut self.rows[r];
                // The thin plate passes through its control points: only the others' are shown.
                if thin && row[CHECK] != "1" && row[USE] != "0" {
                    continue;
                }
                let cell = |x: Option<f64>| x.map_or_else(|| "—".to_owned(), |x| fixed(x, 3));
                row[RES_Y] = cell(v.dx);
                row[RES_X] = cell(v.dy);
                row[RES] = cell(v.d);
            }
        }
    }

    /// The points, as the command takes them.
    pub fn points(&self) -> Vec<GeorefPoint> {
        self.rows.iter().filter_map(point_of).collect()
    }

    /// A pick's point: the source's pixel under it on the raster (through the record of a resampled one), or the
    /// target.
    pub fn picked(&mut self, model: &Model, row: usize, side: Side, p: Vec2) {
        let fields = self.fields(model);
        let uid = self.raster.and_then(|s| model.uid(s)).map(|u| u.to_string());
        let Some(r) = self.rows.get_mut(row) else {
            return;
        };
        match side {
            Side::Source => {
                let Some(f) = fields else {
                    return;
                };
                let pixel = match (&f.georef, uid) {
                    (Some(g), Some(uid)) if g.output.is_some() => rasters_georef::fit(model, &input_of_record(uid, g))
                        .ok()
                        .and_then(|fit| fit.georef.inverse(kentos_geometry_core::vec2::Vec2::new(p.x, p.y)))
                        .map(|q| Vec2::new(q.x, q.y)),
                    _ => kentos_geometry_core::geom::raster::pixel_of(f.affine, p),
                };
                if let Some(q) = pixel {
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

    /// The summary: the solution's state, m0, the check points' errors, the record and what Uygula does.
    fn summary_lines(&self, model: &Model) -> Vec<(Line, String)> {
        let mut out = Vec::new();
        let Some(fields) = self.fields(model) else {
            out.push((Line::Warn, "Çizimde raster yok ya da seçili değil: önce bir raster seçin.".to_owned()));
            return out;
        };
        match &self.solution {
            None => out.push((
                Line::Info,
                format!(
                    "Kontrol noktalarını girin: Sütun ve Satır rasterin üstünde, Hedef çizimde gösterilir. {} en az {} kontrol noktası ister; dönüşüme girmeyen denetim noktaları doğruluğu ölçer.",
                    self.method.title(),
                    self.method.need()
                ),
            )),
            Some(Err(e)) => out.push((Line::Warn, e.message.clone())),
            Some(Ok(f)) => {
                let m0 = match f.m0 {
                    Some(m0) => format!("m0 = ±{} m.", fixed(m0, 3)),
                    None if self.method == GeorefMethod::ThinPlate => {
                        "İnce plaka kontrol noktalarından tam geçer; artık yok.".to_owned()
                    }
                    None => "Fazla kontrol noktası yok: m0 hesaplanmaz.".to_owned(),
                };
                out.push((
                    Line::Ok,
                    format!("{} çözüldü: {} kontrol noktası. {m0}", self.method.title(), f.control),
                ));
                match (f.check_rmse, f.check_max) {
                    (Some(rmse), Some(most)) => out.push((
                        Line::Ok,
                        format!(
                            "Denetim noktaları ({}): karesel ortalama hata ±{} m, en büyük {} m.",
                            f.checks,
                            fixed(rmse, 3),
                            fixed(most, 3)
                        ),
                    )),
                    _ => {
                        for w in &f.warnings {
                            out.push((Line::Warn, w.message.clone()));
                        }
                    }
                }
                if let Some(srid) = f.srid {
                    out.push((Line::Info, format!("Hedefler EPSG:{srid} sisteminden projenin sistemine çevrildi.")));
                }
                if let Some(grid) = &f.grid {
                    out.push((
                        Line::Info,
                        format!(
                            "Çıktı ızgarası: {} × {} piksel, piksel {} m, kuzeye bakan.",
                            grid.width,
                            grid.height,
                            fixed(grid.affine[1], 3)
                        ),
                    ));
                }
                let how = if self.method.places() {
                    "Uygula rasterin dönüşümünü değiştirir ve oturtmayı rasterin kaydına yazar (tek adım)."
                } else if f.source.file.is_some() {
                    "Uygula kaynağı yeniden örnekleyip yanına “-oturtulmus.tif” olarak yazar; kaynak değişmez, raster çıktıyı gösterir, oturtma rasterin kaydında (tek adım)."
                } else {
                    "Uygula kaynağı yeniden örnekleyip projenin kitaplığına gömer; raster çıktıyı gösterir, oturtma rasterin kaydında (tek adım)."
                };
                out.push((Line::Info, how.to_owned()));
            }
        }
        if let Some(g) = &fields.georef {
            let (control, checks) = g.counts();
            let source = g
                .source
                .file
                .as_deref()
                .map(|f| std::path::Path::new(f).file_name().map_or(f.to_owned(), |n| n.to_string_lossy().into_owned()))
                .or_else(|| g.source.asset.clone())
                .or_else(|| g.source.url.clone())
                .unwrap_or_default();
            let digest = g.source.digest.as_deref().map(|d| format!(", özeti {}…", &d[..12.min(d.len())])).unwrap_or_default();
            out.push((
                Line::Info,
                format!(
                    "Kayıtlı oturtma: {}, {control} kontrol ve {checks} denetim noktası; kaynak {source}{digest}.",
                    g.method.title()
                ),
            ));
        }
        out
    }

    pub fn view<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        let all = rasters(model);
        let ids: Vec<u32> = all.iter().map(|(s, _)| s.0).collect();
        let selected = self.raster.and_then(|r| all.iter().position(|(s, _)| *s == r));
        let list = Select::new(
            all.iter().map(|(_, name)| Choice::new(name.clone())).collect::<Vec<_>>(),
            selected,
            move |i| raster_fit_event(Event::Raster(ids.get(i).copied().unwrap_or(0))),
        )
        .searchable(false);
        let raster = words::field(
            "Raster",
            row![
                container(list).width(Fill),
                pick_button(Event::PickRaster, "Rasteri çizimden seç", "Rasterin üstüne tıklayın.")
            ]
            .spacing(6),
            None,
        );
        let srid = super::number_field("Hedeflerin sistemi (EPSG)", &self.srid, "projeninki", |t| {
            raster_fit_event(Event::Srid(t))
        });
        let method = words::field(
            "Dönüşüm",
            Segmented::new(METHODS, Named(self.method), |m| raster_fit_event(Event::Method(m.0))),
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
        let files = row![
            label::strong("Kontrol ve denetim noktaları"),
            iced::widget::space::horizontal(),
            words::ghost("Noktaları al…", Some(raster_fit_event(Event::Import))),
            words::ghost("Noktaları kaydet…", Some(raster_fit_event(Event::Export)))
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        let mut body = column![row![container(raster).width(Fill), srid].spacing(16), method].spacing(16);
        body = body.push(column![files, table].spacing(8));
        if let Some(s) = summary(self.summary_lines(model)) {
            body = body.push(s);
        }
        let linked = self.fields(model).is_some_and(|f| f.file.is_some());
        let options: Element<'a, Message> = if self.method.places() {
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
                super::number_field("Çıktının piksel boyu (m)", &self.pixel, "dönüşümden", |t| {
                    raster_fit_event(Event::Pixel(t))
                }),
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
        let can = !self.warping && matches!(self.solution, Some(Ok(_))) && self.fields(model).is_some();
        let mut dialog = Dialog::new(TITLE)
            .scroll(body)
            .action(footer_button("Kapat", Some(event(Calc::Close)), false))
            .action(footer_button("Uygula", can.then(|| raster_fit_event(Event::Apply)), true))
            .max_height(MAX_HEIGHT)
            .width(1000.0);
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

impl App {
    pub(crate) fn raster_fit_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Raster(slot) => {
                self.calc.raster_fit.raster = Some(Slot(slot));
                if let Some(doc) = &self.document {
                    self.calc.raster_fit.follow(&doc.model);
                }
            }
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
            Event::Srid(t) => self.calc.raster_fit.srid = t,
            Event::Pixel(t) => self.calc.raster_fit.pixel = t,
            Event::Nearest(on) => self.calc.raster_fit.nearest = on,
            Event::World(on) => self.calc.raster_fit.world = on,
            Event::Import => return self.raster_fit_import(),
            Event::Imported(path) => self.raster_fit_imported(path),
            Event::Export => return self.raster_fit_export(),
            Event::Exported(path) => self.raster_fit_exported(path),
            Event::Apply => return self.raster_fit_apply(),
            Event::Warped(result) => {
                self.calc.raster_fit.warping = false;
                self.raster_fit_warped(result);
            }
        }
        if let Some(doc) = &self.document {
            self.calc.raster_fit.solve(&doc.model);
        }
        Task::none()
    }

    /// A pick's answer for Raster oturt: the raster under the point, or a row's end.
    pub(super) fn raster_fit_picked(&mut self, p: Option<Vec2>) -> bool {
        let form = &mut self.calc.raster_fit;
        if form.picking_raster {
            form.picking_raster = false;
            if let (Some(p), Some(doc)) = (p, &self.document) {
                let under = rasters(&doc.model).into_iter().rev().find(|(s, _)| match doc.model.get(*s) {
                    Some(Entity::Raster(r)) => {
                        let x = &r.raster;
                        kentos_geometry_core::geom::raster::pixel_of(x.affine, p).is_some_and(|q| {
                            q.x >= 0.0 && q.y >= 0.0 && q.x < f64::from(x.width) && q.y < f64::from(x.height)
                        })
                    }
                    _ => false,
                });
                if let Some((slot, _)) = under {
                    self.calc.raster_fit.raster = Some(slot);
                    self.calc.raster_fit.follow(&doc.model);
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

    /// Noktaları al…: a `.points` file chosen.
    fn raster_fit_import(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(raster_fit_event(Event::Imported(Some(path.clone()))));
        }
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("Kontrol noktalarını al")
                    .add_filter("Kontrol noktaları (.points, .csv, .txt)", &["points", "csv", "txt"])
                    .pick_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            |path| raster_fit_event(Event::Imported(path)),
        )
    }

    fn raster_fit_imported(&mut self, path: Option<std::path::PathBuf>) {
        let Some(path) = path else {
            return;
        };
        let read = std::fs::read_to_string(&path)
            .map_err(|e| format!("“{}” okunamadı: {e}.", path.display()))
            .and_then(|t| gcp::read(&t));
        match read {
            Ok(r) => {
                let form = &mut self.calc.raster_fit;
                form.rows = r.points.iter().map(row_of).collect();
                form.status = None;
                let n = r.points.len();
                self.say(
                    Level::Info,
                    format!(
                        "{TITLE}: “{}” dosyasından {n} nokta alındı.{}",
                        path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                        if r.crs.is_some() {
                            " Dosyanın koordinat sistemi satırı okunmadı; hedeflerin sistemini pencerede verin."
                        } else {
                            ""
                        }
                    ),
                );
            }
            Err(why) => self.calc.raster_fit.status = Some(why),
        }
    }

    /// Noktaları kaydet…: the points with their residuals as a `.points` file.
    fn raster_fit_export(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(raster_fit_event(Event::Exported(Some(path.clone()))));
        }
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("Kontrol noktalarını kaydet")
                    .add_filter("QGIS kontrol noktaları (.points)", &["points"])
                    .set_file_name("kontrol-noktalari.points")
                    .save_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            |path| raster_fit_event(Event::Exported(path)),
        )
    }

    fn raster_fit_exported(&mut self, path: Option<std::path::PathBuf>) {
        let Some(path) = path else {
            return;
        };
        let form = &self.calc.raster_fit;
        let points = form.points();
        let residuals: Vec<Option<[f64; 2]>> = match &form.solution {
            Some(Ok(f)) => f.residuals.iter().map(|v| Some([v.dx?, v.dy?])).collect(),
            _ => vec![None; points.len()],
        };
        match std::fs::write(&path, gcp::write(&points, &residuals)) {
            Ok(()) => self.say(
                Level::Info,
                format!("{TITLE}: {} nokta “{}” dosyasına yazıldı.", points.len(), path.display()),
            ),
            Err(e) => self.calc.raster_fit.status = Some(format!("“{}” yazılamadı: {e}.", path.display())),
        }
    }

    /// Uygula: the command at once (Helmert, affine), or the resampling started by its plan.
    fn raster_fit_apply(&mut self) -> Task<Message> {
        let drawing = self.drawing_path();
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(input) = self.calc.raster_fit.input(&doc.model, drawing) else {
            return Task::none();
        };
        let Some(fields) = self.calc.raster_fit.fields(&doc.model) else {
            return Task::none();
        };
        if input.method.places() {
            let world = self.calc.raster_fit.world;
            if let Some(after) = self.raster_fit_write(input, "dönüşümü değişti")
                && world
            {
                self.raster_fit_world(&fields, &after.affine);
            }
            return Task::none();
        }
        let p = match rasters_georef::prepare(&doc.model, &input) {
            Ok(p) => p,
            Err(e) => {
                self.calc.raster_fit.status = Some(e.message);
                return Task::none();
            }
        };
        let Some(grid) = p.grid else {
            return Task::none();
        };
        // The source the pixels are of: the raster's own, or its record's (a resampled raster).
        let source = RasterFields {
            asset: p.source.asset.clone(),
            file: p.source.file.clone(),
            url: p.source.url.clone(),
            georef: None,
            ..fields.clone()
        };
        let folder = doc.path.as_deref().and_then(std::path::Path::parent).map(std::path::Path::to_path_buf);
        let key = crate::rasters::key_of(&source);
        let Some(origin) = crate::rasters::origin_of(&key, &doc.model, folder.as_deref()) else {
            self.calc.raster_fit.status = Some("Rasterin kaynağı bulunamadı.".to_owned());
            return Task::none();
        };
        let (output, name) = match &p.output {
            Some((_, at)) => (
                Output::File(at.clone()),
                at.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            ),
            None => (
                Output::Memory,
                self.layer_name(&match doc.model.get(p.slot) {
                    Some(Entity::Raster(r)) => r.base.layer_id.clone(),
                    _ => String::new(),
                }),
            ),
        };
        let inverse = p.inverse();
        let (nearest, srid, geographic) = (p.nearest, p.srid, p.geographic);
        self.calc.raster_fit.pending = Some(RastersGeoref { output: None, ..input });
        self.calc.raster_fit.warping = true;
        self.calc.raster_fit.status = None;
        self.calc.open = None;
        self.dialog = None;
        self.say(
            Level::Info,
            format!("{TITLE}: “{name}” yeniden örnekleniyor; sağ alttaki panel ilerlemeyi gösterir."),
        );
        crate::rasters::off_thread(move || {
            let result = jobs::warp(name, origin, output, move |reader| {
                let i = &reader.info;
                kentos_formats::raster::warp::Job::new(
                    grid,
                    Box::new(inverse),
                    i.width,
                    i.height,
                    i.bands,
                    i.sample,
                    reader.nodata,
                    reader.palette.clone(),
                    nearest,
                    (srid > 0).then_some(srid),
                    geographic,
                )
                .map_err(|e| e.0)
            });
            raster_fit_event(Event::Warped(result))
        })
    }

    /// The resampled output handed to the command (`made`): the raster shows it, the record names it, one step.
    fn raster_fit_warped(&mut self, result: Result<Warped, String>) {
        let w = match result {
            Ok(w) => w,
            Err(why) if why == "durduruldu" => {
                self.say(Level::Info, format!("{TITLE}: durduruldu; raster değişmedi."));
                return;
            }
            Err(why) => {
                self.warn(format!("{TITLE}: {why}"));
                return;
            }
        };
        let Some(mut input) = self.calc.raster_fit.pending.take() else {
            return;
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let made = match (&w.path, &w.bytes) {
            (Some(path), _) => GeorefMade { file: Some(path.to_string_lossy().into_owned()), asset: None },
            (None, Some(bytes)) => {
                let layer = match doc.model.get(self.calc.raster_fit.raster.unwrap_or(Slot(0))) {
                    Some(Entity::Raster(r)) => doc
                        .model
                        .layers()
                        .get(&r.base.layer_id)
                        .map_or_else(|| "raster".to_owned(), |l| l.name.clone()),
                    _ => "raster".to_owned(),
                };
                match crate::rasters::library_item(&format!("{layer}-oturtulmus.tif"), bytes, w.width, w.height) {
                    Ok((id, item)) => {
                        crate::rasters::keep_in_library(&mut doc.model, &id, item);
                        GeorefMade { file: None, asset: Some(id) }
                    }
                    Err(why) => {
                        self.warn(format!("{TITLE}: {why}"));
                        return;
                    }
                }
            }
            (None, None) => return,
        };
        input.made = Some(made);
        self.raster_fit_write(input, "yeniden örneklendi");
    }

    /// The command written: one step “Raster oturt”; the raster as written, or none (the window says why).
    fn raster_fit_write(&mut self, input: RastersGeoref, done: &str) -> Option<RasterFields> {
        let doc = self.document.as_mut()?;
        let slot = self.calc.raster_fit.raster?;
        match rasters_georef::execute(&mut kentos_native_application::ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, warnings } => {
                crate::rasters::tiles::service().forget_tiles();
                self.calc.open = None;
                self.dialog = None;
                self.calc.raster_fit.opened = Some(slot);
                self.say(Level::Success, format!("{TITLE}: rasterin {done}; oturtma kaydına yazıldı. Ctrl+Z geri alır."));
                for w in warnings {
                    self.warn(w.message);
                }
                self.zoom_to(&[slot]);
                Some(output.raster)
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.calc.raster_fit.status = Some(error.message.clone());
                self.warn(format!("{TITLE}: {}", error.message));
                None
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => None,
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
        let ext = path.extension().map_or_else(|| "tif".to_owned(), |e| e.to_string_lossy().into_owned());
        let target = path.with_extension(kentos_formats::raster::world::extension_for(&ext));
        match std::fs::write(&target, kentos_formats::raster::world::write(affine)) {
            Ok(()) => self.say(
                Level::Info,
                format!(
                    "{TITLE}: dünya dosyası “{}” yazıldı.",
                    target.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                ),
            ),
            Err(e) => self.warn(format!("{TITLE}: dünya dosyası yazılamadı: {e}.")),
        }
    }
}
