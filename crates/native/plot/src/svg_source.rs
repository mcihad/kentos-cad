//! SVG output must be self-contained: unresolved external content must never disappear silently.
pub(crate) fn validate(data: &[u8]) -> Result<(), String> {
    inspect(data, 0)
}
fn reference(value: &str, depth: u8) -> Result<(), String> {
    let v = value.trim();
    if v.starts_with('#') {
        return Ok(());
    }
    if v.starts_with("data:") {
        if v.to_ascii_lowercase().starts_with("data:image/svg+xml") {
            let (_, bytes) = kentos_native_style::bitmap::data_url(v)
                .ok_or("Gömülü SVG veri adresi base64 biçiminde olmalı.")?;
            inspect(&bytes, depth + 1)?;
        }
        if v.to_ascii_lowercase().starts_with("data:image/")
            && !v.to_ascii_lowercase().starts_with("data:image/svg+xml")
        {
            let (mime, bytes) = kentos_native_style::bitmap::data_url(v)
                .ok_or("Gömülü SVG resmi okunamadı; geçerli base64 veri adresi kullanın.")?;
            let decoded = match mime {
                "image/png" => kentos_native_style::bitmap::png(&bytes),
                "image/jpg" | "image/jpeg" => kentos_native_style::bitmap::jpeg(&bytes),
                _ => return Err(
                    "SVG içindeki gömülü resim biçimi desteklenmiyor; PNG/JPEG veya SVG kullanın."
                        .into(),
                ),
            };
            if decoded.is_none() {
                return Err(
                    "SVG içindeki gömülü resim çözülemedi; eksik içerikle çıktı üretilmeyecek."
                        .into(),
                );
            }
        }
        return Ok(());
    }
    Err(format!(
        "SVG dış kaynak içeriyor: {v}. Kaynağı SVG içine gömün; eksik içerikle çıktı üretilmeyecek."
    ))
}
fn css(value: &str, depth: u8) -> Result<(), String> {
    let lower = value.to_ascii_lowercase();
    let mut start = 0;
    while let Some(n) = lower[start..].find("url(") {
        let at = start + n + 4;
        let end = value[at..]
            .find(')')
            .ok_or("SVG stilinde kapanmayan url() var.")?
            + at;
        reference(value[at..end].trim().trim_matches(['\'', '"']), depth)?;
        start = end + 1;
    }
    if lower.contains("@import") {
        return Err("SVG dış stil dosyası çağırıyor; stili SVG içine gömün.".into());
    }
    Ok(())
}
fn inspect(data: &[u8], depth: u8) -> Result<(), String> {
    if depth > 8 {
        return Err("SVG iç içe veri sınırını aşıyor; resmi sadeleştirin.".into());
    }
    let text = std::str::from_utf8(data).map_err(|_| "SVG UTF-8 metni okunamadı.")?;
    let xml = roxmltree::Document::parse(text).map_err(|e| format!("SVG okunamadı: {e}"))?;
    for node in xml.descendants().filter(|n| n.is_element()) {
        for a in node.attributes() {
            if a.name() == "href" && node.tag_name().name() != "a" {
                reference(a.value(), depth)?;
                if a.value().trim().starts_with("data:")
                    && (!matches!(node.tag_name().name(), "image" | "feImage")
                        || !a.value().trim().starts_with("data:image/"))
                {
                    return Err(
                        "SVG href veri adresi yalnız desteklenen gömülü resimlerde kullanılabilir."
                            .into(),
                    );
                }
            }
            css(a.value(), depth)?;
        }
        if node.tag_name().name() == "style" {
            for child in node.children().filter(|n| n.is_text()) {
                css(child.text().unwrap_or(""), depth)?;
            }
        }
    }
    Ok(())
}

/// Preserve XML bytes while adding SVG 1.1 references understood by the print rasterizer.
pub(crate) fn prepare(data: &[u8]) -> Result<Vec<u8>, String> {
    validate(data)?;
    let text = std::str::from_utf8(data).map_err(|e| e.to_string())?;
    let xml = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
    let mut insertions = Vec::new();
    for node in xml.descendants().filter(|n| n.is_element()) {
        for a in node.attributes() {
            if a.name() == "href" && a.namespace().is_none() {
                insertions.push((a.range_qname().start, "xlink:"));
            }
        }
    }
    if insertions.is_empty() {
        return Ok(data.to_vec());
    }
    let root = xml.root_element();
    match root.lookup_namespace_uri(Some("xlink")) {
        Some("http://www.w3.org/1999/xlink") => {}
        Some(_) => {
            return Err(
                "SVG xlink ad alanı geçersiz; http://www.w3.org/1999/xlink kullanın.".into(),
            );
        }
        None => {
            let start = root.range().start + 1;
            let at = start
                + text[start..]
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .ok_or("SVG kökü okunamadı.")?;
            insertions.push((at, " xmlns:xlink=\"http://www.w3.org/1999/xlink\""));
        }
    }
    insertions.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    let mut out = text.to_owned();
    for (at, value) in insertions {
        out.insert_str(at, value);
    }
    Ok(out.into_bytes())
}

/// The CPU symbol reader is deliberately smaller than the page SVG renderer; fail on content it would omit.
pub(crate) fn vector_compatible(svg: &str) -> Result<(), String> {
    validate(svg.as_bytes())?;
    let xml = roxmltree::Document::parse(svg).map_err(|e| e.to_string())?;
    for n in xml.descendants().filter(|n| n.is_element()) {
        if matches!(
            n.tag_name().name(),
            "linearGradient"
                | "radialGradient"
                | "pattern"
                | "clipPath"
                | "filter"
                | "style"
                | "marker"
                | "image"
                | "foreignObject"
        ) {
            return Err(format!(
                "SVG sembolündeki {} içeriği bu baskıda korunamıyor; sembolü PNG'ye dönüştürün veya desteklenen düz SVG kullanın.",
                n.tag_name().name()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sheet_preflight_svg_references_and_svg2_normalization_are_explicit() {
        let cases = [
            ("<rect width='10' height='10' fill='red'/>", true),
            (
                "<defs><linearGradient id='p'/></defs><rect fill='url(#p)'/>",
                true,
            ),
            (
                "<defs><rect id='r' width='10' height='10'/></defs><use href='#r'/>",
                true,
            ),
            ("<image href='gone.png'/>", false),
            ("<image href='./gone.png'/>", false),
            ("<image href='../gone.png'/>", false),
            ("<image href='/tmp/gone.png'/>", false),
            ("<image href='https://example.invalid/img.png'/>", false),
            ("<image href='file:///tmp/gone.png'/>", false),
            ("<image href='C:\\gone.png'/>", false),
            ("<use href='shape.svg#r'/>", false),
            ("<rect fill='url(another.svg#p)'/>", false),
            ("<rect style='fill:url(another.svg#p)'/>", false),
            (
                "<style>rect {fill:url(another.svg#p)}</style><rect/>",
                false,
            ),
            ("<style>@import 'other.css';</style>", false),
            ("<image href='data:image/png;base64,bm90IGEgcG5n'/>", false),
            (
                "<image href='data:image/svg+xml;base64,PG5vdC1jbG9zZWQ+'/>",
                false,
            ),
            ("<g><image href='dir&#47;gone.png'/></g>", false),
            (
                "<a href='https://example.invalid'><rect width='1' height='1'/></a>",
                true,
            ),
            (
                "<rect clip-path='url(#clip)'/><defs><clipPath id='clip'><rect width='1' height='1'/></clipPath></defs>",
                true,
            ),
        ];
        assert!(cases.len() >= 20);
        for (body, ok) in cases {
            let svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'>{body}</svg>"
            );
            assert_eq!(prepare(svg.as_bytes()).is_ok(), ok, "{body}");
        }
        let data=prepare(b"<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'><defs><rect id='r' width='10' height='10' fill='red'/></defs><use href='#r'/></svg>").unwrap();
        let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).unwrap();
        let mut pix = resvg::tiny_skia::Pixmap::new(10, 10).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pix.as_mut(),
        );
        assert_eq!(
            &pix.data()[0..4],
            &[255, 0, 0, 255],
            "SVG 2 href must actually draw its referenced geometry"
        );
    }
}
