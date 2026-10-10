//! Raster layers' files (docs/adr/0204): GeoTIFF and TIFF (classic and
//! BigTIFF), PNG and JPEG with a world file, read in pieces as the view
//! needs them, and drawn as 256-pixel tiles of a pyramid; one
//! implementation for the desktop (natively) and the web (in a worker).
//!
//! Nothing here does input or output: a reader says which bytes of its file
//! it needs ([`Need`]), the host reads them (the desktop from the file, the
//! web from the file's slices) and hands them over. JPEG is decoded by the
//! host's decoder (the desktop's zune-jpeg, the browser's); everything else
//! (LZW, Deflate, PackBits, predictors, PNG) here. A file never panics the
//! reader: every fault is a [`RasterError`] that says what is wrong.

pub mod codec;
pub mod geotiff;
pub mod layout;
pub mod place;
pub mod png;
pub mod pyramid;
pub mod samples;
pub mod source;
pub mod stats;
pub mod style;
pub mod tiff;
pub mod warp;
pub mod world;
pub mod write;

pub use samples::Samples;

/// A tile's side, pixels (docs/adr/0204 §3).
pub const TILE: u32 = 256;
/// A drawn tile's side: the tile and a pixel of its neighbours round it, so
/// that a bilinear sampler sees no seam.
pub const TILE_APRON: u32 = TILE + 2;

/// Why a file is not read, in the window's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterError(pub String);

impl RasterError {
    pub fn new(message: impl Into<String>) -> RasterError {
        RasterError(message.into())
    }
}

impl std::fmt::Display for RasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A run of a file's bytes a reader needs: from `offset`, `len` bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Need {
    pub offset: u64,
    pub len: u64,
}

/// The bytes a host has handed over, by where they were in the file.
#[derive(Clone, Debug, Default)]
pub struct ByteStore {
    ranges: Vec<(u64, Vec<u8>)>,
}

impl ByteStore {
    pub fn new() -> ByteStore {
        ByteStore::default()
    }

    /// Takes `bytes`, which were at `offset`.
    pub fn put(&mut self, offset: u64, bytes: Vec<u8>) {
        self.ranges.push((offset, bytes));
    }

    /// `len` bytes from `offset`, when one handed-over run holds them all.
    pub fn get(&self, offset: u64, len: u64) -> Option<&[u8]> {
        let end = offset.checked_add(len)?;
        self.ranges.iter().find_map(|(start, bytes)| {
            let stop = start.checked_add(bytes.len() as u64)?;
            if *start <= offset && end <= stop {
                let a = usize::try_from(offset - start).ok()?;
                let b = usize::try_from(end - start).ok()?;
                bytes.get(a..b)
            } else {
                None
            }
        })
    }

    /// Lets go of every run.
    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    /// Bytes held.
    pub fn held(&self) -> usize {
        self.ranges.iter().map(|(_, b)| b.len()).sum()
    }

    /// The longest handed-over run from the file's start (a header read by
    /// growing prefixes, docs/adr/0243 §2); empty when none starts there.
    pub fn prefix(&self) -> &[u8] {
        self.ranges
            .iter()
            .filter(|(start, _)| *start == 0)
            .map(|(_, b)| b.as_slice())
            .max_by_key(|b| b.len())
            .unwrap_or(&[])
    }
}

/// Either a value or the bytes still needed to make it.
#[derive(Clone, Debug, PartialEq)]
pub enum Step<T> {
    Done(T),
    Need(Need),
}
