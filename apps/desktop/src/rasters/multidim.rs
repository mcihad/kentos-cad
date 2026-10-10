//! NetCDF and meshes added (docs/adr/0243 §11; the web's `ui/raster/MultidimDialog.ts`):
//! Raster ekle hands a NetCDF file over here, Mesh ekle opens it for a 2DM
//! (and its ASCII DATs) or a UGRID NetCDF. The file is read off the
//! interface's thread: a NetCDF's header, coordinates and meshes; a 2DM and
//! its DATs made one UGRID file in memory (written beside the 2DM, or
//! embedded, when added). The window lists the file's grids (Raster ekle) or
//! meshes' datasets (Mesh ekle), a value of each slice dimension, Zaman
//! sürgüsünü izle, a mesh's cell and lines, what the rule of the systems says
//! and how the file is kept; Ekle writes a new layer and the raster on it as
//! one undo step.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, column, container, row, text_input};
use iced::{Element, Fill, Task};
use kentos_contracts::{
    CommandResult, CreateOperation, DatasetDim, EntitiesCreate, EntityGeometry, NewObject,
    RasterDataset, RasterFields, RasterSample, RasterStyle,
};
use kentos_domain::NewLayer;
use kentos_formats::multidim::cube::{Cube, CubeInfo, DimInfo};
use kentos_formats::multidim::mesh::grid_of_box;
use kentos_formats::multidim::write::SmsReport;
use kentos_formats::multidim::{sms, write};
use kentos_formats::raster::place::{self, Rule};
use kentos_formats::raster::source::RasterInfo;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, create};
use kentos_ui::label;
use kentos_ui::widget::pairs::Pairs;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};

use super::Event as Rasters;
use super::add::Keep;
use crate::app::{App, Dialog as Asking, Message, Picker};
use crate::exchange::words::{self, Kind as Line};

pub const MESH_TITLE: &str = "Mesh ekle";
pub const GRID_TITLE: &str = "Raster ekle";

static READS: AtomicU64 = AtomicU64::new(0);

/// Which of the file's variables the window lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// CF grids (Raster ekle).
    Grid,
    /// Meshes' datasets (Mesh ekle).
    Mesh,
}

/// What reading the files gave.
#[derive(Debug, Clone)]
pub struct Read {
    pub info: CubeInfo,
    pub bytes: u64,
    /// A 2DM and its DATs made one UGRID file: its bytes and what the conversion said.
    pub converted: Option<(Arc<Vec<u8>>, SmsReport)>,
}

/// A variable the window offers.
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Grid(usize),
    /// A mesh's (its place) dataset (its place).
    Mesh(usize, usize),
}

#[derive(Debug, Clone)]
pub struct State {
    pub mode: Mode,
    /// The NetCDF or the 2DM.
    pub path: PathBuf,
    /// A 2DM's DATs.
    pub dats: Vec<PathBuf>,
    pub name: String,
    reading: u64,
    pub read: Option<Result<Read, String>>,
    pub choice: usize,
    pub slice: Vec<u32>,
    pub follow: bool,
    /// A mesh's cell, as typed.
    pub cell: String,
    pub edges: bool,
    /// A 2DM's start time, as typed (applied with Enter).
    pub start: String,
    /// Where a 2DM's UGRID file is written.
    pub out: String,
    pub confirmed: bool,
    pub keep: Keep,
    pub layer: String,
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The files chosen (Mesh ekle: a 2DM and DATs, or a NetCDF), or none.
    Picked(Mode, Option<Vec<PathBuf>>),
    Read(u64, Box<Result<Read, String>>),
    Choice(usize),
    Slice(usize, u32),
    Follow,
    Cell(String),
    Edges,
    Start(String),
    ApplyStart,
    Out(String),
    Confirm,
    Keep(Keep),
    Layer(String),
    Another,
    Run,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Rasters(Rasters::Multidim(e))
}

/// A NetCDF file's contents.
fn read_netcdf(path: &Path) -> Result<Read, String> {
    let (info, bytes) = super::tiles::cube_info(&super::tiles::Origin::File(path.to_path_buf()))?;
    Ok(Read {
        info,
        bytes,
        converted: None,
    })
}

/// A 2DM and its DATs made one UGRID file (times from `start`), and its contents.
fn read_sms(
    mesh: &Path,
    dats: &[PathBuf],
    start: Option<f64>,
    epsg: Option<u32>,
) -> Result<Read, String> {
    let text = std::fs::read(mesh).map_err(|e| format!("“{}” okunamadı: {e}.", mesh.display()))?;
    let m = sms::read_2dm(&text).map_err(|e| format!("“{}”: {}", file_name(mesh), e.0))?;
    let mut list = Vec::with_capacity(dats.len());
    for d in dats {
        let bytes = std::fs::read(d).map_err(|e| format!("“{}” okunamadı: {e}.", d.display()))?;
        let name = file_name(d);
        let sets = sms::read_dat(&bytes).map_err(|e| format!("“{name}”: {}", e.0))?;
        list.push((name, sets));
    }
    let mut out = Vec::new();
    let mut sink = |b: &[u8]| -> Result<(), String> {
        out.extend_from_slice(b);
        Ok(())
    };
    let report = write::write_sms(&m, &list, start, epsg, false, &mut sink).map_err(|e| e.0)?;
    let mut cube = Cube::from_bytes(&out).map_err(|e| e.0)?;
    let info = cube.info().map_err(|e| e.0)?;
    Ok(Read {
        info,
        bytes: out.len() as u64,
        converted: Some((Arc::new(out), report)),
    })
}

fn file_name(p: &Path) -> String {
    p.file_name().map_or_else(
        || p.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn is_2dm(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("2dm"))
}

impl State {
    fn items(&self) -> Vec<Item> {
        let Some(Ok(r)) = &self.read else {
            return Vec::new();
        };
        match self.mode {
            Mode::Grid => (0..r.info.grids.len()).map(Item::Grid).collect(),
            Mode::Mesh => r
                .info
                .meshes
                .iter()
                .enumerate()
                .flat_map(|(m, mesh)| (0..mesh.datasets.len()).map(move |d| Item::Mesh(m, d)))
                .collect(),
        }
    }

    fn item(&self) -> Option<Item> {
        self.items().get(self.choice).cloned()
    }

    /// The chosen variable's slice dimensions.
    fn dims(&self) -> &[DimInfo] {
        let Some(Ok(r)) = &self.read else {
            return &[];
        };
        match self.item() {
            Some(Item::Grid(g)) => &r.info.grids[g].dims,
            Some(Item::Mesh(m, d)) => &r.info.meshes[m].datasets[d].dims,
            None => &[],
        }
    }

    /// The slice back to each dimension's first value; the slider followed when there is time.
    fn reset_slice(&mut self) {
        let dims = self.dims();
        let follow = dims.iter().any(|d| d.time);
        self.slice = vec![0; dims.len()];
        self.follow = follow;
    }
}

/// An item's name in the list.
fn item_label(info: &CubeInfo, item: &Item) -> String {
    let named = |variable: &str, long: Option<&String>, units: Option<&String>| {
        let mut s = match long {
            Some(l) if l != variable => format!("{l} ({variable})"),
            _ => variable.to_owned(),
        };
        if let Some(u) = units.filter(|u| !u.is_empty()) {
            s.push_str(&format!(", {u}"));
        }
        s
    };
    match item {
        Item::Grid(g) => {
            let x = &info.grids[*g];
            named(&x.variable, x.long_name.as_ref(), x.units.as_ref())
        }
        Item::Mesh(m, d) => {
            let x = &info.meshes[*m].datasets[*d];
            let mut s = named(&x.variable, x.long_name.as_ref(), x.units.as_ref());
            if let Some(v) = &x.vector {
                s = format!("{s} / {v} (vektör)");
            }
            if x.location == "face" {
                s.push_str(" · yüzlerde");
            }
            s
        }
    }
}

impl App {
    /// Mesh ekle (`mesh.add`): the window, and the file dialog over it.
    pub(crate) fn mesh_add_command(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        if let Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Picked(Mode::Mesh, Some(vec![path.clone()]))));
        }
        self.multidim_open(Mode::Mesh, PathBuf::new(), Vec::new());
        self.multidim_pick(Mode::Mesh)
    }

    /// Raster ekle's NetCDF: this window instead.
    pub(crate) fn multidim_from_raster_add(&mut self, path: PathBuf) -> Task<Message> {
        self.rasters.add = None;
        self.multidim_event(Event::Picked(Mode::Grid, Some(vec![path])))
    }

    fn multidim_open(&mut self, mode: Mode, path: PathBuf, dats: Vec<PathBuf>) -> u64 {
        let id = READS.fetch_add(1, Ordering::Relaxed) + 1;
        let name = if path.as_os_str().is_empty() {
            String::new()
        } else {
            file_name(&path)
        };
        let layer = if name.is_empty() {
            match mode {
                Mode::Mesh => "Mesh".to_owned(),
                Mode::Grid => "Raster".to_owned(),
            }
        } else {
            words::base_name(&name)
        };
        let out = if is_2dm(&path) {
            path.with_extension("nc").to_string_lossy().into_owned()
        } else {
            String::new()
        };
        self.rasters.multidim = Some(State {
            mode,
            path,
            dats,
            name,
            reading: id,
            read: None,
            choice: 0,
            slice: Vec::new(),
            follow: false,
            cell: String::new(),
            edges: false,
            start: String::new(),
            out,
            confirmed: false,
            keep: Keep::Linked,
            layer,
            status: None,
        });
        self.dialog = Some(Asking::Multidim);
        id
    }

    fn multidim_pick(&mut self, mode: Mode) -> Task<Message> {
        Task::perform(
            async move {
                let dialog = rfd::AsyncFileDialog::new().set_title(match mode {
                    Mode::Mesh => MESH_TITLE,
                    Mode::Grid => GRID_TITLE,
                });
                let dialog = match mode {
                    Mode::Mesh => {
                        dialog.add_filter("Mesh (2DM ve DAT, UGRID NetCDF)", &["2dm", "dat", "nc"])
                    }
                    Mode::Grid => dialog.add_filter("NetCDF", &["nc"]),
                };
                dialog
                    .pick_files()
                    .await
                    .map(|files| files.iter().map(|f| f.path().to_path_buf()).collect())
            },
            move |paths| msg(Event::Picked(mode, paths)),
        )
    }

    /// The project's SRID (a 2DM says no system: the project's is offered).
    fn multidim_srid(&self) -> u32 {
        self.document.as_ref().map_or(0, |d| d.settings().srid)
    }

    fn multidim_read(&mut self, id: u64, s: &State) -> Task<Message> {
        let (path, dats) = (s.path.clone(), s.dats.clone());
        let start = kentos_geometry_core::time::read(&s.start).moment();
        if is_2dm(&path) {
            return super::off_thread(move || {
                msg(Event::Read(
                    id,
                    Box::new(read_sms(&path, &dats, start, None)),
                ))
            });
        }
        super::off_thread(move || msg(Event::Read(id, Box::new(read_netcdf(&path)))))
    }

    pub(crate) fn multidim_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Picked(_, None) => return Task::none(),
            Event::Picked(mode, Some(paths)) => {
                // A 2DM with its DATs; else the one NetCDF.
                let main = paths
                    .iter()
                    .find(|p| is_2dm(p))
                    .or_else(|| paths.first())
                    .cloned();
                let Some(main) = main else {
                    return Task::none();
                };
                let dats: Vec<PathBuf> = if is_2dm(&main) {
                    paths
                        .iter()
                        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dat")))
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                };
                let id = self.multidim_open(mode, main, dats);
                let Some(s) = self.rasters.multidim.clone() else {
                    return Task::none();
                };
                return self.multidim_read(id, &s);
            }
            Event::Read(id, read) => {
                let read = *read;
                if let Some(s) = &mut self.rasters.multidim
                    && s.reading == id
                {
                    if let Ok(r) = &read {
                        // A file of the other kind: the window lists what it has.
                        if s.mode == Mode::Grid
                            && r.info.grids.is_empty()
                            && !r.info.meshes.is_empty()
                        {
                            s.mode = Mode::Mesh;
                        } else if s.mode == Mode::Mesh
                            && r.info.meshes.is_empty()
                            && !r.info.grids.is_empty()
                        {
                            s.mode = Mode::Grid;
                        }
                        if let Some(m) = r.info.meshes.first() {
                            s.cell = crate::crs::js_number(m.cell);
                        }
                        if r.bytes > super::MOST_EMBEDDED as u64 {
                            s.keep = Keep::Linked;
                        }
                    }
                    let keep = s.choice;
                    s.read = Some(read);
                    s.choice = keep.min(s.items().len().saturating_sub(1));
                    s.reset_slice();
                }
            }
            Event::Choice(k) => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.choice = k;
                    s.reset_slice();
                }
            }
            Event::Slice(k, i) => {
                if let Some(s) = &mut self.rasters.multidim
                    && let Some(v) = s.slice.get_mut(k)
                {
                    *v = i;
                }
            }
            Event::Follow => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.follow = !s.follow;
                }
            }
            Event::Cell(t) => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.cell = t;
                }
            }
            Event::Edges => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.edges = !s.edges;
                }
            }
            Event::Start(t) => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.start = t;
                }
            }
            Event::ApplyStart => {
                let Some(s) = self.rasters.multidim.clone() else {
                    return Task::none();
                };
                if !s.start.trim().is_empty()
                    && kentos_geometry_core::time::read(&s.start)
                        .moment()
                        .is_none()
                {
                    self.multidim_status(
                        "Başlangıç zamanı okunamadı: 2024-05-01 06:00 gibi yazın.".into(),
                    );
                    return Task::none();
                }
                let id = READS.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(m) = &mut self.rasters.multidim {
                    m.reading = id;
                    m.read = None;
                    m.status = None;
                }
                return self.multidim_read(id, &s);
            }
            Event::Out(t) => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.out = t;
                }
            }
            Event::Confirm => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.confirmed = !s.confirmed;
                }
            }
            Event::Keep(k) => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.keep = k;
                }
            }
            Event::Layer(t) => {
                if let Some(s) = &mut self.rasters.multidim {
                    s.layer = t;
                }
            }
            Event::Another => {
                let mode = self
                    .rasters
                    .multidim
                    .as_ref()
                    .map_or(Mode::Mesh, |s| s.mode);
                return self.multidim_pick(mode);
            }
            Event::Run => self.multidim_run(),
            Event::Close => {
                self.rasters.multidim = None;
                if self.dialog == Some(Asking::Multidim) {
                    self.dialog = None;
                }
            }
        }
        Task::none()
    }

    /// The raster's grid, system and samples as a raster's header would say them.
    fn multidim_info(&self, s: &State) -> Option<RasterInfo> {
        let Some(Ok(r)) = &s.read else { return None };
        let (width, height, affine, epsg, geographic, sample) = match s.item()? {
            Item::Grid(g) => {
                let x = &r.info.grids[g];
                (x.width, x.height, x.affine, x.epsg, x.geographic, x.sample)
            }
            Item::Mesh(m, d) => {
                let mesh = &r.info.meshes[m];
                let cell = s.cell.trim().replace(',', ".").parse::<f64>().ok()?;
                let (affine, w, h) = grid_of_box(mesh.bbox, cell)?;
                (
                    w,
                    h,
                    Some(affine),
                    mesh.epsg,
                    mesh.geographic,
                    mesh.datasets[d].sample,
                )
            }
        };
        Some(RasterInfo {
            width,
            height,
            bands: 1,
            sample,
            color: "gray".into(),
            alpha: false,
            compression: String::new(),
            tiled: false,
            big: false,
            overviews: 0,
            levels: 0,
            affine,
            placed_by: if affine.is_some() { "netcdf" } else { "none" }.into(),
            epsg,
            geographic,
            nodata: None,
            needs_pyramid: false,
        })
    }

    fn multidim_placement(&self, s: &State) -> Option<([f64; 6], u32)> {
        let info = self.multidim_info(s)?;
        let b = self.viewport.camera.visible_bounds();
        place::placement(
            &info,
            self.multidim_srid(),
            s.confirmed,
            [b.min_x, b.min_y, b.max_x, b.max_y],
        )
    }

    fn multidim_can(&self, s: &State) -> bool {
        self.multidim_placement(s).is_some()
            && !s.layer.trim().is_empty()
            && (s.dats.is_empty() && !is_2dm(&s.path)
                || s.keep == Keep::Embedded
                || !s.out.trim().is_empty())
    }

    /// Ekle: the new layer and the raster on it, one undo step (a 2DM's file written, or embedded, first).
    fn multidim_run(&mut self) {
        let Some(s) = self.rasters.multidim.clone() else {
            return;
        };
        let (Some((affine, srid)), Some(info), Some(Ok(read)), Some(item)) = (
            self.multidim_placement(&s),
            self.multidim_info(&s),
            s.read.clone(),
            s.item(),
        ) else {
            return;
        };
        let title = match s.mode {
            Mode::Mesh => MESH_TITLE,
            Mode::Grid => GRID_TITLE,
        };
        // The file the raster shows: the NetCDF chosen, or a 2DM's written beside it.
        let mut library = None;
        let (mut asset, mut file) = (None, None);
        let name = if read.converted.is_some() {
            file_name(Path::new(s.out.trim()))
        } else {
            s.name.clone()
        };
        let bytes_of = |s: &State| -> Result<Arc<Vec<u8>>, String> {
            match &read.converted {
                Some((b, _)) => Ok(b.clone()),
                None => std::fs::read(&s.path)
                    .map(Arc::new)
                    .map_err(|e| format!("“{}” okunamadı: {e}.", s.name)),
            }
        };
        if s.keep == Keep::Embedded {
            let bytes = match bytes_of(&s) {
                Ok(b) => b,
                Err(why) => return self.multidim_status(why),
            };
            match super::library_item(&name, &bytes, info.width, info.height) {
                Ok((id, item)) => {
                    asset = Some(id.clone());
                    library = Some((id, item));
                }
                Err(why) => return self.multidim_status(why),
            }
        } else if let Some((bytes, _)) = &read.converted {
            let out = PathBuf::from(s.out.trim());
            // Never over another file: the same bytes are kept, others refused.
            if out.exists() && std::fs::read(&out).ok().as_deref() != Some(bytes.as_slice()) {
                return self.multidim_status(format!(
                    "“{}” zaten var; başka bir ad yazın ya da o dosyayı taşıyın.",
                    out.display()
                ));
            }
            if let Err(e) = std::fs::write(&out, bytes.as_slice()) {
                return self.multidim_status(format!(
                    "“{}” yazılamadı: {e}. Klasörün yazılabilir olduğunu denetleyin.",
                    out.display()
                ));
            }
            file = Some(out.to_string_lossy().into_owned());
        } else {
            file = Some(s.path.to_string_lossy().into_owned());
        }
        let (variable, vector, mesh, dims) = match &item {
            Item::Grid(g) => {
                let x = &read.info.grids[*g];
                (x.variable.clone(), None, None, &x.dims)
            }
            Item::Mesh(m, d) => {
                let mi = &read.info.meshes[*m];
                let x = &mi.datasets[*d];
                (
                    x.variable.clone(),
                    x.vector.clone(),
                    Some(mi.name.clone()),
                    &x.dims,
                )
            }
        };
        let dims: Vec<DatasetDim> = dims
            .iter()
            .enumerate()
            .map(|(k, d)| DatasetDim {
                name: d.name.clone(),
                index: s.slice.get(k).copied().unwrap_or(0),
                values: d.values.clone(),
                time: d.time,
                units: d.units.clone(),
            })
            .collect();
        let follow = s.follow && dims.iter().any(|d| d.time);
        let style = default_look(info.sample, mesh.is_some(), s.edges && mesh.is_some());
        let fields = RasterFields {
            affine,
            width: info.width,
            height: info.height,
            bands: 1,
            sample: info.sample,
            asset,
            file,
            url: None,
            srid,
            style,
            opacity: None,
            dataset: Some(RasterDataset {
                variable,
                vector,
                mesh,
                dims,
                follow_time: follow,
            }),
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let model = &mut doc.model;
        if let Some((id, item)) = library {
            super::keep_in_library(model, &id, item);
        }
        let group = model.begin_group(title);
        let layer = match model.add_layer(NewLayer::layer(s.layer.trim()), None, true) {
            Ok(id) => id,
            Err(why) => {
                model.cancel_group(group);
                return self.multidim_status(why.0);
            }
        };
        let input = EntitiesCreate {
            layer_id: layer,
            objects: vec![NewObject {
                geometry: EntityGeometry::Raster(fields),
                color: None,
                line_weight: None,
                attrs: None,
                label: None,
                label_of: None,
                label_scale: None,
                symbol: None,
            }],
            operation: Some(CreateOperation::Raster),
            expected_revision: None,
        };
        match create::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed { output, warnings } => {
                model.end_group(group);
                let slots: Vec<kentos_domain::Slot> =
                    output.ids.iter().map(|&i| kentos_domain::Slot(i)).collect();
                self.rasters.multidim = None;
                self.dialog = None;
                self.zoom_to(&slots);
                let how = if s.keep == Keep::Embedded {
                    "gömülü"
                } else {
                    "bağlı"
                };
                self.say(
                    Level::Success,
                    format!(
                        "“{name}” {how} olarak “{}” katmanına eklendi. Ctrl+Z geri alır.",
                        s.layer.trim()
                    ),
                );
                for w in warnings {
                    self.warn(w.message);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                model.cancel_group(group);
                self.multidim_status(error.message);
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {
                model.cancel_group(group);
                self.multidim_status("Raster yazılamadı.".to_owned());
            }
        }
    }

    fn multidim_status(&mut self, why: String) {
        if let Some(s) = &mut self.rasters.multidim {
            s.status = Some(why);
        }
    }

    pub(crate) fn multidim_view(&self) -> Element<'_, Message> {
        let Some(s) = &self.rasters.multidim else {
            return iced::widget::text("").into();
        };
        let mut body = Column::new().spacing(12);
        if s.path.as_os_str().is_empty() {
            body = body.push(words::summary(vec![words::text_line(
                Line::Info,
                match s.mode {
                    Mode::Mesh => "Dosya seçilmedi: Dosya seç… ile bir 2DM'i (ve DAT dosyalarını) ya da UGRID ağlı bir NetCDF'i seçin.",
                    Mode::Grid => "Dosya seçilmedi: Dosya seç… ile bir NetCDF seçin.",
                },
            )]));
            return self.multidim_frame(s, body);
        }
        let mut meta = match &s.read {
            None => "okunuyor…".to_owned(),
            Some(Err(_)) => "okunamadı".to_owned(),
            Some(Ok(r)) => match &r.converted {
                Some(_) => format!("2DM, {} DAT", s.dats.len()),
                None => format!("NetCDF {}", r.info.version),
            },
        };
        if let Some(Ok(r)) = &s.read
            && let Some(m) = r.info.meshes.first()
        {
            meta = format!(
                "{meta} · {} düğüm, {} yüz",
                crate::crs::grouped(m.nodes as f64),
                crate::crs::grouped(m.faces as f64)
            );
        }
        body = body.push(words::file_line(&s.name, meta));
        if is_2dm(&s.path) {
            body = body.push(self.multidim_sms(s));
        }
        match &s.read {
            None => {
                body = body.push(words::summary(vec![words::text_line(
                    Line::Info,
                    "Dosya okunuyor…",
                )]))
            }
            Some(Err(why)) => {
                body = body.push(words::summary(vec![words::text_line(
                    Line::Error,
                    why.clone(),
                )]))
            }
            Some(Ok(r)) if s.items().is_empty() => {
                let mut lines = vec![words::text_line(
                    Line::Error,
                    match s.mode {
                        Mode::Mesh => "Dosyada okunabilen ağ yok.",
                        Mode::Grid => "Dosyada okunabilen düzenli ızgara değişkeni yok.",
                    },
                )];
                lines.extend(
                    r.info
                        .notes
                        .iter()
                        .take(6)
                        .map(|n| words::text_line(Line::Info, n.clone())),
                );
                body = body.push(words::summary(lines));
            }
            Some(Ok(r)) => {
                body = body.push(self.multidim_choice(s, r));
                if let Some(info) = self.multidim_info(s) {
                    body = body.push(self.multidim_facts(s, &info));
                    body = body.push(self.multidim_rule(s, &info));
                } else if s.mode == Mode::Mesh {
                    body = body.push(words::text_line(
                        Line::Error,
                        "Hücre boyu sıfırdan büyük bir sayı olmalı ve ızgaranın kenarı 65 536 hücreyi aşmamalı.",
                    ));
                }
                body = body.push(self.multidim_options(s, r));
            }
        }
        if let Some(why) = &s.status {
            body = body.push(words::text_line(Line::Error, why.clone()));
        }
        self.multidim_frame(s, body)
    }

    fn multidim_frame<'a>(
        &'a self,
        s: &'a State,
        body: Column<'a, Message>,
    ) -> Element<'a, Message> {
        let title = match s.mode {
            Mode::Mesh => MESH_TITLE,
            Mode::Grid => GRID_TITLE,
        };
        let pick = if s.path.as_os_str().is_empty() {
            "Dosya seç…"
        } else {
            "Başka dosya…"
        };
        overlay::blocking(
            Dialog::new(title)
                .scroll(body)
                .action(words::ghost(pick, Some(msg(Event::Another))))
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Ekle",
                    self.multidim_can(s).then(|| msg(Event::Run)),
                ))
                .width(640.0)
                .max_height(780.0),
        )
    }

    /// A 2DM's conversion: its start time, where its file goes, what it gave.
    fn multidim_sms<'a>(&self, s: &'a State) -> Element<'a, Message> {
        let input = |placeholder: &'a str,
                     value: &'a str,
                     on: fn(String) -> Event,
                     submit: Option<Event>| {
            let mut t = text_input(placeholder, value)
                .on_input(move |v| msg(on(v)))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(kentos_ui::style::field::input);
            if let Some(e) = submit {
                t = t.on_submit(msg(e));
            }
            kentos_ui::widget::focus_ring(t)
        };
        let mut col = Column::new().spacing(10).push(
            row![
                container(words::field(
                    "Başlangıç zamanı",
                    input("2024-05-01 06:00", &s.start, Event::Start, Some(Event::ApplyStart)),
                    Some("DAT'ın saatleri bu zamandan sayılır; boşsa RT_JULIAN, o da yoksa başlangıçtan saat. Enter uygular.".to_owned()),
                ))
                .width(iced::Length::FillPortion(2)),
                container(words::field(
                    "Yazılacak NetCDF",
                    input("…/model.nc", &s.out, Event::Out, None),
                    Some("2DM ve DAT'lar bu UGRID dosyasına yazılır (Göm seçiliyse yazılmaz).".to_owned()),
                ))
                .width(iced::Length::FillPortion(3)),
            ]
            .spacing(16),
        );
        if let Some(Ok(Read {
            converted: Some((_, report)),
            ..
        })) = &s.read
        {
            let mut lines = vec![words::text_line(
                Line::Ok,
                format!(
                    "{} düğüm, {} yüz; veri setleri: {}.",
                    crate::crs::grouped(report.nodes as f64),
                    crate::crs::grouped(report.faces as f64),
                    report.datasets.join(", ")
                ),
            )];
            lines.extend(
                report
                    .notes
                    .iter()
                    .map(|n| words::text_line(Line::Info, n.clone())),
            );
            col = col.push(words::summary(lines));
        }
        col.into()
    }

    /// The variable and its slice; a mesh's cell and lines.
    fn multidim_choice<'a>(&self, s: &'a State, r: &'a Read) -> Element<'a, Message> {
        let items = s.items();
        let list = Select::new(
            items
                .iter()
                .map(|it| Choice::new(item_label(&r.info, it)))
                .collect::<Vec<_>>(),
            Some(s.choice),
            |i| msg(Event::Choice(i)),
        );
        let mut col = Column::new().spacing(10).push(words::field(
            match s.mode {
                Mode::Mesh => "Veri seti",
                Mode::Grid => "Değişken",
            },
            list,
            None,
        ));
        let dims = s.dims();
        if !dims.is_empty() {
            let mut line = row![].spacing(12);
            for (k, d) in dims.iter().enumerate() {
                let choice = Select::new(
                    d.labels.iter().map(Choice::new).collect::<Vec<_>>(),
                    s.slice.get(k).map(|&i| i as usize),
                    move |i| msg(Event::Slice(k, i as u32)),
                );
                let name = if d.time { "Zaman" } else { d.name.as_str() };
                line = line.push(container(words::field(name, choice, None)).width(Fill));
            }
            col = col.push(line);
        }
        if dims.iter().any(|d| d.time) {
            col = col.push(words::check(
                s.follow,
                "Zaman sürgüsünü izle",
                Some(msg(Event::Follow)),
            ));
        }
        if s.mode == Mode::Mesh {
            let cell = kentos_ui::widget::focus_ring(
                text_input("0,5", &s.cell)
                    .on_input(|t| msg(Event::Cell(t)))
                    .padding([5, 8])
                    .size(kentos_ui::theme::typography::body())
                    .style(kentos_ui::style::field::input),
            );
            col = col.push(
                row![
                    container(words::field(
                        "Hücre boyu (m)",
                        cell,
                        Some("Mesh her katta bu ızgaranın hücreleriyle örneklenir; varsayılanı yüzlerin ortalama boyunun sekizde biri.".to_owned()),
                    ))
                    .width(Fill),
                    container(words::check(s.edges, "Ağ çizgilerini çiz", Some(msg(Event::Edges)))).width(Fill),
                ]
                .spacing(16)
                .align_y(iced::Alignment::End),
            );
        }
        col.into()
    }

    fn multidim_facts<'a>(&self, s: &'a State, i: &RasterInfo) -> Element<'a, Message> {
        let system = match i.epsg {
            Some(srid) => crate::crs::title_of(srid),
            None => "Dosya söylemiyor".to_owned(),
        };
        let format = self
            .document
            .as_ref()
            .map_or_else(kentos_interaction::Format::default, |d| {
                kentos_interaction::Format::of(d.settings())
            });
        let pixel = match i.affine {
            Some([_, a, b, _, c, d]) => format!(
                "{} × {} {}",
                format.length_bare(kentos_geometry_core::jsmath::js_hypot(a, c)),
                format.length_bare(kentos_geometry_core::jsmath::js_hypot(b, d)),
                format.length_unit_label()
            ),
            None => "—".to_owned(),
        };
        let size = format!(
            "{} × {} {}",
            crate::crs::grouped(f64::from(i.width)),
            crate::crs::grouped(f64::from(i.height)),
            if s.mode == Mode::Mesh {
                "hücre (sanal ızgara)"
            } else {
                "hücre"
            }
        );
        let pairs = Pairs::new()
            .push(label::caption("Izgara"), label::body(size))
            .push(label::caption("Hücre boyu"), label::body(pixel))
            .push(
                label::caption("Örnekler"),
                label::body(i.sample.label().to_owned()),
            )
            .push(label::caption("Sistem"), label::body(system));
        container(pairs)
            .padding([8, 10])
            .width(Fill)
            .style(kentos_ui::style::container::bordered)
            .into()
    }

    fn multidim_rule<'a>(&self, s: &'a State, i: &RasterInfo) -> Element<'a, Message> {
        let srid = self.multidim_srid();
        let project = self
            .document
            .as_ref()
            .and_then(|d| kentos_project::systems::own(d.settings()))
            .map_or_else(|| "Yerel (koordinat sistemi yok)".to_owned(), |n| n.title);
        let line = match place::rule(i, srid) {
            Rule::Same { .. } => words::text_line(
                Line::Ok,
                format!("Dosyanın sistemi projeninkiyle aynı: {project}. Dosyanın dediği yere eklenir."),
            ),
            Rule::Other { srid: other } => words::text_line(
                Line::Error,
                format!(
                    "Dosyanın sistemi ({}) projeninkinden ({project}) başka; eklenmez. KentOS yeniden izdüşürmez: veriyi projenin sistemine çevirip yeniden deneyin.",
                    crate::crs::title_of(other)
                ),
            ),
            Rule::Unknown => words::line(
                Line::Warn,
                column![
                    label::body("Dosya koordinat sistemini söylemiyor. Ancak projenin sisteminde olduğu söylenirse eklenir."),
                    words::check(s.confirmed, format!("Veri projenin sisteminde ({project})"), Some(msg(Event::Confirm))),
                ]
                .spacing(6),
            ),
            Rule::Unplaced => words::text_line(
                Line::Info,
                "Izgaranın koordinatları yok: görünümün ortasına oturtulmamış eklenir; Raster oturt ile yerine oturtulur.",
            ),
        };
        words::summary(vec![line])
    }

    fn multidim_options<'a>(&self, s: &'a State, r: &'a Read) -> Element<'a, Message> {
        let large = r.bytes > super::MOST_EMBEDDED as u64;
        let keep = Segmented::new_with(
            [Keep::Linked, Keep::Embedded],
            s.keep,
            |k| msg(Event::Keep(k)),
            move |k| k == Keep::Linked || !large,
        );
        let hint = if large {
            format!(
                "Dosya {} MB; gömülü raster en çok {} MB olabilir, bağlı kalır.",
                r.bytes >> 20,
                super::MOST_EMBEDDED >> 20
            )
        } else if s.keep == Keep::Embedded {
            "Dosyanın baytları projenin kitaplığına alınır; çizim dosyayla birlikte taşınır."
                .to_owned()
        } else {
            "Çizim dosyanın yolunu tutar; dosya yerinden oynarsa bulunamaz.".to_owned()
        };
        let name = kentos_ui::widget::focus_ring(
            text_input("Katmanın adı", &s.layer)
                .on_input(|t| msg(Event::Layer(t)))
                .on_submit(msg(Event::Run))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(kentos_ui::style::field::input),
        );
        row![
            container(words::field("Saklama", keep, Some(hint)))
                .width(iced::Length::FillPortion(3)),
            container(words::field(
                "Yeni katman",
                name,
                Some("Kendi katmanında durur.".to_owned())
            ))
            .width(iced::Length::FillPortion(2)),
        ]
        .spacing(16)
        .into()
    }
}

/// The look a dataset's raster starts with (the formats core's rule).
pub fn default_look(sample: RasterSample, mesh: bool, edges: bool) -> RasterStyle {
    kentos_formats::raster::style::dataset_style(sample, mesh, edges)
}
