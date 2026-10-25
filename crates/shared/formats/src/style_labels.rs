//! Labels in the style files (docs/adr/0635 §1, plan/gis.md CBS-17): the label classes a layer's labelling gives, a
//! class's text as fields and the words between them, where its labels go, and the classes a file gives read back as
//! KentOS's labelling. Each file's own words (SLD's `TextSymbolizer`, QGIS's labelling, MapLibre's `symbol` layer) are
//! its writer's and reader's; what they share is here.
//!
//! The classes are the style's JSON (`LabelStyle`, `LayerLabels`): a single label goes out as one class named
//! [`SINGLE`] with no condition, and one such class read back is a single label again; any other set of classes is a
//! rule-based labelling. A label's sizes are CSS pixels, which SE's, QGIS's and MapLibre's pixels are taken as. Its
//! scale range is the label engine's (pixels per metre on the screen, `labels::style`); a file's is a map scale's
//! denominator at 96 dots per inch ([`denominator`], [`ppm`]).

use serde_json::{Map, Value, json};

/// The name a single label is written under, and a lone class without a condition read back as a single label.
pub const SINGLE: &str = "Etiket";

/// Metres a CSS pixel is on the paper (96 dpi; `tools::view_link::METRES_PER_PX`).
pub const METRES_PER_PX: f64 = kentos_geometry_core::tools::view_link::METRES_PER_PX;

/// One label class to write: its name, its condition (İfadeyle seç's language) and its style (`LabelStyle`'s JSON).
#[derive(Clone, Debug)]
pub struct Class<'a> {
    pub name: String,
    pub when: Option<String>,
    pub style: &'a Value,
}

/// The classes a layer's labelling gives: none when it is off; its rules' classes; else its single label (none when
/// the layer has no label style of its own: its kinds' defaults label with the objects' own labels, no field a file
/// can name).
pub fn classes<'a>(label: Option<&'a Value>, labels: Option<&'a Value>) -> Vec<Class<'a>> {
    let mode = labels
        .and_then(|l| l.get("mode"))
        .and_then(Value::as_str)
        .unwrap_or("single");
    match mode {
        "off" => Vec::new(),
        "rules" => labels
            .and_then(|l| l.get("classes"))
            .and_then(Value::as_array)
            .map(|cs| {
                cs.iter()
                    .filter_map(|c| {
                        Some(Class {
                            name: c.get("name")?.as_str()?.to_owned(),
                            when: c
                                .get("when")
                                .and_then(Value::as_str)
                                .filter(|w| !w.trim().is_empty())
                                .map(str::to_owned),
                            style: c.get("style")?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => label
            .map(|s| {
                vec![Class {
                    name: SINGLE.to_owned(),
                    when: None,
                    style: s,
                }]
            })
            .unwrap_or_default(),
    }
}

/// A piece of a label's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Field(String),
    Text(String),
}

/// A class's text as pieces: its expression's fields and texts joined by `||` (one field alone too), else its
/// template's `{field}`s and the words between; or why a file cannot say it.
pub fn pieces(style: &Value) -> Result<Vec<Piece>, String> {
    if let Some(text) = style
        .get("text")
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
    {
        return concatenation(text, false).ok_or_else(|| {
            format!(
                "metninin “{text}” ifadesi alanlar ve yazıların || ile birleşimi değil; dosyanın etiket metnine çevrilemedi"
            )
        });
    }
    match style
        .get("template")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
    {
        Some(t) => Ok(template(t)),
        None => Err(
            "metni ne bir ifade ne de bir şablon (türün varsayılan etiketi nesnenin kendi etiketidir, dosyada alan değil)"
                .into(),
        ),
    }
}

/// A template's pieces: `{field}` a field, the rest words.
pub fn template(t: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut rest = t;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        if open > 0 {
            out.push(Piece::Text(rest[..open].to_owned()));
        }
        let field = rest[open + 1..open + close].trim();
        if !field.is_empty() {
            out.push(Piece::Field(field.to_owned()));
        }
        rest = &rest[open + close + 1..];
    }
    if !rest.is_empty() {
        out.push(Piece::Text(rest.to_owned()));
    }
    out
}

/// An expression joining fields and texts by `||`: KentOS's (`[a]` a field, `'t'` or `"t"` a text) or QGIS's (`"a"` a
/// field, `'t'` a text; a bare word a field in both); or none.
pub fn concatenation(src: &str, qgis: bool) -> Option<Vec<Piece>> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut want_piece = true;
    let quoted = |i: usize, q: char| -> Option<(String, usize)> {
        let mut s = String::new();
        let mut j = i + 1;
        loop {
            let x = *c.get(j)?;
            if x == q {
                if c.get(j + 1) == Some(&q) {
                    s.push(q);
                    j += 2;
                    continue;
                }
                return Some((s, j + 1));
            }
            s.push(x);
            j += 1;
        }
    };
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if !want_piece {
            if ch == '|' && c.get(i + 1) == Some(&'|') {
                want_piece = true;
                i += 2;
                continue;
            }
            return None;
        }
        if !qgis && ch == '[' {
            let j = c[i + 1..].iter().position(|&x| x == ']')? + i + 1;
            let name = c[i + 1..j].iter().collect::<String>().trim().to_owned();
            if name.is_empty() {
                return None;
            }
            out.push(Piece::Field(name));
            i = j + 1;
        } else if ch == '\'' || (!qgis && ch == '"') {
            let (s, j) = quoted(i, ch)?;
            out.push(Piece::Text(s));
            i = j;
        } else if qgis && ch == '"' {
            let (s, j) = quoted(i, '"')?;
            if s.is_empty() {
                return None;
            }
            out.push(Piece::Field(s));
            i = j;
        } else if ch.is_alphabetic() || ch == '_' {
            let mut j = i + 1;
            while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
                j += 1;
            }
            let w: String = c[i..j].iter().collect();
            if !crate::sld::bare_field(&w)
                || c[j..].iter().find(|x| !x.is_whitespace()) == Some(&'(')
            {
                return None;
            }
            out.push(Piece::Field(w));
            i = j;
        } else {
            return None;
        }
        want_piece = false;
    }
    (!out.is_empty() && !want_piece).then_some(out)
}

fn quote(s: &str, q: char) -> String {
    let doubled: String = s
        .chars()
        .flat_map(|ch| if ch == q { vec![ch, ch] } else { vec![ch] })
        .collect();
    format!("{q}{doubled}{q}")
}

/// Pieces as KentOS's text expression: `[a] || '/' || [b]`.
pub fn kentos_text(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Field(f) => format!("[{f}]"),
            Piece::Text(t) => quote(t, '\''),
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

/// Pieces as QGIS's expression: `"a" || '/' || "b"`.
pub fn qgis_text(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Field(f) => quote(f, '"'),
            Piece::Text(t) => quote(t, '\''),
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

/// A label's colour as `#rrggbb` (or `#rrggbbaa`); a theme name has none.
pub fn hex(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?.trim();
    let h = s.strip_prefix('#')?;
    (matches!(h.len(), 3 | 6 | 8) && h.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| s.to_ascii_lowercase())
}

/// The label engine's scale (pixels per metre) as a map scale's denominator, to four decimals.
pub fn denominator(ppm: f64) -> f64 {
    ((1.0 / (ppm * METRES_PER_PX)) * 10_000.0).round() / 10_000.0
}

/// A map scale's denominator as the label engine's scale (pixels per metre), five significant digits.
pub fn ppm(denominator: f64) -> f64 {
    let v = 1.0 / (denominator * METRES_PER_PX);
    if !v.is_finite() || v <= 0.0 {
        return v;
    }
    let digits = 4 - libm::floor(libm::log10(v)) as i32;
    let f = libm::pow(10.0, f64::from(digits));
    libm::round(v * f) / f
}

/// Where a class's point labels go: around or on.
pub fn point_mode(style: &Value) -> &str {
    style
        .get("point")
        .and_then(Value::as_str)
        .unwrap_or_else(|| match placement(style) {
            "center" => "center",
            "corner" => "corner",
            _ => "around",
        })
}

/// Where a class's line labels go: parallel, curved, horizontal or a contour's.
pub fn line_mode(style: &Value) -> &str {
    style
        .get("line")
        .and_then(Value::as_str)
        .unwrap_or_else(|| match placement(style) {
            "along" => "parallel",
            "corner" => "corner",
            _ => "horizontal",
        })
}

/// Where a class's area labels go.
pub fn area_mode(style: &Value) -> &str {
    style
        .get("area")
        .and_then(Value::as_str)
        .unwrap_or_else(|| match placement(style) {
            "along" => "perimeter",
            "corner" => "corner",
            _ => "horizontal",
        })
}

fn placement(style: &Value) -> &str {
    style
        .get("placement")
        .and_then(Value::as_str)
        .unwrap_or("center")
}

/// A class's style settings no file here says, by their Turkish names, when the class has them.
pub fn unmapped(style: &Value, keys: &[(&str, &str)]) -> Vec<String> {
    keys.iter()
        .filter(|(k, _)| style.get(*k).is_some_and(|v| !v.is_null()))
        .map(|(_, w)| (*w).to_owned())
        .collect()
}

/// A class read back from a file.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassIn {
    pub name: String,
    pub when: Option<String>,
    pub style: Map<String, Value>,
}

/// What a file's label classes give the layer: its single label, or its rule-based labelling.
#[derive(Clone, Debug, PartialEq)]
pub struct Labelling {
    /// The single label's style (`LabelStyle`'s JSON); none with rules.
    pub label: Option<Value>,
    /// The labelling (`LayerLabels`' JSON).
    pub labels: Value,
    /// The classes taken.
    pub classes: usize,
}

/// The most classes a layer's labelling holds (`LABEL_CLASSES_MAX`).
pub const MAX_CLASSES: usize = 64;

/// The labelling the classes read give, none for none: one class without a condition named [`SINGLE`] a single
/// label, any other rules, their names one of each kind (a second “Ad” is “Ad (2)”), the first [`MAX_CLASSES`].
pub fn labelling(mut classes: Vec<ClassIn>, lost: &mut Vec<String>) -> Option<Labelling> {
    if classes.is_empty() {
        return None;
    }
    if classes.len() > MAX_CLASSES {
        lost.push(format!(
            "{} etiket sınıfından ilk {MAX_CLASSES}'ü alındı",
            classes.len()
        ));
        classes.truncate(MAX_CLASSES);
    }
    if classes.len() == 1 && classes[0].when.is_none() && classes[0].name == SINGLE {
        let style = Value::Object(classes.remove(0).style);
        return Some(Labelling {
            label: Some(style),
            labels: json!({ "mode": "single" }),
            classes: 1,
        });
    }
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for c in classes {
        let base: String = c.name.trim().chars().take(90).collect();
        let base = if base.is_empty() {
            format!("Etiket {}", out.len() + 1)
        } else {
            base
        };
        let mut name = base.clone();
        let mut k = 2;
        while seen.contains(&name) {
            name = format!("{base} ({k})");
            k += 1;
        }
        seen.push(name.clone());
        let mut v = json!({ "name": name, "style": Value::Object(c.style) });
        if let Some(w) = c.when {
            v["when"] = json!(w);
        }
        out.push(v);
    }
    let n = out.len();
    Some(Labelling {
        label: None,
        labels: json!({ "mode": "rules", "classes": out }),
        classes: n,
    })
}

/// A read class's style from what the file said: `placement` from where its labels go (a line's along, a point
/// on it centred, else beside), the size and the text (KentOS's expression); the rest as given.
pub fn style_in(size: f64, text: String, more: Map<String, Value>) -> Map<String, Value> {
    let point = more.get("point").and_then(Value::as_str);
    let line = more.get("line").and_then(Value::as_str);
    let area = more.get("area").and_then(Value::as_str);
    let placement = if line.is_some() && point.is_none() && area.is_none() {
        "along"
    } else if point == Some("center") || (point.is_none() && area.is_some()) {
        "center"
    } else {
        "beside"
    };
    let mut s = Map::new();
    s.insert("placement".into(), json!(placement));
    s.insert("size".into(), json!(size));
    s.insert("text".into(), json!(text));
    s.extend(more);
    s
}

/// KentOS's ready-made heat ramp (Isı, the native style's `HEAT_RAMPS`), a heat map read without a ramp takes it.
pub const HEAT_RAMP: [&str; 6] = [
    "#2b83ba00",
    "#2b83ba",
    "#abdda4",
    "#ffffbf",
    "#fdae61",
    "#d7191c",
];

/// A heat map's ramp as KentOS holds it (2 to 16 colours): none read the ready-made one, one colour twice, the first
/// 16 of more; each change said.
pub fn heat_ramp(mut ramp: Vec<String>, what: &str, lost: &mut Vec<String>) -> Vec<String> {
    match ramp.len() {
        0 => {
            lost.push(format!(
                "{what}: renk rampası yok; KentOS'un Isı rampası alındı"
            ));
            HEAT_RAMP.iter().map(|c| (*c).to_owned()).collect()
        }
        1 => {
            ramp.push(ramp[0].clone());
            ramp
        }
        n if n > 16 => {
            lost.push(format!("{what}: rampanın {n} renginden ilk 16'sı alındı"));
            ramp.truncate(16);
            ramp
        }
        _ => ramp,
    }
}

/// A read class: its style from what the file said ([`style_in`]), its numbers brought into KentOS's ranges and its
/// colours made `#rrggbb(aa)`, each change said.
pub fn class_in(
    name: &str,
    when: Option<String>,
    size: f64,
    text: String,
    more: Map<String, Value>,
    lost: &mut Vec<String>,
) -> ClassIn {
    let mut style = style_in(size, text, more);
    fit(name, &mut style, lost);
    ClassIn {
        name: name.to_owned(),
        when,
        style,
    }
}

/// A colour as `#rrggbb(aa)`: `#rgb` written out; none for another.
fn colour_fit(c: &str) -> Option<String> {
    let h = c.strip_prefix('#')?;
    if !h.chars().all(|x| x.is_ascii_hexdigit()) {
        return None;
    }
    match h.len() {
        3 => Some(format!(
            "#{}",
            h.chars()
                .flat_map(|x| [x, x])
                .collect::<String>()
                .to_ascii_lowercase()
        )),
        6 | 8 => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

/// The ranges of a label style's numbers (`style_problem`).
const RANGES: [(&str, &str, f64, f64); 7] = [
    ("size", "boyu", 1.0, 200.0),
    ("distance", "uzaklığı", 0.0, 500.0),
    ("repeat", "yinelemesi", 20.0, 100_000.0),
    ("maxAngle", "harfler arası en çok açısı", 5.0, 90.0),
    ("minFeaturePx", "en küçük nesne boyu", 0.0, 100_000.0),
    ("minScale", "en küçük ölçeği", 0.0, 1e9),
    ("maxScale", "en büyük ölçeği", 0.0, 1e9),
];

fn fit_number(
    m: &mut Map<String, Value>,
    key: &str,
    word: &str,
    lo: f64,
    hi: f64,
    what: &str,
    lost: &mut Vec<String>,
) {
    let Some(v) = m.get(key).and_then(Value::as_f64) else {
        return;
    };
    if (lo..=hi).contains(&v) {
        return;
    }
    let c = if v.is_finite() { v.clamp(lo, hi) } else { lo };
    m.insert(key.into(), json!(c));
    lost.push(format!(
        "{what}: {word} {} KentOS'un aralığına ({}–{}) çekildi",
        crate::sld::num(v),
        crate::sld::num(lo),
        crate::sld::num(hi)
    ));
}

fn fit(name: &str, style: &mut Map<String, Value>, lost: &mut Vec<String>) {
    let what = format!("“{name}” etiketi");
    for (key, word, lo, hi) in RANGES {
        fit_number(style, key, word, lo, hi, &what, lost);
    }
    if let Some(c) = style
        .get("color")
        .and_then(Value::as_str)
        .map(str::to_owned)
    {
        match colour_fit(&c) {
            Some(f) => {
                style.insert("color".into(), json!(f));
            }
            None => {
                style.remove("color");
                lost.push(format!("{what}: “{c}” rengi okunamadı; alınmadı"));
            }
        }
    }
    if let Some(Value::Object(h)) = style.get_mut("halo") {
        fit_number(h, "width", "halesinin genişliği", 0.0, 10.0, &what, lost);
        if let Some(c) = h.get("color").and_then(Value::as_str).map(str::to_owned) {
            match colour_fit(&c) {
                Some(f) => {
                    h.insert("color".into(), json!(f));
                }
                None => {
                    h.remove("color");
                    lost.push(format!("{what}: halesinin “{c}” rengi okunamadı; alınmadı"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_are_fields_and_words() {
        let p = concatenation("[ada] || '/' || [parsel]", false).expect("reads");
        assert_eq!(
            p,
            vec![
                Piece::Field("ada".into()),
                Piece::Text("/".into()),
                Piece::Field("parsel".into())
            ]
        );
        assert_eq!(kentos_text(&p), "[ada] || '/' || [parsel]");
        assert_eq!(qgis_text(&p), "\"ada\" || '/' || \"parsel\"");
        assert_eq!(concatenation(&qgis_text(&p), true), Some(p));
        assert_eq!(
            concatenation("'it''s' || ad", false),
            Some(vec![Piece::Text("it's".into()), Piece::Field("ad".into())])
        );
        assert!(concatenation("upper([ad])", false).is_none());
        assert!(concatenation("[a] + [b]", false).is_none());
        assert!(concatenation("[a] ||", false).is_none());
        assert_eq!(
            template("No: {no} ({tür})"),
            vec![
                Piece::Text("No: ".into()),
                Piece::Field("no".into()),
                Piece::Text(" (".into()),
                Piece::Field("tür".into()),
                Piece::Text(")".into())
            ]
        );
    }

    #[test]
    fn scales_go_both_ways() {
        let n = denominator(2.0);
        assert_eq!(n, 1889.7876);
        assert_eq!(ppm(n), 2.0);
        assert_eq!(ppm(denominator(0.0125)), 0.0125);
    }

    #[test]
    fn one_unnamed_class_is_a_single_label() {
        let s = style_in(12.0, "[ad]".into(), Map::new());
        let l = labelling(
            vec![ClassIn {
                name: SINGLE.into(),
                when: None,
                style: s.clone(),
            }],
            &mut Vec::new(),
        )
        .expect("some");
        assert_eq!(l.labels, json!({ "mode": "single" }));
        assert_eq!(l.label.expect("label")["text"], "[ad]");
        let l = labelling(
            vec![
                ClassIn {
                    name: "A".into(),
                    when: Some("[t] = 1".into()),
                    style: s.clone(),
                },
                ClassIn {
                    name: "A".into(),
                    when: None,
                    style: s,
                },
            ],
            &mut Vec::new(),
        )
        .expect("some");
        assert_eq!(l.labels["classes"][1]["name"], "A (2)");
        assert_eq!(l.classes, 2);
    }

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

    fn points() -> crate::sld::Kinds {
        crate::sld::Kinds {
            point: true,
            line: false,
            area: false,
        }
    }

    fn same(l: &Labelling) {
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
    fn labels_go_through_sld() {
        let labels = classes_json();
        let w = crate::sld::write(
            "durak",
            &renderer(),
            &classes(None, Some(&labels)),
            points(),
            &none,
        );
        assert_eq!((w.rules, w.labels), (1, 2), "{:?}", w.lost);
        assert!(
            w.xml.contains("<ogc:Function name=\"Concatenate\">"),
            "{}",
            w.xml
        );
        let back = crate::sld::read(&w.xml).expect("reads");
        let l = back.labels.expect("labels");
        same(&l);
        let s = &l.labels["classes"][0]["style"];
        assert_eq!(s["minScale"], 0.5);
        assert_eq!(s["maxScale"], 20.0);
        assert_eq!(s["distance"], 3.0);
        assert_eq!(back.renderer["type"], "single");
    }
}
