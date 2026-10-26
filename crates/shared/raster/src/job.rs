//! An analysis run over a raster (docs/adr/0231 §2): the host opens the
//! raster (the formats core's reader), then, until the run is done, gives
//! the blocks [`Job::needs`] names ([`Job::put`]; a JPEG block's pixels
//! through [`Job::put_pixels`]), and appends what [`Job::step`] gives; at the
//! end [`Job::finish`] gives the GeoTIFF's directories and header, or the
//! lines. A step is one 256-row strip of the result; its rows are worked out
//! on the job's threads.

use kentos_contracts::{RasterRender, RasterSample, RasterStretch, RasterStyle};
use kentos_formats::raster::source::{BlockNeed, Layout, Put, Reader, decode_block};
use kentos_formats::raster::style::default_style;
use kentos_formats::raster::write::Geo;
use kentos_formats::raster::{Samples, TILE};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::crs::System;
use kentos_geometry_core::vec2::Vec2;
use serde::Deserialize;

use crate::contours::{self, Contourer, Line};
use crate::frame::Frame;
use crate::insolation;
use crate::out::{Out, OutSpec, Rows};
use crate::par;
use crate::relief::{Interp, Table};
use crate::terrain::{CurvatureKind, Kernel, Light, Method, RowFrame, RuggednessKind};

/// The widest raster and the most cells a run takes (docs/adr/0231 §2).
pub const MOST_WIDTH: u32 = 65_536;
pub const MOST_CELLS: u64 = 1 << 31;
/// The blocks the reader keeps for a run.
pub const READER_BUDGET: usize = 256 * 1024 * 1024;

/// What the host asks for: a tool and its settings, the band, the raster's place.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Spec {
    pub tool: Tool,
    /// 1 and up.
    pub band: u32,
    /// The raster object's `affine` (its place in the drawing, which Raster oturt may have changed).
    pub affine: [f64; 6],
    /// The raster's look's nodata (it stands for the file's); none: the file's.
    #[serde(default)]
    pub nodata: Option<f64>,
    /// The result's EPSG code (the raster's `srid` when above 0).
    #[serde(default)]
    pub epsg: Option<u32>,
    /// The project's coordinate system as the geometry core reads it
    /// (`crs::System`'s JSON); none for a local project.
    #[serde(default)]
    pub system: Option<serde_json::Value>,
    /// The analysis area (docs/adr/0535): minimum east, north, maximum east, north in the drawing. The result is
    /// the source's cells the area touches, on the source's own grid, each worked out from its neighbours in the
    /// source as the whole raster's would be (the cells round the area are read, not repeated); none: the whole.
    #[serde(default)]
    pub window: Option<[f64; 4]>,
    /// The run's analysis environment (docs/adr/0643 §1): its area when the tool gives none (`window`), its NoData
    /// for a float result's cells without a value. The surface tools read the DEM's own cells: no cell size or snap.
    #[serde(default)]
    pub environment: crate::env::Environment,
}

/// The cells of the source a run works out: the first column and row, the columns and rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub x0: u32,
    pub y0: u32,
    pub width: u32,
    pub height: u32,
}

/// How far within a cell a side of the area may lie and still be taken as on the cell's edge.
const EDGE: f64 = 1e-9;

/// The source's cells an area touches (docs/adr/0535): its corners taken into pixel coordinates by the inverse of
/// the frame's affine, widened to whole cells, clipped to the raster. An area beside the raster is refused.
pub fn window_of(frame: &Frame, area: [f64; 4]) -> Result<Window, String> {
    let [xmin, ymin, xmax, ymax] = area;
    if !area.iter().all(|v| v.is_finite()) || xmin > xmax || ymin > ymax {
        return Err(
            "Analiz alanı en küçük doğu, kuzey, en büyük doğu, kuzey sırasıyla dört sayı olmalı."
                .into(),
        );
    }
    let [e, a, b, n, c, d] = frame.affine;
    let det = a * d - b * c;
    let pixel = |x: f64, y: f64| {
        let (dx, dy) = (x - e, y - n);
        ((d * dx - b * dy) / det, (a * dy - c * dx) / det)
    };
    let corners = [
        pixel(xmin, ymin),
        pixel(xmax, ymin),
        pixel(xmin, ymax),
        pixel(xmax, ymax),
    ];
    let (mut i0, mut j0, mut i1, mut j1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for (i, j) in corners {
        i0 = i0.min(i);
        j0 = j0.min(j);
        i1 = i1.max(i);
        j1 = j1.max(j);
    }
    let (w, h) = (f64::from(frame.width), f64::from(frame.height));
    let x0 = (i0 + EDGE).floor().clamp(0.0, w);
    let y0 = (j0 + EDGE).floor().clamp(0.0, h);
    let x1 = (i1 - EDGE).ceil().clamp(0.0, w);
    let y1 = (j1 - EDGE).ceil().clamp(0.0, h);
    if x1 <= x0 || y1 <= y0 {
        return Err("Analiz alanı rasterle kesişmiyor; rasterin üstünde bir alan verin.".into());
    }
    Ok(Window {
        x0: x0 as u32,
        y0: y0 as u32,
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
    })
}

/// The affine of a window's grid: the source's, its origin at the window's first cell (the whole raster's exactly).
pub fn window_affine(affine: [f64; 6], w: Window) -> [f64; 6] {
    let [e, a, b, n, c, d] = affine;
    let (i, j) = (f64::from(w.x0), f64::from(w.y0));
    [e + a * i + b * j, a, b, n + c * i + d * j, c, d]
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MethodName {
    Horn,
    ZevenbergenThorne,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SlopeUnit {
    Degrees,
    Percent,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CurvatureName {
    Total,
    Profile,
    Plan,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuggednessName {
    TriRiley,
    TriWilson,
    Tpi,
    Roughness,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InterpName {
    Linear,
    Nearest,
}

/// The tools of Yüzey analizi (docs/adr/0231 §1).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Tool {
    Slope {
        method: MethodName,
        unit: SlopeUnit,
        z_factor: f64,
    },
    Aspect {
        method: MethodName,
        z_factor: f64,
    },
    Hillshade {
        azimuth: f64,
        altitude: f64,
        z_factor: f64,
    },
    ColorRelief {
        /// A ramp of ADR 0204's, or none for `table`.
        #[serde(default)]
        ramp: Option<String>,
        #[serde(default)]
        invert: bool,
        /// The ramp's bounds; none: the band's least and largest.
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
        /// Lines `value #RRGGBB[AA]`.
        #[serde(default)]
        table: Option<String>,
        interp: InterpName,
    },
    Curvature {
        curvature: CurvatureName,
        z_factor: f64,
    },
    Ruggedness {
        index: RuggednessName,
    },
    Insolation {
        first_day: u32,
        last_day: u32,
        day_step: u32,
        hour_step: f64,
        transmissivity: f64,
        /// A local project's latitude (degrees).
        #[serde(default)]
        latitude: Option<f64>,
        z_factor: f64,
    },
    Contours {
        interval: f64,
        base: f64,
        index_every: u32,
        #[serde(default)]
        simplify: f64,
    },
}

/// The ramp a tool's result is drawn with (docs/adr/0231 §10): Eğim and Bakı
/// Spektral (flat blue, steep red), Eğrilik Mavi-kırmızı, Pürüzlülük
/// Viridis, Güneşlenme Sıcaklık; none for the others.
fn ramp_of(tool: &Tool) -> Option<&'static str> {
    match tool {
        Tool::Slope { .. } | Tool::Aspect { .. } => Some("Spektral"),
        Tool::Curvature { .. } => Some("Mavi-kırmızı"),
        Tool::Ruggedness { .. } => Some("Viridis"),
        Tool::Insolation { .. } => Some("Sıcaklık"),
        Tool::Hillshade { .. } | Tool::ColorRelief { .. } | Tool::Contours { .. } => None,
    }
}

/// How a run ends.
#[derive(Debug)]
pub enum Finished {
    /// The GeoTIFF's directories to append and the header to write over its start.
    Raster {
        tail: Vec<u8>,
        header: Vec<u8>,
    },
    Lines(Vec<Line>),
}

/// Renkli kabartma's table, or its ramp waiting for the band's bounds (the first pass).
enum Colors {
    Table(Table),
    Ramp(&'static [[u8; 3]], bool),
}

enum Work {
    Kernel(Kernel, Option<Light>),
    Relief(Colors, Interp),
    Insolation {
        days: Vec<(u32, f64)>,
        hour_step: f64,
        tau: f64,
        z: f64,
        /// Each row's latitude.
        latitude: Latitude,
    },
    Contours(Box<Contourer>),
}

enum Latitude {
    Fixed(f64),
    System(Box<System>),
}

/// The run's state.
pub struct Job {
    reader: Reader,
    band: u32,
    nodata: Option<f64>,
    /// The whole source's frame: a row's axes and latitude are the source row's.
    frame: Frame,
    /// The cells worked out (the whole raster without an area).
    win: Window,
    work: Work,
    /// The first pass works out the band's least and largest (Renkli kabartma's ramp).
    stats_pass: bool,
    lo: f64,
    hi: f64,
    /// The next result row.
    next: u32,
    out: Option<Out>,
    threads: usize,
    /// The strip's heights (halo round it), kept between strips.
    heights: Vec<f64>,
    /// The ramp the result is drawn with (docs/adr/0231 §10); none: its samples' own look.
    ramp: Option<&'static str>,
    /// The environment's NoData: what a float result's cell without a value is written as (NaN otherwise).
    fill: Option<f32>,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job").field("next", &self.next).finish()
    }
}

fn finite(v: f64, what: &str) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{what} bir sayı olmalı."))
    }
}

impl Job {
    /// A run of `spec` over the opened raster, its rows on `threads`; the
    /// bytes to write first (a raster result's header; none for lines).
    pub fn new(reader: Reader, spec: &Spec, threads: usize) -> Result<(Job, Vec<u8>), String> {
        let info = reader.info.clone();
        if spec.band == 0 || spec.band > info.bands {
            return Err(format!(
                "Rasterin {} bandı var; {}. bant yok.",
                info.bands, spec.band
            ));
        }
        if info.width > MOST_WIDTH || u64::from(info.width) * u64::from(info.height) > MOST_CELLS {
            return Err(format!(
                "Raster çözümleme için çok büyük ({} × {}): genişlik en çok {MOST_WIDTH}, hücre sayısı en çok 2³¹. Rasteri parçalara bölün.",
                info.width, info.height
            ));
        }
        let system = match &spec.system {
            Some(v) => Some(
                Json::parse(&v.to_string())
                    .and_then(|j| System::from_json(&j))
                    .map_err(|e| format!("Projenin koordinat sistemi okunamadı: {e}"))?,
            ),
            None => None,
        };
        let geographic = system.as_ref().is_some_and(System::is_geographic);
        let frame = Frame::new(spec.affine, info.width, info.height, geographic)?;
        if spec.environment.regrids() {
            return Err(
                "Yüzey analizi yükseklik modelinin kendi hücreleriyle çalışır: hücre boyu ve hizalama ortamı için önce Yeniden örnekle ile o ızgaraya getirin."
                    .into(),
            );
        }
        let win = match spec.window.or(spec.environment.area) {
            Some(area) => window_of(&frame, area)?,
            None => Window {
                x0: 0,
                y0: 0,
                width: info.width,
                height: info.height,
            },
        };
        let out_affine = window_affine(spec.affine, win);
        let nodata = spec.nodata.or(info.nodata);
        let mut stats_pass = false;
        let (work, sample, bands, alpha, mut out_nodata) = match &spec.tool {
            Tool::Slope {
                method,
                unit,
                z_factor,
            } => (
                Work::Kernel(
                    Kernel::Slope {
                        method: method_of(*method),
                        percent: *unit == SlopeUnit::Percent,
                        z: finite(*z_factor, "Z çarpanı")?,
                    },
                    None,
                ),
                RasterSample::F32,
                1,
                false,
                Some(f64::NAN),
            ),
            Tool::Aspect { method, z_factor } => (
                Work::Kernel(
                    Kernel::Aspect {
                        method: method_of(*method),
                        z: finite(*z_factor, "Z çarpanı")?,
                    },
                    None,
                ),
                RasterSample::F32,
                1,
                false,
                Some(f64::NAN),
            ),
            Tool::Hillshade {
                azimuth,
                altitude,
                z_factor,
            } => {
                let (az, alt) = (
                    finite(*azimuth, "Işığın açısı")?,
                    finite(*altitude, "Işığın yüksekliği")?,
                );
                if !(0.0..=90.0).contains(&alt) {
                    return Err("Işığın yüksekliği 0 ile 90 derece arasında olmalı.".into());
                }
                (
                    Work::Kernel(
                        Kernel::Hillshade {
                            azimuth: az,
                            altitude: alt,
                            z: finite(*z_factor, "Z çarpanı")?,
                        },
                        Some(Light::new(az, alt)),
                    ),
                    RasterSample::U8,
                    1,
                    false,
                    Some(0.0),
                )
            }
            Tool::ColorRelief {
                ramp,
                invert,
                min,
                max,
                table,
                interp,
            } => {
                let interp = match interp {
                    InterpName::Linear => Interp::Linear,
                    InterpName::Nearest => Interp::Nearest,
                };
                let t = match (table, ramp) {
                    (Some(text), _) if !text.trim().is_empty() => {
                        Colors::Table(Table::parse(text)?)
                    }
                    (_, Some(name)) => {
                        if !kentos_contracts::RASTER_RAMPS.contains(&name.as_str()) {
                            return Err(format!("“{name}” adında rampa yok."));
                        }
                        let stops = kentos_formats::raster::style::ramp_stops(name);
                        match (min, max) {
                            (Some(lo), Some(hi)) => Colors::Table(Table::ramp(
                                stops,
                                *invert,
                                finite(*lo, "En küçük değer")?,
                                finite(*hi, "En büyük değer")?,
                            )),
                            _ => {
                                stats_pass = true;
                                Colors::Ramp(stops, *invert)
                            }
                        }
                    }
                    _ => {
                        return Err(
                            "Renkli kabartma için bir rampa ya da renk tablosu verin.".into()
                        );
                    }
                };
                (Work::Relief(t, interp), RasterSample::U8, 4, true, None)
            }
            Tool::Curvature {
                curvature,
                z_factor,
            } => {
                let [a, b, c, d] = frame.axes(info.height / 2);
                if (a * b + c * d).abs() > 1e-9 * (a * a + c * c).sqrt() * (b * b + d * d).sqrt() {
                    return Err("Eğrilik dik pikselli raster ister; bu rasterin pikselleri eğik (afinin eksenleri dik değil).".into());
                }
                (
                    Work::Kernel(
                        Kernel::Curvature {
                            kind: match curvature {
                                CurvatureName::Total => CurvatureKind::Total,
                                CurvatureName::Profile => CurvatureKind::Profile,
                                CurvatureName::Plan => CurvatureKind::Plan,
                            },
                            z: finite(*z_factor, "Z çarpanı")?,
                        },
                        None,
                    ),
                    RasterSample::F32,
                    1,
                    false,
                    Some(f64::NAN),
                )
            }
            Tool::Ruggedness { index } => (
                Work::Kernel(
                    Kernel::Ruggedness {
                        kind: match index {
                            RuggednessName::TriRiley => RuggednessKind::TriRiley,
                            RuggednessName::TriWilson => RuggednessKind::TriWilson,
                            RuggednessName::Tpi => RuggednessKind::Tpi,
                            RuggednessName::Roughness => RuggednessKind::Roughness,
                        },
                    },
                    None,
                ),
                RasterSample::F32,
                1,
                false,
                Some(f64::NAN),
            ),
            Tool::Insolation {
                first_day,
                last_day,
                day_step,
                hour_step,
                transmissivity,
                latitude,
                z_factor,
            } => {
                if ![0.25, 0.5, 1.0, 2.0].contains(hour_step) {
                    return Err("Saat aralığı 0,25, 0,5, 1 ya da 2 saat olmalı.".into());
                }
                let tau = finite(*transmissivity, "Geçirgenlik")?;
                if !(tau > 0.0 && tau <= 1.0) {
                    return Err("Geçirgenlik 0'dan büyük, en çok 1 olmalı.".into());
                }
                let latitude = match (&system, latitude) {
                    (Some(s), _) => Latitude::System(Box::new(s.clone())),
                    (None, Some(phi)) if phi.is_finite() && phi.abs() <= 90.0 => Latitude::Fixed(*phi),
                    _ => return Err("Koordinat sistemi olmayan projede güneşlenme için Enlem'i yazın (−90 ile 90 derece).".into()),
                };
                (
                    Work::Insolation {
                        days: insolation::blocks(*first_day, *last_day, *day_step)?,
                        hour_step: *hour_step,
                        tau,
                        z: finite(*z_factor, "Z çarpanı")?,
                        latitude,
                    },
                    RasterSample::F32,
                    1,
                    false,
                    Some(f64::NAN),
                )
            }
            Tool::Contours {
                interval,
                base,
                index_every,
                simplify,
            } => {
                let s = contours::Spec {
                    interval: *interval,
                    base: *base,
                    index_every: *index_every,
                    simplify: *simplify,
                };
                if let Some(p) = s.problem() {
                    return Err(p);
                }
                // The lines are worked out in the window's own grid (its rows from 0).
                let lines_frame = Frame::new(out_affine, win.width, win.height, geographic)?;
                let job = Job {
                    reader,
                    band: spec.band,
                    nodata,
                    frame: frame.clone(),
                    win,
                    work: Work::Contours(Box::new(Contourer::new(s, lines_frame))),
                    stats_pass: false,
                    lo: f64::INFINITY,
                    hi: f64::NEG_INFINITY,
                    next: 0,
                    out: None,
                    threads: threads.max(1),
                    heights: Vec::new(),
                    ramp: None,
                    fill: None,
                };
                return Ok((job, Vec::new()));
            }
        };
        // The environment's NoData for a float result; an 8-bit look's 0 is its own.
        let fill = match spec.environment.nodata {
            None => None,
            Some(v) if sample.float() => {
                out_nodata = Some(v);
                Some(v as f32)
            }
            Some(_) => {
                return Err(
                    "Gölgeli ve renkli kabartma boş hücreyi 0 yazar; ortamın NoData'sını boşaltın."
                        .into(),
                );
            }
        };
        let (out, header) = Out::new(
            OutSpec {
                width: win.width,
                height: win.height,
                bands,
                sample,
                alpha,
                nodata: out_nodata,
                geo: Geo {
                    affine: out_affine,
                    epsg: spec.epsg,
                    geographic,
                },
            },
            threads,
        )?;
        Ok((
            Job {
                reader,
                band: spec.band,
                nodata,
                frame,
                win,
                work,
                stats_pass,
                lo: f64::INFINITY,
                hi: f64::NEG_INFINITY,
                next: 0,
                out: Some(out),
                threads: threads.max(1),
                heights: Vec::new(),
                ramp: ramp_of(&spec.tool),
                fill,
            },
            header,
        ))
    }

    /// What the work reads round each strip: cells each side, rows above and below.
    fn halo(&self) -> (u32, u32, u32) {
        match self.work {
            _ if self.stats_pass => (0, 0, 0),
            Work::Relief(..) => (0, 0, 0),
            // A strip's squares reach into the next strip's first row.
            Work::Contours(_) => (0, 0, 1),
            Work::Kernel(..) | Work::Insolation { .. } => (1, 1, 1),
        }
    }

    /// The next strip's rows: first (the window's) and count.
    fn strip(&self) -> (u32, u32) {
        let h = self.win.height;
        (self.next, TILE.min(h - self.next.min(h)))
    }

    /// The region the next strip reads: x, y, width, height (level 0, the
    /// source's pixels; the reader repeats the raster's edge past its sides,
    /// a window's halo inside the raster is read).
    fn region(&self) -> (i64, i64, u32, u32) {
        let (y0, n) = self.strip();
        let (side, above, below) = self.halo();
        (
            i64::from(self.win.x0) - i64::from(side),
            i64::from(self.win.y0) + i64::from(y0) - i64::from(above),
            self.win.width + 2 * side,
            n + above + below,
        )
    }

    /// The result's grid: its affine, columns and rows (the window's; the source's without an area).
    pub fn grid(&self) -> ([f64; 6], u32, u32) {
        (
            window_affine(self.frame.affine, self.win),
            self.win.width,
            self.win.height,
        )
    }

    /// The cells worked out.
    pub fn window(&self) -> Window {
        self.win
    }

    /// The blocks the next step reads that the reader does not hold.
    pub fn needs(&mut self) -> Vec<BlockNeed> {
        if self.done() {
            return Vec::new();
        }
        let (x, y, w, h) = self.region();
        self.reader.needs(0, x, y, w, h)
    }

    /// A block's bytes, as [`Job::needs`] named it.
    pub fn put(&mut self, need: &BlockNeed, bytes: &[u8]) -> Result<Put, String> {
        self.reader.put_block(need, bytes).map_err(|e| e.0)
    }

    /// The blocks [`Job::needs`] named, with their bytes: decoded on the job's
    /// threads and kept in their order; each JPEG block's stream comes back
    /// for the host's pixels ([`Job::put_pixels`]).
    pub fn put_all(
        &mut self,
        blocks: Vec<(BlockNeed, Vec<u8>)>,
    ) -> Result<Vec<(BlockNeed, Vec<u8>)>, String> {
        let items: Vec<(BlockNeed, Vec<u8>, Option<Layout>)> = blocks
            .into_iter()
            .map(|(need, bytes)| {
                let layout = self.reader.layout_for(&need);
                (need, bytes, layout)
            })
            .collect();
        let decoded = par::map(
            self.threads,
            &items,
            &|(need, bytes, layout)| match layout {
                Some(l) => decode_block(l, need.index, bytes).map_err(|e| e.0),
                None => Err("Bu rasterin böyle bir bloğu yok.".to_owned()),
            },
        );
        let mut jpeg = Vec::new();
        for ((need, _, _), d) in items.iter().zip(decoded) {
            if let Put::Jpeg(stream) = self.reader.keep(need, d?) {
                jpeg.push((*need, stream));
            }
        }
        Ok(jpeg)
    }

    /// A JPEG block's pixels (`components` a pixel), decoded by the host.
    pub fn put_pixels(
        &mut self,
        need: &BlockNeed,
        pixels: Vec<u8>,
        components: u32,
    ) -> Result<(), String> {
        self.reader
            .put_pixels(need, pixels, components)
            .map_err(|e| e.0)
    }

    /// A raster result's bands and samples; none for lines.
    pub fn result(&self) -> Option<(u32, RasterSample)> {
        self.out.as_ref().map(|o| (o.spec().bands, o.spec().sample))
    }

    /// Eş yükselti eğrileri's settings (their Kot texts, `Spec::level_text`); none for a raster result.
    pub fn contours(&self) -> Option<&contours::Spec> {
        match &self.work {
            Work::Contours(c) => Some(c.spec()),
            _ => None,
        }
    }

    /// A raster result's look (docs/adr/0231 §10): a single band of numbers
    /// in its tool's ramp, stretched over its least to its most (Eğrilik's
    /// over its 2nd to 98th percentile: a few sharp cells would pale the
    /// rest); Gölgeli kabartma grey and Renkli kabartma's colours as they
    /// are (the look its samples start with, docs/adr/0204 §4). None for lines.
    pub fn style(&self) -> Option<RasterStyle> {
        let (bands, sample) = self.result()?;
        let style = default_style(bands, sample, false);
        let percent = matches!(self.work, Work::Kernel(Kernel::Curvature { .. }, _));
        Some(match self.ramp {
            Some(ramp) => RasterStyle {
                render: RasterRender::Ramp,
                bands: vec![1],
                stretch: if percent {
                    RasterStretch::Percent
                } else {
                    RasterStretch::MinMax
                },
                ramp: Some(ramp.to_owned()),
                invert: false,
                ..style
            },
            None => style,
        })
    }

    /// Whether every strip is done.
    pub fn done(&self) -> bool {
        !self.stats_pass && self.next >= self.win.height
    }

    /// The share done, 0..1 (the first pass of two counts as a fifth).
    pub fn share(&self) -> f64 {
        let h = f64::from(self.win.height.max(1));
        let f = f64::from(self.next.min(self.win.height)) / h;
        match (&self.work, self.stats_pass) {
            (Work::Relief(..), true) => 0.2 * f,
            (Work::Relief(..), false) => 0.2 + 0.8 * f,
            _ => f,
        }
    }

    /// The strip's heights of the band: NaN where nodata.
    fn read_strip(&mut self) -> Result<(u32, u32), String> {
        let (x, y, w, h) = self.region();
        let region = self
            .reader
            .region(0, x, y, w, h)
            .ok_or_else(|| "Rasterin blokları eksik verildi.".to_owned())?;
        let b = (self.band - 1) as usize;
        let bands = region.bands as usize;
        let nodata = self.nodata;
        self.heights.clear();
        self.heights.reserve(w as usize * h as usize);
        macro_rules! band {
            ($v:expr) => {
                self.heights
                    .extend($v.iter().skip(b).step_by(bands).map(|&s| {
                        let v = f64::from(s);
                        if v.is_nan() || nodata.is_some_and(|d| v == d) {
                            f64::NAN
                        } else {
                            v
                        }
                    }))
            };
        }
        match &region.samples {
            Samples::U8(v) => band!(v),
            Samples::I8(v) => band!(v),
            Samples::U16(v) => band!(v),
            Samples::I16(v) => band!(v),
            Samples::U32(v) => band!(v),
            Samples::I32(v) => band!(v),
            Samples::F32(v) => band!(v),
            Samples::F64(v) => band!(v),
        }
        Ok((w, h))
    }

    /// Works out the next strip; gives the bytes to append.
    pub fn step(&mut self) -> Result<Vec<u8>, String> {
        if self.done() {
            return Ok(Vec::new());
        }
        let (w, _) = self.read_strip()?;
        let (y0, n) = self.strip();
        let width = self.win.width as usize;
        // The source row of the strip's first: a row's axes and latitude are the source's.
        let row0 = self.win.y0 + y0;
        if self.stats_pass {
            for &v in &self.heights {
                if !v.is_nan() {
                    if v < self.lo {
                        self.lo = v;
                    }
                    if v > self.hi {
                        self.hi = v;
                    }
                }
            }
            self.next += n;
            if self.next >= self.win.height {
                // The ramp's bounds are the band's: the second pass colours.
                self.stats_pass = false;
                self.next = 0;
                if let Work::Relief(colors, _) = &mut self.work
                    && let Colors::Ramp(stops, invert) = *colors
                {
                    *colors = Colors::Table(Table::ramp(stops, invert, self.lo, self.hi));
                }
            }
            return Ok(Vec::new());
        }
        let sw = w as usize;
        let threads = self.threads;
        let bytes = match &mut self.work {
            Work::Kernel(kernel, light) => {
                let kernel = *kernel;
                let z = kernel.z();
                let frames: Vec<RowFrame> = (0..n)
                    .map(|r| RowFrame::new(self.frame.axes(row0 + r), z))
                    .collect();
                let heights = &self.heights;
                let out = self.out.as_mut().ok_or("Sonuç rasteri yok.")?;
                match light {
                    Some(light) => {
                        let light = *light;
                        let mut rows = vec![0u8; n as usize * width];
                        par::rows(threads, &mut rows, width, &|first, chunk: &mut [u8]| {
                            for (k, row) in chunk.chunks_mut(width).enumerate() {
                                let r = first + k;
                                let up = &heights[r * sw..(r + 1) * sw];
                                let mid = &heights[(r + 1) * sw..(r + 2) * sw];
                                let down = &heights[(r + 2) * sw..(r + 3) * sw];
                                kernel.row_u8(&frames[r], &light, [up, mid, down], row);
                            }
                        });
                        out.push(Rows::U8(&rows), n)?
                    }
                    None => {
                        let mut rows = vec![0f32; n as usize * width];
                        par::rows(threads, &mut rows, width, &|first, chunk: &mut [f32]| {
                            for (k, row) in chunk.chunks_mut(width).enumerate() {
                                let r = first + k;
                                let up = &heights[r * sw..(r + 1) * sw];
                                let mid = &heights[(r + 1) * sw..(r + 2) * sw];
                                let down = &heights[(r + 2) * sw..(r + 3) * sw];
                                kernel.row_f32(&frames[r], [up, mid, down], row);
                            }
                        });
                        {
                            if let Some(f) = self.fill {
                                rows.iter_mut().filter(|v| v.is_nan()).for_each(|v| *v = f);
                            }
                            out.push(Rows::F32(&rows), n)?
                        }
                    }
                }
            }
            Work::Relief(colors, interp) => {
                let Colors::Table(table) = &*colors else {
                    return Err("Renkli kabartmanın rampası bandın sınırlarını bekliyor.".into());
                };
                let interp = *interp;
                let heights = &self.heights;
                let out = self.out.as_mut().ok_or("Sonuç rasteri yok.")?;
                let mut rows = vec![0u8; n as usize * width * 4];
                par::rows(threads, &mut rows, width * 4, &|first, chunk: &mut [u8]| {
                    for (k, row) in chunk.chunks_mut(width * 4).enumerate() {
                        let src = &heights[(first + k) * sw..(first + k + 1) * sw];
                        for (i, px) in row.chunks_mut(4).enumerate() {
                            let v = src[i];
                            px.copy_from_slice(&if v.is_nan() {
                                [0, 0, 0, 0]
                            } else {
                                table.color(v, interp)
                            });
                        }
                    }
                });
                out.push(Rows::U8(&rows), n)?
            }
            Work::Insolation {
                days,
                hour_step,
                tau,
                z,
                latitude,
            } => {
                let heights = &self.heights;
                let frame = &self.frame;
                // The rows' suns, worked out on the threads too.
                let rows_lat: Vec<f64> = (0..n)
                    .map(|r| match latitude {
                        Latitude::Fixed(phi) => Ok(*phi),
                        Latitude::System(s) => {
                            let (x, y) = frame.point(f64::from(frame.width) / 2.0, f64::from(row0 + r) + 0.5);
                            s.geographic_of(Vec2::new(x, y))
                                .map(|(lat, _)| lat)
                                .ok_or_else(|| "Rasterin bir satırı koordinat sisteminin dışında: enlemi bulunamadı.".to_owned())
                        }
                    })
                    .collect::<Result<_, String>>()?;
                let frames: Vec<RowFrame> = (0..n)
                    .map(|r| RowFrame::new(frame.axes(row0 + r), *z))
                    .collect();
                let (days, hour_step, tau) = (&*days, *hour_step, *tau);
                let out = self.out.as_mut().ok_or("Sonuç rasteri yok.")?;
                let mut rows = vec![0f32; n as usize * width];
                par::rows(threads, &mut rows, width, &|first, chunk: &mut [f32]| {
                    let mut suns = Vec::new();
                    for (k, row) in chunk.chunks_mut(width).enumerate() {
                        let r = first + k;
                        insolation::suns(rows_lat[r], days, hour_step, tau, &mut suns);
                        let up = &heights[r * sw..(r + 1) * sw];
                        let mid = &heights[(r + 1) * sw..(r + 2) * sw];
                        let down = &heights[(r + 2) * sw..(r + 3) * sw];
                        for (i, o) in row.iter_mut().enumerate() {
                            *o = match crate::terrain::window(up, mid, down, i) {
                                Some(win) => {
                                    let (pi, pj) = crate::terrain::derivatives(&win, Method::Horn);
                                    let (gx, gy) = frames[r].gradient.of(pi, pj);
                                    insolation::energy(gx, gy, &suns) as f32
                                }
                                None => f32::NAN,
                            };
                        }
                    }
                });
                {
                    if let Some(f) = self.fill {
                        rows.iter_mut().filter(|v| v.is_nan()).for_each(|v| *v = f);
                    }
                    out.push(Rows::F32(&rows), n)?
                }
            }
            Work::Contours(c) => {
                // The strip's rows and the next strip's first (its squares'
                // bottom row; the contourer takes a row it already has as none).
                let rows = (self.heights.len() / sw) as u32;
                for r in 0..rows {
                    let j = y0 + r;
                    if j >= self.win.height {
                        break;
                    }
                    c.push(j, &self.heights[r as usize * sw..(r as usize + 1) * sw])?;
                }
                Vec::new()
            }
        };
        self.next += n;
        Ok(bytes)
    }

    /// The result: the GeoTIFF's last pieces, or the lines.
    pub fn finish(self) -> Result<Finished, String> {
        if !self.done() {
            return Err("Çözümleme bitmeden bırakıldı.".into());
        }
        match self.work {
            Work::Contours(c) => Ok(Finished::Lines(c.finish()?)),
            _ => {
                let out = self.out.ok_or("Sonuç rasteri yok.")?;
                let (tail, header) = out.finish()?;
                Ok(Finished::Raster { tail, header })
            }
        }
    }
}

fn method_of(m: MethodName) -> Method {
    match m {
        MethodName::Horn => Method::Horn,
        MethodName::ZevenbergenThorne => Method::ZevenbergenThorne,
    }
}
