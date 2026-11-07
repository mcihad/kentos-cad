//! What a file for the field says of its systems, and what it needs (plan/road.md ROAD-SEC-10, docs/adr/1031). A
//! stakeout file and a design for machine control give heights an instrument sets out: BÖHHBÜY (26.06.2018) Madde 1
//! asks for TUREF's coordinates and TUDKA99's Helmert orthometric heights, and a GNSS rover gives ellipsoidal heights
//! until a geoid or a localisation turns them. Heights whose system the project does not say are not sent; a LandXML
//! file says the systems in its `CoordinateSystem`, as buildingSMART's LandXML 1.2 profile asks.

use kentos_commands::{HeightKind, TerrainSurfaceDef};
use kentos_domain::Document;
use kentos_formats::landxml_write::WCoordinateSystem;
use kentos_project::crs::{LOCAL_SRID, system};

use crate::checks::Stop;
use crate::codes;
use crate::road_heights::System;
use crate::road_terrains::fail;

/// The project's systems as a LandXML file says them; none when it knows neither.
pub(crate) fn coordinate_system(doc: &Document) -> Option<WCoordinateSystem> {
    let s = doc.settings();
    let mut c = WCoordinateSystem::default();
    match s.custom_crs.as_ref().filter(|_| s.srid == LOCAL_SRID) {
        Some(own) => c.horizontal_name = Some(own.name.clone()),
        None if s.srid != LOCAL_SRID => {
            match system(s.srid) {
                Some(sys) => {
                    c.name = Some(sys.name.clone());
                    c.horizontal_datum = Some(sys.datum.clone());
                }
                None => c.name = Some(format!("EPSG:{}", s.srid)),
            }
            c.epsg_code = Some(s.srid);
        }
        None => {}
    }
    if let Some(h) = &s.heights {
        match h.kind {
            HeightKind::Orthometric => {
                if let Some(v) = h.vertical() {
                    c.vertical_name = Some(v.name.to_owned());
                    c.vertical_datum = Some(v.datum.to_owned());
                }
            }
            HeightKind::Ellipsoidal => c.vertical_name = Some("ellipsoidal height".to_owned()),
            HeightKind::Unknown => {}
        }
        if h.kind != HeightKind::Unknown {
            let mut words = format!("Kotlar: {}", h.title());
            if let Some(e) = s.coordinate_epoch {
                words.push_str(&format!("; epok {e}"));
            }
            c.desc = Some(words);
        }
    }
    (c != WCoordinateSystem::default()).then_some(c)
}

/// Whether the project says its heights' system, as a file for the field needs (`what`: “Aplikasyon dosyasının”).
pub(crate) fn heights_said(doc: &Document, what: &str) -> Result<(), Stop> {
    let why = match &doc.settings().heights {
        None => "projenin kotlarının düşey sistemi söylenmemiş",
        Some(h) if h.kind == HeightKind::Unknown => "projenin kotlarının düşey sistemi söylenmemiş",
        Some(h) if h.kind == HeightKind::Orthometric && h.vertical().is_none() => {
            "projenin kotları ortometrik ama düşey sistemi belirtilmemiş"
        }
        Some(_) => return Ok(()),
    };
    Err(fail(
        codes::FIELD_HEIGHTS_UNKNOWN,
        format!(
            "{what} kotları bir düşey sisteme bağlı olmalı: {why}. Proje ayarları › Kotlar'da sistemi seçin (BÖHHBÜY: TUDKA99'a dayalı Helmert ortometrik yükseklik, Antalya height EPSG:5775). Kotlar bir varsayımla sahaya gönderilmez."
        ),
        "",
    ))
}

/// Whether a surface sent to the field rests on the project's heights (docs/adr/1029): the ground taken from a cloud
/// must be in the project's system, said by its files. The project's heights are said (`heights_said` first).
pub(crate) fn surface_on_project(doc: &Document, def: &TerrainSurfaceDef, path: &str) -> Result<(), Stop> {
    let project = System::of_project(doc);
    for c in &def.clouds {
        let why = match (c.vertical.as_deref(), &project) {
            (None, _) => "dosyası düşey sistemini söylemiyor ya da alındığında kaydedilmedi".to_owned(),
            (Some(v), Some(p)) if System::of_file(v) != *p => {
                format!("{}, projenin kotları {}", System::of_file(v).words(), p.words())
            }
            _ => continue,
        };
        return Err(fail(
            codes::UNPROVEN_HEIGHTS,
            format!(
                "“{}” yüzeyinin “{}” katmanındaki bulut noktaları: {why}. Kotları projenin sisteminde gösterilmeyen yüzey makine kontrolüne gönderilmez.",
                def.name, c.layer
            ),
            path,
        ));
    }
    Ok(())
}
