//! A CBS project's data in the wizard (docs/adr/9196 §2, plan/ayrim.md AY-05): where its layers' tables are made (a
//! GeoPackage file, a PostgreSQL schema, KentOS Bulut) and where its project file goes (a `.kcad` beside the data, in
//! the GeoPackage or the schema, the cloud). Oluştur makes the purpose's tables there before the project opens, puts a
//! data layer reading each one on its node, and writes the project to its place at once: the tables and the project
//! that reads them are made together. KentOS Bulut makes no table: a cloud project keeps its objects in its database.

use std::path::{Path, PathBuf};

use kentos_commands::{CreateGeometry, CreateTarget, DocumentSnapshotV2, LayerGeometry, LayerNode, LayersCreate};
use kentos_project::cbs_tables;
use kentos_project::wizard::{DataPlace, Draft, Storage};

use crate::document::StoredSource;

/// Where the wizard makes a CBS project's tables.
#[derive(Debug, Clone, Default)]
pub(crate) struct Place {
    /// The GeoPackage's path; it follows the project's name until it is typed.
    pub(crate) path: String,
    pub(crate) path_typed: bool,
    /// A saved connection chosen, by its place in the list.
    pub(crate) connection: Option<usize>,
    /// The database, `postgresql://user@host:port/db` (no password).
    pub(crate) address: String,
    /// The schema; empty: `public`.
    pub(crate) schema: String,
}

/// What Oluştur does once the project is on screen.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum After {
    /// Writes the project to its `.kcad` beside its data.
    File(PathBuf),
    /// Writes the project into its data's GeoPackage or schema (docs/adr/9107).
    Store(StoredSource),
    /// Opens Buluta yükle on a CBS database project: the project and its data go to the cloud together.
    Cloud,
    /// Nothing: a CAD project, or a `.kcad` whose place its first Kaydet asks.
    Nothing,
}

/// The folder a new GeoPackage is offered in: the open drawing's, else the documents folder; a test's own scratch.
pub(crate) fn default_folder(open: Option<&Path>) -> PathBuf {
    if cfg!(test) {
        // Each wizard of a test its own folder: no project of another test or run is in the way.
        static MADE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return std::env::temp_dir().join(format!("kentos-sihirbaz-{}-{n}", std::process::id()));
    }
    open.and_then(Path::parent)
        .map(Path::to_path_buf)
        .or_else(|| std::env::home_dir().map(|h| h.join("Documents")).filter(|d| d.is_dir()))
        .or_else(std::env::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Place {
    pub(crate) fn new(folder: PathBuf, name: &str) -> Self {
        Self { path: gpkg_in(&folder, name), ..Self::default() }
    }

    /// The project's name changed: the GeoPackage follows it until its path is typed.
    pub(crate) fn named(&mut self, name: &str) {
        if !self.path_typed {
            let folder = Path::new(&self.path).parent().map(Path::to_path_buf).unwrap_or_default();
            self.path = gpkg_in(&folder, name);
        }
    }

    /// Why Oluştur cannot make the tables here, or none.
    pub(crate) fn blocked(&self, d: &Draft) -> Option<String> {
        match d.data {
            DataPlace::Geopackage if self.path.trim().is_empty() => {
                Some("GeoPackage dosyasının yolu boş: Gözat… ile bir dosya seçin ya da yolunu yazın.".to_owned())
            }
            DataPlace::Geopackage if d.storage == Storage::File && Path::new(&self.kcad()).exists() => Some(format!(
                "“{}” zaten var; proje dosyası onun üzerine yazılmaz. Başka bir proje adı ya da klasör seçin.",
                self.kcad()
            )),
            DataPlace::Postgres if self.address.trim().is_empty() => Some(
                "Veritabanı seçilmedi: bir bağlantı seçin ya da adresini (postgresql://kullanıcı@sunucu:5432/veritabanı) yazın."
                    .to_owned(),
            ),
            _ => None,
        }
    }

    /// The `.kcad` beside the GeoPackage.
    fn kcad(&self) -> String {
        Path::new(self.path.trim()).with_extension("kcad").display().to_string()
    }

    fn schema(&self) -> String {
        let s = self.schema.trim();
        if s.is_empty() { "public".to_owned() } else { s.to_owned() }
    }

    /// The tables of `v2`'s tree made in this place, each table layer's node a data layer reading its table; what Oluştur
    /// does once the project opens. A CBS project in the cloud makes no table; a CAD project nothing at all.
    pub(crate) fn make(&self, d: &Draft, v2: &mut DocumentSnapshotV2) -> Result<After, String> {
        let (target, path, address, schema) = match d.data {
            DataPlace::Cloud => return Ok(After::Cloud),
            DataPlace::Geopackage => (CreateTarget::Gpkg, Some(self.path.trim().to_owned()), None, None),
            DataPlace::Postgres => {
                (CreateTarget::Postgis, None, Some(self.address.trim().to_owned()), Some(self.schema()))
            }
        };
        // The GeoPackage's folder is made: the project file goes beside it even before its first table.
        if let Some(folder) = path.as_deref().map(Path::new).and_then(Path::parent).filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder).map_err(|e| format!("“{}” klasörü yapılamadı: {e}", folder.display()))?;
        }
        // The system and the names taken are the new project's.
        let checks = crate::document::Document::from_v2(v2.clone(), None)?;
        for table in cbs_tables::tables(&v2.layers) {
            let Some(node) = find(&mut v2.layers, &table.id) else { continue };
            let input = LayersCreate {
                name: node.name.clone(),
                target,
                path: path.clone(),
                address: address.clone(),
                schema: schema.clone(),
                table: Some(cbs_tables::table_name(&table.id)),
                geometry: match table.geometry {
                    LayerGeometry::Point => CreateGeometry::Point,
                    LayerGeometry::Line => CreateGeometry::Line,
                    LayerGeometry::Area | LayerGeometry::Annotation => CreateGeometry::Area,
                },
                z: Some(table.z),
                fields: (!table.fields.is_empty()).then(|| table.fields.clone()),
                ..LayersCreate::default()
            };
            let (data, fields) = kentos_native_application::layers_create::new_project_table(&checks.model, &input)
                .map_err(|why| format!("“{}” katmanının tablosu yapılamadı: {why}", node.name))?;
            node.data = Some(data);
            node.fields = fields;
        }
        Ok(match (d.data, d.storage) {
            (DataPlace::Geopackage, Storage::Geopackage) => After::Store(StoredSource {
                address: self.path.trim().to_owned(),
                schema: None,
                name: d.name.trim().to_owned(),
            }),
            (DataPlace::Postgres, Storage::Database) => After::Store(StoredSource {
                address: self.address.trim().to_owned(),
                schema: Some(self.schema()),
                name: d.name.trim().to_owned(),
            }),
            (DataPlace::Geopackage, _) => After::File(PathBuf::from(self.kcad())),
            _ => After::Nothing,
        })
    }
}

/// `<folder>/<name>.gpkg`, the name as a file name.
fn gpkg_in(folder: &Path, name: &str) -> String {
    let stem: String = name
        .trim()
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '-' } else { c })
        .collect();
    let stem = if stem.is_empty() { "Yeni proje".to_owned() } else { stem };
    folder.join(format!("{stem}.gpkg")).display().to_string()
}

/// The node `id` of a tree.
fn find<'a>(nodes: &'a mut [LayerNode], id: &str) -> Option<&'a mut LayerNode> {
    for n in nodes {
        if n.id == id {
            return Some(n);
        }
        if let Some(found) = find(&mut n.children, id) {
            return Some(found);
        }
    }
    None
}
