//! Our own PDF reader for underlays (docs/adr/0351 §2; plan/cad.md `CAD-REF-17`): a page's vector drawing as an
//! underlay takes it, without a renderer or a library. The file's objects (`file`, `object`, `filter`), the page tree
//! with what pages inherit, and the page's content (`content`, `font`): its paths with their stroke and fill, its texts
//! with their place, height and turn, its optional content groups as layers. Coordinates are points (1/72 inch) on the
//! page as it shows: from its crop box's lower left corner, turned by its `Rotate`. Pictures, shadings and letters
//! that read as no character are counted, never drawn or guessed. An encrypted file is refused.

pub mod content;
pub mod file;
pub mod filter;
pub mod font;
pub mod object;

use content::{IDENTITY, M, Run};
use file::PdfFile;
use object::{Dict, Obj};

/// A path as the page paints it: its parts (points, and whether the part closes), its stroke and its fill.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfPath {
    pub parts: Vec<(Vec<[f64; 2]>, bool)>,
    pub stroke: Option<PdfStroke>,
    pub fill: Option<[u8; 3]>,
    /// Its layer (`PdfPage::layers`), when it is in an optional content group.
    pub layer: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PdfStroke {
    pub color: [u8; 3],
    /// Its width on the page, points.
    pub width: f64,
    pub dashed: bool,
}

/// A text shown: where its first letter's baseline starts, its height and turn on the page.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfText {
    pub p: [f64; 2],
    pub height: f64,
    /// Radians, counter-clockwise.
    pub angle: f64,
    pub text: String,
    pub color: [u8; 3],
    pub layer: Option<usize>,
}

/// What a page holds that an underlay does not take.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PdfLeft {
    pub images: u32,
    pub shadings: u32,
    /// Letters that read as no character (a font without a map to letters).
    pub unreadable: u32,
    /// The page held more than a page is read for (`content::MAX_POINTS`): the rest is left out.
    pub truncated: bool,
}

/// A page read.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfPage {
    /// The page's number, from 1, and how many the file has.
    pub number: u32,
    pub count: u32,
    /// Its size as it shows, points.
    pub width: f64,
    pub height: f64,
    pub layers: Vec<String>,
    /// Whether each layer is off in the file's default view.
    pub hidden: Vec<bool>,
    pub paths: Vec<PdfPath>,
    pub texts: Vec<PdfText>,
    pub left: PdfLeft,
}

#[derive(Debug, PartialEq)]
pub enum PdfError {
    NotPdf(String),
    Encrypted,
    /// The file has `count` pages and not the one asked for.
    NoPage { count: u32 },
    Broken(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::NotPdf(w) | PdfError::Broken(w) => f.write_str(w),
            PdfError::Encrypted => f.write_str("PDF şifreli; şifresi açılmış bir kopyasını verin"),
            PdfError::NoPage { count } => write!(f, "PDF'te {count} sayfa var"),
        }
    }
}

/// The most pages a page tree is followed for.
const MAX_PAGES: usize = 100_000;

struct PageNode {
    dict: Dict,
    resources: Dict,
    media: [f64; 4],
    crop: Option<[f64; 4]>,
    rotate: i64,
}

fn rect(file: &PdfFile<'_>, o: &Obj) -> Option<[f64; 4]> {
    let a = file.resolve(o);
    let a = a.array()?;
    if a.len() < 4 {
        return None;
    }
    let v: Vec<f64> = a.iter().map(|x| file.resolve(x).num().unwrap_or(0.0)).collect();
    Some([v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])])
}

fn pages(file: &PdfFile<'_>) -> Result<Vec<PageNode>, PdfError> {
    let root = file.field(&file.trailer, "Root");
    let root = root.dict().ok_or_else(|| PdfError::Broken("PDF'in kataloğu okunamadı".into()))?;
    let mut out = Vec::new();
    let mut seen = Vec::new();
    #[allow(clippy::too_many_arguments)]
    fn walk(
        file: &PdfFile<'_>,
        node: &Obj,
        res: Option<Dict>,
        media: Option<[f64; 4]>,
        crop: Option<[f64; 4]>,
        rotate: i64,
        out: &mut Vec<PageNode>,
        seen: &mut Vec<u32>,
        depth: usize,
    ) {
        if depth > 64 || out.len() >= MAX_PAGES {
            return;
        }
        if let Obj::Ref(n, _) = node {
            if seen.contains(n) {
                return;
            }
            seen.push(*n);
        }
        let o = file.resolve(node);
        let Some(d) = o.dict() else { return };
        let res = match file.field(d, "Resources") {
            Obj::Dict(r) => Some(r),
            _ => res,
        };
        let media = d.get("MediaBox").and_then(|m| rect(file, m)).or(media);
        let crop = d.get("CropBox").and_then(|m| rect(file, m)).or(crop);
        let rotate = file.field(d, "Rotate").num().map_or(rotate, |r| r as i64);
        let kind = file.field(d, "Type");
        let kids = file.field(d, "Kids");
        if kind.name() == Some("Pages") || (kind.name() != Some("Page") && kids.array().is_some()) {
            for k in kids.array().unwrap_or_default() {
                walk(file, k, res.clone(), media, crop, rotate, out, seen, depth + 1);
            }
        } else {
            out.push(PageNode {
                dict: d.clone(),
                resources: res.unwrap_or_default(),
                media: media.unwrap_or([0.0, 0.0, 612.0, 792.0]),
                crop,
                rotate,
            });
        }
    }
    walk(file, &root.get("Pages").cloned().unwrap_or(Obj::Null), None, None, None, 0, &mut out, &mut seen, 0);
    Ok(out)
}

fn open(bytes: &[u8]) -> Result<PdfFile<'_>, PdfError> {
    PdfFile::open(bytes).map_err(|e| {
        if e.contains("şifreli") {
            PdfError::Encrypted
        } else if e.contains("PDF değil") {
            PdfError::NotPdf(e)
        } else {
            PdfError::Broken(e)
        }
    })
}

/// How many pages the file has.
pub fn page_count(bytes: &[u8]) -> Result<u32, PdfError> {
    let file = open(bytes)?;
    Ok(pages(&file)?.len() as u32)
}

/// Whether the bytes are a PDF file's (its header in the first kilobyte).
pub fn is_pdf(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(1024)].windows(5).any(|w| w == b"%PDF-")
}

/// Page `number` (from 1) read.
pub fn read_page(bytes: &[u8], number: u32) -> Result<PdfPage, PdfError> {
    let file = open(bytes)?;
    let list = pages(&file)?;
    let count = list.len() as u32;
    if number == 0 || number > count {
        return Err(PdfError::NoPage { count });
    }
    let page = &list[number as usize - 1];
    // The crop box inside the media box; the page as it shows from its lower left corner.
    let [mx0, my0, mx1, my1] = page.media;
    let [x0, y0, x1, y1] = page.crop.map_or(page.media, |c| [c[0].max(mx0), c[1].max(my0), c[2].min(mx1), c[3].min(my1)]);
    let (w, h) = ((x1 - x0).max(0.0), (y1 - y0).max(0.0));
    let shift: M = [1.0, 0.0, 0.0, 1.0, -x0, -y0];
    let (turn, width, height): (M, f64, f64) = match page.rotate.rem_euclid(360) {
        90 => ([0.0, -1.0, 1.0, 0.0, 0.0, w], h, w),
        180 => ([-1.0, 0.0, 0.0, -1.0, w, h], w, h),
        270 => ([0.0, 1.0, -1.0, 0.0, h, 0.0], h, w),
        _ => (IDENTITY, w, h),
    };
    let ctm = content::mul(&shift, &turn);
    // The groups off in the default view.
    let mut off = Vec::new();
    if let Some(root) = file.field(&file.trailer, "Root").dict()
        && let Obj::Dict(ocp) = file.field(root, "OCProperties")
        && let Obj::Dict(d) = file.field(&ocp, "D")
        && let Some(Obj::Array(list)) = d.get("OFF").map(|o| file.resolve(o))
    {
        for o in list {
            if let Obj::Ref(n, _) = o {
                off.push(n);
            }
        }
    }
    let mut run = Run::new(&file, off);
    let contents = file.field(&page.dict, "Contents");
    let streams: Vec<Obj> = match &contents {
        Obj::Array(a) => a.clone(),
        Obj::Stream(..) => vec![page.dict.get("Contents").cloned().unwrap_or(Obj::Null)],
        _ => Vec::new(),
    };
    // A page's content streams are one stream cut where the writer liked.
    let mut data = Vec::new();
    for s in &streams {
        match file.stream_data(s) {
            Ok(Some(bytes)) => {
                data.extend_from_slice(&bytes);
                data.push(b'\n');
            }
            Ok(None) => {}
            Err(e) => return Err(PdfError::Broken(format!("sayfanın içeriği okunamadı: {e:?}"))),
        }
    }
    run.content(&data, &page.resources, ctm, 0, &mut Vec::new());
    let out = run.out;
    Ok(PdfPage {
        number,
        count,
        width,
        height,
        layers: out.layers,
        hidden: out.hidden,
        paths: out.paths,
        texts: out.texts,
        left: out.left,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small PDF written by hand, its cross-reference table from the objects' places.
    fn pdf(objects: &[&str]) -> Vec<u8> {
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut at = Vec::new();
        for (i, o) in objects.iter().enumerate() {
            at.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for a in at {
            out.extend_from_slice(format!("{a:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
        out
    }

    fn stream(content: &str) -> String {
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())
    }

    #[test]
    fn a_page_of_lines_a_square_and_a_text() {
        let content = "1 0 0 RG 2 w 10 10 m 110 10 l S 0 0 1 rg 20 20 50 30 re f BT /F1 12 Tf 1 0 0 1 100 200 Tm (Ada 101) Tj ET";
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 595 842] >>",
            "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
            &stream(content),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        ]);
        let page = read_page(&bytes, 1).unwrap();
        assert_eq!((page.width, page.height, page.count), (595.0, 842.0, 1));
        assert_eq!(page.paths.len(), 2);
        assert_eq!(page.paths[0].parts, vec![(vec![[10.0, 10.0], [110.0, 10.0]], false)]);
        assert_eq!(page.paths[0].stroke, Some(PdfStroke { color: [255, 0, 0], width: 2.0, dashed: false }));
        assert_eq!(page.paths[1].fill, Some([0, 0, 255]));
        assert_eq!(page.texts.len(), 1);
        assert_eq!(page.texts[0].text, "Ada 101");
        assert_eq!(page.texts[0].p, [100.0, 200.0]);
        assert_eq!(page.texts[0].height, 12.0);
        assert_eq!(read_page(&bytes, 2), Err(PdfError::NoPage { count: 1 }));
    }

    #[test]
    fn a_turned_page_shows_upright() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Rotate 90 /Contents 4 0 R >>",
            &stream("0 0 m 200 0 l S"),
        ]);
        let page = read_page(&bytes, 1).unwrap();
        assert_eq!((page.width, page.height), (100.0, 200.0));
        assert_eq!(page.paths[0].parts[0].0, vec![[0.0, 200.0], [0.0, 0.0]]);
    }
}
