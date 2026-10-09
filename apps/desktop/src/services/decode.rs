//! A service's answers made into what the drawing shows (docs/adr/0208 §3,
//! §9), on the services' own threads: a picture tile (PNG or JPEG) decoded
//! and cut into the raster atlas's slots by the core
//! (`kentos_services::picture`, as the browser's worker cuts it); a vector
//! tile read once (`mvt`) and drawn by its style at a zoom into a styled
//! layer and label candidates, its vertices through the tile's mesh.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::tiles::{MeshMap, TileRef, cells_of};
use kentos_native_style::batches::{DecodeOptions, StyledLayer, decode};
use kentos_native_style::color::{StylePalette, ViewColors};
use kentos_native_style::library::StyleLibrary;
use kentos_render_wgpu::styled::Picture;
use kentos_render_wgpu::styled::raster_tiles::SLOT;
use kentos_render_wgpu::styled::{ServiceImage, ServiceTile, ServiceView, StyledLayerPart};
use kentos_services::labels::Candidate;
use kentos_services::mvt;
use kentos_services::style::Style;
use kentos_services::style::build::{TileInput, build};

pub use kentos_services::picture::slots_across;

// The core's slot is the renderer's.
const _: () = assert!(kentos_services::picture::SLOT == SLOT);

/// A picture tile's bytes as the atlas takes them, cut as a tile of `want`
/// pixels is (the grid's); empty when nothing of it shows.
pub fn picture(bytes: &[u8], want: Option<(u32, u32)>) -> Result<ServiceTile, String> {
    let read = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        crate::style::images::png(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        crate::style::images::jpeg(bytes)
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Err("Karo WebP biçiminde; KentOS şimdilik PNG ve JPEG karoları gösterir. Servisten PNG ya da JPEG isteyin.".into());
    } else {
        None
    };
    let Some(Picture::Bitmap {
        width,
        height,
        rgba,
    }) = read
    else {
        return Err("Karo bir PNG ya da JPEG resmi değil; servisin cevabı okunamadı.".into());
    };
    Ok(slots(width as usize, height as usize, rgba, want))
}

/// `rgba` (straight alpha) cut into premultiplied slots (`kentos_services::picture`).
pub fn slots(width: usize, height: usize, rgba: Vec<u8>, want: Option<(u32, u32)>) -> ServiceTile {
    match kentos_services::picture::slots(width, height, rgba, want) {
        None => ServiceTile::Empty,
        Some(cut) => ServiceTile::Image(Arc::new(ServiceImage {
            across: cut.across,
            down: cut.down,
            slots: cut.slots.into_iter().map(Arc::new).collect(),
        })),
    }
}

/// A vector tile's layers, gzipped or not.
pub fn vector_layers(bytes: &[u8]) -> Result<Vec<mvt::Layer>, String> {
    let raw = mvt::inflate(bytes)?;
    mvt::read(&raw)
}

/// What a vector tile is drawn with besides its features: the drawing's
/// anchor (its batches' origin, docs/adr/0157) and the colours the drawing
/// is drawn with now.
#[derive(Clone)]
pub struct VectorLook {
    pub origin: Vec2,
    pub palette: StylePalette,
    pub view: ViewColors,
}

impl VectorLook {
    /// What tells one look from another: a tile built with one is built again with another.
    pub fn key(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.origin.x.to_bits().hash(&mut h);
        self.origin.y.to_bits().hash(&mut h);
        self.palette.fg.hash(&mut h);
        self.palette.paper.hash(&mut h);
        format!("{:?}", self.view).hash(&mut h);
        h.finish()
    }
}

/// A styled layer's id the renderer has never seen.
static NEXT_TILE: AtomicU64 = AtomicU64::new(1 << 62);

/// A vector tile drawn by `style` at `zoom`: its styled layer and label candidates.
pub fn vector_tile(
    layers: &[mvt::Layer],
    style: &Style,
    source: &str,
    zoom: f64,
    view: &ServiceView,
    t: TileRef,
    look: &VectorLook,
) -> (Arc<StyledLayerPart>, Vec<Candidate>) {
    // The vertices through the tile's mesh, as a picture tile is drawn (docs/adr/0208 §9): one
    // transformation a node, not one a vertex.
    let n = cells_of(
        &view.grid,
        t.level,
        view.same_system,
        view.metres_per_unit,
        1,
    );
    let map = MeshMap::new(&view.grid, t, n, |x, y| (view.to_project)(x, y));
    let to_project = |u: f64, v: f64| map.at(u, v).map(|(x, y)| Vec2 { x, y });
    let id = {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        (view.id, t.level, t.col, t.row).hash(&mut h);
        h.finish()
    };
    let out = build(&TileInput {
        layers,
        style,
        source,
        zoom,
        to_project: &to_project,
        origin: look.origin,
        tile: id,
    });
    static EMPTY: std::sync::OnceLock<StyleLibrary> = std::sync::OnceLock::new();
    let layer = decode(
        out.batches,
        &DecodeOptions {
            palette: &look.palette,
            plot_scale: 1.0,
            library: EMPTY.get_or_init(StyleLibrary::default),
            view: look.view,
        },
    )
    .unwrap_or_else(|_| StyledLayer::default());
    (
        Arc::new(StyledLayerPart {
            id: NEXT_TILE.fetch_add(1, Ordering::Relaxed),
            layer,
        }),
        out.labels,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webp_and_other_bytes_say_why() {
        assert!(
            picture(b"RIFF\0\0\0\0WEBPVP8 ", None)
                .err()
                .is_some_and(|e| e.contains("WebP"))
        );
        assert!(
            picture(b"<html>", None)
                .err()
                .is_some_and(|e| e.contains("PNG ya da JPEG"))
        );
    }
}
