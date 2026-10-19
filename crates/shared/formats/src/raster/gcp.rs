//! Control points to and from a file (docs/adr/0518 §4): QGIS's georeferencer's `.points` (GDAL's GCPs' columns, a
//! comma-separated text): `mapX,mapY,sourceX,sourceY,enable,dX,dY,residual`, the source's y the negative row (pixel
//! corners, the upper left corner 0, 0), an optional `#CRS: …` line first. KentOS writes two columns more, `role`
//! (`control`, `check`, `off`) and `name`, which QGIS leaves alone; read back, `role` decides, else `enable` (1 a control
//! point, 0 left out). Columns are found by their header's names; a file without a header is taken in QGIS's order.

use kentos_contracts::GeorefPoint;

/// The header KentOS writes.
pub const HEADER: &str = "mapX,mapY,sourceX,sourceY,enable,dX,dY,residual,role,name";

/// The most lines a file may have.
pub const MAX_LINES: usize = 100_000;

/// A points file read: the points, and the `#CRS:` line's text when there was one.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub points: Vec<GeorefPoint>,
    pub crs: Option<String>,
}

/// The fields of a line, split at commas (no quoting: QGIS writes none).
fn fields(line: &str) -> Vec<&str> {
    line.split(',').map(str::trim).collect()
}

fn number(text: &str, what: &str, line: usize) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{line}. satırın {what} değeri sayı değil: “{text}”."))
}

/// The points of a `.points` file; why not, with its line.
pub fn read(text: &str) -> Result<Read, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut crs = None;
    let mut columns: Option<Vec<String>> = None;
    let mut points = Vec::new();
    for (k, raw) in text.lines().enumerate() {
        let n = k + 1;
        if n > MAX_LINES {
            return Err(format!("Dosyada {MAX_LINES} satırdan çok satır var."));
        }
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#CRS:") {
            crs = Some(rest.trim().to_owned()).filter(|t| !t.is_empty());
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let f = fields(line);
        if columns.is_none() && f.iter().any(|c| c.eq_ignore_ascii_case("mapX")) {
            columns = Some(f.iter().map(|c| c.to_ascii_lowercase()).collect());
            continue;
        }
        let names = columns.get_or_insert_with(|| {
            ["mapx", "mapy", "sourcex", "sourcey", "enable", "dx", "dy", "residual"]
                .into_iter()
                .map(str::to_owned)
                .collect()
        });
        let at = |name: &str| names.iter().position(|c| c == name).and_then(|i| f.get(i).copied());
        let need = |name: &str, what: &str| -> Result<f64, String> {
            let t = at(name).ok_or_else(|| format!("{n}. satırda {what} yok."))?;
            number(t, what, n)
        };
        let (x, y) = (need("mapx", "mapX")?, need("mapy", "mapY")?);
        let (c, r) = (need("sourcex", "sourceX")?, need("sourcey", "sourceY")?);
        let enabled = match at("enable") {
            None | Some("") => true,
            Some(t) => number(t, "enable", n)? != 0.0,
        };
        let (check, off) = match at("role").map(str::to_ascii_lowercase).as_deref() {
            Some("check") => (true, false),
            Some("off") => (false, true),
            Some("control") => (false, false),
            None | Some("") => (false, !enabled),
            Some(other) => return Err(format!("{n}. satırın rolü bilinmiyor: “{other}” (control, check ya da off).")),
        };
        let name = at("name").filter(|t| !t.is_empty()).map(str::to_owned);
        points.push(GeorefPoint { name, pixel: [c, -r], target: [x, y], check, off });
    }
    if points.is_empty() {
        return Err("Dosyada kontrol noktası yok.".into());
    }
    Ok(Read { points, crs })
}

/// A number as the file keeps it: the shortest text that reads back to it.
fn text(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let t = format!("{v}");
    if t.contains('e') { format!("{v:.17}") } else { t }
}

/// The points as a `.points` file, with each point's residual (the transformed pixel less its target) when known.
pub fn write(points: &[GeorefPoint], residuals: &[Option<[f64; 2]>]) -> String {
    let mut out = String::from(HEADER);
    out.push('\n');
    for (k, p) in points.iter().enumerate() {
        let (dx, dy) = residuals.get(k).copied().flatten().map_or((0.0, 0.0), |[a, b]| (a, b));
        let role = if p.check {
            "check"
        } else if p.off {
            "off"
        } else {
            "control"
        };
        // The name never holds a comma or a line's end: they are left out.
        let name: String = p.name.as_deref().unwrap_or("").chars().filter(|c| *c != ',' && !c.is_control()).collect();
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{role},{name}\n",
            text(p.target[0]),
            text(p.target[1]),
            text(p.pixel[0]),
            text(-p.pixel[1]),
            u8::from(p.control()),
            text(dx),
            text(dy),
            // √(dx² + dy²) as written, the same bits on every platform.
            text((dx * dx + dy * dy).sqrt()),
        ));
    }
    out
}
