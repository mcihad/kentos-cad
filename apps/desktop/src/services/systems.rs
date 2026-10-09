//! Between the project's system and a service's (docs/adr/0208 §4): a
//! service's tiles are in their own system (Web Mercator, a TM zone,
//! latitude and longitude), the drawing in the project's. The same system
//! draws the tiles as they are; another takes every mesh node through the
//! core's transformation with the project's datum choices (`crs::transform_in`,
//! the one Koordinat dönüştür uses). A project without a system (a local
//! one) shows no service: nothing is guessed (CLAUDE.md §5).

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use kentos_contracts::ProjectSettings;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::crs::{Choice, System, transform_in};
use kentos_render_wgpu::styled::Transform;

/// The project's system as the services read it.
#[derive(Clone, Debug)]
pub struct ProjectSystem {
    pub srid: u32,
    /// Its definition's own, when it has one (then `srid` is 0).
    pub custom: bool,
    pub system: Option<System>,
    pub choices: Vec<Choice>,
    /// What tells one project's system from another's.
    pub key: u64,
}

impl ProjectSystem {
    pub fn of(settings: &ProjectSettings) -> ProjectSystem {
        let own = kentos_project::systems::own(settings);
        let mut h = DefaultHasher::new();
        settings.srid.hash(&mut h);
        serde_json::to_string(&settings.custom_crs)
            .unwrap_or_default()
            .hash(&mut h);
        serde_json::to_string(&settings.datum_transforms)
            .unwrap_or_default()
            .hash(&mut h);
        ProjectSystem {
            srid: settings.srid,
            custom: settings.custom_crs.is_some(),
            system: own.and_then(|n| n.system),
            choices: kentos_project::systems::choices(settings),
            key: h.finish(),
        }
    }
}

/// The two ways between the project and a service's system.
pub struct Pair {
    pub same: bool,
    pub to_grid: Transform,
    pub to_project: Transform,
    /// Metres a unit of the service's system is about.
    pub metres_per_unit: f64,
}

/// The ways between `project` and the service system `srid`, or why there are none.
pub fn pair(project: &ProjectSystem, srid: u32) -> Result<Pair, String> {
    let Some(theirs) = kentos_project::crs::system(srid).and_then(|s| s.transform_system()) else {
        return Err(format!(
            "Servisin koordinat sistemi (EPSG:{srid}) KentOS'un kaydında yok; servisi başka bir sistemde isteyin."
        ));
    };
    let metres_per_unit = if matches!(theirs, System::Geographic { .. }) {
        111_320.0
    } else {
        1.0
    };
    if project.srid == srid && !project.custom {
        let same: Transform = Arc::new(|x, y| Some((x, y)));
        return Ok(Pair {
            same: true,
            to_grid: same.clone(),
            to_project: same,
            metres_per_unit,
        });
    }
    let Some(ours) = project.system.clone() else {
        return Err(
            "Projenin koordinat sistemi yok: harita servisi gösterilemez. Proje ayarlarında projeye bir koordinat sistemi verin.".into(),
        );
    };
    let (ours, theirs) = (Arc::new(ours), Arc::new(theirs));
    let choices = Arc::new(project.choices.clone());
    let to_grid: Transform = {
        let (a, b, c) = (ours.clone(), theirs.clone(), choices.clone());
        Arc::new(move |x, y| {
            transform_in(&a, &b, Vec2 { x, y }, &c)
                .ok()
                .map(|t| (t.point.x, t.point.y))
        })
    };
    let to_project: Transform = Arc::new(move |x, y| {
        transform_in(&theirs, &ours, Vec2 { x, y }, &choices)
            .ok()
            .map(|t| (t.point.x, t.point.y))
    });
    Ok(Pair {
        same: false,
        to_grid,
        to_project,
        metres_per_unit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(srid: u32) -> ProjectSettings {
        kentos_project::new_project::default_settings(srid)
    }

    #[test]
    fn the_same_system_as_it_is_another_through_the_core_none_for_a_local_project() {
        let tm = ProjectSystem::of(&settings(5256));
        let same = pair(&tm, 5256).expect("same");
        assert!(same.same);
        assert_eq!(
            (same.to_grid)(500_000.0, 4_400_000.0),
            Some((500_000.0, 4_400_000.0))
        );
        let merc = pair(&tm, 3857).expect("mercator");
        assert!(!merc.same);
        let (x, y) = (merc.to_grid)(500_000.0, 4_400_000.0).expect("in mercator");
        let (bx, by) = (merc.to_project)(x, y).expect("back");
        assert!(
            (bx - 500_000.0).abs() < 1e-6 && (by - 4_400_000.0).abs() < 1e-6,
            "{bx} {by}"
        );
        assert!(pair(&ProjectSystem::of(&settings(0)), 3857).is_err());
        assert!(pair(&tm, 99_999).is_err());
    }
}
