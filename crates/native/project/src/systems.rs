//! The project's coordinate systems as the transforms read them (docs/adr/0168
//! §1–§3; the web's `model/projectCrs.ts`): its own system and its second,
//! each one of the registry's or a definition of the project's
//! (`customCrs`, `secondCustomCrs`), named; its datum choices
//! (`datumTransforms`). The shared cases are fixtures/geodesy/v1/project.json
//! (scripts/fixtures/project_crs_cases.py, from the ADR's rules).

use kentos_contracts::{
    Convention, CrsBase, CrsDefinition, CrsPlane, CrsSystem, CustomDatum, DatumRef, DatumTransform,
    Ellipsoid, GeographicDefinition, Helmert, LocalDefinition, ProjectSettings, RegistryDatum,
    TmDefinition,
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

/// A system the core reads (from WKT or PROJ, docs/adr/0168 §5) as the
/// project's definition named `name`: the registry's datums by name, any
/// other as the project's own; a local system's base the registry's
/// projected system with every value the same, else a definition of its own
/// named “<name> tabanı”. None for what a definition cannot be (the
/// Pseudo-Mercator, a local system on another base).
pub fn definition_from(name: &str, s: &core::System) -> Option<CrsDefinition> {
    Some(CrsDefinition {
        name: name.to_owned(),
        system: match s {
            core::System::Geographic { datum } => {
                let (datum, custom_datum) = datum_fields(datum);
                CrsSystem::Geographic(GeographicDefinition {
                    datum,
                    custom_datum,
                })
            }
            core::System::Tm { .. } => CrsSystem::Tm(tm_from(s)?),
            core::System::Local { base, plane } => {
                let srid = crate::crs::systems()
                    .iter()
                    .find(|r| r.kind == "projected" && r.transform_system().as_ref() == Some(base))
                    .map(|r| r.srid);
                let definition = match srid {
                    Some(_) => None,
                    None => Some(Box::new(CrsDefinition {
                        name: format!("{name} tabanı"),
                        system: CrsSystem::Tm(tm_from(base)?),
                    })),
                };
                CrsSystem::Local(LocalDefinition {
                    base: CrsBase { srid, definition },
                    plane: match *plane {
                        core::Plane::Similarity {
                            east,
                            north,
                            rotation,
                            scale,
                        } => CrsPlane::Similarity {
                            east,
                            north,
                            rotation,
                            scale,
                        },
                        core::Plane::Affine { a, b, c, d, e, f } => {
                            CrsPlane::Affine { a, b, c, d, e, f }
                        }
                    },
                })
            }
            core::System::Mercator {} => return None,
        },
    })
}

/// A transverse Mercator the core reads as a definition's.
fn tm_from(s: &core::System) -> Option<TmDefinition> {
    let core::System::Tm {
        datum,
        latitude_of_origin,
        central_meridian,
        scale_factor,
        false_easting,
        false_northing,
    } = s
    else {
        return None;
    };
    let (datum, custom_datum) = datum_fields(datum);
    Some(TmDefinition {
        datum,
        custom_datum,
        latitude_of_origin: latitude_of_origin.filter(|v| *v != 0.0),
        central_meridian: *central_meridian,
        scale_factor: *scale_factor,
        false_easting: *false_easting,
        false_northing: *false_northing,
    })
}

/// A datum the core reads as a definition's two fields: the registry's by
/// name, or the project's own.
fn datum_fields(d: &core::Datum) -> (Option<RegistryDatum>, Option<Box<CustomDatum>>) {
    match d {
        core::Datum::Turef => (Some(RegistryDatum::Turef), None),
        core::Datum::Ed50 => (Some(RegistryDatum::Ed50), None),
        core::Datum::Wgs84 => (Some(RegistryDatum::Wgs84), None),
        core::Datum::Custom(c) => (
            None,
            Some(Box::new(CustomDatum {
                name: c.name.clone(),
                ellipsoid: Ellipsoid {
                    name: c.ellipsoid.name.clone(),
                    semi_major: c.ellipsoid.semi_major,
                    inverse_flattening: c.ellipsoid.inverse_flattening,
                },
                to_wgs84: c.to_wgs84.as_ref().map(|h| Helmert {
                    translation: h.translation,
                    rotation: h.rotation,
                    scale: h.scale,
                    convention: match h.convention {
                        core::Convention::PositionVector => Convention::PositionVector,
                        core::Convention::CoordinateFrame => Convention::CoordinateFrame,
                    },
                    accuracy: h.accuracy,
                }),
            })),
        ),
    }
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

    /// Every system the core reads from the texts of fixtures/geodesy/v1/text.json
    /// (PROJ's) is a definition that the transforms read back as it was; a
    /// local system's base is the registry's when every value is, else a
    /// definition of its own (the web's `definitionFrom`).
    #[test]
    fn a_system_read_is_a_definition_read_back_as_it_was() {
        let file: Value =
            serde_json::from_str(include_str!("../../../../fixtures/geodesy/v1/text.json"))
                .expect("the cases read");
        let mut read = 0;
        for case in file["reads"].as_array().expect("reads") {
            let Some(want) = case.get("expect") else {
                continue;
            };
            let name = want["name"].as_str().expect("a name");
            let system: core::System = core_json(&want["system"]);
            let d = definition_from(name, &system).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(definition_system(&d), Some(system), "{}", case["name"]);
            read += 1;
        }
        assert!(read >= 19);
        // A local system on a base the registry does not have: the base is a definition of its own.
        let base = core::System::Tm {
            datum: core::Datum::Turef,
            latitude_of_origin: None,
            central_meridian: 30.0,
            scale_factor: 1.0,
            false_easting: 400_000.0,
            false_northing: 0.0,
        };
        let local = core::System::Local {
            base: Box::new(base.clone()),
            plane: core::Plane::Similarity {
                east: 1.0,
                north: 2.0,
                rotation: 0.5,
                scale: 1.0,
            },
        };
        let d = definition_from("Şantiye", &local).expect("a definition");
        let CrsSystem::Local(l) = &d.system else {
            panic!("a local system");
        };
        assert_eq!(l.base.srid, None);
        assert_eq!(
            l.base.definition.as_ref().map(|b| b.name.as_str()),
            Some("Şantiye tabanı")
        );
        assert_eq!(definition_system(&d), Some(local));
        assert_eq!(definition_from("Web", &core::System::Mercator {}), None);
    }
}
