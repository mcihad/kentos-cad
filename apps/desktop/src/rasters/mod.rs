//! Rasters in the drawing (docs/adr/0204): an orthophoto, a scanned sheet or
//! an elevation model whose file is linked (its path) or embedded (the
//! project's library keeps its bytes, named by their content). What
//! Öznitelikler says of a raster's source and Göm come from here; the
//! raster pass's tiles from `tiles`, read and coloured off the interface's
//! thread.

use std::path::Path;
use std::sync::Arc;

use iced::Task;
use kentos_contracts::{Entity, RasterFields};
use kentos_domain::{Document as Model, Slot};
use serde_json::{Value, json};

pub mod add;
pub mod jobs;
pub mod look;
pub mod multidim;
pub mod pyramid;
#[cfg(test)]
mod tests;
pub mod tiles;

/// The largest file a raster is embedded from (docs/adr/0204 §2).
pub const MOST_EMBEDDED: usize = 32 * 1024 * 1024;

/// A raster's id in the project's library: `raster-` and its content's
/// SHA-256's first sixteen hex digits, so the same file is kept once.
pub fn raster_id(bytes: &[u8]) -> String {
    let hex = kentos_sheet::template::sha256_hex(bytes);
    format!("raster-{}", &hex[..16])
}

/// The library's format of a raster file: `tiff`, `png`, `jpeg` or `netcdf`
/// (a classic NetCDF, docs/adr/0243); none for another.
pub fn format_of(bytes: &[u8]) -> Option<&'static str> {
    if matches!(
        kentos_formats::multidim::netcdf::sniff(bytes),
        kentos_formats::multidim::netcdf::Sniff::Classic(_)
    ) {
        Some("netcdf")
    } else if kentos_formats::raster::tiff::sniff(bytes) {
        Some("tiff")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        Some("jpeg")
    } else {
        None
    }
}

/// A raster file as an item of the project's library: its id from its
/// content, its name the file's stem, under Rasterler, its size in pixels;
/// why not when it is too large or no raster.
pub fn library_item(
    name: &str,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<(String, Value), String> {
    if bytes.len() > MOST_EMBEDDED {
        return Err(format!(
            "“{name}” {} MB; gömülü raster en çok {} MB olabilir. Rasteri bağlı bırakın.",
            bytes.len() / (1024 * 1024),
            MOST_EMBEDDED / (1024 * 1024)
        ));
    }
    let Some(format) = format_of(bytes) else {
        return Err(format!(
            "“{name}” GeoTIFF, PNG, JPEG ya da NetCDF değil; gömülemez."
        ));
    };
    let id = raster_id(bytes);
    let stem = Path::new(name)
        .file_stem()
        .map_or_else(|| name.to_owned(), |s| s.to_string_lossy().into_owned());
    let item = json!({
        "kind": "asset",
        "id": id,
        "name": stem,
        "path": ["Rasterler"],
        "format": format,
        "data": format!(
            "data:{};base64,{}",
            if format == "netcdf" { "application/x-netcdf".to_owned() } else { format!("image/{format}") },
            kentos_sheet::template::base64_encode(bytes)
        ),
        "width": width,
        "height": height,
    });
    Ok((id, item))
}

/// Adds the raster to the project's library unless it is there (an edit, no undo step; docs/adr/0092).
pub fn keep_in_library(model: &mut Model, id: &str, item: Value) {
    if kentos_native_application::edit::has_raster(model, id) {
        return;
    }
    let mut styles = model.styles().clone();
    styles.items.push(item);
    model.set_styles(styles);
}

/// An embedded raster's bytes from the project's library.
pub fn asset_bytes(model: &Model, id: &str) -> Option<Vec<u8>> {
    let item = model.styles().items.iter().find(|it| {
        it.get("kind").and_then(Value::as_str) == Some("asset")
            && it.get("id").and_then(Value::as_str) == Some(id)
    })?;
    let url = item.get("data").and_then(Value::as_str)?;
    let (_, data) = url.split_once(";base64,")?;
    kentos_sheet::template::base64_decode(data)
}

/// Göm: the linked raster at `slot` read and kept in the project's library,
/// the raster made embedded in one step “Değiştir”; why not otherwise.
pub fn embed(model: &mut Model, slot: Slot, folder: Option<&Path>) -> Vec<String> {
    let Some(Entity::Raster(r)) = model.get(slot).cloned() else {
        return Vec::new();
    };
    let Some(file) = r.raster.file.clone() else {
        return Vec::new();
    };
    let path = crate::pictures::resolve(&file, folder);
    let name = path
        .file_name()
        .map_or_else(|| file.clone(), |n| n.to_string_lossy().into_owned());
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size > MOST_EMBEDDED as u64 {
        return vec![format!(
            "“{name}” {} MB; gömülü raster en çok {} MB olabilir. Rasteri bağlı bırakın.",
            size / (1024 * 1024),
            MOST_EMBEDDED / (1024 * 1024)
        )];
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return vec![format!(
                "“{name}” okunamadı: {e}. Dosyanın yerini denetleyin."
            )];
        }
    };
    let (id, item) = match library_item(&name, &bytes, r.raster.width, r.raster.height) {
        Ok(it) => it,
        Err(why) => return vec![why],
    };
    keep_in_library(model, &id, item);
    let mut raster = r.clone();
    raster.raster.asset = Some(id);
    raster.raster.file = None;
    kentos_interaction::properties::set_geometry(model, slot, &Entity::Raster(raster))
}

/// A raster's key split at its dataset (docs/adr/0243 §5): the file's part and
/// what of the NetCDF file it shows (the style core's `raster_key`), when any.
pub fn split_key(key: &str) -> (&str, Option<kentos_formats::multidim::cube::Part>) {
    kentos_formats::multidim::cube::split_key(key)
}

/// What of a NetCDF file a raster shows (docs/adr/0243 §5): its dataset's
/// variable and slice, a mesh's sanal grid; none for another raster.
pub fn part_of(r: &RasterFields) -> Option<kentos_formats::multidim::cube::Part> {
    kentos_formats::multidim::cube::Part::of(r)
}

/// Where the scene's raster `key` (`file:<path>`, `asset:<id>`, `url:<address>`)
/// reads its bytes: a linked file beside the drawing (`folder`), an embedded
/// one's from the library, an address by HTTP ranges (docs/adr/0207 §1).
pub fn origin_of(key: &str, model: &Model, folder: Option<&Path>) -> Option<tiles::Origin> {
    let (key, _) = split_key(key);
    if let Some(file) = key.strip_prefix("file:") {
        Some(tiles::Origin::File(crate::pictures::resolve(file, folder)))
    } else if let Some(id) = key.strip_prefix("asset:") {
        asset_bytes(model, id).map(|b| tiles::Origin::Bytes(Arc::new(b)))
    } else {
        key.strip_prefix("url:")
            .map(|u| tiles::Origin::Url(u.to_owned()))
    }
}

/// The scene's name of a raster's tiles (the style core's `raster_key`, the
/// slider closed): `asset:<id>`, `file:<path>` or `url:<address>`, and a
/// NetCDF variable's slice after `#` (docs/adr/0243 §5).
pub fn key_of(r: &RasterFields) -> String {
    kentos_native_style::raster_key(&kentos_native_application::geometry::core_raster(r), None)
        .unwrap_or_default()
}

/// The raster windows' state while the app runs.
#[derive(Debug, Default)]
pub struct Windows {
    pub add: Option<add::State>,
    pub look: Option<look::State>,
    /// Mesh ekle, and Raster ekle's NetCDF (docs/adr/0243 §11).
    pub multidim: Option<multidim::State>,
}

/// What the raster windows and the pyramids' panel ask for.
#[derive(Debug, Clone)]
pub enum Event {
    Add(add::Event),
    Look(look::Event),
    Multidim(multidim::Event),
    /// Durdur on a pyramid's line, by its raster's key.
    StopPyramid(String),
    /// Durdur on Raster oturt's resampling.
    StopWarp,
    /// Koordinat oku's values of the rasters under a point: the layer's name and the bands' values.
    Values(Vec<(String, Vec<f64>)>),
}

/// Runs `work` on a thread of its own and brings its message back.
pub(crate) fn off_thread(
    work: impl FnOnce() -> crate::app::Message + Send + 'static,
) -> Task<crate::app::Message> {
    iced_runtime::task::blocking(
        move |mut out: iced::futures::channel::mpsc::Sender<crate::app::Message>| {
            let answer = work();
            let _ =
                iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, answer));
        },
    )
}

impl crate::app::App {
    pub(crate) fn rasters_event(&mut self, e: Event) -> Task<crate::app::Message> {
        match e {
            Event::Add(e) => self.raster_add_event(e),
            Event::Look(e) => self.raster_look_event(e),
            Event::Multidim(e) => self.multidim_event(e),
            Event::StopPyramid(key) => {
                pyramid::stop(&key);
                Task::none()
            }
            Event::StopWarp => {
                jobs::stop_warp();
                Task::none()
            }
            Event::Values(found) => {
                for (layer, values) in found {
                    let text = values
                        .iter()
                        .map(|&v| crate::crs::js_number(v))
                        .collect::<Vec<_>>()
                        .join("; ");
                    self.say(
                        kentos_interaction::Level::Info,
                        format!("Raster {layer}: {text}"),
                    );
                }
                Task::none()
            }
        }
    }

    /// Koordinat oku's points: their rasters' values read off the thread.
    pub(crate) fn raster_values_tasks(&mut self) -> Task<crate::app::Message> {
        let points = std::mem::take(&mut self.raster_values_wanted);
        Task::batch(
            points
                .into_iter()
                .map(|p| self.raster_values_at(p))
                .collect::<Vec<_>>(),
        )
    }

    /// Koordinat oku at `p`: the values of the shown rasters under it, read off the thread.
    pub(crate) fn raster_values_at(
        &mut self,
        p: kentos_interaction::Vec2,
    ) -> Task<crate::app::Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let folder = doc
            .path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        let layers = doc.model.layers();
        let mut wanted = Vec::new();
        for e in doc.model.entities() {
            let Entity::Raster(r) = e else {
                continue;
            };
            if !layers.is_visible(&r.base.layer_id) {
                continue;
            }
            let x = &r.raster;
            let Some(q) = kentos_geometry_core::geom::raster::pixel_of(
                x.affine,
                kentos_geometry_core::vec2::Vec2::new(p.x, p.y),
            ) else {
                continue;
            };
            if !(q.x >= 0.0 && q.y >= 0.0 && q.x < f64::from(x.width) && q.y < f64::from(x.height))
            {
                continue;
            }
            let key = key_of(x);
            if let Some(origin) = origin_of(&key, &doc.model, folder.as_deref()) {
                wanted.push((
                    self.layer_name(&r.base.layer_id),
                    key,
                    origin,
                    q.x.floor() as i64,
                    q.y.floor() as i64,
                ));
            }
        }
        if wanted.is_empty() {
            return Task::none();
        }
        off_thread(move || {
            let found = wanted
                .into_iter()
                .filter_map(|(layer, key, origin, i, j)| {
                    tiles::values_at(&key, origin, i, j).map(|v| (layer, v))
                })
                .collect();
            crate::app::Message::Rasters(Event::Values(found))
        })
    }
}
