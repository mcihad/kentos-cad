//! Labels in QML (docs/adr/0635 §3): a layer's label classes as QGIS's labelling, a `simple` one for a single label
//! and a `rule-based` one for classes, each class's `<settings>` with its text (a field, or a QGIS expression joining
//! fields and texts), font, buffer, placement and rendering; and back. QGIS's placements are said for the QML's one
//! geometry (`layerGeometryType`); its sizes in millimetres, points and pixels turn into KentOS's pixels at 96 dpi.

use std::fmt::Write as _;

use serde_json::{Map, Value, json};

use super::{Node, child, colour_in, esc, num, qgis_text, rgba};
use kentos_formats::sld::{filter_text, parse_filter, parse_qgis};
use kentos_formats::style_labels::{self as sl, Class, ClassIn, Piece};

/// A class's settings QGIS's labelling here has not.
const UNMAPPED: [(&str, &str); 9] = [
    ("callout", "çağrı çizgisi"),
    ("background", "zemin"),
    ("shadow", "gölge"),
    ("stack", "yığma"),
    ("abbreviate", "kısaltma"),
    ("shrink", "küçültme"),
    ("duplicates", "yinelenenleri tek yazma"),
    ("maxSize", "en büyük boy"),
    ("grow", "nesneye göre büyüme"),
];

/// Pixels a millimetre is at 96 dpi.
const PX_PER_MM: f64 = 96.0 / 25.4;

fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// QGIS's placement and line flags for the class on the QML's geometry (0 points, 1 lines, 2 areas).
fn placement(style: &Value, geom: u8, what: &str, lost: &mut Vec<String>) -> (u8, u8) {
    match geom {
        0 => match sl::point_mode(style) {
            "center" => (1, 0),
            "corner" => {
                lost.push(format!(
                    "{what}: kutunun köşesinden etiket QGIS'te yok; noktanın üstüne yazıldı"
                ));
                (1, 0)
            }
            _ => (6, 0),
        },
        1 => {
            let flags = match style.get("position").and_then(Value::as_str) {
                Some("on") => 1,
                Some("below") => 4,
                Some("sides") => 6,
                _ => 2,
            } | 8;
            match sl::line_mode(style) {
                "horizontal" => (4, flags),
                "curved" => (3, flags),
                "contour" => {
                    lost.push(format!(
                        "{what}: eş yükseltinin yokuş yukarı etiketi QGIS'te yok; eğri boyunca yazıldı"
                    ));
                    (3, flags)
                }
                _ => (2, flags),
            }
        }
        _ => match sl::area_mode(style) {
            "free" => (5, 0),
            "perimeter" if style.get("curved") == Some(&Value::Bool(true)) => (7, 2 | 8),
            "perimeter" => (2, 2 | 8),
            "boundary" => {
                lost.push(format!(
                    "{what}: alanın sınırı boyunca yinelenen etiket QGIS'te yok; çevresi boyunca yazıldı"
                ));
                (7, 2 | 8)
            }
            "parcel" => (5, 0),
            "corner" => {
                lost.push(format!(
                    "{what}: kutunun köşesinden etiket QGIS'te yok; alanın içine yazıldı"
                ));
                (4, 0)
            }
            _ => (4, 0),
        },
    }
}

fn settings_xml(
    style: &Value,
    text: &(String, u8),
    geom: u8,
    what: &str,
    pad: &str,
    lost: &mut Vec<String>,
) -> String {
    let (field, is_expr) = text.clone();
    let size = style.get("size").and_then(Value::as_f64).unwrap_or(10.0);
    let bold = style
        .get("weight")
        .and_then(Value::as_u64)
        .is_some_and(|w| w >= 600);
    let italic = style.get("italic") == Some(&Value::Bool(true));
    let colour = match sl::hex(style.get("color")) {
        Some(c) => c,
        None => {
            if let Some(t) = style.get("color").and_then(Value::as_str) {
                lost.push(format!(
                    "{what}: “{t}” tema rengi dosyada yok; siyah yazıldı"
                ));
            }
            "#000000".into()
        }
    };
    let mut s = String::new();
    let _ = writeln!(s, "{pad}<settings calloutType=\"simple\">");
    let _ = writeln!(
        s,
        "{pad} <text-style fieldName=\"{}\" isExpression=\"{is_expr}\" fontFamily=\"Noto Sans\" namedStyle=\"\" fontSize=\"{}\" fontSizeUnit=\"Pixel\" fontWeight=\"{}\" forcedBold=\"{}\" fontItalic=\"{}\" forcedItalic=\"0\" textColor=\"{}\" textOpacity=\"1\">",
        esc(&field),
        num(size),
        if bold { 700 } else { 400 },
        u8::from(bold),
        u8::from(italic),
        rgba(&json!(colour)).unwrap_or_else(|| "0,0,0,255".into()),
    );
    let halo = style
        .get("halo")
        .and_then(|h| Some((h.get("width")?.as_f64()?, h.get("color"))))
        .filter(|(w, _)| *w > 0.0);
    match halo {
        Some((w, c)) => {
            let c = sl::hex(c).unwrap_or_else(|| "#ffffff".into());
            let _ = writeln!(
                s,
                "{pad}  <text-buffer bufferDraw=\"1\" bufferSize=\"{}\" bufferSizeUnits=\"Pixel\" bufferColor=\"{}\" bufferOpacity=\"1\"/>",
                num(w),
                rgba(&json!(c)).unwrap_or_else(|| "255,255,255,255".into())
            );
        }
        None => {
            let _ = writeln!(s, "{pad}  <text-buffer bufferDraw=\"0\"/>");
        }
    }
    let _ = writeln!(s, "{pad} </text-style>");
    let (place, flags) = placement(style, geom, what, lost);
    let dist = style.get("distance").and_then(Value::as_f64).unwrap_or(2.0);
    let priority = style
        .get("priority")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .min(10);
    let overlap = match style.get("overlap").and_then(Value::as_str) {
        Some("ifNeeded") => "AllowOverlapIfRequired",
        Some("always") => "AllowOverlapAtNoCost",
        _ => "PreventOverlap",
    };
    let angle = style
        .get("maxAngle")
        .and_then(Value::as_f64)
        .unwrap_or(25.0);
    let repeat = style.get("repeat").and_then(Value::as_f64).unwrap_or(0.0);
    let inside = style.get("inside") == Some(&Value::Bool(true))
        || (geom == 2 && sl::area_mode(style) == "parcel");
    let _ = writeln!(
        s,
        "{pad} <placement placement=\"{place}\" placementFlags=\"{flags}\" dist=\"{}\" distUnits=\"Pixel\" priority=\"{priority}\" repeatDistance=\"{}\" repeatDistanceUnits=\"Pixel\" overlapHandling=\"{overlap}\" maxCurvedCharAngleIn=\"{}\" maxCurvedCharAngleOut=\"{}\" fitInPolygonOnly=\"{}\"/>",
        num(dist),
        num(repeat),
        num(angle),
        num(-angle),
        u8::from(inside),
    );
    let lo = style
        .get("maxScale")
        .and_then(Value::as_f64)
        .filter(|v| *v > 0.0)
        .map(sl::denominator);
    let hi = style
        .get("minScale")
        .and_then(Value::as_f64)
        .filter(|v| *v > 0.0)
        .map(sl::denominator);
    let min_feature = style
        .get("minFeaturePx")
        .and_then(Value::as_f64)
        .map_or(0.0, |px| r3(px / PX_PER_MM));
    // QGIS's `scaleMax` is the most zoomed out (the largest denominator), `scaleMin` the most zoomed in.
    let _ = writeln!(
        s,
        "{pad} <rendering drawLabels=\"1\" scaleVisibility=\"{}\" scaleMin=\"{}\" scaleMax=\"{}\" mergeLines=\"{}\" minFeatureSize=\"{}\"/>",
        u8::from(lo.is_some() || hi.is_some()),
        num(lo.unwrap_or(0.0)),
        num(hi.unwrap_or(0.0)),
        u8::from(style.get("mergeLines") == Some(&Value::Bool(true))),
        num(min_feature),
    );
    let _ = writeln!(s, "{pad}</settings>");
    let mut unmapped = sl::unmapped(style, &UNMAPPED);
    if style.get("outside") == Some(&Value::Bool(true)) {
        unmapped.push("dışarıda çağrı çizgisiyle".into());
    }
    if style.get("align").is_some_and(|a| !a.is_null()) {
        unmapped.push("satırların hizası".into());
    }
    if !unmapped.is_empty() {
        lost.push(format!(
            "{what}: {} QGIS'in buradaki etiketinde yok; yazılmadı",
            unmapped.join(", ")
        ));
    }
    s
}

/// The label classes as QGIS's labelling, or nothing; how many it wrote.
pub(super) fn labeling_xml(
    classes: &[Class<'_>],
    geom: u8,
    lost: &mut Vec<String>,
) -> (String, usize) {
    let mut ok: Vec<(&Class<'_>, (String, u8), Option<String>)> = Vec::new();
    for c in classes {
        let what = format!("“{}” etiketi", c.name);
        // A field, a join of fields and texts, or an expression with functions (docs/adr/0635 §6).
        let pieces = match sl::pieces(c.style) {
            Ok(p) => match p.as_slice() {
                [Piece::Field(f)] => (f.clone(), 0),
                _ => (sl::qgis_text(&p), 1),
            },
            Err(why) => match c
                .style
                .get("text")
                .and_then(Value::as_str)
                .and_then(kentos_formats::style_expr::to_qgis)
            {
                Some(q) => (q, 1),
                None => {
                    lost.push(format!("{what}: {why}; yazılmadı"));
                    continue;
                }
            },
        };
        let filter = match &c.when {
            Some(src) => match parse_filter(src)
                .map(|f| qgis_text(&f))
                .or_else(|| kentos_formats::style_expr::to_qgis(src))
            {
                Some(f) => Some(f),
                None => {
                    lost.push(format!(
                        "{what}: “{src}” koşulu QGIS'in ifadesine çevrilemedi; yazılmadı"
                    ));
                    continue;
                }
            },
            None => None,
        };
        ok.push((c, pieces, filter));
    }
    if ok.is_empty() {
        return (String::new(), 0);
    }
    let mut s = String::new();
    if ok.len() == 1 && ok[0].0.when.is_none() && ok[0].0.name == sl::SINGLE {
        let (c, pieces, _) = &ok[0];
        s.push_str("  <labeling type=\"simple\">\n");
        s.push_str(&settings_xml(
            c.style,
            pieces,
            geom,
            &format!("“{}” etiketi", c.name),
            "   ",
            lost,
        ));
        s.push_str("  </labeling>\n");
        return (s, 1);
    }
    s.push_str("  <labeling type=\"rule-based\">\n");
    let _ = writeln!(s, "   <rules key=\"{}\">", super::key_of(900));
    for (k, (c, pieces, filter)) in ok.iter().enumerate() {
        let _ = write!(
            s,
            "    <rule key=\"{}\" description=\"{}\"",
            super::key_of(901 + k),
            esc(&c.name)
        );
        if let Some(f) = filter {
            let _ = write!(s, " filter=\"{}\"", esc(f));
        }
        s.push_str(">\n");
        s.push_str(&settings_xml(
            c.style,
            pieces,
            geom,
            &format!("“{}” etiketi", c.name),
            "     ",
            lost,
        ));
        s.push_str("    </rule>\n");
    }
    s.push_str("   </rules>\n  </labeling>\n");
    (s, ok.len())
}

// ─── Reading ──────────────────────────────────────────────────────────────────────────────────────────

fn attr_num(n: Node<'_, '_>, name: &str) -> Option<f64> {
    n.attribute(name)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

/// A size in QGIS's unit as pixels at 96 dpi; none for a map unit or a percentage.
fn px_of(v: f64, unit: Option<&str>) -> Option<f64> {
    Some(match unit.unwrap_or("MM") {
        "Pixel" => v,
        "MM" => v * PX_PER_MM,
        "Point" => v * 96.0 / 72.0,
        "Inch" => v * 96.0,
        _ => return None,
    })
}

/// A class read from QGIS's `<settings>`, named `name` with condition `when`; none, said, when it cannot be.
fn read_settings(
    st: Node<'_, '_>,
    name: &str,
    when: Option<String>,
    geom: Option<u8>,
    lost: &mut Vec<String>,
    families: &mut Vec<String>,
) -> Option<ClassIn> {
    let what = format!("“{name}” etiketi");
    let ts = child(st, "text-style")?;
    let field = ts.attribute("fieldName").unwrap_or("").trim();
    if field.is_empty() {
        lost.push(format!("{what}: metni yok; alınmadı"));
        return None;
    }
    // A join of fields and texts, else an expression with functions word by word (docs/adr/0635 §6).
    let text = if ts.attribute("isExpression") == Some("1") {
        match sl::concatenation(field, true)
            .map(|p| sl::kentos_text(&p))
            .or_else(|| kentos_formats::style_expr::to_kentos(field))
        {
            Some(t) => t,
            None => {
                lost.push(format!(
                    "{what}: “{field}” ifadesi KentOS'un ifadesine çevrilemedi; alınmadı"
                ));
                return None;
            }
        }
    } else {
        sl::kentos_text(&[Piece::Field(field.to_owned())])
    };
    if child(st, "rendering").is_some_and(|r| r.attribute("drawLabels") == Some("0")) {
        return None;
    }
    let mut more = Map::new();
    let size = match attr_num(ts, "fontSize") {
        Some(v) => match px_of(v, ts.attribute("fontSizeUnit")) {
            Some(px) => r3(px),
            None => {
                lost.push(format!(
                    "{what}: harita biriminde yazı boyu KentOS'un etiketinde yok; boyu piksel alındı"
                ));
                v
            }
        },
        None => 10.0,
    };
    let weight = attr_num(ts, "fontWeight").unwrap_or(400.0);
    // Qt 6's weights (400 normal, 700 bold) and Qt 5's (50 normal, 75 bold).
    let bold = ts.attribute("forcedBold") == Some("1")
        || weight >= 600.0
        || (63.0..100.0).contains(&weight);
    if bold {
        more.insert("weight".into(), json!(600));
    }
    if ts.attribute("fontItalic") == Some("1") || ts.attribute("forcedItalic") == Some("1") {
        more.insert("italic".into(), json!(true));
    }
    if let Some(f) = ts.attribute("fontFamily").filter(|f| !f.is_empty())
        && !families.iter().any(|x| x == f)
    {
        families.push(f.to_owned());
    }
    if let Some(c) = colour_in(ts.attribute("textColor")) {
        let opacity = attr_num(ts, "textOpacity").unwrap_or(1.0);
        let c = match kentos_formats::sld::colour(&json!(c)) {
            Some((rgb, a)) if opacity < 1.0 => {
                kentos_formats::sld::colour_with(&rgb, Some(a * opacity))
            }
            _ => c,
        };
        more.insert("color".into(), json!(c));
    }
    if let Some(b) = child(ts, "text-buffer").filter(|b| b.attribute("bufferDraw") == Some("1")) {
        let w = attr_num(b, "bufferSize")
            .and_then(|w| px_of(w, b.attribute("bufferSizeUnits")))
            .map_or(1.0, r3);
        let c = colour_in(b.attribute("bufferColor")).unwrap_or_else(|| "#ffffff".into());
        more.insert("halo".into(), json!({ "width": w, "color": c }));
    }
    let mut gone = Vec::new();
    if child(ts, "background").is_some_and(|b| b.attribute("shapeDraw") == Some("1")) {
        gone.push("zemin");
    }
    if child(ts, "shadow").is_some_and(|b| b.attribute("shadowDraw") == Some("1")) {
        gone.push("gölge");
    }
    if child(st, "callout").is_some_and(|c| c.attribute("enabled") == Some("1")) {
        gone.push("çağrı çizgisi");
    }
    if let Some(p) = child(st, "placement") {
        let place = attr_num(p, "placement").map_or(0, |v| v as i64);
        let geom = geom.unwrap_or(match p.attribute("layerType") {
            Some("LineGeometry") => 1,
            Some("PolygonGeometry") => 2,
            _ if matches!(place, 2 | 3) => 1,
            _ => 0,
        });
        match geom {
            0 => {
                more.insert(
                    "point".into(),
                    json!(if place == 1 { "center" } else { "around" }),
                );
                if !matches!(place, 0 | 1 | 6) {
                    lost.push(format!(
                        "{what}: QGIS'in {place} yerleşimi noktada KentOS'ta yok; çevresine alındı"
                    ));
                }
            }
            1 => {
                more.insert(
                    "line".into(),
                    json!(match place {
                        3 => "curved",
                        4 => "horizontal",
                        _ => "parallel",
                    }),
                );
                let flags = attr_num(p, "placementFlags").map_or(10, |v| v as i64);
                let position = match (flags & 1 != 0, flags & 2 != 0, flags & 4 != 0) {
                    (_, true, true) => "sides",
                    (true, false, false) => "on",
                    (_, false, true) => "below",
                    _ => "above",
                };
                if position != "above" {
                    more.insert("position".into(), json!(position));
                }
            }
            _ => {
                let area = match place {
                    5 => "free",
                    2 => "perimeter",
                    7 => {
                        more.insert("curved".into(), json!(true));
                        "perimeter"
                    }
                    8 => {
                        lost.push(format!(
                            "{what}: alanın dışına yerleşim KentOS'ta yok; içine alındı"
                        ));
                        "horizontal"
                    }
                    _ => "horizontal",
                };
                more.insert("area".into(), json!(area));
                if p.attribute("fitInPolygonOnly") == Some("1") {
                    more.insert("inside".into(), json!(true));
                }
            }
        }
        if let Some(d) = attr_num(p, "dist")
            .filter(|d| *d > 0.0)
            .and_then(|d| px_of(d, p.attribute("distUnits")))
        {
            more.insert("distance".into(), json!(r3(d)));
        }
        if let Some(r) = attr_num(p, "repeatDistance")
            .filter(|d| *d > 0.0)
            .and_then(|d| px_of(d, p.attribute("repeatDistanceUnits")))
        {
            more.insert("repeat".into(), json!(r3(r)));
        }
        if let Some(pr) = attr_num(p, "priority") {
            more.insert("priority".into(), json!(pr.round().clamp(0.0, 10.0) as u8));
        }
        match p.attribute("overlapHandling") {
            Some("AllowOverlapIfRequired") => {
                more.insert("overlap".into(), json!("ifNeeded"));
            }
            Some("AllowOverlapAtNoCost") => {
                more.insert("overlap".into(), json!("always"));
            }
            _ => {}
        }
        if let Some(a) = attr_num(p, "maxCurvedCharAngleIn").filter(|a| *a != 25.0 && *a > 0.0) {
            more.insert("maxAngle".into(), json!(a));
        }
    }
    if let Some(r) = child(st, "rendering") {
        if r.attribute("scaleVisibility") == Some("1") {
            if let Some(n) = attr_num(r, "scaleMax").filter(|n| *n > 0.0) {
                more.insert("minScale".into(), json!(sl::ppm(n)));
            }
            if let Some(n) = attr_num(r, "scaleMin").filter(|n| *n > 0.0) {
                more.insert("maxScale".into(), json!(sl::ppm(n)));
            }
        }
        if r.attribute("mergeLines") == Some("1") {
            more.insert("mergeLines".into(), json!(true));
        }
        if let Some(mm) = attr_num(r, "minFeatureSize").filter(|m| *m > 0.0) {
            more.insert("minFeaturePx".into(), json!(r3(mm * PX_PER_MM)));
        }
    }
    if !gone.is_empty() {
        lost.push(format!("{what}: {} KentOS'a alınmadı", gone.join(", ")));
    }
    Some(sl::class_in(name, when, size, text, more, lost))
}

/// Condition `a` and `b` joined.
fn and_of(a: Option<&str>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => Some(format!("({a}) ve ({b})")),
        (Some(a), None) => Some(a.to_owned()),
        (None, b) => b,
    }
}

#[allow(clippy::too_many_arguments)]
fn read_label_rules(
    parent: Node<'_, '_>,
    upper: Option<&str>,
    geom: Option<u8>,
    lost: &mut Vec<String>,
    families: &mut Vec<String>,
    out: &mut Vec<ClassIn>,
) {
    for r in parent
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "rule")
    {
        let name = r
            .attribute("description")
            .filter(|l| !l.is_empty())
            .map_or_else(|| format!("Etiket {}", out.len() + 1), str::to_owned);
        if r.attribute("active") == Some("0") {
            lost.push(format!("kapalı “{name}” etiket kuralı alınmadı"));
            continue;
        }
        let own = match r
            .attribute("filter")
            .map(str::trim)
            .filter(|f| !f.is_empty())
        {
            Some(f) if f.eq_ignore_ascii_case("ELSE") => {
                lost.push(format!(
                    "“{name}” etiketi: “Diğerleri” etiketi KentOS'ta yok; her nesneye alındı"
                ));
                None
            }
            Some(f) => match parse_qgis(f)
                .map(|filter| filter_text(&filter))
                .or_else(|| kentos_formats::style_expr::to_kentos(f))
            {
                Some(filter) => Some(filter),
                None => {
                    lost.push(format!(
                        "“{name}” etiketinin “{f}” süzgeci KentOS'un ifadesine çevrilemedi; alınmadı"
                    ));
                    continue;
                }
            },
            None => None,
        };
        let when = and_of(upper, own);
        if let Some(st) = child(r, "settings")
            && let Some(mut c) = read_settings(st, &name, when.clone(), geom, lost, families)
        {
            for (attr, key) in [("scalemaxdenom", "minScale"), ("scalemindenom", "maxScale")] {
                if let Some(n) = attr_num(r, attr).filter(|n| *n > 0.0) {
                    c.style.insert(key.into(), json!(sl::ppm(n)));
                }
            }
            out.push(c);
        }
        read_label_rules(r, when.as_deref(), geom, lost, families, out);
    }
}

/// The label classes of a QML's labelling (none when its labels are off).
pub(super) fn read(root: Node<'_, '_>, lost: &mut Vec<String>) -> Vec<ClassIn> {
    let Some(l) = child(root, "labeling") else {
        return Vec::new();
    };
    if root.attribute("labelsEnabled") == Some("0") {
        return Vec::new();
    }
    let geom = child(root, "layerGeometryType")
        .and_then(|g| g.text())
        .and_then(|t| t.trim().parse::<u8>().ok())
        .filter(|g| *g <= 2);
    let mut families = Vec::new();
    let mut out = Vec::new();
    match l.attribute("type") {
        Some("simple") => {
            if let Some(st) = child(l, "settings")
                && let Some(c) = read_settings(st, sl::SINGLE, None, geom, lost, &mut families)
            {
                out.push(c);
            }
        }
        Some("rule-based") => {
            if let Some(rules) = child(l, "rules") {
                read_label_rules(rules, None, geom, lost, &mut families, &mut out);
            }
        }
        other => lost.push(format!(
            "QGIS'in “{}” etiketlemesi KentOS'a çevrilmedi",
            other.unwrap_or("")
        )),
    }
    if !families.is_empty() {
        lost.push(format!(
            "etiketin yazı tipi ({}) alınmadı; KentOS etiketleri projenin yazı tipiyle çizer",
            families.join(", ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    fn none(_: &str) -> Option<Value> {
        None
    }

    fn classes_json() -> Value {
        json!({ "mode": "rules", "classes": [
            { "name": "Konut", "when": "[tür] = 'konut'", "style": {
                "placement": "beside", "size": 12, "text": "[ada] || '/' || [parsel]", "color": "#aa0000",
                "italic": true, "weight": 600, "halo": { "width": 2, "color": "#ffffff" }, "priority": 8,
                "point": "around", "distance": 3, "minScale": 0.5, "maxScale": 20 } },
            { "name": "Diğer", "style": { "placement": "center", "size": 9, "text": "[ad]" } } ] })
    }

    fn renderer() -> Value {
        json!({ "type": "single", "symbols": { "marker": { "type": "marker", "layers": [
            { "id": "m", "type": "shape", "shape": "circle", "size": 2, "unit": "mm", "fill": "#00ff00" } ] } } })
    }

    fn points() -> kentos_formats::sld::Kinds {
        kentos_formats::sld::Kinds {
            point: true,
            line: false,
            area: false,
        }
    }

    fn same(l: &kentos_formats::style_labels::Labelling) {
        let c = &l.labels["classes"];
        assert_eq!(l.labels["mode"], "rules", "{l:?}");
        assert_eq!(c[0]["name"], "Konut");
        assert_eq!(c[0]["when"], "[tür] = 'konut'");
        let s = &c[0]["style"];
        assert_eq!(s["text"], "[ada] || '/' || [parsel]");
        assert_eq!(s["size"], 12.0);
        assert_eq!(s["color"], "#aa0000");
        assert_eq!(s["italic"], true);
        assert_eq!(s["weight"], 600);
        assert_eq!(s["halo"]["width"], 2.0);
        assert_eq!(s["priority"], 8);
        assert_eq!(s["point"], "around");
        assert_eq!(c[1]["style"]["text"], "[ad]");
        assert_eq!(c[1]["style"]["point"], "center");
    }

    #[test]
    fn labels_go_through_qml() {
        let labels = classes_json();
        let w = crate::write(
            &renderer(),
            &kentos_formats::style_labels::classes(None, Some(&labels)),
            points(),
            &none,
        );
        assert_eq!(w.labels, 2, "{:?}", w.lost);
        assert!(w.xml.contains("labelsEnabled=\"1\""), "{}", w.xml);
        let back = crate::read(&w.xml).expect("reads");
        let l = back.labels.expect("labels");
        same(&l);
        assert_eq!(l.labels["classes"][0]["style"]["minScale"], 0.5);
        // A single label is a simple labelling and back a single label.
        let one = json!({ "placement": "along", "size": 10, "text": "[ad]", "line": "curved" });
        let lines = kentos_formats::sld::Kinds {
            point: false,
            line: true,
            area: false,
        };
        let line = json!({ "type": "single", "symbols": { "line": { "type": "line", "layers": [
            { "id": "l", "type": "simpleLine", "color": "#0000ff", "width": 0.5, "unit": "mm" } ] } } });
        let w = crate::write(
            &line,
            &kentos_formats::style_labels::classes(Some(&one), None),
            lines,
            &none,
        );
        assert!(w.xml.contains("<labeling type=\"simple\">"), "{}", w.xml);
        let back = crate::read(&w.xml).expect("reads").labels.expect("labels");
        assert_eq!(back.labels["mode"], "single");
        let s = back.label.expect("label");
        assert_eq!(
            (s["line"].as_str(), s["placement"].as_str()),
            (Some("curved"), Some("along"))
        );
    }
}

#[cfg(test)]
mod function_tests {
    use serde_json::json;

    #[test]
    fn a_label_with_functions_goes_to_qml() {
        let labels = json!({ "mode": "rules", "classes": [
            { "name": "İşlevli", "when": "uzunluk([ada]) > 0", "style": { "placement": "center", "size": 8.0,
                "text": "büyük([ada]) || '-' || metin([parsel_no])" } } ] });
        let r = json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
            { "id": "f", "type": "simpleFill", "color": "#f2e6c9" } ] } } });
        let w = crate::write(
            &r,
            &kentos_formats::style_labels::classes(None, Some(&labels)),
            kentos_formats::sld::Kinds::ALL,
            &|_| None,
        );
        assert!(
            w.xml.contains("length(&quot;ada&quot;) &gt; 0"),
            "{:?}\n{}",
            w.lost,
            w.xml
        );
        assert!(w.xml.contains("upper(&quot;ada&quot;)"), "{:?}", w.lost);
        let back = crate::read(&w.xml).expect("reads").labels.expect("labels");
        assert_eq!(back.labels["classes"][0]["when"], "uzunluk([ada]) > 0");
        assert_eq!(
            back.labels["classes"][0]["style"]["text"],
            "büyük([ada]) || '-' || metin([parsel_no])"
        );
    }
}
