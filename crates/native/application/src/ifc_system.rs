//! The project's system as an IFC file says it (EK-7 Tablo 7.6–7.7; docs/adr/8047 §8, 8072 §3). The building's model
//! and the road's alignments take it from here, so that the two files stand on each other: the registry's projected
//! system the drawing is in, unturned; or the projected base of the project's local system, the local plane turned,
//! scaled and shifted onto it; and the height of the file's zero in the project's heights. A file says no system, with
//! the reason, for what it cannot carry: no system, a geographic one, a local system on no projected base or by an
//! affine transform that shears or mirrors the plane, a zero of unknown height. Nothing is assumed in their place: no
//! height is taken as 0, no similarity is fitted for an affine transform.

use kentos_commands::{CrsBase, CrsPlane, CrsSystem, HeightKind};
use kentos_domain::Document;
use kentos_ifc::crs::Crs;
use kentos_project::crs::{LOCAL_SRID, code, system, title};
use kentos_project::systems::definition_title;

/// Whether the project has a system at all: a drawing in local coordinates with none says no place.
pub fn has_system(doc: &Document) -> bool {
    let settings = doc.settings();
    settings.srid != LOCAL_SRID || settings.custom_crs.is_some()
}

/// Why a geographic system is no IFC system.
fn geographic(name: &str) -> String {
    format!(
        "projenin sistemi {name} coğrafi (enlem, boylam); IFC'nin harita dönüşümü düzlem bir sisteme yazılır"
    )
}

/// The base a local system stands on, as the file names it: its EPSG code or the project's name for it, and its title;
/// why not, for a base that is no projected system.
fn base_of(base: &CrsBase) -> Result<(String, String), String> {
    match (base.srid, base.definition.as_deref()) {
        (Some(srid), None) => match system(srid) {
            Some(s) if s.kind == "projected" => Ok((code(s), title(s))),
            Some(s) => Err(format!(
                "tabanı {} düzlem (projeksiyonlu) bir sistem değil",
                title(s)
            )),
            None => Err(format!("tabanı EPSG:{srid} sistem listesinde yok")),
        },
        (None, Some(d)) => match &d.system {
            CrsSystem::Tm(_) => Ok((d.name.clone(), definition_title(d))),
            CrsSystem::Geographic(_) => Err(format!(
                "tabanı {} coğrafi (enlem, boylam)",
                definition_title(d)
            )),
            CrsSystem::Local(_) => Err(format!(
                "tabanı {} de yerel bir sistem",
                definition_title(d)
            )),
        },
        _ => Err("tabanı belli değil".into()),
    }
}

/// The project's system in an IFC file whose zero stands `height` metres high in the project's heights (the
/// building's ±0.00; the road's 0, its heights the project's own); why the file says none, when it cannot say one.
pub fn of(doc: &Document, height: Option<f64>) -> Result<Crs, String> {
    let settings = doc.settings();
    let (name, words, plane) = match (settings.srid, &settings.custom_crs) {
        (LOCAL_SRID, None) => {
            return Err("projenin koordinat sistemi yok: çizim yerel koordinatlarda".into());
        }
        (LOCAL_SRID, Some(d)) => match &d.system {
            CrsSystem::Tm(_) => (d.name.clone(), definition_title(d), None),
            CrsSystem::Geographic(_) => return Err(geographic(&definition_title(d))),
            CrsSystem::Local(l) => {
                let (n, w) = base_of(&l.base).map_err(|why| {
                    format!(
                        "projenin yerel sistemi “{}” IFC'ye yazılamıyor: {why}; IFC'nin harita dönüşümü düzlem bir \
                         sisteme yazılır",
                        d.name
                    )
                })?;
                (n, w, Some((d.name.as_str(), &l.plane)))
            }
        },
        (srid, _) => match system(srid) {
            Some(s) if s.kind == "projected" => (code(s), title(s), None),
            Some(s) => return Err(geographic(&title(s))),
            None => {
                return Err(format!(
                    "projenin sistemi EPSG:{srid} sistem listesinde yok"
                ));
            }
        },
    };
    let Some(height) = height else {
        return Err(
            "±0,00'ın mutlak kotu bilinmiyor: IFC'nin harita dönüşümü modelin sıfırının yüksekliğini ister, 0 sayılmadı. \
             ±0,00 kotunu Proje ayarları › Mimari › İmar durumu'nda verin"
                .into(),
        );
    };
    let vertical = settings.heights.as_ref().and_then(|h| match h.kind {
        HeightKind::Orthometric => h
            .vertical()
            .map(|v| format!("{} (EPSG:{})", v.name, v.srid)),
        HeightKind::Ellipsoidal => Some("ITRF96".to_owned()),
        HeightKind::Unknown => None,
    });
    let crs = kentos_ifc::crs::projected(&name, Some(words), vertical, [0.0, 0.0, height]);
    let Some((local, plane)) = plane else {
        return Ok(crs);
    };
    match *plane {
        CrsPlane::Similarity { east, north, rotation, scale } => crs.turned(east, north, rotation, scale),
        CrsPlane::Affine { a, b, c, d, e, f } => crs.on_plane([a, b, c, d, e, f]),
    }
    .map_err(|why| {
        format!(
            "projenin yerel sistemi “{local}” IFC'ye yazılamıyor: {why}. IFC'nin harita dönüşümü düzlemi döndürür, eksen \
             başına ölçekler ve öteler; yerel sistemi benzerlik dönüşümüyle tanımlayın"
        )
    })
}
