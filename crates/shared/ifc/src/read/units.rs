//! The project's units and its map conversion (docs/adr/8061 §11.1): read from the records by the file's own schema's
//! attribute names, after the second pass.

use ifc_lite_core::{AttributeValue, EntityDecoder, SchemaVersion};

use super::graph::named;
use super::{Location, Unit, Units};

/// The project's units (`UnitsInContext`): length, area, volume and plane angle.
pub(super) fn units(decoder: &mut EntityDecoder, schema: SchemaVersion, project: u32) -> Units {
    let mut out = Units::default();
    let Ok(p) = decoder.decode_by_id(project) else {
        return out;
    };
    let Some(assignment) =
        named(schema, &p, "UnitsInContext").and_then(AttributeValue::as_entity_ref)
    else {
        return out;
    };
    let Ok(a) = decoder.decode_by_id(assignment) else {
        return out;
    };
    let refs: Vec<u32> = a
        .get(0)
        .and_then(AttributeValue::as_list)
        .unwrap_or(&[])
        .iter()
        .filter_map(AttributeValue::as_entity_ref)
        .collect();
    for r in refs {
        let Some((kind, unit)) = unit(decoder, schema, r, 0) else {
            continue;
        };
        let slot = match kind.as_str() {
            "LENGTHUNIT" => &mut out.length,
            "AREAUNIT" => &mut out.area,
            "VOLUMEUNIT" => &mut out.volume,
            "PLANEANGLEUNIT" => &mut out.plane_angle,
            _ => continue,
        };
        slot.get_or_insert(unit);
    }
    out
}

/// A unit record: its unit type and the unit (an SI unit with its prefix, or a conversion through its factor).
fn unit(
    decoder: &mut EntityDecoder,
    schema: SchemaVersion,
    step: u32,
    depth: usize,
) -> Option<(String, Unit)> {
    if depth > 4 {
        return None;
    }
    let u = decoder.decode_by_id(step).ok()?;
    let kind = named(schema, &u, "UnitType")
        .and_then(AttributeValue::as_enum)?
        .to_owned();
    let name = named(schema, &u, "Name")
        .and_then(|v| v.as_enum().or_else(|| v.as_string()))?
        .to_owned();
    if let Some(factor) =
        named(schema, &u, "ConversionFactor").and_then(AttributeValue::as_entity_ref)
    {
        let m = decoder.decode_by_id(factor).ok()?;
        let value = named(schema, &m, "ValueComponent").and_then(AttributeValue::as_float)?;
        let of = named(schema, &m, "UnitComponent").and_then(AttributeValue::as_entity_ref)?;
        let (_, base) = unit(decoder, schema, of, depth + 1)?;
        return Some((
            kind,
            Unit {
                name,
                prefix: None,
                to_si: value * base.to_si,
            },
        ));
    }
    let prefix = named(schema, &u, "Prefix")
        .and_then(AttributeValue::as_enum)
        .map(str::to_owned);
    let power = match name.as_str() {
        "SQUARE_METRE" => 2,
        "CUBIC_METRE" => 3,
        _ => 1,
    };
    // The prefix's factor to the unit's power, as products: `powi` is not the same bits on every platform.
    let factor = prefix
        .as_deref()
        .map_or(1.0, ifc_lite_core::get_si_prefix_multiplier);
    let to_si = match power {
        2 => factor * factor,
        3 => factor * factor * factor,
        _ => factor,
    };
    Some((
        kind,
        Unit {
            name,
            prefix,
            to_si,
        },
    ))
}

/// The first map conversion and its target system.
pub(super) fn location(
    decoder: &mut EntityDecoder,
    schema: SchemaVersion,
    step: u32,
) -> Option<Location> {
    let c = decoder.decode_by_id(step).ok()?;
    let f = |name: &str| named(schema, &c, name).and_then(AttributeValue::as_float);
    let mut out = Location {
        eastings: f("Eastings").unwrap_or(0.0),
        northings: f("Northings").unwrap_or(0.0),
        orthogonal_height: f("OrthogonalHeight").unwrap_or(0.0),
        x_axis_abscissa: f("XAxisAbscissa"),
        x_axis_ordinate: f("XAxisOrdinate"),
        scale: f("Scale"),
        factors: match (f("FactorX"), f("FactorY"), f("FactorZ")) {
            (Some(x), Some(y), Some(z)) => Some([x, y, z]),
            _ => None,
        },
        ..Location::default()
    };
    if let Some(target) = named(schema, &c, "TargetCRS").and_then(AttributeValue::as_entity_ref)
        && let Ok(t) = decoder.decode_by_id(target)
    {
        let s = |name: &str| {
            named(schema, &t, name)
                .and_then(AttributeValue::as_string)
                .map(str::to_owned)
        };
        out.crs = s("Name");
        out.crs_description = s("Description");
        out.geodetic_datum = s("GeodeticDatum");
        out.vertical_datum = s("VerticalDatum");
        out.map_projection = s("MapProjection");
        out.map_zone = s("MapZone");
    }
    Some(out)
}
