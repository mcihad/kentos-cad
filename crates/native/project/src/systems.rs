//! The project's coordinate systems as the transforms read them (docs/adr/0168
//! §1–§3; the web's `model/projectCrs.ts`): its own system and its second,
//! each one of the registry's or a definition of the project's
//! (`customCrs`, `secondCustomCrs`), named; its datum choices
//! (`datumTransforms`). The shared cases are fixtures/geodesy/v1/project.json
//! (scripts/fixtures/project_crs_cases.py, from the ADR's rules).

use kentos_contracts::{
    Convention, CrsDefinition, CrsPlane, CrsSystem, DatumRef, DatumTransform, Helmert,
    ProjectSettings, RegistryDatum,
};
use kentos_geometry_core::crs as core;

use crate::crs::{LOCAL_SRID, System, code, system, title};

/// A coordinate system of the project's, named: one of the registry's, or a
/// definition of the project's own.
#[derive(Clone, Debug, PartialEq)]
pub struct Named {
    /// “TUREF / TM30”, “Şantiye”.
    pub name: String,
    /// As a sentence names it: “TUREF / TM30 (EPSG:5254)”, “Şantiye (özel sistem)”.
    pub title: String,
    /// As a value or a chip shows it: “EPSG:5254”, “Özel sistem”.
    pub code: String,
    /// As the transforms read it; none for a definition whose base is not a
    /// projected system of the registry.
    pub system: Option<core::System>,
}

impl Named {
    fn registry(s: &System) -> Self {
        Self {
            name: s.name.clone(),
            title: title(s),
            code: code(s),
            system: s.transform_system(),
        }
    }

    fn definition(d: &CrsDefinition) -> Self {
        Self {
            name: d.name.clone(),
            title: definition_title(d),
            code: DEFINITION_CODE.to_owned(),
            system: definition_system(d),
        }
    }

    /// Whether its values are a latitude and a longitude.
    pub fn geographic(&self) -> bool {
        matches!(self.system, Some(core::System::Geographic { .. }))
    }
}

/// A definition as a sentence names it: “Şantiye (özel sistem)”.
pub fn definition_title(d: &CrsDefinition) -> String {
    format!("{} (özel sistem)", d.name)
}

/// A definition's code as a value or a chip shows it.
pub const DEFINITION_CODE: &str = "Özel sistem";

/// Whether the project has a system of its own the registry or its
/// definition names.
fn has_own(settings: &ProjectSettings) -> bool {
    match settings.srid {
        LOCAL_SRID => settings.custom_crs.is_some(),
        srid => system(srid).is_some(),
    }
}

/// The project's own system: the registry's by its SRID, or its definition;
/// none for a project without one (SRID 0, docs/adr/0165 §2) and an SRID the
/// registry does not have.
pub fn own(settings: &ProjectSettings) -> Option<Named> {
    match (settings.srid, &settings.custom_crs) {
        (LOCAL_SRID, Some(d)) => Some(Named::definition(d)),
        (LOCAL_SRID, None) => None,
        (srid, _) => system(srid).map(Named::registry),
    }
}

/// The project's second system (docs/adr/0167 §1): the registry's or a
/// definition; none without one, or without a system of the project's.
pub fn second(settings: &ProjectSettings) -> Option<Named> {
    if !has_own(settings) {
        return None;
    }
    match (settings.second(), &settings.second_custom_crs) {
        (Some(srid), _) => system(srid).map(Named::registry),
        (None, Some(d)) => Some(Named::definition(d)),
        (None, None) => None,
    }
}

/// The project's datum choices as the transforms take them (§3).
pub fn choices(settings: &ProjectSettings) -> Vec<core::Choice> {
    settings
        .datum_transforms
        .iter()
        .filter_map(choice)
        .collect()
}

fn choice(t: &DatumTransform) -> Option<core::Choice> {
    let method = match (&t.helmert, &t.grid) {
        (Some(h), None) => core::Method::Helmert(helmert(h)),
        (None, Some(g)) => core::Method::Grid {
            id: g.id.clone(),
            accuracy: g.accuracy,
        },
        _ => return None,
    };
    Some(core::Choice {
        from: registry_datum(t.from),
        to: registry_datum(t.to),
        name: t.name.clone(),
        method,
    })
}

fn registry_datum(d: RegistryDatum) -> core::Datum {
    match d {
        RegistryDatum::Turef => core::Datum::Turef,
        RegistryDatum::Ed50 => core::Datum::Ed50,
        RegistryDatum::Wgs84 => core::Datum::Wgs84,
    }
}

fn helmert(h: &Helmert) -> core::Helmert {
    core::Helmert {
        translation: h.translation,
        rotation: h.rotation,
        scale: h.scale,
        convention: match h.convention {
            Convention::PositionVector => core::Convention::PositionVector,
            Convention::CoordinateFrame => core::Convention::CoordinateFrame,
        },
        accuracy: h.accuracy,
    }
}

fn datum(d: DatumRef<'_>) -> core::Datum {
    match d {
        DatumRef::Registry(r) => registry_datum(r),
        DatumRef::Custom(c) => core::Datum::Custom(Box::new(core::CustomDatum {
            name: c.name.clone(),
            ellipsoid: core::Ellipsoid {
                name: c.ellipsoid.name.clone(),
                semi_major: c.ellipsoid.semi_major,
                inverse_flattening: c.ellipsoid.inverse_flattening,
            },
            to_wgs84: c.to_wgs84.as_ref().map(helmert),
        })),
    }
}

/// A definition as the transforms read it (§1); none when a local system's
/// base is not a projected system of the registry.
pub fn definition_system(d: &CrsDefinition) -> Option<core::System> {
    Some(match &d.system {
        CrsSystem::Tm(t) => core::System::Tm {
            datum: datum(t.datum()?),
            latitude_of_origin: t.latitude_of_origin,
            central_meridian: t.central_meridian,
            scale_factor: t.scale_factor,
            false_easting: t.false_easting,
            false_northing: t.false_northing,
        },
        CrsSystem::Geographic(g) => core::System::Geographic {
            datum: datum(g.datum()?),
        },
        CrsSystem::Local(l) => core::System::Local {
            base: Box::new(match (l.base.srid, &l.base.definition) {
                (Some(srid), None) => system(srid)
                    .filter(|s| s.kind == "projected")?
                    .transform_system()?,
                (None, Some(b)) => definition_system(b)?,
                _ => return None,
            }),
            plane: match l.plane {
                CrsPlane::Similarity {
                    east,
                    north,
                    rotation,
                    scale,
                } => core::Plane::Similarity {
                    east,
                    north,
                    rotation,
                    scale,
                },
                CrsPlane::Affine { a, b, c, d, e, f } => core::Plane::Affine { a, b, c, d, e, f },
            },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::api::json::{FromJson, Json};
    use serde_json::Value;

    fn core_json<T: FromJson>(v: &Value) -> T {
        T::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("the core reads it")
    }

    fn check(want: &Value, got: Option<Named>, case: &str) {
        if want.is_null() {
            assert_eq!(got, None, "{case}");
            return;
        }
        let got = got.unwrap_or_else(|| panic!("{case}: none"));
        let text = |k: &str| want[k].as_str().expect("a text").to_owned();
        assert_eq!(
            (got.name.clone(), got.title.clone(), got.code.clone()),
            (text("name"), text("title"), text("code")),
            "{case}"
        );
        let system =
            (!want["system"].is_null()).then(|| core_json::<core::System>(&want["system"]));
        assert_eq!(got.system, system, "{case}");
    }

    /// The project's systems and choices as the shared cases say (the web
    /// reads the same file, `model/projectCrs.test.ts`).
    #[test]
    fn the_projects_systems_are_the_shared_cases() {
        let file: Value =
            serde_json::from_str(include_str!("../../../../fixtures/geodesy/v1/project.json"))
                .expect("the cases read");
        assert_eq!(file["format"], "kentos.project-crs");
        let cases = file["cases"].as_array().expect("cases");
        assert!(cases.len() >= 9);
        for case in cases {
            let name = case["name"].as_str().expect("a name");
            let settings: ProjectSettings =
                serde_json::from_value(case["settings"].clone()).expect("settings");
            check(&case["own"], own(&settings), name);
            check(&case["second"], second(&settings), name);
            let want: Vec<core::Choice> = case["choices"]
                .as_array()
                .expect("choices")
                .iter()
                .map(core_json)
                .collect();
            assert_eq!(choices(&settings), want, "{name}");
        }
    }
}
