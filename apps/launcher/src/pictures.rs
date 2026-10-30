//! The plates' pictures (docs/adr/0803 §3): the owner's pictures of each
//! program, those still to come too (30 October 2026), whole at 16:9 and
//! scaled to about twice a plate's size by `scripts/launcher/pictures.py --fit`
//! (800 × 450, 256-colour PNG), bundled with the launcher. A manifest names
//! one (`"picture": "mimari"`) or gives a PNG of its own. Pictures are
//! decoded once and kept as textures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iced::widget::image::Handle;

/// A bundled picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shot {
    /// A floor plan of two flats around a central core.
    Mimari,
    /// A zoning plan: blocks with their uses and floor-area ratios, parks, roads.
    Harita,
    /// A city on its aerial image with parcels, roads and a highlighted area.
    Cbs,
    /// A road: its plan, cross-sections and long profile.
    Yol,
    /// A city at dusk in 3D: its blocks, a river and its bridges, the digital
    /// twin's sensors marked on it (KentOS 3D, still to come).
    Model3d,
    /// A town's hazards on its map: a fault and a quake's epicentre with the
    /// blocks it struck, the river's flood, a landslide, a forest fire, the
    /// assembly points (KentOS Afet, still to come).
    Afet,
}

impl Shot {
    pub const ALL: [Shot; 6] = [
        Shot::Mimari,
        Shot::Harita,
        Shot::Cbs,
        Shot::Yol,
        Shot::Model3d,
        Shot::Afet,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Shot::Mimari => "mimari",
            Shot::Harita => "harita",
            Shot::Cbs => "cbs",
            Shot::Yol => "yol",
            Shot::Model3d => "3d",
            Shot::Afet => "afet",
        }
    }

    pub fn parse(key: &str) -> Option<Shot> {
        Shot::ALL.into_iter().find(|s| s.key() == key)
    }

    pub fn known() -> String {
        Shot::ALL.map(Shot::key).join(", ")
    }

    /// Its PNG.
    pub const fn bytes(self) -> &'static [u8] {
        match self {
            Shot::Mimari => include_bytes!("../assets/pictures/mimari.png"),
            Shot::Harita => include_bytes!("../assets/pictures/harita.png"),
            Shot::Cbs => include_bytes!("../assets/pictures/cbs.png"),
            Shot::Yol => include_bytes!("../assets/pictures/yol.png"),
            Shot::Model3d => include_bytes!("../assets/pictures/3d.png"),
            Shot::Afet => include_bytes!("../assets/pictures/afet.png"),
        }
    }
}

/// A decoded picture: its pixels, RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The largest picture a manifest may give (a card shows it a few hundred pixels wide).
const MOST: u32 = 4096;

/// Decodes a PNG of any colour type and depth to 8-bit RGBA.
pub fn decode(bytes: &[u8]) -> Result<Pixels, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG okunamadı ({e})"))?;
    let (width, height) = {
        let info = reader.info();
        (info.width, info.height)
    };
    if width == 0 || height == 0 || width > MOST || height > MOST {
        return Err(format!(
            "resim {width} × {height}; en çok {MOST} × {MOST} piksel olur"
        ));
    }
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "PNG'nin boyu okunamadı".to_owned())?;
    let mut buffer = vec![0; size];
    let frame = reader
        .next_frame(&mut buffer)
        .map_err(|e| format!("PNG okunamadı ({e})"))?;
    buffer.truncate(frame.buffer_size());
    let pixels = (width as usize) * (height as usize);
    let rgba = match frame.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buffer
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buffer.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("PNG'nin paleti açılamadı".to_owned()),
    };
    if rgba.len() != pixels * 4 {
        return Err("PNG'nin pikselleri eksik".to_owned());
    }
    Ok(Pixels {
        width,
        height,
        rgba,
    })
}

/// The pictures decoded, by what they are.
#[derive(Debug, Default)]
pub struct Pictures {
    shots: HashMap<Shot, Handle>,
    files: HashMap<PathBuf, Result<Handle, String>>,
}

impl Pictures {
    /// A bundled screenshot's texture (decoded the first time it is asked for).
    pub fn shot(&mut self, shot: Shot) -> Option<Handle> {
        if let Some(handle) = self.shots.get(&shot) {
            return Some(handle.clone());
        }
        let pixels = decode(shot.bytes()).ok()?;
        let handle = Handle::from_rgba(pixels.width, pixels.height, pixels.rgba);
        self.shots.insert(shot, handle.clone());
        Some(handle)
    }

    /// A manifest's own PNG: read and decoded once; a refusal says why.
    pub fn file(&mut self, path: &Path) -> Result<Handle, String> {
        if let Some(result) = self.files.get(path) {
            return result.clone();
        }
        let result = std::fs::read(path)
            .map_err(|e| format!("“{}” okunamadı ({e})", path.display()))
            .and_then(|bytes| decode(&bytes).map_err(|e| format!("“{}”: {e}", path.display())))
            .map(|p| Handle::from_rgba(p.width, p.height, p.rgba));
        self.files.insert(path.to_path_buf(), result.clone());
        result
    }

    /// The textures already made: what the view draws (it decodes nothing).
    pub fn ready(&self, shot: Shot) -> Option<&Handle> {
        self.shots.get(&shot)
    }

    pub fn ready_file(&self, path: &Path) -> Option<&Handle> {
        self.files.get(path).and_then(|r| r.as_ref().ok())
    }

    /// Forgets the files' pictures: they are read again (after Yenile).
    pub fn forget_files(&mut self) {
        self.files.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_picture_is_a_plate_at_about_twice_its_size_and_small() {
        let mut total = 0;
        for shot in Shot::ALL {
            let pixels = decode(shot.bytes()).expect("decodes");
            assert_eq!((pixels.width, pixels.height), (800, 450), "{shot:?}");
            assert_eq!(Shot::parse(shot.key()), Some(shot));
            total += shot.bytes().len();
        }
        assert!(total <= 2_000_000, "the pictures weigh {total} bytes");
    }

    #[test]
    fn colour_types_become_rgba_and_broken_files_are_refused() {
        let encode = |color: png::ColorType, data: &[u8]| {
            let mut bytes = Vec::new();
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(data).expect("data");
            drop(writer);
            bytes
        };
        let gray = decode(&encode(png::ColorType::Grayscale, &[10, 200])).expect("gray");
        assert_eq!(gray.rgba, [10, 10, 10, 255, 200, 200, 200, 255]);
        let rgb = decode(&encode(png::ColorType::Rgb, &[1, 2, 3, 4, 5, 6])).expect("rgb");
        assert_eq!(rgb.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
        assert!(
            decode(b"not a png")
                .expect_err("refused")
                .contains("PNG okunamadı")
        );
        let mut pictures = Pictures::default();
        let missing = pictures
            .file(Path::new("/yok/resim.png"))
            .expect_err("missing");
        assert!(missing.contains("okunamadı"), "{missing}");
    }
}
