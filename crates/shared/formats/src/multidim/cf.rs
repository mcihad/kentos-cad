//! The CF conventions a NetCDF variable is read by (docs/adr/0243 §3, §7):
//! time as `<unit> since <date>` in the Gregorian calendars, the axes of a
//! regular grid, packed values and the values that are nothing, and the
//! coordinate system a grid mapping names.

use kentos_contracts::RasterSample;
use kentos_geometry_core::time::days_from_civil;

use super::netcdf::{Attr, Header, NcType, Var};

const SECOND: f64 = 1000.0;
const DAY_MS: i64 = 86_400_000;

/// A CF time axis: moments are `origin` plus a value times `unit` (milliseconds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CfTime {
    pub unit: f64,
    pub origin: i64,
}

impl CfTime {
    /// A value's moment, milliseconds since 1970 rounded to a whole one (halves away from zero).
    pub fn moment(&self, value: f64) -> Option<f64> {
        let t = (value * self.unit).round();
        let m = self.origin as f64 + t;
        (value.is_finite() && m.is_finite() && m.abs() < 9.0e15).then_some(m)
    }
}

/// A time unit's milliseconds, by its udunits names.
fn unit_ms(word: &str) -> Option<f64> {
    Some(match word.to_ascii_lowercase().as_str() {
        "millisecond" | "milliseconds" | "msec" | "msecs" | "ms" => 1.0,
        "second" | "seconds" | "sec" | "secs" | "s" => SECOND,
        "minute" | "minutes" | "min" | "mins" => 60.0 * SECOND,
        "hour" | "hours" | "hr" | "hrs" | "h" => 3600.0 * SECOND,
        "day" | "days" | "d" => 86_400.0 * SECOND,
        _ => return None,
    })
}

/// `units` and `calendar` as a time axis; none when they are not one this reads.
pub fn cf_time(units: &str, calendar: Option<&str>) -> Option<CfTime> {
    let calendar = calendar.map(|c| c.trim().to_ascii_lowercase());
    let proleptic = match calendar.as_deref() {
        None | Some("") | Some("standard") | Some("gregorian") => false,
        Some("proleptic_gregorian") => true,
        _ => return None,
    };
    let s = units.trim();
    let lower = s.to_ascii_lowercase();
    let at = lower.find(" since ")?;
    let unit = unit_ms(s[..at].trim())?;
    let origin = origin_ms(s[at + 7..].trim())?;
    // The standard calendar is Julian before 1582-10-15: such an origin is not read.
    if !proleptic && origin < days_from_civil(1582, 10, 15) * DAY_MS {
        return None;
    }
    Some(CfTime { unit, origin })
}

/// A CF reference date: `Y-M-D`, then optionally (after a space or `T`) `h:m[:s[.f]]`, then optionally `Z`, `UTC` or a zone `±h[:mm]`.
fn origin_ms(text: &str) -> Option<i64> {
    let text = text.trim();
    if !text.is_ascii() {
        return None;
    }
    let (date, rest) = match text.find([' ', 'T']) {
        Some(i) => (&text[..i], text[i + 1..].trim()),
        None => (text, ""),
    };
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some()
        || !(1..=12).contains(&m)
        || d < 1
        || d > kentos_geometry_core::time::days_in(y, m)
    {
        return None;
    }
    let mut ms = days_from_civil(y, m, d) * DAY_MS;
    if rest.is_empty() {
        return Some(ms);
    }
    // The clock, then a zone.
    let (clock, zone) = match rest.find([' ', 'Z', 'z', '+', '-']) {
        Some(i) => (&rest[..i], rest[i..].trim()),
        None => (rest, ""),
    };
    let mut fields = clock.split(':');
    let h: i64 = fields.next()?.parse().ok()?;
    let mi: i64 = fields.next().map_or(Some(0), |f| f.parse().ok())?;
    let sec: f64 = fields.next().map_or(Some(0.0), |f| f.parse().ok())?;
    if fields.next().is_some()
        || !(0..=23).contains(&h)
        || !(0..=59).contains(&mi)
        || !(0.0..60.0).contains(&sec)
    {
        return None;
    }
    ms += h * 3_600_000 + mi * 60_000 + (sec * 1000.0).round() as i64;
    let zone = zone.trim();
    let offset = match zone.to_ascii_uppercase().as_str() {
        "" | "Z" | "UTC" | "GMT" => 0,
        z => {
            let (sign, digits) = match z.as_bytes().first()? {
                b'+' => (1, &z[1..]),
                b'-' => (-1, &z[1..]),
                _ => return None,
            };
            let (zh, zm): (i64, i64) = match digits.split_once(':') {
                Some((a, b)) => (a.parse().ok()?, b.parse().ok()?),
                None if digits.len() > 2 => (
                    digits[..digits.len() - 2].parse().ok()?,
                    digits[digits.len() - 2..].parse().ok()?,
                ),
                None => (digits.parse().ok()?, 0),
            };
            if zh > 14 || zm > 59 {
                return None;
            }
            sign * (zh * 3_600_000 + zm * 60_000)
        }
    };
    Some(ms - offset)
}

/// How a variable's stored values become what it holds (§3).
#[derive(Clone, Debug, PartialEq)]
pub struct Unpack {
    /// The type the file stores.
    pub raw: NcType,
    /// `scale_factor` and `add_offset` (1 and 0 when unpacked).
    pub scale: f64,
    pub offset: f64,
    pub packed: bool,
    /// Stored values that are nothing (`_FillValue`, `missing_value`, the default fill).
    pub fill: Vec<f64>,
    /// Stored values outside it are nothing.
    pub valid: Option<(f64, f64)>,
    /// The samples it gives.
    pub sample: RasterSample,
    /// Their nodata (a raw integer variable's fill); none: NaN (floats).
    pub nodata: Option<f64>,
}

impl Unpack {
    /// A stored value as what it holds: NaN when it is nothing (a raw integer's fill stays, its nodata).
    #[inline]
    pub fn value(&self, raw: f64) -> f64 {
        if self.sample.float() {
            if raw.is_nan()
                || self.fill.contains(&raw)
                || self.valid.is_some_and(|(lo, hi)| raw < lo || raw > hi)
            {
                return f64::NAN;
            }
            if self.packed {
                raw * self.scale + self.offset
            } else {
                raw
            }
        } else {
            raw
        }
    }
}

impl Unpack {
    /// A stored value as a raster's reader keeps it: opened (NaN when it is
    /// nothing), a 32-bit float sample rounded to 32 bits (docs/adr/0243 §9).
    #[inline]
    pub fn stored(&self, raw: f64) -> f64 {
        let v = self.value(raw);
        match self.sample {
            RasterSample::F32 => f64::from(v as f32),
            _ => v,
        }
    }
}

/// The unpacking of a variable (§3).
pub fn unpack_of(v: &Var) -> Unpack {
    let scale = v.attr("scale_factor");
    let offset = v.attr("add_offset");
    let packed = scale.is_some() || offset.is_some();
    let mut fill: Vec<f64> = ["_FillValue", "missing_value"]
        .iter()
        .filter_map(|k| v.attr(k))
        .flat_map(|a| a.numbers().to_vec())
        .collect();
    if fill.is_empty() && matches!(v.kind, NcType::Float | NcType::Double) {
        fill.push(v.kind.default_fill());
    }
    let valid = match (
        v.attr("valid_range"),
        v.attr("valid_min"),
        v.attr("valid_max"),
    ) {
        (Some(r), _, _) if r.numbers().len() >= 2 => Some((r.numbers()[0], r.numbers()[1])),
        (_, lo, hi) if lo.is_some() || hi.is_some() => Some((
            lo.and_then(Attr::number).unwrap_or(f64::NEG_INFINITY),
            hi.and_then(Attr::number).unwrap_or(f64::INFINITY),
        )),
        _ => None,
    };
    let unpacked_kind = scale.or(offset).and_then(Attr::kind);
    let sample = if packed {
        match unpacked_kind {
            Some(NcType::Float) => RasterSample::F32,
            _ => RasterSample::F64,
        }
    } else {
        match v.kind {
            NcType::Byte => RasterSample::I8,
            NcType::UByte | NcType::Char => RasterSample::U8,
            NcType::Short => RasterSample::I16,
            NcType::UShort => RasterSample::U16,
            NcType::Int => RasterSample::I32,
            NcType::UInt => RasterSample::U32,
            NcType::Float => RasterSample::F32,
            NcType::Double | NcType::Int64 | NcType::UInt64 => RasterSample::F64,
        }
    };
    // A 64-bit integer is read as a float: its fills are nothing like a float's.
    let nodata = (!sample.float()).then(|| fill.first().copied()).flatten();
    Unpack {
        raw: v.kind,
        scale: scale.and_then(Attr::number).unwrap_or(1.0),
        offset: offset.and_then(Attr::number).unwrap_or(0.0),
        packed,
        fill,
        valid,
        sample,
        nodata,
    }
}

/// An axis of a grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    /// Longitude and latitude (degrees east and north).
    Lon,
    Lat,
    Other,
}

impl Axis {
    pub fn is_x(self) -> bool {
        matches!(self, Axis::X | Axis::Lon)
    }

    pub fn is_y(self) -> bool {
        matches!(self, Axis::Y | Axis::Lat)
    }
}

/// The coordinate variable of dimension `d`: a one-dimensional variable on it named as it.
pub fn coordinate_of(h: &Header, d: usize) -> Option<&Var> {
    let name = &h.dims.get(d)?.name;
    h.vars
        .iter()
        .find(|v| &v.name == name && v.dims.len() == 1 && v.dims[0] == d && v.kind != NcType::Char)
}

/// What axis dimension `d` is: by its coordinate variable's `axis`, `standard_name` and `units`, else by names.
pub fn axis_of(h: &Header, d: usize) -> Axis {
    if let Some(c) = coordinate_of(h, d) {
        let a = axis_of_var(c);
        if a != Axis::Other {
            return a;
        }
    }
    axis_by_name(h.dims.get(d).map_or("", |x| x.name.as_str()))
}

/// What axis a coordinate variable is, by its attributes and name.
pub fn axis_of_var(c: &Var) -> Axis {
    let units = c.text("units").unwrap_or("").trim().to_ascii_lowercase();
    let standard = c
        .text("standard_name")
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let axis = c.text("axis").unwrap_or("").trim().to_ascii_uppercase();
    let east = matches!(
        units.as_str(),
        "degrees_east" | "degree_east" | "degree_e" | "degrees_e" | "degreee" | "degreese"
    );
    let north = matches!(
        units.as_str(),
        "degrees_north" | "degree_north" | "degree_n" | "degrees_n" | "degreen" | "degreesn"
    );
    if standard == "longitude" || east {
        return Axis::Lon;
    }
    if standard == "latitude" || north {
        return Axis::Lat;
    }
    if matches!(
        standard.as_str(),
        "projection_x_coordinate" | "grid_longitude"
    ) || axis == "X"
    {
        return Axis::X;
    }
    if matches!(
        standard.as_str(),
        "projection_y_coordinate" | "grid_latitude"
    ) || axis == "Y"
    {
        return Axis::Y;
    }
    axis_by_name(&c.name)
}

fn axis_by_name(name: &str) -> Axis {
    match name.to_ascii_lowercase().as_str() {
        "x" | "easting" => Axis::X,
        "y" | "northing" => Axis::Y,
        "lon" | "longitude" | "long" => Axis::Lon,
        "lat" | "latitude" => Axis::Lat,
        _ => Axis::Other,
    }
}

/// A regular axis from its cell centres: the first centre and the step;
/// none when a centre strays from the first plus a multiple of the step by
/// more than a thousandth of the step (or there are fewer than two).
pub fn regular(values: &[f64]) -> Option<(f64, f64)> {
    let n = values.len();
    if n < 2 || !values.iter().all(|v| v.is_finite()) {
        return None;
    }
    let step = (values[n - 1] - values[0]) / (n - 1) as f64;
    if !(step.is_finite() && step != 0.0) {
        return None;
    }
    let tol = step.abs() * 1e-3;
    values
        .iter()
        .enumerate()
        .all(|(k, &v)| (v - (values[0] + k as f64 * step)).abs() <= tol)
        .then_some((values[0], step))
}

/// The EPSG code a WKT names for its root (WKT 1's last `AUTHORITY["EPSG","n"]`, WKT 2's `ID["EPSG",n]` at depth one).
pub fn epsg_of_wkt(wkt: &str) -> Option<u32> {
    let mut depth = 0usize;
    let bytes = wkt.as_bytes();
    let mut found = None;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'[' | b'(' => depth += 1,
            b']' | b')' => depth = depth.saturating_sub(1),
            b'"' => {
                // Skip a quoted text ("" is a quote inside it).
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'"' {
                        if bytes.get(i + 1) == Some(&b'"') {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    i += 1;
                }
            }
            _ if depth == 1 => {
                let rest = &wkt[i..];
                let upper = rest.get(..10).unwrap_or(rest).to_ascii_uppercase();
                let key = if upper.starts_with("AUTHORITY[") {
                    Some(10)
                } else if upper.starts_with("ID[") {
                    Some(3)
                } else {
                    None
                };
                if let Some(k) = key {
                    let inner = &rest[k..];
                    let end = inner.find(']').unwrap_or(inner.len());
                    let mut it = inner[..end].split(',').map(|s| s.trim().trim_matches('"'));
                    if it.next().is_some_and(|a| a.eq_ignore_ascii_case("EPSG")) {
                        found = it.next().and_then(|c| c.parse::<u32>().ok());
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    found
}

/// The coordinate system a variable's `grid_mapping` names: its EPSG code
/// (from `crs_wkt`, `spatial_ref` or `epsg_code`) and its WKT.
pub fn crs_of(h: &Header, v: &Var) -> (Option<u32>, Option<String>) {
    let Some(map) = v.text("grid_mapping").map(str::trim).and_then(|n| {
        // CF 1.7's extended form “crs: x y” names the variable first.
        let name = n.split([':', ' ']).next().unwrap_or(n);
        h.var(name)
    }) else {
        return (None, None);
    };
    let wkt = map
        .text("crs_wkt")
        .or_else(|| map.text("spatial_ref"))
        .map(str::to_owned);
    let epsg = wkt.as_deref().and_then(epsg_of_wkt).or_else(|| {
        map.text("epsg_code")
            .and_then(|t| {
                t.trim()
                    .to_ascii_uppercase()
                    .strip_prefix("EPSG:")
                    .map(str::to_owned)
            })
            .and_then(|c| c.trim().parse::<u32>().ok())
            .or_else(|| {
                map.attr("epsg_code")
                    .and_then(Attr::number)
                    .map(|n| n as u32)
            })
    });
    (epsg, wkt)
}

/// A slice dimension: its name, its values (coordinates; moments when `time`) and units.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceDim {
    pub dim: usize,
    pub name: String,
    pub values: Vec<f64>,
    pub time: bool,
    pub units: Option<String>,
}

/// Dimension `d` as a slice dimension from its coordinate values (none: 0, 1, …).
pub fn slice_dim(h: &Header, d: usize, coords: Option<&[f64]>) -> SliceDim {
    let n = h.dim_len(d) as usize;
    let c = coordinate_of(h, d);
    let units = c
        .and_then(|c| c.text("units"))
        .map(|u| u.trim().to_owned())
        .filter(|u| !u.is_empty());
    let calendar = c.and_then(|c| c.text("calendar"));
    let raw: Vec<f64> = match coords {
        Some(v) if v.len() == n => v.to_vec(),
        _ => (0..n).map(|k| k as f64).collect(),
    };
    let time = coords
        .filter(|v| v.len() == n)
        .and(units.as_deref())
        .and_then(|u| cf_time(u, calendar));
    let moments: Option<Vec<f64>> = time.and_then(|t| raw.iter().map(|&v| t.moment(v)).collect());
    match moments {
        Some(m) if m.windows(2).all(|w| w[0] <= w[1]) => SliceDim {
            dim: d,
            name: h.dims[d].name.clone(),
            values: m,
            time: true,
            units: None,
        },
        _ => SliceDim {
            dim: d,
            name: h.dims[d].name.clone(),
            values: raw,
            time: false,
            units,
        },
    }
}
