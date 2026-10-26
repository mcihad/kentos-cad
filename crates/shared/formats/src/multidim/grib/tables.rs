//! GRIB parameters and levels named (docs/adr/0645 §4): the common ones of WMO's GRIB2 code table 4.2, GRIB1's WMO
//! table 2 and ECMWF's table 128 by their short names (ERA5's NetCDF names where they are known: `t2m`, `u10`); any
//! other as its numbers (`p0_1_8`, `var61`). Levels by WMO's code tables 4.5 and GRIB1's table 3.

use super::Message;
use crate::multidim::netcdf::Attr;

/// A parameter's short name, long name and units, by its number in its table.
type Names<K> = (K, &'static str, &'static str, &'static str);

/// GRIB2: (discipline, category, number) → short name, long name, units.
const G2: &[Names<(u8, u8, u8)>] = &[
    ((0, 0, 0), "t", "Temperature", "K"),
    ((0, 0, 2), "pt", "Potential temperature", "K"),
    ((0, 0, 4), "tmax", "Maximum temperature", "K"),
    ((0, 0, 5), "tmin", "Minimum temperature", "K"),
    ((0, 0, 6), "dpt", "Dew point temperature", "K"),
    ((0, 0, 17), "skt", "Skin temperature", "K"),
    ((0, 1, 0), "q", "Specific humidity", "kg kg-1"),
    ((0, 1, 1), "r", "Relative humidity", "%"),
    ((0, 1, 3), "pwat", "Precipitable water", "kg m-2"),
    ((0, 1, 7), "prate", "Precipitation rate", "kg m-2 s-1"),
    ((0, 1, 8), "tp", "Total precipitation", "kg m-2"),
    ((0, 1, 11), "sde", "Snow depth", "m"),
    (
        (0, 1, 13),
        "sd",
        "Water equivalent of accumulated snow depth",
        "kg m-2",
    ),
    ((0, 2, 0), "wdir", "Wind direction", "degree true"),
    ((0, 2, 1), "ws", "Wind speed", "m s-1"),
    ((0, 2, 2), "u", "U component of wind", "m s-1"),
    ((0, 2, 3), "v", "V component of wind", "m s-1"),
    ((0, 2, 8), "w", "Vertical velocity (pressure)", "Pa s-1"),
    ((0, 2, 22), "gust", "Wind speed (gust)", "m s-1"),
    ((0, 3, 0), "pres", "Pressure", "Pa"),
    ((0, 3, 1), "prmsl", "Pressure reduced to MSL", "Pa"),
    ((0, 3, 4), "z", "Geopotential", "m2 s-2"),
    ((0, 3, 5), "gh", "Geopotential height", "gpm"),
    (
        (0, 4, 7),
        "dswrf",
        "Downward short-wave radiation flux",
        "W m-2",
    ),
    ((0, 6, 1), "tcc", "Total cloud cover", "%"),
    (
        (0, 7, 6),
        "cape",
        "Convective available potential energy",
        "J kg-1",
    ),
    ((0, 19, 0), "vis", "Visibility", "m"),
    (
        (2, 0, 0),
        "lsm",
        "Land cover (1 = land, 0 = sea)",
        "Proportion",
    ),
    (
        (10, 0, 3),
        "swh",
        "Significant height of combined wind waves and swell",
        "m",
    ),
];

/// GRIB1: (table, parameter) → short name, long name, units (WMO's table 2 for versions 1–3, ECMWF's 128).
const G1: &[Names<(u8, u8)>] = &[
    ((2, 1), "pres", "Pressure", "Pa"),
    ((2, 2), "prmsl", "Pressure reduced to MSL", "Pa"),
    ((2, 6), "z", "Geopotential", "m2 s-2"),
    ((2, 7), "gh", "Geopotential height", "gpm"),
    ((2, 11), "t", "Temperature", "K"),
    ((2, 17), "dpt", "Dew point temperature", "K"),
    ((2, 33), "u", "U component of wind", "m s-1"),
    ((2, 34), "v", "V component of wind", "m s-1"),
    ((2, 51), "q", "Specific humidity", "kg kg-1"),
    ((2, 52), "r", "Relative humidity", "%"),
    ((2, 61), "tp", "Total precipitation", "kg m-2"),
    ((2, 71), "tcc", "Total cloud cover", "%"),
    ((2, 81), "lsm", "Land-sea mask", "Proportion"),
    ((128, 129), "z", "Geopotential", "m2 s-2"),
    ((128, 130), "t", "Temperature", "K"),
    ((128, 131), "u", "U component of wind", "m s-1"),
    ((128, 132), "v", "V component of wind", "m s-1"),
    ((128, 133), "q", "Specific humidity", "kg kg-1"),
    ((128, 134), "sp", "Surface pressure", "Pa"),
    ((128, 141), "sd", "Snow depth", "m of water equivalent"),
    ((128, 151), "msl", "Mean sea level pressure", "Pa"),
    ((128, 157), "r", "Relative humidity", "%"),
    ((128, 164), "tcc", "Total cloud cover", "(0 - 1)"),
    ((128, 165), "u10", "10 metre U wind component", "m s-1"),
    ((128, 166), "v10", "10 metre V wind component", "m s-1"),
    ((128, 167), "t2m", "2 metre temperature", "K"),
    ((128, 168), "d2m", "2 metre dewpoint temperature", "K"),
    (
        (128, 169),
        "ssrd",
        "Surface short-wave (solar) radiation downwards",
        "J m-2",
    ),
    ((128, 172), "lsm", "Land-sea mask", "(0 - 1)"),
    ((128, 228), "tp", "Total precipitation", "m"),
    ((128, 235), "skt", "Skin temperature", "K"),
];

/// A field's name and attributes.
pub fn describe(edition: u8, m: &Message) -> (String, Vec<(String, Attr)>) {
    let found = if edition == 1 {
        let table = if m.table <= 3 { 2 } else { m.table };
        G1.iter()
            .find(|(k, ..)| *k == (table, m.number))
            .map(|(_, s, l, u)| (*s, *l, *u))
    } else {
        G2.iter()
            .find(|(k, ..)| *k == (m.discipline, m.category, m.number))
            .map(|(_, s, l, u)| (*s, *l, *u))
    };
    let param = if edition == 1 {
        format!("{}.{}", m.table, m.number)
    } else {
        format!("{}.{}.{}", m.discipline, m.category, m.number)
    };
    let (name, long, units) = match found {
        Some((s, l, u)) => (s.to_owned(), l.to_owned(), u.to_owned()),
        None if edition == 1 => (
            format!("var{}_{}", m.table, m.number),
            format!("GRIB1 parameter {param}"),
            String::new(),
        ),
        None => (
            format!("p{}_{}_{}", m.discipline, m.category, m.number),
            format!("GRIB2 parameter {param}"),
            String::new(),
        ),
    };
    let mut attrs = vec![("long_name".to_owned(), Attr::Text(long))];
    if !units.is_empty() {
        attrs.push(("units".into(), Attr::Text(units)));
    }
    attrs.push(("GRIB_parameter".into(), Attr::Text(param)));
    if let Some(s) = m.stat {
        let what = match s {
            0 => "mean",
            1 => "accumulation",
            2 => "maximum",
            3 => "minimum",
            _ => "statistic",
        };
        attrs.push(("cell_methods".into(), Attr::Text(format!("time: {what}"))));
    }
    (name, attrs)
}

/// A level type's dimension name, units and CF `positive`.
pub fn level_name(edition: u8, t: u16) -> (&'static str, &'static str, Option<&'static str>) {
    match (edition, t) {
        (2, 100) => ("isobaric", "Pa", Some("down")),
        (1, 100) => ("isobaric", "hPa", Some("down")),
        (2, 103) | (1, 105) => ("height", "m", Some("up")),
        (2, 102) | (1, 103) => ("altitude", "m", Some("up")),
        (2, 106) | (1, 111) | (1, 112) => ("depth", "m", Some("down")),
        (2, 105) | (1, 109) => ("hybrid", "1", Some("down")),
        (2, 107) | (1, 113) => ("isentropic", "K", Some("up")),
        _ => ("level", "1", None),
    }
}

/// A one-level variable's name with its level (`t_2m`, `t_850hPa`); surface and sea-level fields keep their name.
pub fn with_level(name: &str, edition: u8, t: u16, v: f64) -> String {
    let n = |x: f64| {
        if x.fract() == 0.0 {
            format!("{}", x as i64)
        } else {
            format!("{x}")
        }
    };
    match (edition, t) {
        (2, 100) => format!("{name}_{}hPa", n(v / 100.0)),
        (1, 100) => format!("{name}_{}hPa", n(v)),
        (2, 103) | (1, 105) | (2, 102) | (1, 103) => format!("{name}_{}m", n(v)),
        (2, 106) => format!("{name}_{}m_depth", n(v)),
        (_, 1 | 101 | 102 | 255 | 200) => name.to_owned(),
        _ => format!("{name}_l{t}_{}", n(v)),
    }
}
