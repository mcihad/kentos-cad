//! Çok boyutlu veri's tools (docs/adr/0243 §8–§10): Kesit (a raster's values
//! along lines), Zaman serisi (values at points over a dataset's time steps
//! or a raster's bands) and Mesh hesaplayıcı (a new dataset on a mesh from an
//! expression over its datasets). Each is a job over the file's bytes as the
//! raster jobs are: it says which runs of the file it needs, the host reads
//! and puts them, and it steps. Its tables and texts are the same on both
//! platforms (the web's come from here through WASM).

pub mod calc;
pub mod points;
pub mod profile;
pub mod series;

use kentos_geometry_core::display::fixed;

/// A table a tool gives: its columns and rows, as shown.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// A Kesit line drawn with its values: the line's number and its points
/// (x, y and the value as the vertex's elevation).
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub line: u32,
    pub points: Vec<[f64; 3]>,
}

/// What a tool's job gave.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Finished {
    pub table: Option<Table>,
    pub pieces: Vec<Piece>,
    pub summary: String,
    pub warnings: Vec<String>,
}

/// A value in a table: three decimals, nothing where there is none.
pub fn value_text(v: f64) -> String {
    if v.is_finite() {
        fixed(v, 3)
    } else {
        String::new()
    }
}

/// A count in Turkish grouping (12.345).
pub fn grouped(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

/// A job of one of the tools.
pub enum MultidimJob {
    /// Kesit, and Zaman serisi over bands: values at points of a raster read by its blocks.
    Points(Box<points::PointsJob>),
    /// Zaman serisi over a NetCDF dataset's time steps.
    Series(Box<series::TimeJob>),
    /// Mesh hesaplayıcı.
    Calc(Box<calc::MeshCalc>),
}

impl std::fmt::Debug for MultidimJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MultidimJob::Points(_) => "MultidimJob::Points",
            MultidimJob::Series(_) => "MultidimJob::Series",
            MultidimJob::Calc(_) => "MultidimJob::Calc",
        })
    }
}

impl MultidimJob {
    /// The runs of the file the next step needs (offset, length), in order;
    /// [`MultidimJob::put`] takes each by its place in this list.
    pub fn needs(&mut self) -> Result<Vec<(u64, u64)>, String> {
        match self {
            MultidimJob::Points(j) => Ok(j.needs()),
            MultidimJob::Series(j) => Ok(j.needs()),
            MultidimJob::Calc(j) => j.needs(),
        }
    }

    /// The bytes of need `k`; a JPEG block's stream back for the host's decoder.
    pub fn put(&mut self, k: usize, bytes: Vec<u8>) -> Result<Option<Vec<u8>>, String> {
        match self {
            MultidimJob::Points(j) => j.put(k, &bytes),
            MultidimJob::Series(j) => j.put(k, bytes).map(|()| None),
            MultidimJob::Calc(j) => j.put(k, bytes).map(|()| None),
        }
    }

    /// A JPEG block's pixels (`components` a pixel).
    pub fn put_pixels(&mut self, k: usize, pixels: Vec<u8>, components: u32) -> Result<(), String> {
        match self {
            MultidimJob::Points(j) => j.put_pixels(k, pixels, components),
            _ => Err("Bu işin JPEG bloğu yok.".into()),
        }
    }

    /// Works out what the runs put allow.
    pub fn step(&mut self) -> Result<(), String> {
        match self {
            MultidimJob::Points(j) => j.step(),
            MultidimJob::Series(j) => j.step(),
            MultidimJob::Calc(j) => j.step(),
        }
    }

    pub fn done(&self) -> bool {
        match self {
            MultidimJob::Points(j) => j.done(),
            MultidimJob::Series(j) => j.done(),
            MultidimJob::Calc(j) => j.done(),
        }
    }

    /// How much is done, 0 … 1.
    pub fn share(&self) -> f64 {
        match self {
            MultidimJob::Points(j) => j.share(),
            MultidimJob::Series(j) => j.share(),
            MultidimJob::Calc(j) => j.share(),
        }
    }

    /// What the job gave; Mesh hesaplayıcı's file written into `sink`.
    pub fn finish(
        self,
        sink: &mut dyn FnMut(&[u8]) -> Result<(), String>,
    ) -> Result<Finished, String> {
        match self {
            MultidimJob::Points(j) => Ok(j.finish()),
            MultidimJob::Series(j) => j.finish(),
            MultidimJob::Calc(j) => j.finish(sink),
        }
    }
}
