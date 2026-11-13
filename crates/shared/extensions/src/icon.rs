//! A package's icons (plan/eklenti.md §3.1): the 20×20 line-icon subset the desktop draws (kentos_ui's `icon::svg`):
//! `viewBox="0 0 20 20"`; `path`, `rect`, `circle` and `ellipse` with the attributes it reads. The desktop leaves out
//! what it cannot read, so an icon that draws partly would pass unnoticed: anything else is refused here.
//!
//! Nothing that could act or reach out: no script, `style`, `foreignObject`, `image`, `use`, link (`href`) or event
//! attribute, since none is an allowed element or attribute. The desktop reads the markup by hand, so the parts where
//! that reading and XML's would differ are refused too: comments, CDATA, processing instructions (but the XML
//! declaration), entity references, single quotes, spaces around `=`. Colours are refused: an icon is drawn in the
//! theme's colour, so `currentColor` or `none`. roxmltree refuses a DTD, so no entity bomb.

use roxmltree::{Document, Node, ParsingOptions};

/// The SVG namespace.
const SVG: &str = "http://www.w3.org/2000/svg";
/// Bytes an icon may have.
pub(crate) const MAX_BYTES: usize = 32 * 1024;
/// Elements an icon may have.
const MAX_ELEMENTS: usize = 200;

const PAINT: &[&str] = &[
    "fill",
    "fill-opacity",
    "fill-rule",
    "stroke",
    "stroke-width",
    "stroke-opacity",
    "stroke-dasharray",
    "stroke-linecap",
    "stroke-linejoin",
];

/// The attributes an element may carry besides [`PAINT`].
fn own(element: &str) -> Option<&'static [&'static str]> {
    Some(match element {
        "svg" => &["viewBox", "width", "height"],
        "path" => &["d"],
        "rect" => &["x", "y", "width", "height", "rx", "ry"],
        "circle" => &["cx", "cy", "r"],
        "ellipse" => &["cx", "cy", "rx", "ry", "transform"],
        _ => return None,
    })
}

fn numbers(text: &str) -> Option<Vec<f64>> {
    text.split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect()
}

fn number(text: &str) -> Option<f64> {
    match numbers(text)?.as_slice() {
        [v] if v.abs() <= 1000.0 => Some(*v),
        _ => None,
    }
}

/// What is wrong with the value of an allowed attribute.
fn value_problem(name: &str, value: &str) -> Option<String> {
    let ok = match name {
        "fill" | "stroke" => {
            if !matches!(value, "none" | "currentColor") {
                return Some(format!(
                    "“{name}=\"{value}\"”: ikon tema rengiyle çizilir; renk yerine “currentColor” ya da “none” yazın."
                ));
            }
            true
        }
        "fill-opacity" | "stroke-opacity" => {
            number(value).is_some_and(|v| (0.0..=1.0).contains(&v))
        }
        "stroke-width" => number(value).is_some_and(|v| v > 0.0 && v <= 20.0),
        "stroke-dasharray" => {
            numbers(value).is_some_and(|v| !v.is_empty() && v.iter().all(|d| *d >= 0.0))
        }
        "stroke-linecap" => matches!(value, "butt" | "round" | "square"),
        "stroke-linejoin" => matches!(value, "round" | "miter" | "bevel"),
        "fill-rule" => matches!(value, "nonzero" | "evenodd"),
        "viewBox" => numbers(value).is_some_and(|v| v == [0.0, 0.0, 20.0, 20.0]),
        "width" | "height" => {
            number(value.strip_suffix("px").unwrap_or(value)).is_some_and(|v| v > 0.0)
        }
        "transform" => value
            .trim()
            .strip_prefix("rotate(")
            .and_then(|t| t.strip_suffix(')'))
            .and_then(numbers)
            .is_some_and(|v| (1..=3).contains(&v.len())),
        "d" => {
            value.len() <= 8000
                && value.trim_start().starts_with(['M', 'm'])
                && value.chars().all(|c| {
                    "MmLlHhVvCcSsQqTtAaZz0123456789.,+-eE".contains(c) || c.is_ascii_whitespace()
                })
        }
        _ => number(value).is_some(),
    };
    (!ok).then(|| match name {
        "viewBox" => {
            "İkonun viewBox'ı “0 0 20 20” olmalı: masaüstü ikonları 20×20'lik ızgarada çizer."
                .to_owned()
        }
        "d" => "Yol verisi (d) yalnız SVG yol komutları ve sayılardan oluşur ve M ile başlar."
            .to_owned(),
        "transform" => "Elipste yalnız “rotate(açı)” dönüşümü okunur.".to_owned(),
        _ => format!("“{name}=\"{value}\"” okunamadı: geçerli bir değer yazın."),
    })
}

/// What is wrong with an SVG icon; none when the desktop draws it whole.
pub(crate) fn problem(text: &str) -> Option<String> {
    if text.len() > MAX_BYTES {
        return Some(format!(
            "İkon {} bayt; en çok {} KB olur.",
            text.len(),
            MAX_BYTES / 1024
        ));
    }
    let body = text.trim_start_matches('\u{FEFF}').trim_start();
    let body = match body.strip_prefix("<?xml") {
        Some(rest) => match rest.find("?>") {
            Some(end) => &rest[end + 2..],
            None => return Some("XML bildirimi kapanmıyor.".to_owned()),
        },
        None => body,
    };
    for (marker, what) in [
        ("<!--", "yorum"),
        ("<![CDATA[", "CDATA bölümü"),
        ("<?", "işleme talimatı"),
        ("<!", "DOCTYPE ya da bildirim"),
        ("&", "varlık başvurusu (&…;)"),
        ("='", "tek tırnaklı öznitelik"),
    ] {
        if body.contains(marker) {
            return Some(format!(
                "İkonda {what} var; masaüstünün okuduğu alt kümede olmaz. Düz SVG olarak kaydedin ve silin."
            ));
        }
    }
    let doc = match Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 4 * MAX_ELEMENTS as u32,
        },
    ) {
        Ok(doc) => doc,
        Err(e) => return Some(format!("İkon geçerli bir XML değil: {e}.")),
    };
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Some("İkonun kök öğesi <svg> olmalı.".to_owned());
    }
    if root.attribute("viewBox").is_none() {
        return Some("İkonun viewBox'ı yok; viewBox=\"0 0 20 20\" yazın.".to_owned());
    }
    let mut elements = 0usize;
    for node in root.descendants() {
        if node.is_text() {
            if node.text().is_some_and(|t| !t.trim().is_empty()) {
                return Some("İkonda yazı var; ikon yalnız çizimden oluşur.".to_owned());
            }
            continue;
        }
        if !node.is_element() {
            return Some(
                "İkonda okunmayan bir düğüm var (yorum ya da işleme talimatı).".to_owned(),
            );
        }
        elements += 1;
        if elements > MAX_ELEMENTS {
            return Some(format!("İkonda {MAX_ELEMENTS}'den çok öğe var."));
        }
        if let Some(problem) = element_problem(text, node, node == root) {
            return Some(problem);
        }
    }
    None
}

fn element_problem(text: &str, node: Node<'_, '_>, is_root: bool) -> Option<String> {
    let name = node.tag_name().name();
    if node.tag_name().namespace().is_some_and(|ns| ns != SVG) {
        return Some(format!(
            "“{name}” başka bir ad alanından (bir çizim programının ek bilgisi); düz SVG olarak kaydedin."
        ));
    }
    if is_root != (name == "svg") {
        return Some(if is_root {
            "İkonun kök öğesi <svg> olmalı.".to_owned()
        } else {
            "İç içe <svg> olmaz.".to_owned()
        });
    }
    let Some(own) = own(name) else {
        return Some(format!(
            "<{name}> masaüstünün çizdiği alt kümede yok; yalnız path, rect, circle ve ellipse olur. Grupları (g) \
             açın, metni yola çevirin."
        ));
    };
    // The `xml` prefix is every document's, declared or not.
    if is_root
        && node
            .namespaces()
            .any(|ns| ns.name() != Some("xml") && (ns.name().is_some() || ns.uri() != SVG))
    {
        return Some(
            "İkonda SVG dışında bir ad alanı tanımlı (bir çizim programının ek bilgisi); düz SVG olarak kaydedin."
                .to_owned(),
        );
    }
    for attribute in node.attributes() {
        let attr = attribute.name();
        if attribute.namespace().is_some() || !(own.contains(&attr) || PAINT.contains(&attr)) {
            return Some(format!(
                "<{name}> öğesinde “{attr}” özniteliği okunmaz (bağlantı, olay, stil ve sınıf olmaz); silin."
            ));
        }
        let range = attribute.range();
        let raw = text.get(range.clone()).unwrap_or("");
        if !(raw.starts_with(attr) && raw[attr.len()..].starts_with("=\"") && raw.ends_with('"')) {
            return Some(format!(
                "“{attr}” {attr}=\"…\" biçiminde yazılmalı: çift tırnak, eşittirin çevresinde boşluk yok."
            ));
        }
        if let Some(problem) = value_problem(attr, attribute.value()) {
            return Some(problem);
        }
    }
    let required: &[&str] = match name {
        "path" => &["d"],
        "rect" => &["width", "height"],
        "circle" => &["r"],
        "ellipse" => &["rx", "ry"],
        _ => &[],
    };
    required
        .iter()
        .find(|a| node.attribute(**a).is_none())
        .map(|a| format!("<{name}> öğesinin “{a}” özniteliği yok; o olmadan çizilmez."))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 5h14M3 10h14"/>
  <rect x="2" y="2" width="16" height="16" rx="2"/>
  <circle cx="10" cy="10" r="2" fill="currentColor" stroke="none"/>
  <ellipse cx="10" cy="10" rx="4" ry="2" transform="rotate(30 10 10)"/>
</svg>
"#;

    #[test]
    fn the_desktops_subset_passes() {
        assert_eq!(problem(GOOD), None);
        let declared = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{GOOD}");
        assert_eq!(problem(&declared), None);
        let bare = r#"<svg viewBox="0 0 20 20"><path d="M1 1L19 19"/></svg>"#;
        assert_eq!(problem(bare), None, "no namespace is the same subset");
    }

    #[test]
    fn what_could_act_reach_out_or_be_read_otherwise_is_refused() {
        let refused = [
            (
                r#"<svg viewBox="0 0 20 20"><script>alert(1)</script></svg>"#,
                "<script>",
            ),
            (
                r#"<svg viewBox="0 0 20 20" onload="x()"><path d="M0 0"/></svg>"#,
                "onload",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><image href="http://x/y.png"/></svg>"#,
                "<image>",
            ),
            (
                r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 20 20"><use xlink:href="#a"/></svg>"##,
                "ad alanı",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><style>path{}</style></svg>"#,
                "<style>",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><foreignObject/></svg>"#,
                "<foreignObject>",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><g><path d="M0 0"/></g></svg>"#,
                "<g>",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><!-- <path d="M0 0"/> --></svg>"#,
                "yorum",
            ),
            (
                r#"<!DOCTYPE svg [<!ENTITY a "b">]><svg viewBox="0 0 20 20"/>"#,
                "DOCTYPE",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><path d='M0 0'/></svg>"#,
                "tek tırnaklı",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><path d = "M0 0"/></svg>"#,
                "çift tırnak",
            ),
            (
                r#"<svg viewBox="0 0 24 24"><path d="M0 0"/></svg>"#,
                "0 0 20 20",
            ),
            (r#"<svg><path d="M0 0"/></svg>"#, "viewBox"),
            (
                r##"<svg viewBox="0 0 20 20"><path d="M0 0" fill="#f00"/></svg>"##,
                "currentColor",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><path d="M0 0" class="a"/></svg>"#,
                "“class”",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><path d="url(#x)"/></svg>"#,
                "Yol verisi",
            ),
            (
                r#"<svg viewBox="0 0 20 20"><rect width="2"/></svg>"#,
                "“height”",
            ),
            (r#"<svg viewBox="0 0 20 20">yazı</svg>"#, "yazı"),
            (
                r#"<svg viewBox="0 0 20 20"><svg viewBox="0 0 20 20"/></svg>"#,
                "İç içe",
            ),
            (r#"<svg viewBox="0 0 20 20"><text>a</text></svg>"#, "<text>"),
            (r#"<html/>"#, "kök"),
            ("<svg viewBox=\"0 0 20 20\"><path d=\"M0 0\"></svg>", "XML"),
        ];
        for (svg, needle) in refused {
            let problem = problem(svg).unwrap_or_else(|| panic!("passed: {svg}"));
            assert!(problem.contains(needle), "{svg}\n→ {problem}");
        }
        assert!(
            problem(&format!("{GOOD}{}", " ".repeat(MAX_BYTES)))
                .unwrap()
                .contains("KB")
        );
    }
}
