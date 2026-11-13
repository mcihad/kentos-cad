//! What the writers into WGS 84 share (GeoJSON, KML, GPX; docs/adr/0222): the system they write, the decimals of a
//! degree, XML's escaped text and the objects WGS 84 does not reach, said by why. Its own module, not KML's, so
//! the GeoJSON and GPX writers build without the KML codec (docs/adr/8054 §11).

use kentos_geometry_core::crs::{Datum, System, Unreached};

use crate::report::Report;

/// WGS 84 longitude and latitude.
pub(crate) const WGS84: System = System::Geographic {
    datum: Datum::Wgs84,
};

/// Decimals of a longitude or latitude (1e-10° is about 0.01 mm).
pub(crate) const DEGREE_DECIMALS: usize = 10;

/// Text in an XML element or attribute.
pub(crate) fn escape(t: &str, out: &mut String) {
    for c in t.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // XML 1.0 has no other control characters.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
}

/// One more of `u` among the objects left out.
pub(crate) fn tally(list: &mut Vec<(Unreached, u32)>, u: Unreached) {
    match list.iter_mut().find(|(w, _)| *w == u) {
        Some((_, n)) => *n += 1,
        None => list.push((u, 1)),
    }
}

/// The objects one of whose vertices WGS 84 does not reach, said by why.
pub(crate) fn say_unreached(rep: &mut Report, mut list: Vec<(Unreached, u32)>) {
    list.sort_by_key(|(u, _)| u.as_str());
    for (u, n) in list {
        rep.skip_n(
            "Çevrilemeyen nesne",
            &format!(
                "bir köşesi WGS 84'e çevrilemedi ({}); yazılmadı",
                crate::reproject::why(u)
            ),
            0,
            n,
        );
    }
}
