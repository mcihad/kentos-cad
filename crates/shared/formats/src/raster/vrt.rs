//! A virtual mosaic (docs/adr/0517): GDAL's VRT, many raster files shown and
//! analysed as one, no pixel copied. The file is GDAL's own (`VRTDataset`
//! with `SimpleSource` and `ComplexSource` bands), so that GDAL, QGIS and
//! KentOS open the same mosaic; a later source lies over the earlier ones,
//! a source's nodata (its `NODATA`) lets the ones below show through.
//!
//! What KentOS reads of a VRT: its size, `GeoTransform`, `SRS` (an `EPSG:n`
//! or a WKT whose last authority is EPSG's), each band's data type and
//! `NoDataValue`, and the sources' file, band, `SrcRect`, `DstRect` and
//! `NODATA`. Every band must take the same files with the same rectangles
//! (as gdalbuildvrt writes them); a warped, derived or pansharpened VRT, a
//! source of a VRT, a kernel filter or a scale/LUT is refused, said why.
//! Limits (docs/adr/0009): [`MAX_TEXT`] bytes, [`MAX_SOURCES`] sources.

use kentos_contracts::RasterSample;

use super::RasterError;
use crate::xml;

/// The largest VRT text read.
pub const MAX_TEXT: usize = 16 * 1024 * 1024;
/// The most sources a mosaic takes.
pub const MAX_SOURCES: usize = 10_000;

/// A source of a band: a rectangle of a file's band placed on the mosaic.
#[derive(Clone, Debug, PartialEq)]
pub struct VrtSource {
    /// As the VRT holds it.
    pub file: String,
    /// The file's path is from the VRT's folder.
    pub relative: bool,
    /// The file's band, from 1.
    pub band: u32,
    /// Pixels of the file, `[x, y, w, h]`.
    pub src: [f64; 4],
    /// Pixels of the mosaic, `[x, y, w, h]`.
    pub dst: [f64; 4],
    /// The file's samples that are no data (a `ComplexSource`'s `NODATA`).
    pub nodata: Option<f64>,
}

/// A band of the mosaic.
#[derive(Clone, Debug, PartialEq)]
pub struct VrtBand {
    pub sample: RasterSample,
    pub nodata: Option<f64>,
    /// GDAL's colour name (`Gray`, `Red`, `Green`, `Blue`, `Alpha`, …).
    pub color: Option<String>,
    /// Bottom first: a later source lies over the earlier ones.
    pub sources: Vec<VrtSource>,
}

/// A VRT mosaic.
#[derive(Clone, Debug, PartialEq)]
pub struct Vrt {
    pub width: u32,
    pub height: u32,
    /// GDAL's geotransform `[x₀, a, b, y₀, c, d]`.
    pub affine: Option<[f64; 6]>,
    /// The `SRS` as written.
    pub srs: Option<String>,
    /// Its EPSG code, when it names one.
    pub epsg: Option<u32>,
    pub bands: Vec<VrtBand>,
}

/// A file of the mosaic, as every band takes it: its rectangles, nodata and
/// which of its bands each band of the mosaic is.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub file: String,
    pub relative: bool,
    pub src: [f64; 4],
    pub dst: [f64; 4],
    pub nodata: Option<f64>,
    /// The file's band (from 1) for each band of the mosaic.
    pub bands: Vec<u32>,
}

/// Whether the bytes begin a VRT (`<VRTDataset`, after a byte order mark and white space).
pub fn sniff(head: &[u8]) -> bool {
    let head = head.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(head);
    let start = head.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(head.len());
    head[start..].starts_with(b"<VRTDataset")
}

/// GDAL's name of a sample type.
pub fn type_name(s: RasterSample) -> &'static str {
    match s {
        RasterSample::U8 => "Byte",
        RasterSample::I8 => "Int8",
        RasterSample::U16 => "UInt16",
        RasterSample::I16 => "Int16",
        RasterSample::U32 => "UInt32",
        RasterSample::I32 => "Int32",
        RasterSample::F32 => "Float32",
        RasterSample::F64 => "Float64",
    }
}

fn sample_of(name: &str) -> Option<RasterSample> {
    Some(match name {
        "Byte" => RasterSample::U8,
        "Int8" => RasterSample::I8,
        "UInt16" => RasterSample::U16,
        "Int16" => RasterSample::I16,
        "UInt32" => RasterSample::U32,
        "Int32" => RasterSample::I32,
        "Float32" => RasterSample::F32,
        "Float64" => RasterSample::F64,
        _ => return None,
    })
}

/// The EPSG code an `SRS` names: `EPSG:n`, or a WKT's last `AUTHORITY["EPSG","n"]` or `ID["EPSG",n]` (the
/// coordinate system's own; the datum's and units' come before).
pub fn epsg_of(srs: &str) -> Option<u32> {
    let t = srs.trim();
    if let Some(rest) = t.strip_prefix("EPSG:").or_else(|| t.strip_prefix("epsg:")) {
        return rest.trim().parse().ok().filter(|&n| n > 0);
    }
    let mut found = None;
    for key in ["AUTHORITY[\"EPSG\",", "ID[\"EPSG\","] {
        let mut from = 0;
        while let Some(at) = t[from..].find(key) {
            let after = &t[from + at + key.len()..];
            let digits: String = after.trim_start_matches('"').chars().take_while(char::is_ascii_digit).collect();
            if let Ok(n) = digits.parse::<u32>() {
                found = Some((from + at, n));
            }
            from += at + key.len();
        }
    }
    // The last one in the text is the outermost's in WKT 1 and WKT 2 alike.
    found.map(|(_, n)| n)
}

fn number(text: &str, what: &str) -> Result<f64, RasterError> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| RasterError::new(format!("VRT'nin {what} değeri sayı değil: “{}”.", text.trim())))
}

fn rect(node: roxmltree::Node<'_, '_>, what: &str) -> Result<[f64; 4], RasterError> {
    let mut r = [0.0; 4];
    for (k, name) in ["xOff", "yOff", "xSize", "ySize"].into_iter().enumerate() {
        r[k] = number(node.attribute(name).unwrap_or("0"), &format!("{what} {name}"))?;
    }
    if !(r[2] > 0.0 && r[3] > 0.0) {
        return Err(RasterError::new(format!("VRT'nin {what} dikdörtgeni boş.")));
    }
    Ok(r)
}

fn child<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.children().find(|c| c.has_tag_name(name))
}

/// A VRT's text read; why not, in the window's words.
pub fn parse(text: &str) -> Result<Vrt, RasterError> {
    if text.len() > MAX_TEXT {
        return Err(RasterError::new("VRT dosyası 16 MB'tan büyük."));
    }
    let doc = xml::document(text).map_err(|_| RasterError::new("VRT dosyası okunamadı: XML değil ya da çok derin."))?;
    let root = doc.root_element();
    if !root.has_tag_name("VRTDataset") {
        return Err(RasterError::new("Dosya bir VRT (VRTDataset) değil."));
    }
    if let Some(sub) = root.attribute("subClass") {
        return Err(RasterError::new(format!(
            "Bu VRT bir mozaik değil ({sub}); yalnız dosyaları yan yana koyan VRT açılır."
        )));
    }
    let size = |name: &str| -> Result<u32, RasterError> {
        root.attribute(name)
            .and_then(|v| v.trim().parse::<u32>().ok())
            .filter(|&v| v > 0)
            .ok_or_else(|| RasterError::new(format!("VRT'nin {name} değeri yok ya da geçersiz.")))
    };
    let (width, height) = (size("rasterXSize")?, size("rasterYSize")?);
    let affine = match child(root, "GeoTransform") {
        Some(g) => {
            let parts: Vec<&str> = g.text().unwrap_or("").split(',').collect();
            if parts.len() != 6 {
                return Err(RasterError::new("VRT'nin GeoTransform'u altı sayı değil."));
            }
            let mut a = [0.0; 6];
            for (k, p) in parts.iter().enumerate() {
                a[k] = number(p, "GeoTransform")?;
            }
            if a[1] * a[5] - a[2] * a[4] == 0.0 {
                return Err(RasterError::new("VRT'nin GeoTransform'u tersine çevrilemiyor."));
            }
            Some(a)
        }
        None => None,
    };
    let srs = child(root, "SRS").and_then(|s| s.text()).map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    let epsg = srs.as_deref().and_then(epsg_of);
    let mut bands = Vec::new();
    let mut count = 0usize;
    for b in root.children().filter(|c| c.has_tag_name("VRTRasterBand")) {
        if let Some(sub) = b.attribute("subClass") {
            return Err(RasterError::new(format!("VRT'nin bandı türetilmiş ({sub}); açılmaz.")));
        }
        let t = b.attribute("dataType").unwrap_or("Byte");
        let sample = sample_of(t).ok_or_else(|| RasterError::new(format!("VRT'nin “{t}” veri türü desteklenmiyor.")))?;
        let nodata = match child(b, "NoDataValue").and_then(|n| n.text()) {
            Some(v) if v.trim().eq_ignore_ascii_case("nan") => Some(f64::NAN),
            Some(v) => Some(number(v, "NoDataValue")?),
            None => None,
        };
        let color = child(b, "ColorInterp").and_then(|c| c.text()).map(|c| c.trim().to_owned());
        let mut sources = Vec::new();
        for s in b.children().filter(|c| c.is_element()) {
            let tag = s.tag_name().name();
            match tag {
                "SimpleSource" | "ComplexSource" => {}
                "AveragedSource" | "KernelFilteredSource" | "ArraySource" => {
                    return Err(RasterError::new(format!("VRT'nin {tag} kaynağı desteklenmiyor; yalnız SimpleSource ve ComplexSource.")));
                }
                _ => continue,
            }
            for unsupported in ["ScaleOffset", "ScaleRatio", "LUT", "ComplexSourceRatio"] {
                if child(s, unsupported).is_some() {
                    return Err(RasterError::new(format!("VRT'nin kaynağında {unsupported} var; değer dönüştüren kaynak açılmaz.")));
                }
            }
            count += 1;
            if count > MAX_SOURCES {
                return Err(RasterError::new(format!("VRT'de {MAX_SOURCES}'dan çok kaynak var.")));
            }
            let name = child(s, "SourceFilename").ok_or_else(|| RasterError::new("VRT'nin bir kaynağının dosyası yok."))?;
            let file = name.text().unwrap_or("").trim().to_owned();
            if file.is_empty() {
                return Err(RasterError::new("VRT'nin bir kaynağının dosya adı boş."));
            }
            let relative = name.attribute("relativeToVRT").is_some_and(|v| v.trim() == "1");
            let band = match child(s, "SourceBand").and_then(|n| n.text()) {
                Some(v) => v.trim().parse::<u32>().ok().filter(|&n| n > 0).ok_or_else(|| {
                    RasterError::new(format!("VRT'nin kaynak bandı geçersiz: “{}”.", v.trim()))
                })?,
                None => 1,
            };
            let props = child(s, "SourceProperties");
            let whole = |k: &str| props.and_then(|p| p.attribute(k)).and_then(|v| v.parse::<f64>().ok());
            let src = match child(s, "SrcRect") {
                Some(r) => rect(r, "SrcRect")?,
                None => match (whole("RasterXSize"), whole("RasterYSize")) {
                    (Some(w), Some(h)) => [0.0, 0.0, w, h],
                    _ => return Err(RasterError::new("VRT'nin bir kaynağında SrcRect yok.")),
                },
            };
            let dst = match child(s, "DstRect") {
                Some(r) => rect(r, "DstRect")?,
                None => src,
            };
            let nodata = match child(s, "NODATA").and_then(|n| n.text()) {
                Some(v) if v.trim().eq_ignore_ascii_case("nan") => Some(f64::NAN),
                Some(v) => Some(number(v, "NODATA")?),
                None => None,
            };
            sources.push(VrtSource { file, relative, band, src, dst, nodata });
        }
        bands.push(VrtBand { sample, nodata, color, sources });
    }
    if bands.is_empty() {
        return Err(RasterError::new("VRT'de band yok."));
    }
    Ok(Vrt { width, height, affine, srs, epsg, bands })
}

/// The mosaic's files, bottom first, as every band takes them; why not when the bands take different files or
/// rectangles, or differ in data type.
pub fn members(v: &Vrt) -> Result<Vec<Member>, RasterError> {
    let first = &v.bands[0];
    if first.sources.is_empty() {
        return Err(RasterError::new("VRT'de kaynak dosya yok."));
    }
    let mut out: Vec<Member> = first
        .sources
        .iter()
        .map(|s| Member {
            file: s.file.clone(),
            relative: s.relative,
            src: s.src,
            dst: s.dst,
            nodata: s.nodata,
            bands: vec![s.band],
        })
        .collect();
    for b in &v.bands[1..] {
        if b.sample != first.sample {
            return Err(RasterError::new("VRT'nin bantlarının veri türleri farklı; açılmaz."));
        }
        let same = b.sources.len() == out.len()
            && b.sources.iter().zip(&out).all(|(s, m)| {
                s.file == m.file && s.relative == m.relative && s.src == m.src && s.dst == m.dst
                    && (s.nodata == m.nodata || (s.nodata.is_some_and(f64::is_nan) && m.nodata.is_some_and(f64::is_nan)))
            });
        if !same {
            return Err(RasterError::new(
                "VRT'nin bantları farklı dosyalardan; yalnız her bandı aynı dosyalardan olan mozaik açılır.",
            ));
        }
        for (s, m) in b.sources.iter().zip(out.iter_mut()) {
            m.bands.push(s.band);
        }
    }
    Ok(out)
}

/// The bytes of blocks a mosaic's file keeps at least (its share of the mosaic's budget otherwise).
pub const MEMBER_BUDGET_LEAST: usize = 4 * 1024 * 1024;

/// Where a mosaic's file is: from `folder` (the VRT's) when the VRT names it relatively, its `/` and `\` alike.
pub fn member_path(folder: &std::path::Path, m: &Member) -> std::path::PathBuf {
    let name = m.file.replace('\\', "/");
    if m.relative && !std::path::Path::new(&name).is_absolute() {
        let mut p = folder.to_path_buf();
        for part in name.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    p.pop();
                }
                n => p.push(n),
            }
        }
        p
    } else {
        std::path::PathBuf::from(m.file.clone())
    }
}

/// A VRT's mosaic reader (docs/adr/0517 §2): its text read, each file opened by the host's `open` (its path and the
/// bytes of blocks it may keep) bottom first, and the files' paths, which a block's `member` (from 1) names. A file
/// that is itself a VRT is refused.
pub fn open_mosaic(
    text: &str,
    folder: &std::path::Path,
    budget: usize,
    mut open: impl FnMut(&std::path::Path, usize) -> Result<super::source::Reader, String>,
) -> Result<(super::source::Reader, Vec<std::path::PathBuf>), RasterError> {
    let v = parse(text)?;
    let members = members(&v)?;
    let each = (budget / members.len().max(1)).max(MEMBER_BUDGET_LEAST);
    let mut files = Vec::with_capacity(members.len());
    let mut paths = Vec::with_capacity(members.len());
    for m in members {
        let path = member_path(folder, &m);
        let reader = open(&path, each).map_err(|why| RasterError::new(format!("Mozaiğin dosyası “{}” açılamadı: {why}", m.file)))?;
        paths.push(path);
        files.push((m, reader));
    }
    Ok((super::source::Reader::mosaic(&v, files, budget)?, paths))
}

/// XML's text with `&`, `<`, `>`, `"` and `'` escaped.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// A number as the VRT holds it: the shortest text that reads back the same.
fn num(v: f64) -> String {
    if v.is_nan() {
        "nan".to_owned()
    } else if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// A source's rectangle element.
fn rect_text(name: &str, r: &[f64; 4]) -> String {
    format!(
        "<{name} xOff=\"{}\" yOff=\"{}\" xSize=\"{}\" ySize=\"{}\" />",
        num(r[0]),
        num(r[1]),
        num(r[2]),
        num(r[3])
    )
}

/// A VRT's text, as GDAL writes one: a source with nodata is a `ComplexSource`, one without a `SimpleSource`.
pub fn write(v: &Vrt) -> String {
    let mut s = format!("<VRTDataset rasterXSize=\"{}\" rasterYSize=\"{}\">\n", v.width, v.height);
    if let Some(srs) = &v.srs {
        s.push_str(&format!("  <SRS>{}</SRS>\n", escaped(srs)));
    }
    if let Some(a) = &v.affine {
        s.push_str(&format!(
            "  <GeoTransform>{}</GeoTransform>\n",
            a.iter().map(|x| num(*x)).collect::<Vec<_>>().join(", ")
        ));
    }
    for (k, b) in v.bands.iter().enumerate() {
        s.push_str(&format!("  <VRTRasterBand dataType=\"{}\" band=\"{}\">\n", type_name(b.sample), k + 1));
        if let Some(n) = b.nodata {
            s.push_str(&format!("    <NoDataValue>{}</NoDataValue>\n", num(n)));
        }
        if let Some(c) = &b.color {
            s.push_str(&format!("    <ColorInterp>{}</ColorInterp>\n", escaped(c)));
        }
        for src in &b.sources {
            let tag = if src.nodata.is_some() { "ComplexSource" } else { "SimpleSource" };
            s.push_str(&format!("    <{tag}>\n"));
            s.push_str(&format!(
                "      <SourceFilename relativeToVRT=\"{}\">{}</SourceFilename>\n",
                u8::from(src.relative),
                escaped(&src.file)
            ));
            s.push_str(&format!("      <SourceBand>{}</SourceBand>\n", src.band));
            s.push_str(&format!("      {}\n", rect_text("SrcRect", &src.src)));
            s.push_str(&format!("      {}\n", rect_text("DstRect", &src.dst)));
            if let Some(n) = src.nodata {
                s.push_str(&format!("      <NODATA>{}</NODATA>\n", num(n)));
            }
            s.push_str(&format!("    </{tag}>\n"));
        }
        s.push_str("  </VRTRasterBand>\n");
    }
    s.push_str("</VRTDataset>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_vrt_reads_back() {
        let v = Vrt {
            width: 130,
            height: 80,
            affine: Some([500000.0, 5.0, 0.0, 4400300.0, 0.0, -5.0]),
            srs: Some("EPSG:5254".into()),
            epsg: Some(5254),
            bands: vec![VrtBand {
                sample: RasterSample::F32,
                nodata: Some(-9999.0),
                color: Some("Gray".into()),
                sources: vec![
                    VrtSource {
                        file: "a & b.tif".into(),
                        relative: true,
                        band: 1,
                        src: [0.0, 0.0, 40.0, 30.0],
                        dst: [0.0, 0.0, 80.0, 60.0],
                        nodata: Some(-9999.0),
                    },
                    VrtSource {
                        file: "/abs/c.tif".into(),
                        relative: false,
                        band: 1,
                        src: [0.0, 0.0, 30.0, 30.0],
                        dst: [40.5, 10.25, 30.0, 30.0],
                        nodata: None,
                    },
                ],
            }],
        };
        assert_eq!(parse(&write(&v)).unwrap(), v);
        assert!(sniff(write(&v).as_bytes()));
    }

    #[test]
    fn the_coordinate_systems_code_is_the_last_authority() {
        assert_eq!(epsg_of("EPSG:5254"), Some(5254));
        assert_eq!(
            epsg_of("PROJCS[\"TUREF / TM30\",GEOGCS[\"TUREF\",AUTHORITY[\"EPSG\",\"5252\"]],AUTHORITY[\"EPSG\",\"5254\"]]"),
            Some(5254)
        );
        assert_eq!(epsg_of("LOCAL_CS[\"x\"]"), None);
    }

    #[test]
    fn what_is_not_a_mosaic_is_refused() {
        assert!(parse("<VRTDataset rasterXSize=\"2\" rasterYSize=\"2\" subClass=\"VRTWarpedDataset\"/>").is_err());
        assert!(parse("<GDAL/>").is_err());
        let two = "<VRTDataset rasterXSize=\"2\" rasterYSize=\"2\"><VRTRasterBand dataType=\"Byte\" band=\"1\"><SimpleSource><SourceFilename>a.tif</SourceFilename><SrcRect xOff=\"0\" yOff=\"0\" xSize=\"2\" ySize=\"2\"/><DstRect xOff=\"0\" yOff=\"0\" xSize=\"2\" ySize=\"2\"/></SimpleSource></VRTRasterBand><VRTRasterBand dataType=\"Byte\" band=\"2\"><SimpleSource><SourceFilename>b.tif</SourceFilename><SrcRect xOff=\"0\" yOff=\"0\" xSize=\"2\" ySize=\"2\"/><DstRect xOff=\"0\" yOff=\"0\" xSize=\"2\" ySize=\"2\"/></SimpleSource></VRTRasterBand></VRTDataset>";
        assert!(members(&parse(two).unwrap()).is_err());
    }
}
