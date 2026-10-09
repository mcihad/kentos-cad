//! A tiled GeoTIFF written as a stream (docs/adr/0204 §3, §6): the pyramid
//! file and a resampled raster. The host appends what the writer gives in
//! order (the header, the tiles, the directories) and finally writes the
//! header again at the start (it names the first directory, known last).
//! Tiles are 256 × 256 and Deflate-coded; the file is BigTIFF when its
//! samples might pass 3.5 GB. Every directory but the first is a reduced
//! image (NewSubfileType 1), as GDAL writes overviews.

use kentos_contracts::RasterSample;

use super::{RasterError, Samples, TILE};

/// One image (directory) of the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub sample: RasterSample,
    /// The last band is alpha (unassociated).
    pub alpha: bool,
    /// The first image's place and system.
    pub geo: Option<Geo>,
    pub nodata: Option<f64>,
}

/// A GeoTIFF's place and system.
#[derive(Clone, Debug, PartialEq)]
pub struct Geo {
    pub affine: [f64; 6],
    pub epsg: Option<u32>,
    pub geographic: bool,
}

/// A tile's little-endian sample bytes as the file holds them: zlib-wrapped
/// Deflate at `level` (1 fast … 9 small).
pub fn code(bytes: &[u8], level: u8) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(bytes, level)
}

/// The writer's state between the pieces it gives.
#[derive(Debug)]
pub struct Writer {
    images: Vec<Image>,
    big: bool,
    level: u8,
    /// Each image's tiles' offsets and byte counts, by tile index.
    offsets: Vec<Vec<u64>>,
    counts: Vec<Vec<u64>>,
    /// The file's length so far.
    pos: u64,
}

/// A tile's index in its image (row by row).
fn tiles_across(w: u32) -> u32 {
    w.div_ceil(TILE)
}

impl Writer {
    /// A writer for `images` and the header to write first; `level` is
    /// Deflate's (1 fast … 9 small).
    pub fn new(images: Vec<Image>, level: u8) -> Result<(Writer, Vec<u8>), RasterError> {
        if images.is_empty() {
            return Err(RasterError::new("Yazılacak görüntü yok."));
        }
        let raw: u64 = images
            .iter()
            .map(|i| {
                u64::from(i.width)
                    * u64::from(i.height)
                    * u64::from(i.bands)
                    * i.sample.bytes() as u64
            })
            .sum();
        let big = raw > 3_500_000_000;
        let offsets = images
            .iter()
            .map(|i| vec![0u64; (tiles_across(i.width) * i.height.div_ceil(TILE)) as usize])
            .collect::<Vec<_>>();
        let counts = offsets.clone();
        let header = header(big, 0);
        let pos = header.len() as u64;
        Ok((
            Writer {
                images,
                big,
                level,
                offsets,
                counts,
                pos,
            },
            header,
        ))
    }

    /// The bytes of tile (`tx`, `ty`) of image `image` to append: its
    /// samples (`TILE` × `TILE`, bands interleaved; the part past the image's
    /// edge anything) Deflate-coded.
    pub fn tile(
        &mut self,
        image: usize,
        tx: u32,
        ty: u32,
        samples: &Samples,
    ) -> Result<Vec<u8>, RasterError> {
        let img = self
            .images
            .get(image)
            .ok_or_else(|| RasterError::new("Böyle bir görüntü yok."))?;
        let want = (TILE * TILE * img.bands) as usize;
        if samples.len() != want || samples.kind() != img.sample {
            return Err(RasterError::new("Karonun örnekleri görüntüye uymuyor."));
        }
        let coded = code(&samples.to_bytes(true), self.level);
        self.coded(image, tx, ty, coded)
    }

    /// The bytes to append for tile (`tx`, `ty`) of image `image` already
    /// coded by [`code`] (a host codes a band's tiles on its threads, then
    /// hands them over in order; docs/adr/0231 §11).
    pub fn coded(
        &mut self,
        image: usize,
        tx: u32,
        ty: u32,
        coded: Vec<u8>,
    ) -> Result<Vec<u8>, RasterError> {
        let img = self
            .images
            .get(image)
            .ok_or_else(|| RasterError::new("Böyle bir görüntü yok."))?;
        let index = (ty * tiles_across(img.width) + tx) as usize;
        let (Some(o), Some(c)) = (
            self.offsets[image].get_mut(index),
            self.counts[image].get_mut(index),
        ) else {
            return Err(RasterError::new("Karo görüntünün dışında."));
        };
        *o = self.pos;
        *c = coded.len() as u64;
        self.pos += coded.len() as u64;
        // Word-aligned next piece.
        let mut out = coded;
        if self.pos % 2 == 1 {
            out.push(0);
            self.pos += 1;
        }
        Ok(out)
    }

    /// The directories to append last, and the header to write over the first one.
    pub fn finish(self) -> Result<(Vec<u8>, Vec<u8>), RasterError> {
        if self.offsets.iter().flatten().any(|&o| o == 0) {
            return Err(RasterError::new("Bazı karolar yazılmadı."));
        }
        let mut out: Vec<u8> = Vec::new();
        let mut starts = Vec::with_capacity(self.images.len());
        let mut nexts = Vec::with_capacity(self.images.len());
        for (k, img) in self.images.iter().enumerate() {
            let at = self.pos + out.len() as u64;
            starts.push(at);
            let dir = directory(
                self.big,
                at,
                &entries(img, k > 0, &self.offsets[k], &self.counts[k]),
            );
            let base = out.len();
            out.extend_from_slice(&dir.bytes);
            nexts.push(base + dir.bytes.len() - dir.next_at_from_end);
            if out.len() % 2 == 1 {
                out.push(0);
            }
        }
        // Each directory names the one after it.
        for (k, &at) in nexts.iter().enumerate() {
            let next = starts.get(k + 1).copied().unwrap_or(0);
            if self.big {
                out[at..at + 8].copy_from_slice(&next.to_le_bytes());
            } else {
                out[at..at + 4].copy_from_slice(&(next as u32).to_le_bytes());
            }
        }
        Ok((out, header(self.big, starts[0])))
    }
}

/// The header: classic (8 bytes) or BigTIFF (16), little endian, naming the first directory.
fn header(big: bool, first: u64) -> Vec<u8> {
    if big {
        let mut h = vec![0x49, 0x49, 43, 0, 8, 0, 0, 0];
        h.extend_from_slice(&first.to_le_bytes());
        h
    } else {
        let mut h = vec![0x49, 0x49, 42, 0];
        h.extend_from_slice(&(first as u32).to_le_bytes());
        h
    }
}

/// A tag's values.
enum Val {
    Short(Vec<u16>),
    Long(Vec<u32>),
    Long8(Vec<u64>),
    Double(Vec<f64>),
    Ascii(String),
}

struct Dir {
    bytes: Vec<u8>,
    /// Where the next directory's offset sits, counted from the end of `bytes`.
    next_at_from_end: usize,
}

fn entries(img: &Image, reduced: bool, offsets: &[u64], counts: &[u64]) -> Vec<(u16, Val)> {
    let bands = img.bands as usize;
    let bits = (img.sample.bytes() * 8) as u16;
    let format: u16 = match img.sample {
        RasterSample::U8 | RasterSample::U16 | RasterSample::U32 => 1,
        RasterSample::I8 | RasterSample::I16 | RasterSample::I32 => 2,
        RasterSample::F32 | RasterSample::F64 => 3,
    };
    let colour = img.bands - u32::from(img.alpha) >= 3;
    let mut e: Vec<(u16, Val)> = vec![
        (254, Val::Long(vec![u32::from(reduced)])),
        (256, Val::Long(vec![img.width])),
        (257, Val::Long(vec![img.height])),
        (258, Val::Short(vec![bits; bands])),
        (259, Val::Short(vec![8])),
        (262, Val::Short(vec![if colour { 2 } else { 1 }])),
        (277, Val::Short(vec![img.bands as u16])),
        (284, Val::Short(vec![1])),
        (322, Val::Short(vec![TILE as u16])),
        (323, Val::Short(vec![TILE as u16])),
    ];
    let big_offsets = offsets
        .iter()
        .chain(counts)
        .any(|&v| v > u64::from(u32::MAX));
    if big_offsets {
        e.push((324, Val::Long8(offsets.to_vec())));
        e.push((325, Val::Long8(counts.to_vec())));
    } else {
        e.push((324, Val::Long(offsets.iter().map(|&v| v as u32).collect())));
        e.push((325, Val::Long(counts.iter().map(|&v| v as u32).collect())));
    }
    // The bands past the colour ones: unspecified, the last alpha when it is.
    let extra = img.bands as usize - if colour { 3 } else { 1 };
    if extra > 0 {
        let mut v = vec![0u16; extra];
        if img.alpha {
            v[extra - 1] = 2;
        }
        e.push((338, Val::Short(v)));
    }
    e.push((339, Val::Short(vec![format; bands])));
    if let Some(g) = &img.geo {
        let [x0, a, b, y0, c, d] = g.affine;
        if b == 0.0 && c == 0.0 && d < 0.0 {
            e.push((33550, Val::Double(vec![a, -d, 0.0])));
            e.push((33922, Val::Double(vec![0.0, 0.0, 0.0, x0, y0, 0.0])));
        } else {
            e.push((
                34264,
                Val::Double(vec![
                    a, b, 0.0, x0, c, d, 0.0, y0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ]),
            ));
        }
        let mut keys: Vec<u16> = vec![1, 1, 0, 0];
        let model: u16 = if g.geographic { 2 } else { 1 };
        keys.extend_from_slice(&[1024, 0, 1, model, 1025, 0, 1, 1]);
        if let Some(code) = g.epsg.and_then(|c| u16::try_from(c).ok()) {
            keys.extend_from_slice(&[if g.geographic { 2048 } else { 3072 }, 0, 1, code]);
        }
        keys[3] = ((keys.len() - 4) / 4) as u16;
        e.push((34735, Val::Short(keys)));
    }
    if let Some(v) = img.nodata {
        let text = if v.is_nan() {
            "nan".to_owned()
        } else {
            format!("{v}")
        };
        e.push((42113, Val::Ascii(text)));
    }
    e.sort_by_key(|(t, _)| *t);
    e
}

fn directory(big: bool, at: u64, entries: &[(u16, Val)]) -> Dir {
    let entry = if big { 20usize } else { 12 };
    let count_size = if big { 8usize } else { 2 };
    let next_size = if big { 8usize } else { 4 };
    let n = entries.len();
    let head = count_size + n * entry + next_size;
    let mut dir: Vec<u8> = Vec::with_capacity(head);
    let mut data: Vec<u8> = Vec::new();
    if big {
        dir.extend_from_slice(&(n as u64).to_le_bytes());
    } else {
        dir.extend_from_slice(&(n as u16).to_le_bytes());
    }
    for (tag, val) in entries {
        let (kind, count, bytes): (u16, u64, Vec<u8>) = match val {
            Val::Short(v) => (
                3,
                v.len() as u64,
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
            Val::Long(v) => (
                4,
                v.len() as u64,
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
            Val::Long8(v) => (
                16,
                v.len() as u64,
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
            Val::Double(v) => (
                12,
                v.len() as u64,
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
            Val::Ascii(s) => {
                let mut b = s.as_bytes().to_vec();
                b.push(0);
                (2, b.len() as u64, b)
            }
        };
        dir.extend_from_slice(&tag.to_le_bytes());
        dir.extend_from_slice(&kind.to_le_bytes());
        let inline = if big { 8 } else { 4 };
        if big {
            dir.extend_from_slice(&count.to_le_bytes());
        } else {
            dir.extend_from_slice(&(count as u32).to_le_bytes());
        }
        if bytes.len() <= inline {
            let mut v = bytes.clone();
            v.resize(inline, 0);
            dir.extend_from_slice(&v);
        } else {
            if data.len() % 2 == 1 {
                data.push(0);
            }
            let where_ = at + head as u64 + data.len() as u64;
            if big {
                dir.extend_from_slice(&where_.to_le_bytes());
            } else {
                dir.extend_from_slice(&(where_ as u32).to_le_bytes());
            }
            data.extend_from_slice(&bytes);
        }
    }
    dir.extend(std::iter::repeat_n(0u8, next_size));
    let next_at = dir.len() - next_size;
    let mut bytes = dir;
    bytes.extend_from_slice(&data);
    Dir {
        next_at_from_end: bytes.len() - next_at,
        bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::super::tiff;
    use super::super::{ByteStore, Step};
    use super::*;

    #[test]
    fn a_written_file_reads_back() {
        let images = vec![
            Image {
                width: 300,
                height: 260,
                bands: 1,
                sample: RasterSample::F32,
                alpha: false,
                geo: Some(Geo {
                    affine: [487000.0, 0.5, 0.0, 4420130.0, 0.0, -0.5],
                    epsg: Some(5256),
                    geographic: false,
                }),
                nodata: Some(-9999.0),
            },
            Image {
                width: 150,
                height: 130,
                bands: 1,
                sample: RasterSample::F32,
                alpha: false,
                geo: None,
                nodata: Some(-9999.0),
            },
        ];
        let (mut w, header) = Writer::new(images, 1).expect("a writer");
        let mut file = header;
        for (img, across, down) in [(0usize, 2u32, 2u32), (1, 1, 1)] {
            for ty in 0..down {
                for tx in 0..across {
                    let mut s = Samples::filled(RasterSample::F32, 256 * 256, 0.0);
                    for k in 0..256 * 256 {
                        s.set(
                            k,
                            (img * 1000 + (ty * across + tx) as usize) as f64 + k as f64 * 0.001,
                        );
                    }
                    file.extend(w.tile(img, tx, ty, &s).expect("a tile"));
                }
            }
        }
        let (dirs, header) = w.finish().expect("finished");
        file.extend(dirs);
        file[..header.len()].copy_from_slice(&header);
        let mut store = ByteStore::new();
        let size = file.len() as u64;
        store.put(0, file);
        let Step::Done(t) = tiff::parse(&store, size).expect("parses") else {
            panic!("all bytes were there");
        };
        assert_eq!(t.ifds.len(), 2);
        assert_eq!(
            (t.ifds[0].width, t.ifds[0].height, t.ifds[0].subfile),
            (300, 260, 0)
        );
        assert_eq!((t.ifds[1].width, t.ifds[1].subfile), (150, 1));
        let geo = super::super::geotiff::read(&t.ifds[0]);
        assert_eq!(geo.affine, Some([487000.0, 0.5, 0.0, 4420130.0, 0.0, -0.5]));
        assert_eq!((geo.epsg, geo.nodata), (Some(5256), Some(-9999.0)));
    }
}
