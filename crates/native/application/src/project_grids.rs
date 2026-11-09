//! The grids a project names found beside its file (plan/map-cad.md MAP-17, docs/adr/3580). A delivery package carries
//! them as `izgaralar/<SHA-256>.<gsb|gtx|tif>` beside its project file (docs/adr/3180). Opening a project from a file,
//! each grid it names (a datum choice's NTv2 grid, the heights' geoid model, a survey campaign's velocity grid) that
//! the core does not hold and this device's library does not keep is read from that folder into the core, its SHA-256
//! checked; the commands then use it as they would the library's. The device's library is not written: taking a
//! grid into it stays Izgaralar's choice (the user's machine changes only by the user's hand).

use std::path::{Path, PathBuf};

use kentos_commands::ProjectSettings;
use kentos_domain::Document;
use kentos_geometry_core::crs::{geoid, ntv2};

use crate::crs_models::{self, ModelKind};

/// The folder of grids beside a project file: `izgaralar/`, as a delivery package writes it.
pub const FOLDER: &str = "izgaralar";

/// What a named grid is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridKind {
    /// A datum choice's NTv2 grid.
    Ntv2,
    Geoid,
    Velocity,
}

impl GridKind {
    pub fn name(self) -> &'static str {
        match self {
            GridKind::Ntv2 => "ntv2",
            GridKind::Geoid => "geoid",
            GridKind::Velocity => "velocity",
        }
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            GridKind::Ntv2 => &["gsb"],
            GridKind::Geoid => &["gtx", "tif"],
            GridKind::Velocity => &["tif", "gtx"],
        }
    }
}

/// Where a named grid was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridPlace {
    /// Read into the core already (this run).
    Core,
    /// This device's library keeps it (read when a command asks).
    Library,
    /// Read from the project's `izgaralar/` into the core now.
    Package,
    /// Neither holds it.
    Missing,
    /// The folder's file is not the one its name says (its SHA-256 differs): not used.
    Changed,
    /// The folder's file is not read as that kind of grid: why.
    Unread(String),
}

impl GridPlace {
    pub fn name(&self) -> &'static str {
        match self {
            GridPlace::Core => "core",
            GridPlace::Library => "library",
            GridPlace::Package => "package",
            GridPlace::Missing => "missing",
            GridPlace::Changed => "changed",
            GridPlace::Unread(_) => "unread",
        }
    }
}

/// A grid the project names and where it was found.
#[derive(Clone, Debug, PartialEq)]
pub struct GridFound {
    pub sha256: String,
    pub name: String,
    pub kind: GridKind,
    /// The setting naming it first (`heights.geoid`, `datumTransforms[0].grid` …).
    pub setting: String,
    pub place: GridPlace,
}

/// The grids `settings` names, each SHA-256 once (its first setting), in the settings' order: the datum choices', the
/// heights' geoid, the campaigns' velocity grids. A setting without a SHA-256 names no file and is left out.
pub fn named(settings: &ProjectSettings) -> Vec<(String, String, GridKind, String)> {
    let mut out: Vec<(String, String, GridKind, String)> = Vec::new();
    let mut put = |id: &str, name: &str, kind, at: String| {
        if crs_models::is_sha256(id) && !out.iter().any(|(x, ..)| x == id) {
            out.push((id.to_owned(), name.to_owned(), kind, at));
        }
    };
    for (i, t) in settings.datum_transforms.iter().enumerate() {
        if let Some(g) = &t.grid {
            put(&g.id, &g.file, GridKind::Ntv2, format!("datumTransforms[{i}].grid"));
        }
    }
    if let Some(g) = settings.heights.as_ref().and_then(|h| h.geoid.as_ref())
        && let Some(id) = &g.sha256
    {
        put(id, &g.name, GridKind::Geoid, "heights.geoid".into());
    }
    if let Some(s) = &settings.map_survey {
        for (i, c) in s.campaigns.iter().enumerate() {
            if let Some(v) = &c.velocity {
                put(&v.sha256, &v.name, GridKind::Velocity, format!("mapSurvey.campaigns[{i}].velocity"));
            }
        }
    }
    out
}

/// Whether the core holds grid `id` of `kind` already.
fn in_core(id: &str, kind: GridKind) -> bool {
    match kind {
        GridKind::Ntv2 => ntv2::get(id).is_some(),
        GridKind::Geoid | GridKind::Velocity => geoid::get(id).is_some(),
    }
}

/// The file of grid `id` of `kind` in `dir`, by the extensions such a grid is kept with.
fn file_in(dir: &Path, id: &str, kind: GridKind) -> Option<PathBuf> {
    kind.extensions().iter().map(|ext| dir.join(format!("{id}.{ext}"))).find(|p| p.is_file())
}

/// Grid `id` of `kind` read from `path` into the core, its SHA-256 checked.
fn read_in(path: &Path, id: &str, kind: GridKind) -> GridPlace {
    let Ok(bytes) = std::fs::read(path) else { return GridPlace::Missing };
    if crs_models::sha256(&bytes) != id {
        return GridPlace::Changed;
    }
    match kind {
        GridKind::Ntv2 => match ntv2::read(&bytes) {
            Ok(grid) => {
                ntv2::register(id, grid);
                GridPlace::Package
            }
            Err(e) => GridPlace::Unread(format!("{e:?}")),
        },
        GridKind::Geoid | GridKind::Velocity => {
            let want = if kind == GridKind::Geoid { ModelKind::Geoid } else { ModelKind::Velocity };
            match crs_models::read(&bytes) {
                Ok(model) if model.kind == want => {
                    geoid::register(id, model.grid);
                    GridPlace::Package
                }
                Ok(_) => GridPlace::Unread(format!("{} değil", if want == ModelKind::Geoid { "geoit modeli" } else { "hız ızgarası" })),
                Err(why) => GridPlace::Unread(why),
            }
        }
    }
}

/// Each grid `doc` names and where it was found, opened from `project_file`: the core, this device's library (`library`:
/// the device's folder when none), or `izgaralar/` beside the file (read into the core now); else missing.
pub fn find_in(doc: &Document, project_file: &Path, library: Option<&Path>) -> Vec<GridFound> {
    let device = library.map(Path::to_path_buf).or_else(crs_models::device_dir);
    let folder = project_file.parent().map(|p| p.join(FOLDER));
    named(doc.settings())
        .into_iter()
        .map(|(sha256, name, kind, setting)| {
            let place = if in_core(&sha256, kind) {
                GridPlace::Core
            } else if device.as_deref().and_then(|d| file_in(d, &sha256, kind)).is_some() {
                GridPlace::Library
            } else {
                match folder.as_deref().and_then(|d| file_in(d, &sha256, kind)) {
                    Some(path) => read_in(&path, &sha256, kind),
                    None => GridPlace::Missing,
                }
            };
            GridFound { sha256, name, kind, setting, place }
        })
        .collect()
}

/// [`find_in`] with this device's library.
pub fn find(doc: &Document, project_file: &Path) -> Vec<GridFound> {
    find_in(doc, project_file, None)
}

/// What opening said of the grids found beside the file: none when no grid came from there or is missing.
pub fn said(found: &[GridFound]) -> Option<String> {
    let names = |place: &str| -> Vec<String> {
        found.iter().filter(|g| g.place.name() == place).map(|g| format!("“{}”", g.name)).collect()
    };
    let (taken, missing, changed) = (names("package"), names("missing"), names("changed"));
    let mut parts = Vec::new();
    if !taken.is_empty() {
        parts.push(format!(
            "Projenin yanındaki {FOLDER}/ klasöründen {} ızgara kullanılıyor: {} (kitaplığa eklenmedi; kalıcı olsun isterseniz Izgaralar'dan ekleyin).",
            taken.len(),
            taken.join(", ")
        ));
    }
    if !changed.is_empty() {
        parts.push(format!("{FOLDER}/ klasöründeki {} dosyası değişmiş (SHA-256'sı tutmuyor), kullanılmadı.", changed.join(", ")));
    }
    if !missing.is_empty() {
        parts.push(format!("Projenin adladığı {} ızgarası ne bu cihazda ne {FOLDER}/ klasöründe var.", missing.join(", ")));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TG-20 with its last node changed by a millimetre: a geoid model no other test reads, its own SHA-256.
    fn own_geoid() -> (Vec<u8>, String) {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut bytes = std::fs::read(root.join("fixtures/geoid/v1/tg.gtx")).expect("the geoid");
        let n = bytes.len();
        let last = f32::from_be_bytes([bytes[n - 4], bytes[n - 3], bytes[n - 2], bytes[n - 1]]) + 0.001;
        bytes[n - 4..].copy_from_slice(&last.to_be_bytes());
        let id = crs_models::sha256(&bytes);
        (bytes, id)
    }

    fn project(id: &str) -> Document {
        let mut snapshot = kentos_kcad::decode(include_bytes!("../../../../fixtures/kcad/v2/minimal.kcad")).expect("the minimal drawing");
        snapshot.settings.heights = Some(kentos_commands::HeightReference {
            kind: kentos_commands::HeightKind::Orthometric,
            vertical_srid: Some(5775),
            geoid: Some(kentos_commands::GeoidModel { name: "TG-20 (paket)".into(), sha256: Some(id.into()) }),
        });
        Document::from_snapshot_v2(snapshot).expect("opens")
    }

    #[test]
    fn a_package_s_grid_is_read_from_beside_the_file_and_a_changed_one_is_not() {
        let (bytes, id) = own_geoid();
        let dir = std::env::temp_dir().join(format!("kentos-project-grids-{}", std::process::id()));
        let empty = dir.join("kitaplik");
        std::fs::create_dir_all(dir.join(FOLDER)).expect("a folder");
        std::fs::create_dir_all(&empty).expect("a library");
        let doc = project(&id);
        // Not there: missing.
        let away = find_in(&doc, &dir.join("baska").join("ada.kcad"), Some(&empty));
        assert_eq!(away[0].place, GridPlace::Missing);
        // Changed: refused, not read.
        let mut other = bytes.clone();
        other[0] ^= 1;
        std::fs::write(dir.join(FOLDER).join(format!("{id}.gtx")), &other).expect("writes");
        assert_eq!(find_in(&doc, &dir.join("ada.kcad"), Some(&empty))[0].place, GridPlace::Changed);
        assert!(geoid::get(&id).is_none());
        // The right file: read into the core; the next look finds it there.
        std::fs::write(dir.join(FOLDER).join(format!("{id}.gtx")), &bytes).expect("writes");
        let found = find_in(&doc, &dir.join("ada.kcad"), Some(&empty));
        assert_eq!((found[0].place.clone(), found[0].setting.as_str(), found[0].kind), (GridPlace::Package, "heights.geoid", GridKind::Geoid));
        assert!(said(&found).is_some_and(|w| w.contains("1 ızgara") && w.contains("TG-20 (paket)")));
        assert!(crs_models::load(&id, ModelKind::Geoid, Some(&empty)).is_ok(), "a command finds it in the core");
        assert_eq!(find_in(&doc, &dir.join("ada.kcad"), Some(&empty))[0].place, GridPlace::Core);
        let _ = std::fs::remove_dir_all(dir);
    }
}
