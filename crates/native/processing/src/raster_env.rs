//! A run's raster analysis environment (docs/adr/0643 §1) from the command's words: the area's objects become their
//! box, the snap raster its place; the raster core's `Environment` every raster tool of the run takes.

use kentos_commands::{CommandError, Entity, RasterEnvironment, codes};
use kentos_domain::Document;
pub use kentos_raster::env::Environment;

use crate::Tool;
use crate::builtin::{
    distance, hydrology, interpolation, raster_ops, raster_vector, remote, suitability, surface,
};
use crate::command::{refused, slots_of};

/// The categories whose tools take the environment: the raster analysis tools that write a raster (docs/adr/0643
/// §2). The point cloud and multidimensional tools do not.
pub const CATEGORIES: [&str; 11] = [
    raster_ops::OPS,
    raster_ops::STATS,
    surface::CATEGORY,
    interpolation::INTERPOLATION,
    interpolation::DENSITY,
    raster_vector::RASTER_VECTOR,
    raster_vector::SCANNED,
    distance::DISTANCE,
    hydrology::HYDROLOGY,
    suitability::SUITABILITY,
    remote::REMOTE,
];

/// Whether `tool` takes the environment.
pub fn takes(tool: &Tool) -> bool {
    CATEGORIES.contains(&tool.category.as_str())
}

/// The environment the raster core takes, or why the command's is refused (its path under `environment`).
pub fn resolve(
    doc: &Document,
    given: &RasterEnvironment,
) -> Result<Environment, Box<CommandError>> {
    let bad = |path: &str, message: String| {
        refused(
            codes::INVALID_PARAMETERS,
            message,
            Some(format!("environment.{path}")),
        )
    };
    let mut area = match given.area {
        None => None,
        Some(a) if a.iter().all(|v| v.is_finite()) && a[0] <= a[2] && a[1] <= a[3] => Some(a),
        Some(_) => {
            return Err(bad(
                "area",
                "Analiz alanı en küçük doğu, kuzey, en büyük doğu, kuzey sırasıyla dört sayı olmalı.".into(),
            ));
        }
    };
    let slots = slots_of(doc, &given.area_objects, "environment.areaObjects")?;
    for slot in slots {
        let Some(e) = doc.get(slot) else { continue };
        let b = kentos_geometry_core::entity::entity_bounds(
            &kentos_native_application::geometry::shape(e),
        );
        if !(b.min_x.is_finite() && b.max_x.is_finite()) {
            continue;
        }
        let k = [b.min_x, b.min_y, b.max_x, b.max_y];
        area = Some(match area {
            None => k,
            Some(a) => [
                a[0].min(k[0]),
                a[1].min(k[1]),
                a[2].max(k[2]),
                a[3].max(k[3]),
            ],
        });
    }
    if !given.area_objects.is_empty() && area.is_none() {
        return Err(bad(
            "areaObjects",
            "Analiz alanının nesnelerinin yeri okunamadı.".into(),
        ));
    }
    if let Some(c) = given.cell
        && !(c.is_finite() && c > 0.0)
    {
        return Err(bad(
            "cell",
            "Ortamın hücre boyu sıfırdan büyük bir sayı olmalı.".into(),
        ));
    }
    let snap = match &given.snap {
        None => None,
        Some(uid) => {
            let slot =
                slots_of(doc, std::slice::from_ref(uid), "environment.snap").map_err(|mut e| {
                    e.path = Some("environment.snap".into());
                    e
                })?;
            match slot.first().and_then(|s| doc.get(*s)) {
                Some(Entity::Raster(r)) => Some(r.raster.affine),
                _ => {
                    return Err(bad(
                        "snap",
                        format!(
                            "{uid} bir raster değil; hizalama rasteri bir raster nesnesi olmalı."
                        ),
                    ));
                }
            }
        }
    };
    if let Some(v) = given.nodata
        && !v.is_finite()
    {
        return Err(bad(
            "nodata",
            "Ortamın NoData değeri bir sayı olmalı.".into(),
        ));
    }
    Ok(Environment {
        area,
        cell: given.cell,
        snap,
        nodata: given.nodata,
    })
}
