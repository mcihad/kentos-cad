//! The vertical system a file's coordinate system statement names (docs/adr/0631 §4): the heights' half of a
//! compound system in WKT 1 (`COMPD_CS[…, VERT_CS[…]]`, a `.prj`, a GeoPackage's definition; ESRI's `VERTCS[…]`), WKT 2 (`VERTCRS`,
//! `VERTICALCRS`), PROJJSON (a `VerticalCRS`, alone or among a `CompoundCRS`'s components, GeoParquet's) and an OGC
//! compound URN or a `+`-joined EPSG pair (GML's `srsName`). Its name and EPSG code when the statement gives one;
//! nothing is guessed from a name. The import commands say it; the heights are taken as they are.

use serde_json::Value;

/// A file's vertical system: its EPSG code when the statement gives one, its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerticalCrs {
    pub srid: Option<u32>,
    pub name: String,
}

/// The vertical system `text` names, or none.
pub fn vertical_of(text: &str) -> Option<VerticalCrs> {
    let t = text.trim();
    if t.starts_with('{') {
        let v: Value = serde_json::from_str(t).ok()?;
        return projjson(&v);
    }
    if let Some(v) = urn(t) {
        return Some(v);
    }
    for key in [
        "VERT_CS[",
        "VERTCRS[",
        "VERTICALCRS[",
        "VERTCS[",
        "VERT_CS(",
        "VERTCRS(",
    ] {
        if let Some(at) = find_key(t, key) {
            return wkt_block(&t[at + key.len() - 1..]);
        }
    }
    None
}

/// Where `key` starts as a WKT keyword (not inside a quoted name), without case.
fn find_key(t: &str, key: &str) -> Option<usize> {
    let upper = t.to_ascii_uppercase();
    let mut quoted = false;
    let bytes = upper.as_bytes();
    let k = key.as_bytes();
    let mut i = 0;
    while i + k.len() <= bytes.len() {
        if bytes[i] == b'"' {
            quoted = !quoted;
        } else if !quoted && &bytes[i..i + k.len()] == k {
            // A keyword starts after a delimiter: `VERT_CS` is not the tail of another word.
            let before = i.checked_sub(1).map(|j| bytes[j]);
            if before.is_none_or(|b| !(b.is_ascii_alphanumeric() || b == b'_')) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// A WKT block from its opening bracket: its first quoted name and the EPSG code of its own (outermost) authority.
fn wkt_block(t: &str) -> Option<VerticalCrs> {
    let open = t.chars().next()?;
    let close = if open == '(' { ')' } else { ']' };
    // The block's text up to its matching bracket.
    let (mut depth, mut quoted, mut end) = (0i32, false, None);
    for (i, c) in t.char_indices() {
        match c {
            '"' => quoted = !quoted,
            c if !quoted && (c == '[' || c == '(') => depth += 1,
            c if !quoted && (c == ']' || c == ')') => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let _ = close;
    let block = &t[1..end?];
    let name = {
        let s = block.find('"')?;
        let e = block[s + 1..].find('"')? + s + 1;
        block[s + 1..e].to_owned()
    };
    // The block's own authority is at its own depth: AUTHORITY["EPSG","5775"] or ID["EPSG",5775].
    let mut depth = 0i32;
    let mut quoted = false;
    let mut srid = None;
    let up = block.to_ascii_uppercase();
    let bytes = up.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'"' {
            quoted = !quoted;
        } else if !quoted && (c == b'[' || c == b'(') {
            depth += 1;
        } else if !quoted && (c == b']' || c == b')') {
            depth -= 1;
        } else if !quoted && depth == 0 {
            for key in ["AUTHORITY[", "ID["] {
                if up[i..].starts_with(key)
                    && i.checked_sub(1)
                        .map(|j| bytes[j])
                        .is_none_or(|b| !(b.is_ascii_alphanumeric() || b == b'_'))
                {
                    let args = &block[i + key.len()..];
                    let args = &args[..args.find(']').unwrap_or(args.len())];
                    let mut parts = args.split(',').map(|p| p.trim().trim_matches('"'));
                    if parts.next().is_some_and(|a| a.eq_ignore_ascii_case("EPSG")) {
                        srid = parts.next().and_then(|c| c.parse::<u32>().ok());
                    }
                }
            }
        }
        i += 1;
    }
    Some(VerticalCrs { srid, name })
}

/// A vertical system of a PROJJSON value: itself, or a compound system's vertical component.
fn projjson(v: &Value) -> Option<VerticalCrs> {
    match v.get("type").and_then(Value::as_str) {
        Some("VerticalCRS") => {
            let name = v
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let id = v.get("id");
            let srid = id
                .filter(|id| id.get("authority").and_then(Value::as_str) == Some("EPSG"))
                .and_then(|id| id.get("code"))
                .and_then(|c| {
                    c.as_u64()
                        .or_else(|| c.as_str().and_then(|s| s.parse().ok()))
                })
                .and_then(|c| u32::try_from(c).ok());
            Some(VerticalCrs { srid, name })
        }
        Some("CompoundCRS") => v
            .get("components")
            .and_then(Value::as_array)?
            .iter()
            .find_map(projjson),
        _ => None,
    }
}

/// An OGC compound URN (`urn:ogc:def:crs,crs:EPSG::5256,crs:EPSG::5775`) or a `+`-joined pair (`EPSG:5256+5775`):
/// its second code, named by KentOS's vertical systems when it knows it.
fn urn(t: &str) -> Option<VerticalCrs> {
    let low = t.to_ascii_lowercase();
    let code = if low.starts_with("urn:ogc:def:crs,") {
        let parts: Vec<&str> = t.split(',').collect();
        parts
            .get(2)?
            .rsplit(':')
            .next()?
            .trim()
            .parse::<u32>()
            .ok()?
    } else if low.starts_with("epsg:") && t.contains('+') {
        t.split('+').nth(1)?.trim().parse::<u32>().ok()?
    } else {
        return None;
    };
    let name = kentos_contracts::vertical_system(code)
        .map_or_else(|| format!("EPSG:{code}"), |v| v.name.to_owned());
    Some(VerticalCrs {
        srid: Some(code),
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_heights_half_of_a_statement_is_found() {
        let wkt1 = r#"COMPD_CS["TUREF / TM36 + Antalya height",PROJCS["TUREF / TM36",GEOGCS["TUREF",AUTHORITY["EPSG","5252"]],AUTHORITY["EPSG","5256"]],VERT_CS["Antalya height",VERT_DATUM["Antalya",2005,AUTHORITY["EPSG","5175"]],UNIT["metre",1],AUTHORITY["EPSG","5775"]]]"#;
        assert_eq!(
            vertical_of(wkt1),
            Some(VerticalCrs {
                srid: Some(5775),
                name: "Antalya height".into()
            })
        );
        let wkt2 = r#"COMPOUNDCRS["x",PROJCRS["TUREF / TM36",ID["EPSG",5256]],VERTCRS["EGM2008 height",VDATUM["EGM2008 geoid",ID["EPSG",1027]],ID["EPSG",3855]]]"#;
        assert_eq!(vertical_of(wkt2).and_then(|v| v.srid), Some(3855));
        assert_eq!(
            vertical_of("urn:ogc:def:crs,crs:EPSG::5256,crs:EPSG::5775"),
            Some(VerticalCrs {
                srid: Some(5775),
                name: "Antalya height".into()
            })
        );
        assert_eq!(
            vertical_of("EPSG:5256+3855").and_then(|v| v.srid),
            Some(3855)
        );
        let pj = r#"{"type":"CompoundCRS","components":[{"type":"ProjectedCRS","name":"a"},{"type":"VerticalCRS","name":"MSL height","id":{"authority":"EPSG","code":5714}}]}"#;
        assert_eq!(vertical_of(pj).and_then(|v| v.srid), Some(5714));
        assert_eq!(
            vertical_of(r#"PROJCS["VERT_CS look-alike",AUTHORITY["EPSG","5256"]]"#),
            None
        );
        assert_eq!(vertical_of("EPSG:5256"), None);
        let esri = r#"PROJCS["TUREF_TM36",GEOGCS["GCS_TUREF",DATUM["D_Turkish_National_Reference_Frame",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],UNIT["Meter",1.0]],VERTCS["Antalya",VDATUM["Antalya"],PARAMETER["Vertical_Shift",0.0],PARAMETER["Direction",1.0],UNIT["Meter",1.0]]"#;
        assert_eq!(
            vertical_of(esri),
            Some(VerticalCrs {
                srid: None,
                name: "Antalya".into()
            })
        );
    }
}
