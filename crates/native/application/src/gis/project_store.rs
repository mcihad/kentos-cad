//! A project kept in a data source with its data (docs/adr/9107): written into and read from a GeoPackage's or a
//! PostgreSQL schema's `kentos_projects` table through the source's driver ([`kentos_data::ProjectStore`]), so a new
//! database driver keeps projects as well. The desktop's Veri kaynağı window and Python's `Document.save_to` /
//! `Document.open_from` go through here.
//!
//! What is kept is the project's `.kcad` bytes, as a file save writes them; a write is read back and compared before
//! it counts. A project kept in a GeoPackage takes that file as its place: its data layers', bound GeoPackages' and
//! external references' relative paths start from it, so layers in the same file travel with it (a database has no
//! relative path).

use std::path::{Path, PathBuf};

use kentos_commands::DocumentSnapshotV2;
use kentos_data::StoredProject;

/// A GeoPackage's file of an address; none for a database.
pub fn file_of(address: &str) -> Option<PathBuf> {
    let a = address.trim();
    (!a.starts_with("postgres")).then(|| PathBuf::from(a.strip_prefix("gpkg:").unwrap_or(a)))
}

/// The account the program runs as: who wrote a project last.
pub fn user() -> Option<String> {
    std::env::var("USER").or_else(|_| std::env::var("USERNAME")).ok().filter(|u| !u.trim().is_empty())
}

/// The snapshot as a project kept at `address` writes it: a GeoPackage's file is where the relative paths start from.
pub fn with_relative(snapshot: &mut DocumentSnapshotV2, address: &str) {
    if let Some(file) = file_of(address) {
        crate::source_places::with_relative(&mut snapshot.settings, &file);
        crate::source_places::with_relative_layers(&mut snapshot.layers, &file);
        crate::xrefs::with_relative(&mut snapshot.blocks, &file);
        crate::source_places::with_relative_files(&mut snapshot.entities, &file);
    }
}

/// A project's bytes read from `address` as its snapshot: a GeoPackage's linked rasters and pictures written relative
/// to that file absolute again (docs/adr/9117 §3).
pub fn decode(address: &str, bytes: &[u8]) -> Result<DocumentSnapshotV2, String> {
    let mut snapshot = kentos_kcad::decode(bytes).map_err(|e| e.message)?;
    if let Some(file) = file_of(address) {
        crate::source_places::with_absolute_files(&mut snapshot.entities, &file);
    }
    Ok(snapshot)
}

fn store_of(address: &str) -> Result<std::sync::Arc<dyn kentos_data::Source>, String> {
    let address = address.trim();
    if address.is_empty() {
        return Err("Kaynak verilmedi: bir GeoPackage dosyası ya da PostgreSQL bağlantısı seçin.".into());
    }
    // A new GeoPackage file is made: a project may be its first content.
    if let Some(file) = file_of(address)
        && !file.exists()
    {
        if file.extension().is_none_or(|e| !e.eq_ignore_ascii_case("gpkg")) {
            return Err(format!("“{}” bir GeoPackage dosyası değil (.gpkg).", file.display()));
        }
        if let Some(folder) = file.parent().filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder).map_err(|e| format!("“{}” klasörü yapılamadı: {e}", folder.display()))?;
        }
        kentos_geopackage::GeoPackage::create(&file).map_err(|e| format!("“{}” yapılamadı: {}", file.display(), e.0))?;
        kentos_data::registry().forget(address);
    }
    kentos_data::registry().source(address).map_err(|e| e.to_string())
}

fn kept_in(source: &dyn kentos_data::Source) -> Result<&dyn kentos_data::ProjectStore, String> {
    source
        .project_store()
        .ok_or_else(|| "Bu kaynak proje saklayamaz; bir GeoPackage dosyası ya da PostgreSQL veritabanı seçin.".to_owned())
}

/// The projects kept at a place, by name.
pub fn list(address: &str, schema: Option<&str>) -> Result<Vec<StoredProject>, String> {
    if file_of(address).is_some_and(|f| !f.exists()) {
        return Ok(Vec::new());
    }
    let source = store_of(address)?;
    kept_in(source.as_ref())?.projects(schema).map_err(|e| e.to_string())
}

/// A project's bytes.
pub fn read(address: &str, schema: Option<&str>, name: &str) -> Result<Vec<u8>, String> {
    if let Some(f) = file_of(address).filter(|f| !f.exists()) {
        return Err(format!("“{}” bulunamadı: dosya taşınmış ya da silinmiş olabilir.", f.display()));
    }
    let source = store_of(address)?;
    kept_in(source.as_ref())?.read_project(schema, name).map_err(|e| e.to_string())
}

/// Writes a project's bytes and reads them back: kept only when they come back the same.
pub fn write(address: &str, schema: Option<&str>, name: &str, bytes: &[u8]) -> Result<(), String> {
    let source = store_of(address)?;
    let store = kept_in(source.as_ref())?;
    store.write_project(schema, name, bytes, user().as_deref()).map_err(|e| e.to_string())?;
    let back = store.read_project(schema, name).map_err(|e| e.to_string())?;
    if back != bytes {
        return Err(format!(
            "“{}” projesi yazıldı ama geri okunan {} bayt yazılan {} bayttan farklı; kaynağı denetleyin.",
            name.trim(),
            back.len(),
            bytes.len()
        ));
    }
    Ok(())
}

/// Removes a project.
pub fn delete(address: &str, schema: Option<&str>, name: &str) -> Result<(), String> {
    let source = store_of(address)?;
    kept_in(source.as_ref())?.delete_project(schema, name).map_err(|e| e.to_string())
}

/// Where its relative paths start from when a project read from `address` is opened (a GeoPackage's file).
pub fn place_of(address: &str) -> Option<PathBuf> {
    file_of(address)
}

/// Whether a path names a GeoPackage file.
pub fn is_geopackage(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("gpkg"))
}
