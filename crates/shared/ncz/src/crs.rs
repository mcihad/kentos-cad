//! What an NCZ says of its coordinate system: its MPROJ block's datum,
//! projection and zone, and the SRS its TILED_XML block names. The import
//! window shows it as the answer to its question and the user confirms or
//! changes it; nothing is transformed (CLAUDE.md §5).
//!
//! Only what the bytes say for certain becomes an EPSG code: a three-degree
//! zone's byte is its central meridian (the Suşehri plan's 39 comes with
//! `SRS=5257`, TUREF / TM39), and a geographic declaration is the datum's
//! longitude and latitude. What a six-degree zone's byte means has not been
//! seen in a file, so such a statement is shown as the file writes it, with
//! no code: a statement is never guessed into a system. (A map sheet's frame
//! reads it as the UTM zone number, the owner's reading, and keeps the file's
//! box unless the result is a cell of a sheet grid: `sheet`.)

use kentos_contracts::{CrsSource, DeclaredCrs};

use crate::format::Header;

/// The TM zones' central meridians, in the registry's order.
const TM_MERIDIANS: [u8; 7] = [27, 30, 33, 36, 39, 42, 45];

/// The EPSG code the statement names, when it names one for certain.
fn srid(h: &Header) -> Option<u32> {
    if h.mproj {
        let zone = TM_MERIDIANS
            .iter()
            .position(|&m| m == h.zone)
            .map(|i| i as u32);
        return match (h.datum, h.projection) {
            // ITRF is TUREF's frame (ITRF96).
            (1, 3) => zone.map(|i| 5253 + i),
            (1, 1) => Some(5252),
            (4 | 254, 3) => zone.map(|i| 2319 + i),
            (0, 1) => Some(4326),
            _ => None,
        };
    }
    // `SRS=5257`, as the reference cleans `SRS="5257"`.
    let digits: String = h
        .epsg
        .split(['=', ':'])
        .next_back()?
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok().filter(|&n: &u32| n > 0)
}

/// The statement in Turkish: “ITRF, 3° dilim, orta meridyen 39° (TILED_XML: SRS=5257)”.
fn statement(h: &Header) -> String {
    let mut out = String::new();
    if h.mproj {
        out.push_str(match h.datum {
            0 => "WGS-84",
            1 => "ITRF",
            4 => "ED50",
            254 => "ED50 (HGK)",
            _ => "tanımsız datum",
        });
        match h.projection {
            1 => out.push_str(", coğrafi (enlem, boylam)"),
            3 => out.push_str(&format!(", 3° dilim, orta meridyen {}°", h.zone)),
            2 => out.push_str(&format!(", 6° dilim, dilim bilgisi {}", h.zone)),
            _ => out.push_str(", tanımsız projeksiyon"),
        }
    }
    if !h.epsg.is_empty() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&format!("(TILED_XML: {})", h.epsg.trim()));
    }
    out
}

/// The file's statement, or none when it makes none.
pub fn declared(h: &Header) -> Option<DeclaredCrs> {
    let text = statement(h);
    (!text.is_empty()).then(|| DeclaredCrs {
        srid: srid(h),
        text,
        source: CrsSource::Ncz,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mproj(datum: u8, projection: u8, zone: u8) -> Header {
        Header {
            mproj: true,
            datum,
            projection,
            zone,
            ..Header::default()
        }
    }

    #[test]
    fn three_degree_zones_and_geographic_systems_are_named() {
        assert_eq!(srid(&mproj(1, 3, 39)), Some(5257));
        assert_eq!(srid(&mproj(1, 3, 27)), Some(5253));
        assert_eq!(srid(&mproj(4, 3, 36)), Some(2322));
        assert_eq!(srid(&mproj(1, 1, 0)), Some(5252));
        assert_eq!(srid(&mproj(0, 1, 0)), Some(4326));
        // A meridian no zone has, a six-degree zone: said, never guessed.
        assert_eq!(srid(&mproj(1, 3, 38)), None);
        assert_eq!(srid(&mproj(0, 2, 36)), None);
    }

    #[test]
    fn the_tiled_xml_code_counts_only_without_mproj() {
        let mut h = Header {
            epsg: "SRS=5257".into(),
            ..Header::default()
        };
        assert_eq!(srid(&h), Some(5257));
        assert_eq!(
            declared(&h).map(|d| d.text),
            Some("(TILED_XML: SRS=5257)".to_owned())
        );
        // The Sivas UİP: MPROJ says TM36, TILED_XML a code of its own.
        h.mproj = true;
        h.datum = 1;
        h.projection = 3;
        h.zone = 36;
        h.epsg = "SRS=7934".into();
        let d = declared(&h).expect("declared");
        assert_eq!(d.srid, Some(5256));
        assert_eq!(
            d.text,
            "ITRF, 3° dilim, orta meridyen 36° (TILED_XML: SRS=7934)"
        );
        assert!(declared(&Header::default()).is_none());
    }
}
