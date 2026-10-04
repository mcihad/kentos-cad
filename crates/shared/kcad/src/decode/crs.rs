//! Schema 13's coordinate system definitions and datum choices in the
//! settings (docs/specs/kcad-v2.md §6.4.1, docs/adr/0168): every field read
//! and checked as the writer writes it (`kentos_contracts::crs`'s rules); a
//! key of another kind is unknown, a value against the rules `bad_value`.

use kentos_contracts::crs::choices_problem;
use kentos_contracts::{
    Convention, CrsBase, CrsDefinition, CrsPlane, CrsSystem, CustomDatum, DatumTransform,
    Ellipsoid, GeographicDefinition, GridChoice, Helmert, LocalDefinition, RegistryDatum,
    TmDefinition,
};

use super::{floats, list, map, named, required, text, unknown};
use crate::cbor::{Reader, Seg};
use crate::error::{Code, KcadError};

const REGISTRY: [(&str, RegistryDatum); 3] = [
    ("TUREF", RegistryDatum::Turef),
    ("ED50", RegistryDatum::Ed50),
    ("WGS84", RegistryDatum::Wgs84),
];

/// A definition (`name`, `system`), checked whole.
pub(super) fn crs_definition(r: &mut Reader<'_>) -> Result<CrsDefinition, KcadError> {
    let at = r.position();
    let d = definition(r)?;
    match d.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(d),
    }
}

fn definition(r: &mut Reader<'_>) -> Result<CrsDefinition, KcadError> {
    let (mut name, mut system) = (None, None);
    map(r, |r, key| {
        match key {
            "name" => name = Some(text(r)?),
            "system" => system = Some(crs_system(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(CrsDefinition {
        name: required(r, name, "name")?,
        system: required(r, system, "system")?,
    })
}

/// Three numbers.
fn three(r: &mut Reader<'_>) -> Result<[f64; 3], KcadError> {
    let at = r.position();
    let v = floats(r)?;
    <[f64; 3]>::try_from(v).map_err(|v| {
        r.fail_at(
            Code::BadValue,
            at,
            &format!("3 sayı olmalı, {} var", v.len()),
        )
    })
}

fn crs_system(r: &mut Reader<'_>) -> Result<CrsSystem, KcadError> {
    #[derive(Default)]
    struct Fields {
        kind: Option<&'static str>,
        datum: Option<RegistryDatum>,
        custom_datum: Option<CustomDatum>,
        lat0: Option<f64>,
        cm: Option<f64>,
        k: Option<f64>,
        fe: Option<f64>,
        fnorth: Option<f64>,
        base: Option<CrsBase>,
        plane: Option<CrsPlane>,
        seen: Vec<&'static str>,
    }
    let mut f = Fields::default();
    map(r, |r, key| {
        match key {
            "kind" => {
                f.kind = Some(named(
                    r,
                    &[
                        ("tm", "tm"),
                        ("geographic", "geographic"),
                        ("local", "local"),
                    ],
                )?)
            }
            "datum" => f.datum = Some(named(r, &REGISTRY)?),
            "customDatum" => f.custom_datum = Some(custom_datum(r)?),
            "latitudeOfOrigin" => f.lat0 = Some(r.float()?),
            "centralMeridian" => f.cm = Some(r.float()?),
            "scaleFactor" => f.k = Some(r.float()?),
            "falseEasting" => f.fe = Some(r.float()?),
            "falseNorthing" => f.fnorth = Some(r.float()?),
            "base" => f.base = Some(crs_base(r)?),
            "plane" => f.plane = Some(crs_plane(r)?),
            _ => return Err(unknown(r)),
        }
        f.seen.push(match key {
            "kind" => "kind",
            "datum" => "datum",
            "customDatum" => "customDatum",
            "latitudeOfOrigin" => "latitudeOfOrigin",
            "centralMeridian" => "centralMeridian",
            "scaleFactor" => "scaleFactor",
            "falseEasting" => "falseEasting",
            "falseNorthing" => "falseNorthing",
            "base" => "base",
            _ => "plane",
        });
        Ok(())
    })?;
    let kind = required(r, f.kind, "kind")?;
    let allowed: &[&str] = match kind {
        "tm" => &[
            "kind",
            "datum",
            "customDatum",
            "latitudeOfOrigin",
            "centralMeridian",
            "scaleFactor",
            "falseEasting",
            "falseNorthing",
        ],
        "geographic" => &["kind", "datum", "customDatum"],
        _ => &["kind", "base", "plane"],
    };
    if let Some(&stray) = f.seen.iter().find(|k| !allowed.contains(k)) {
        r.push(Seg::Name(stray));
        let e = r.fail(
            Code::UnknownField,
            &format!("“{kind}” sisteminde bu alan olmaz"),
        );
        r.pop();
        return Err(e);
    }
    Ok(match kind {
        "tm" => CrsSystem::Tm(TmDefinition {
            datum: f.datum,
            custom_datum: f.custom_datum.map(Box::new),
            latitude_of_origin: f.lat0,
            central_meridian: required(r, f.cm, "centralMeridian")?,
            scale_factor: required(r, f.k, "scaleFactor")?,
            false_easting: required(r, f.fe, "falseEasting")?,
            false_northing: required(r, f.fnorth, "falseNorthing")?,
        }),
        "geographic" => CrsSystem::Geographic(GeographicDefinition {
            datum: f.datum,
            custom_datum: f.custom_datum.map(Box::new),
        }),
        _ => CrsSystem::Local(LocalDefinition {
            base: required(r, f.base, "base")?,
            plane: required(r, f.plane, "plane")?,
        }),
    })
}

fn custom_datum(r: &mut Reader<'_>) -> Result<CustomDatum, KcadError> {
    let (mut name, mut ellipsoid, mut to_wgs84) = (None, None, None);
    map(r, |r, key| {
        match key {
            "name" => name = Some(text(r)?),
            "toWgs84" => to_wgs84 = Some(helmert(r)?),
            "ellipsoid" => {
                let (mut n, mut a, mut rf) = (None, None, None);
                map(r, |r, key| {
                    match key {
                        "name" => n = Some(text(r)?),
                        "semiMajor" => a = Some(r.float()?),
                        "inverseFlattening" => rf = Some(r.float()?),
                        _ => return Err(unknown(r)),
                    }
                    Ok(())
                })?;
                ellipsoid = Some(Ellipsoid {
                    name: required(r, n, "name")?,
                    semi_major: required(r, a, "semiMajor")?,
                    inverse_flattening: required(r, rf, "inverseFlattening")?,
                });
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(CustomDatum {
        name: required(r, name, "name")?,
        ellipsoid: required(r, ellipsoid, "ellipsoid")?,
        to_wgs84,
    })
}

fn helmert(r: &mut Reader<'_>) -> Result<Helmert, KcadError> {
    let (mut scale, mut accuracy, mut rotation, mut convention, mut translation) =
        (None, None, None, None, None);
    map(r, |r, key| {
        match key {
            "scale" => scale = Some(r.float()?),
            "accuracy" => accuracy = Some(r.float()?),
            "rotation" => rotation = Some(three(r)?),
            "convention" => {
                convention = Some(named(
                    r,
                    &[
                        ("positionVector", Convention::PositionVector),
                        ("coordinateFrame", Convention::CoordinateFrame),
                    ],
                )?)
            }
            "translation" => translation = Some(three(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(Helmert {
        translation: required(r, translation, "translation")?,
        rotation: required(r, rotation, "rotation")?,
        scale: required(r, scale, "scale")?,
        convention: required(r, convention, "convention")?,
        accuracy,
    })
}

fn crs_base(r: &mut Reader<'_>) -> Result<CrsBase, KcadError> {
    let (mut srid, mut definition_) = (None, None);
    map(r, |r, key| {
        match key {
            "srid" => srid = Some(r.uint(u64::from(u32::MAX))? as u32),
            "definition" => definition_ = Some(Box::new(definition(r)?)),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(CrsBase {
        srid,
        definition: definition_,
    })
}

fn crs_plane(r: &mut Reader<'_>) -> Result<CrsPlane, KcadError> {
    let mut v = [None; 6];
    let (mut east, mut north, mut rotation, mut scale, mut kind) = (None, None, None, None, None);
    map(r, |r, key| {
        match key {
            "a" | "b" | "c" | "d" | "e" | "f" => {
                v[usize::from(key.as_bytes()[0] - b'a')] = Some(r.float()?)
            }
            "east" => east = Some(r.float()?),
            "north" => north = Some(r.float()?),
            "rotation" => rotation = Some(r.float()?),
            "scale" => scale = Some(r.float()?),
            "kind" => kind = Some(named(r, &[("similarity", false), ("affine", true)])?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let affine = required(r, kind, "kind")?;
    let similarity_keys = [east, north, rotation, scale].iter().any(Option::is_some);
    if (affine && similarity_keys) || (!affine && v.iter().any(Option::is_some)) {
        return Err(r.fail(
            Code::UnknownField,
            "düzlem dönüşümünde türünün olmayan bir alanı var",
        ));
    }
    Ok(if affine {
        let get = |i: usize, k: &'static str, r: &mut Reader<'_>| required(r, v[i], k);
        CrsPlane::Affine {
            a: get(0, "a", r)?,
            b: get(1, "b", r)?,
            c: get(2, "c", r)?,
            d: get(3, "d", r)?,
            e: get(4, "e", r)?,
            f: get(5, "f", r)?,
        }
    } else {
        CrsPlane::Similarity {
            east: required(r, east, "east")?,
            north: required(r, north, "north")?,
            rotation: required(r, rotation, "rotation")?,
            scale: required(r, scale, "scale")?,
        }
    })
}

/// The datum choices, each pair at most once.
pub(super) fn datum_transforms(r: &mut Reader<'_>) -> Result<Vec<DatumTransform>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| datum_transform(r))?;
    match choices_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}

fn datum_transform(r: &mut Reader<'_>) -> Result<DatumTransform, KcadError> {
    let (mut to, mut from, mut grid, mut name, mut helmert_) = (None, None, None, None, None);
    map(r, |r, key| {
        match key {
            "to" => to = Some(named(r, &REGISTRY)?),
            "from" => from = Some(named(r, &REGISTRY)?),
            "name" => name = Some(text(r)?),
            "helmert" => helmert_ = Some(helmert(r)?),
            "grid" => {
                let (mut id, mut file, mut size, mut accuracy) = (None, None, None, None);
                map(r, |r, key| {
                    match key {
                        "id" => id = Some(text(r)?),
                        "file" => file = Some(text(r)?),
                        "size" => size = Some(r.uint(u64::MAX)?),
                        "accuracy" => accuracy = Some(r.float()?),
                        _ => return Err(unknown(r)),
                    }
                    Ok(())
                })?;
                grid = Some(GridChoice {
                    id: required(r, id, "id")?,
                    file: required(r, file, "file")?,
                    size: required(r, size, "size")?,
                    accuracy,
                });
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(DatumTransform {
        from: required(r, from, "from")?,
        to: required(r, to, "to")?,
        name: required(r, name, "name")?,
        helmert: helmert_,
        grid,
    })
}
