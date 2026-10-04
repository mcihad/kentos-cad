//! GNSS files (docs/adr/0169 §1, §6): a receiver's positions in WGS 84,
//! from GPX 1.1 ([`gpx`]) or an NMEA 0183 log ([`nmea`]), told by their
//! content ([`read`]): a file whose first character that is not blank is
//! `<` is read as GPX, any other as NMEA (whose reader passes over lines
//! without a sentence). The positions are moved into the project's system
//! by the app (docs/adr/0167), not here.

use kentos_contracts::GnssRead;

use crate::text;

pub mod gpx;
pub mod nmea;

/// Reads a GNSS file: GPX when it begins with `<`, else NMEA.
pub fn read(bytes: &[u8]) -> GnssRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc);
    if body.trim_start().starts_with('<') {
        gpx::read(bytes)
    } else {
        nmea::read(bytes)
    }
}
