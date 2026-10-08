//! Point clouds in the drawing (docs/adr/0207): LAS, LAZ, COPC and text
//! clouds, one file or many (a virtual cloud), each linked (its path), read
//! from an address by ranges, or embedded (the project's library keeps its
//! bytes, named by their content). The scene's clouds are opened and their
//! nodes read and coloured off the interface's thread (`service`); a cloud
//! that is not a COPC is indexed once into the device's cache (`index`).
//! Nokta bulutu ekle (`add`), Nokta bulutu stili (`look`), XYZ sor's reading
//! (`query`) and Sanal bulut olarak kaydet (`vpc`) are here.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use iced::Task;
use iced::futures::Stream;
use kentos_contracts::{CloudFormat, CloudSource, Entity, PointCloudFields};
use kentos_domain::{Document as Model, Slot};
use serde_json::{Value, json};

use bytes::Origin;

pub mod add;
pub mod bytes;
pub mod files;
pub mod index;
pub mod look;
#[cfg(test)]
mod ops_tests;
pub mod query;
pub mod service;
#[cfg(test)]
mod tests;
pub mod vpc;

/// The largest file a cloud is embedded from (docs/adr/0207 §3).
pub const MOST_EMBEDDED: usize = 32 * 1024 * 1024;

/// A cloud file's id in the project's library: `pointcloud-` and its
/// content's SHA-256's first sixteen hex digits, so the same file is kept once.
pub fn cloud_id(bytes: &[u8]) -> String {
    let hex = kentos_sheet::template::sha256_hex(bytes);
    format!("pointcloud-{}", &hex[..16])
}

/// A cloud file as an item of the project's library: its id from its
/// content, its name the file's stem, under Nokta bulutları, its format;
/// why not when it is too large.
pub fn library_item(
    name: &str,
    bytes: &[u8],
    format: CloudFormat,
) -> Result<(String, Value), String> {
    if bytes.len() > MOST_EMBEDDED {
        return Err(format!(
            "“{name}” {} MB; gömülü nokta bulutu en çok {} MB olabilir. Bulutu bağlı bırakın.",
            bytes.len() / (1024 * 1024),
            MOST_EMBEDDED / (1024 * 1024)
        ));
    }
    let id = cloud_id(bytes);
    let stem = file_stem(name);
    let item = json!({
        "kind": "asset",
        "id": id,
        "name": stem,
        "path": ["Nokta bulutları"],
        "format": format.name(),
        "data": format!(
            "data:application/octet-stream;base64,{}",
            kentos_sheet::template::base64_encode(bytes)
        ),
    });
    Ok((id, item))
}

/// A file's name without its cloud extensions (`a.copc.laz` → `a`).
pub fn file_stem(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let base = base.split(['?', '#']).next().unwrap_or(base);
    let lower = base.to_ascii_lowercase();
    let cut = [
        ".copc.laz",
        ".laz",
        ".las",
        ".xyz",
        ".pts",
        ".txt",
        ".csv",
        ".vpc",
    ]
    .iter()
    .find(|e| lower.ends_with(*e))
    .map_or(base.len(), |e| base.len() - e.len());
    let stem = &base[..cut];
    if stem.is_empty() {
        base.to_owned()
    } else {
        stem.to_owned()
    }
}

/// Adds the cloud to the project's library unless it is there (an edit, no undo step; docs/adr/0092).
pub fn keep_in_library(model: &mut Model, id: &str, item: Value) {
    if kentos_native_application::edit::has_cloud(model, id) {
        return;
    }
    let mut styles = model.styles().clone();
    styles.items.push(item);
    model.set_styles(styles);
}

/// The scene's name of a cloud's files (`geom::pointcloud::cloud_key`): as
/// the style core names the paint, from the files' JSON as the core carries it.
pub fn key_of(c: &PointCloudFields) -> String {
    use kentos_geometry_core::api::json::Json;
    let sources =
        Json::parse(&kentos_contracts::sources_json_text(&c.sources)).unwrap_or(Json::Null);
    kentos_geometry_core::geom::pointcloud::cloud_key(&sources)
}

/// A member's name as the windows say it: its file's, its address's last
/// part, or the library's name of an embedded one.
pub fn source_name(s: &CloudSource, model: Option<&Model>) -> String {
    if let Some(f) = &s.file {
        return f.rsplit(['/', '\\']).next().unwrap_or(f).to_owned();
    }
    if let Some(u) = &s.url {
        let path = u.split(['?', '#']).next().unwrap_or(u);
        let last = path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(path);
        return if last.is_empty() {
            u.clone()
        } else {
            last.to_owned()
        };
    }
    if let Some(a) = &s.asset {
        let named = model.and_then(|m| {
            m.styles().items.iter().find_map(|it| {
                (it.get("kind").and_then(Value::as_str) == Some("asset")
                    && it.get("id").and_then(Value::as_str) == Some(a))
                .then(|| it.get("name").and_then(Value::as_str).map(str::to_owned))
                .flatten()
            })
        });
        return named.unwrap_or_else(|| a.clone());
    }
    String::new()
}

/// A cloud's files as the service opens them: where each is read from, its
/// format, its name, and an embedded one's bytes from the project's library.
pub fn files_of(
    c: &PointCloudFields,
    model: &Model,
    folder: Option<&Path>,
) -> Vec<service::MemberFile> {
    c.sources
        .iter()
        .map(|s| {
            let name = source_name(s, Some(model));
            let (origin, embedded) = match (&s.asset, &s.file, &s.url) {
                (Some(a), _, _) => (
                    Origin::Asset(a.clone()),
                    crate::rasters::asset_bytes(model, a).map(Arc::new),
                ),
                (None, Some(f), _) => (Origin::File(crate::pictures::resolve(f, folder)), None),
                (None, None, Some(u)) => (Origin::Url(u.clone()), None),
                (None, None, None) => (Origin::Asset(String::new()), None),
            };
            (origin, s.format, name, embedded)
        })
        .collect()
}

/// Registers the scene's clouds (`keys`, from its points paints) with the
/// service, and lets go of the others: a cloud is registered once, its
/// files then opened off the interface's thread.
pub fn register_scene(keys: &HashSet<String>, model: &Model, folder: Option<&Path>) {
    let s = service::service();
    let missing: Vec<&String> = keys.iter().filter(|k| s.entry(k).is_none()).collect();
    if !missing.is_empty() {
        for e in model.entities() {
            let Entity::PointCloud(c) = e else {
                continue;
            };
            let key = key_of(&c.cloud);
            if missing.contains(&&key) && s.entry(&key).is_none() {
                let z = [c.cloud.bounds[2], c.cloud.bounds[5]];
                s.register(&key, || files_of(&c.cloud, model, folder), z);
            }
        }
    }
    if service::in_use() {
        s.keep_only(keys);
    }
}

/// How the index of the cloud `c` stands, as Öznitelikler's Dizin says it.
pub fn index_words(c: &PointCloudFields) -> String {
    let Some(entry) = service::service().entry(&key_of(c)) else {
        return if c.sources.iter().all(|s| s.format == CloudFormat::Copc) {
            "Dosyanın kendi dizini (COPC)".to_owned()
        } else {
            "Çizimde gösterilince hazırlanır".to_owned()
        };
    };
    let mut ready = 0;
    let mut indexing: Vec<f64> = Vec::new();
    let mut stopped = 0;
    let mut failed: Option<String> = None;
    for m in &entry.members {
        match m.state() {
            service::State::Ready(_) => ready += 1,
            service::State::Indexing(d) => indexing.push(d),
            service::State::Waiting => indexing.push(0.0),
            service::State::Stopped => stopped += 1,
            service::State::Failed(why) => {
                failed.get_or_insert(why);
            }
        }
    }
    let n = entry.members.len();
    if let Some(why) = failed {
        return format!("Açılamadı: {why}");
    }
    if !indexing.is_empty() {
        let done = indexing.iter().sum::<f64>() / indexing.len() as f64;
        return format!(
            "Hazırlanıyor, %{}{}",
            (done * 100.0).round(),
            if n > 1 {
                format!(" ({ready}/{n} dosya hazır)")
            } else {
                String::new()
            }
        );
    }
    if stopped > 0 {
        return if n > 1 {
            format!("{stopped} dosyanınki durduruldu; yalnız çerçevesi çizilir")
        } else {
            "Durduruldu; yalnız çerçevesi çizilir".to_owned()
        };
    }
    if c.sources.iter().all(|s| s.format == CloudFormat::Copc) {
        "Hazır (dosyanın kendi dizini, COPC)".to_owned()
    } else {
        "Hazır (önbellekte)".to_owned()
    }
}

/// The nodes made and the files opened as they come: a message for each
/// burst, at most one a frame (docs/adr/0207 §6).
pub fn ready() -> impl Stream<Item = crate::app::Message> {
    let (mut out, stream) = iced::futures::channel::mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            service::service().wait_fresh();
            if iced::futures::executor::block_on(iced::futures::SinkExt::send(
                &mut out,
                crate::app::Message::CloudsReady,
            ))
            .is_err()
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
    stream
}

/// The point cloud windows' state while the app runs.
#[derive(Debug, Default)]
pub struct Windows {
    pub add: Option<add::State>,
    pub look: Option<look::State>,
}

/// What the point cloud windows, XYZ sor and the indexes' panel ask for.
#[derive(Debug, Clone)]
pub enum Event {
    Add(add::Event),
    Look(look::Event),
    /// Durdur on an index's line, by its key.
    StopIndex(String),
    /// XYZ sor's answers, read off the thread: a line each, and the point found.
    Queried(Vec<String>, Option<[f64; 3]>),
    /// Sanal bulut olarak kaydet's file, or none (the dialog cancelled).
    VpcSave(Option<std::path::PathBuf>),
    /// Its writing's end: the file, or why not.
    VpcSaved(Result<std::path::PathBuf, String>),
}

impl crate::app::App {
    pub(crate) fn pointclouds_event(&mut self, e: Event) -> Task<crate::app::Message> {
        match e {
            Event::Add(e) => self.cloud_add_event(e),
            Event::Look(e) => self.cloud_look_event(e),
            Event::StopIndex(key) => {
                index::stop(&key);
                Task::none()
            }
            Event::Queried(lines, found) => {
                self.cloud_queried(lines, found);
                Task::none()
            }
            Event::VpcSave(path) => self.cloud_vpc_write(path),
            Event::VpcSaved(result) => {
                match result {
                    Ok(path) => self.say(
                        kentos_interaction::Level::Success,
                        format!("Sanal bulut “{}” olarak kaydedildi.", path.display()),
                    ),
                    Err(why) => self.warn(why),
                }
                Task::none()
            }
        }
    }

    /// The selected point clouds (or the one a menu was opened over), each with its slot.
    pub(crate) fn selected_clouds(&self) -> Vec<(Slot, PointCloudFields, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        self.selection
            .ids()
            .iter()
            .filter_map(|&s| match doc.model.get(s) {
                Some(Entity::PointCloud(c)) => Some((s, c.cloud.clone(), c.base.layer_id.clone())),
                _ => None,
            })
            .collect()
    }
}
