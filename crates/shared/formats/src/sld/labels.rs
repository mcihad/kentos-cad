//! Labels in SLD (docs/adr/0635 §2): a layer's label class as a rule holding an SE `TextSymbolizer` (its text a
//! `PropertyName`, a `Literal` or QGIS's `Concatenate` of them; its font, fill, halo, placement and QGIS's and
//! GeoServer's `Priority` 0–1000), and back. A rule's one placement is said for every geometry it draws: a layer of
//! lines alone gets a `LinePlacement`, any other a `PointPlacement`.

use serde_json::{Map, Value, json};

use super::{
    Filter, Kinds, Node, RuleOut, child, colour, colour_with, esc, filter_text, num, param,
    parse_filter, text_of,
};
use crate::style_labels::{self as sl, Class, ClassIn, Piece};

/// A class's settings SE's text symbolizer has not.
const UNMAPPED: [(&str, &str); 16] = [
    ("callout", "çağrı çizgisi"),
    ("background", "zemin"),
    ("shadow", "gölge"),
    ("stack", "yığma"),
    ("abbreviate", "kısaltma"),
    ("shrink", "küçültme"),
    ("duplicates", "yinelenenleri tek yazma"),
    ("mergeLines", "çizgileri birleştirme"),
    ("minFeaturePx", "en küçük nesne boyu"),
    ("maxSize", "en büyük boy"),
    ("grow", "nesneye göre büyüme"),
    ("outside", "dışarıda çağrı çizgisiyle"),
    ("inside", "yalnız içine"),
    ("maxAngle", "harflerin en çok dönüşü"),
    ("align", "satırların hizası"),
    ("overlap", "çakışma kuralı"),
];

fn svg(name: &str, value: &str) -> String {
    format!(
        "<se:SvgParameter name=\"{name}\">{}</se:SvgParameter>",
        esc(value)
    )
}

/// A colour as `fill` and, when not opaque, `fill-opacity`.
fn fill_xml(hex: &str) -> String {
    let (rgb, alpha) = colour(&json!(hex)).unwrap_or_else(|| ("#000000".into(), 1.0));
    let mut s = svg("fill", &rgb);
    if alpha < 1.0 {
        s.push_str(&svg(
            "fill-opacity",
            &num((alpha * 1000.0).round() / 1000.0),
        ));
    }
    format!("<se:Fill>{s}</se:Fill>")
}

fn label_xml(pieces: &[Piece]) -> String {
    let one = |p: &Piece| match p {
        Piece::Field(f) => format!("<ogc:PropertyName>{}</ogc:PropertyName>", esc(f)),
        Piece::Text(t) => format!("<ogc:Literal>{}</ogc:Literal>", esc(t)),
    };
    if pieces.len() == 1 {
        one(&pieces[0])
    } else {
        format!(
            "<ogc:Function name=\"Concatenate\">{}</ogc:Function>",
            pieces.iter().map(one).collect::<String>()
        )
    }
}

fn placement_xml(style: &Value, kinds: Kinds, what: &str, lost: &mut Vec<String>) -> String {
    let d = style.get("distance").and_then(Value::as_f64).unwrap_or(2.0);
    let centre = "<se:PointPlacement><se:AnchorPoint><se:AnchorPointX>0.5</se:AnchorPointX><se:AnchorPointY>0.5</se:AnchorPointY></se:AnchorPoint></se:PointPlacement>";
    let lines_only = kinds.line && !kinds.point && !kinds.area;
    let inner = if lines_only {
        match sl::line_mode(style) {
            "horizontal" => centre.to_owned(),
            mode => {
                let mut s = String::from("<se:LinePlacement>");
                match style.get("position").and_then(Value::as_str) {
                    Some("above") => s.push_str(&format!(
                        "<se:PerpendicularOffset>{}</se:PerpendicularOffset>",
                        num(d)
                    )),
                    Some("below") => s.push_str(&format!(
                        "<se:PerpendicularOffset>{}</se:PerpendicularOffset>",
                        num(-d)
                    )),
                    Some("sides") => lost.push(format!(
                        "{what}: çizginin iki yanı SLD'de yok; çizginin üstüne yazıldı"
                    )),
                    _ => {}
                }
                if let Some(r) = style.get("repeat").and_then(Value::as_f64) {
                    s.push_str(&format!(
                        "<se:IsRepeated>true</se:IsRepeated><se:Gap>{}</se:Gap>",
                        num(r)
                    ));
                }
                s.push_str("</se:LinePlacement>");
                match mode {
                    "contour" => lost.push(format!(
                        "{what}: eş yükseltinin yokuş yukarı etiketi SLD'de yok; eğri boyunca yazıldı"
                    )),
                    "corner" => lost.push(format!(
                        "{what}: kutunun köşesinden etiket SLD'de yok; çizgi boyunca yazıldı"
                    )),
                    _ => {}
                }
                s
            }
        }
    } else {
        if kinds.line {
            lost.push(format!(
                "{what}: SLD'nin kuralı tek yerleşim söyler; çizgilerin etiketi de noktanınkiyle yazıldı"
            ));
        }
        let mode = if kinds.point {
            sl::point_mode(style)
        } else {
            match sl::area_mode(style) {
                "horizontal" | "free" | "parcel" => "center",
                other => {
                    lost.push(format!(
                        "{what}: alanın “{other}” yerleşimi SLD'de yok; alanın ortasına yazıldı"
                    ));
                    "center"
                }
            }
        };
        if mode == "around" {
            format!(
                "<se:PointPlacement><se:AnchorPoint><se:AnchorPointX>0</se:AnchorPointX><se:AnchorPointY>0.5</se:AnchorPointY></se:AnchorPoint><se:Displacement><se:DisplacementX>{}</se:DisplacementX><se:DisplacementY>0</se:DisplacementY></se:Displacement></se:PointPlacement>",
                num(d)
            )
        } else {
            if mode == "corner" {
                lost.push(format!(
                    "{what}: kutunun köşesinden etiket SLD'de yok; noktanın üstüne yazıldı"
                ));
            }
            centre.to_owned()
        }
    };
    format!("<se:LabelPlacement>{inner}</se:LabelPlacement>")
}

/// The label classes as SE rules of text symbolizers.
pub(super) fn rules(classes: &[Class<'_>], kinds: Kinds, lost: &mut Vec<String>) -> Vec<RuleOut> {
    let mut out = Vec::new();
    for c in classes {
        let what = format!("“{}” etiketi", c.name);
        let style = c.style;
        let pieces = match sl::pieces(style) {
            Ok(p) => p,
            Err(why) => {
                lost.push(format!("{what}: {why}; yazılmadı"));
                continue;
            }
        };
        let filter = match &c.when {
            Some(src) => match parse_filter(src) {
                Some(f) => Some(f),
                None => {
                    lost.push(format!(
                        "{what}: “{src}” koşulu SLD'nin süzgeciyle söylenemiyor; yazılmadı"
                    ));
                    continue;
                }
            },
            None => None,
        };
        let size = style.get("size").and_then(Value::as_f64).unwrap_or(10.0);
        let mut font = svg("font-family", "Noto Sans") + &svg("font-size", &num(size));
        if style.get("italic") == Some(&Value::Bool(true)) {
            font.push_str(&svg("font-style", "italic"));
        }
        if style
            .get("weight")
            .and_then(Value::as_u64)
            .is_some_and(|w| w >= 600)
        {
            font.push_str(&svg("font-weight", "bold"));
        }
        let mut s = format!(
            "<se:TextSymbolizer><se:Label>{}</se:Label><se:Font>{font}</se:Font>",
            label_xml(&pieces)
        );
        s.push_str(&placement_xml(style, kinds, &what, lost));
        if let Some(h) = style.get("halo")
            && let Some(w) = h.get("width").and_then(Value::as_f64).filter(|w| *w > 0.0)
        {
            let c = sl::hex(h.get("color")).unwrap_or_else(|| "#ffffff".into());
            s.push_str(&format!(
                "<se:Halo><se:Radius>{}</se:Radius>{}</se:Halo>",
                num(w),
                fill_xml(&c)
            ));
        }
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
        s.push_str(&fill_xml(&colour));
        if let Some(p) = style.get("priority").and_then(Value::as_u64) {
            s.push_str(&format!("<se:Priority>{}</se:Priority>", p.min(10) * 100));
        }
        if matches!(sl::line_mode(style), "curved" | "contour")
            && kinds.line
            && !kinds.point
            && !kinds.area
        {
            s.push_str("<se:VendorOption name=\"followLine\">true</se:VendorOption>");
        }
        s.push_str("</se:TextSymbolizer>");
        let unmapped = sl::unmapped(style, &UNMAPPED);
        if !unmapped.is_empty() {
            lost.push(format!(
                "{what}: {} SLD'nin yazısında yok; yazılmadı",
                unmapped.join(", ")
            ));
        }
        out.push(RuleOut {
            name: c.name.clone(),
            filter,
            is_else: false,
            // A label's smallest scale (zoomed out) is the largest denominator.
            min_scale: style
                .get("maxScale")
                .and_then(Value::as_f64)
                .filter(|v| *v > 0.0)
                .map(sl::denominator),
            max_scale: style
                .get("minScale")
                .and_then(Value::as_f64)
                .filter(|v| *v > 0.0)
                .map(sl::denominator),
            symbolizers: vec![s],
        });
    }
    out
}

// ─── Reading ──────────────────────────────────────────────────────────────────────────────────────────

/// An element's text as written (a `Literal`'s spaces are its).
fn raw_text(n: Node<'_, '_>) -> String {
    n.descendants()
        .filter(|d| d.is_text())
        .filter_map(|d| d.text())
        .collect()
}

fn label_pieces(n: Node<'_, '_>, out: &mut Vec<Piece>) -> Result<(), String> {
    for c in n.children() {
        if c.is_text() {
            let t = c.text().unwrap_or("");
            if !t.trim().is_empty() {
                out.push(Piece::Text(t.trim().to_owned()));
            }
            continue;
        }
        if !c.is_element() {
            continue;
        }
        match c.tag_name().name() {
            "PropertyName" => out.push(Piece::Field(text_of(c))),
            "Literal" => out.push(Piece::Text(raw_text(c))),
            "Function"
                if matches!(
                    c.attribute("name")
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .as_str(),
                    "concatenate" | "strconcat" | "concat"
                ) =>
            {
                label_pieces(c, out)?;
            }
            "Function" => {
                return Err(format!(
                    "metnindeki SE'nin “{}” işlevi KentOS'un etiket metnine çevrilemedi",
                    c.attribute("name").unwrap_or("")
                ));
            }
            other => {
                return Err(format!(
                    "metnindeki “{other}” KentOS'un etiket metnine çevrilemedi"
                ));
            }
        }
    }
    Ok(())
}

/// Pieces with neighbouring words joined and empty ones dropped.
fn tidy(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for p in pieces {
        match (out.last_mut(), p) {
            (_, Piece::Text(t)) if t.is_empty() => {}
            (Some(Piece::Text(a)), Piece::Text(b)) => a.push_str(&b),
            (_, p) => out.push(p),
        }
    }
    out
}

fn number(n: Option<Node<'_, '_>>) -> Option<f64> {
    n.map(text_of)
        .and_then(|t| t.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

/// The label classes of a rule's text symbolizers: its name, filter and scale range.
pub(super) fn read_rule(
    rule: Node<'_, '_>,
    label: &str,
    filter: Option<&Filter>,
    is_else: bool,
    lost: &mut Vec<String>,
    families: &mut Vec<String>,
    out: &mut Vec<ClassIn>,
) {
    let scale = |tag: &str| number(child(rule, tag)).filter(|v| *v > 0.0);
    let (lo, hi) = (scale("MinScaleDenominator"), scale("MaxScaleDenominator"));
    for t in rule
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "TextSymbolizer")
    {
        let what = format!("“{label}” kuralının etiketi");
        let Some(l) = child(t, "Label") else {
            lost.push(format!("{what}: metni yok; alınmadı"));
            continue;
        };
        let mut pieces = Vec::new();
        if let Err(why) = label_pieces(l, &mut pieces) {
            lost.push(format!("{what}: {why}; alınmadı"));
            continue;
        }
        let pieces = tidy(pieces);
        if pieces.is_empty() {
            lost.push(format!("{what}: metni boş; alınmadı"));
            continue;
        }
        if t.attribute("uom").is_some_and(|u| u.ends_with("metre")) {
            lost.push(format!(
                "{what}: metre boyutlu yazı KentOS'un etiketinde yok; boyu piksel alındı"
            ));
        }
        let mut more = Map::new();
        let mut size = 10.0;
        if let Some(f) = child(t, "Font") {
            if let Some(s) = param(f, "font-size").and_then(|s| s.parse::<f64>().ok()) {
                size = s;
            }
            if param(f, "font-style").is_some_and(|s| s == "italic" || s == "oblique") {
                more.insert("italic".into(), json!(true));
            }
            if param(f, "font-weight").is_some_and(|w| {
                w == "bold" || w == "bolder" || w.parse::<u32>().is_ok_and(|n| n >= 600)
            }) {
                more.insert("weight".into(), json!(600));
            }
            if let Some(fam) = param(f, "font-family").filter(|f| !f.is_empty())
                && !families.contains(&fam)
            {
                families.push(fam);
            }
        }
        if let Some(f) = child(t, "Fill") {
            match param(f, "fill") {
                Some(rgb) => {
                    let o = param(f, "fill-opacity").and_then(|o| o.parse::<f64>().ok());
                    more.insert("color".into(), json!(colour_with(&rgb, o)));
                }
                None => lost.push(format!(
                    "{what}: veriye bağlı rengi KentOS'un etiketinde yok; siyah alındı"
                )),
            }
        }
        if let Some(h) = child(t, "Halo") {
            let w = number(child(h, "Radius")).unwrap_or(1.0);
            let c = child(h, "Fill")
                .and_then(|f| {
                    let o = param(f, "fill-opacity").and_then(|o| o.parse::<f64>().ok());
                    param(f, "fill").map(|rgb| colour_with(&rgb, o))
                })
                .unwrap_or_else(|| "#ffffff".into());
            more.insert("halo".into(), json!({ "width": w, "color": c }));
        }
        if let Some(p) = child(t, "LabelPlacement") {
            if let Some(lp) = child(p, "LinePlacement") {
                more.insert("line".into(), json!("parallel"));
                if let Some(o) = number(child(lp, "PerpendicularOffset")).filter(|o| *o != 0.0) {
                    more.insert(
                        "position".into(),
                        json!(if o > 0.0 { "above" } else { "below" }),
                    );
                    more.insert("distance".into(), json!(o.abs()));
                }
                if child(lp, "IsRepeated").is_some_and(|r| text_of(r) == "true")
                    && let Some(g) = number(child(lp, "Gap")).filter(|g| *g > 0.0)
                {
                    more.insert("repeat".into(), json!(g));
                }
            } else if let Some(pp) = child(p, "PointPlacement") {
                let ax = child(pp, "AnchorPoint").and_then(|a| number(child(a, "AnchorPointX")));
                let ay = child(pp, "AnchorPoint").and_then(|a| number(child(a, "AnchorPointY")));
                let d = child(pp, "Displacement").map(|d| {
                    let x = number(child(d, "DisplacementX")).unwrap_or(0.0);
                    let y = number(child(d, "DisplacementY")).unwrap_or(0.0);
                    libm::hypot(x, y)
                });
                let centred = ax.unwrap_or(0.0) == 0.5
                    && ay.unwrap_or(0.0) == 0.5
                    && d.is_none_or(|d| d == 0.0);
                more.insert(
                    "point".into(),
                    json!(if centred { "center" } else { "around" }),
                );
                if let Some(d) = d.filter(|d| *d > 0.0) {
                    more.insert("distance".into(), json!((d * 1000.0).round() / 1000.0));
                }
                if number(child(pp, "Rotation")).is_some_and(|r| r != 0.0)
                    || child(pp, "Rotation").is_some_and(|r| r.children().any(|c| c.is_element()))
                {
                    lost.push(format!(
                        "{what}: döndürmesi KentOS'un etiketinde yok; alınmadı"
                    ));
                }
            }
        }
        if let Some(p) = child(t, "Priority") {
            match text_of(p).parse::<f64>() {
                Ok(v) if v.is_finite() => {
                    more.insert(
                        "priority".into(),
                        json!((v / 100.0).round().clamp(0.0, 10.0) as u8),
                    );
                }
                _ => lost.push(format!(
                    "{what}: veriye bağlı önceliği KentOS'un etiketinde yok; alınmadı"
                )),
            }
        }
        for v in t
            .children()
            .filter(|c| c.is_element() && c.tag_name().name() == "VendorOption")
        {
            // GeoServer's options QGIS writes too.
            let value = text_of(v);
            let positive = value
                .parse::<f64>()
                .ok()
                .filter(|x| x.is_finite() && *x > 0.0);
            match (v.attribute("name").unwrap_or(""), positive) {
                ("followLine", _) if value == "true" => {
                    more.insert("line".into(), json!("curved"));
                }
                ("repeat", Some(r)) => {
                    more.insert("repeat".into(), json!(r));
                }
                ("maxAngleDelta", Some(a)) => {
                    more.insert("maxAngle".into(), json!(a));
                }
                ("group", _) if value == "yes" || value == "true" => {
                    more.insert("mergeLines".into(), json!(true));
                }
                ("conflictResolution", _) if value == "false" => {
                    more.insert("overlap".into(), json!("always"));
                }
                // KentOS seeks its own places around the point; a label's graphic is not read (said below).
                (
                    "maxDisplacement" | "followLine" | "repeat" | "maxAngleDelta" | "group"
                    | "conflictResolution" | "graphic-resize" | "graphic-margin",
                    _,
                ) => {}
                (other, _) => lost.push(format!(
                    "{what}: SLD'nin “{other}” seçeneği KentOS'un etiketinde yok; alınmadı"
                )),
            }
        }
        if child(t, "Graphic").is_some() {
            lost.push(format!(
                "{what}: resmi (Graphic) KentOS'un etiketinde yok; alınmadı"
            ));
        }
        if let Some(n) = hi {
            more.insert("minScale".into(), json!(sl::ppm(n)));
        }
        if let Some(n) = lo {
            more.insert("maxScale".into(), json!(sl::ppm(n)));
        }
        if is_else {
            lost.push(format!(
                "{what}: “Diğerleri” etiketi KentOS'ta yok; her nesneye alındı"
            ));
        }
        out.push(sl::class_in(
            label,
            filter.map(filter_text),
            size,
            sl::kentos_text(&pieces),
            more,
            lost,
        ));
    }
}
