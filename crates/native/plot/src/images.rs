//! Fonts and images for the shared CPU painter: deterministic bundled faces and local project bytes (CAD-SHEET-06).
use kentos_domain::Document;
use kentos_native_style::{
    AtlasImage,
    picture::{Bitmap, ImageSource, Picture, Segment, TextOutline},
};
use std::{cell::RefCell, collections::HashMap, sync::Arc};

type Pictures = RefCell<HashMap<String, Option<Arc<Picture>>>>;
type Bitmaps = RefCell<HashMap<String, Option<Arc<Bitmap>>>>;

pub(crate) struct Images<'a> {
    doc: &'a Document,
    drawing: Option<String>,
    pictures: Pictures,
    bitmaps: Bitmaps,
    pub missing: RefCell<std::collections::BTreeSet<String>>,
}
impl<'a> Images<'a> {
    pub fn new(doc: &'a Document, drawing: Option<&str>) -> Self {
        Self {
            doc,
            drawing: drawing.map(str::to_owned),
            pictures: RefCell::new(HashMap::new()),
            bitmaps: RefCell::new(HashMap::new()),
            missing: RefCell::new(Default::default()),
        }
    }
    pub fn made(&self, layer: &kentos_native_style::StyledLayer) {
        for p in &layer.pictures {
            self.bitmaps.borrow_mut().insert(
                p.key.clone(),
                Some(Arc::new(Bitmap {
                    width: p.width,
                    height: p.height,
                    rgba: p.rgba.clone(),
                })),
            );
        }
    }
    fn bitmap_bytes(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(id) = key.strip_prefix("asset:") {
            let item = self
                .doc
                .styles()
                .items
                .iter()
                .find(|v| v.get("id").and_then(serde_json::Value::as_str) == Some(id))?;
            return kentos_native_style::bitmap::data_url(item.get("data")?.as_str()?)
                .map(|(_, b)| b);
        }
        let path = key.strip_prefix("file:")?;
        let found = kentos_native_application::links::find(
            path,
            None,
            self.drawing.as_deref(),
            &self.doc.settings().xref_paths,
        )
        .at?;
        if std::fs::metadata(&found).ok()?.len() > kentos_contracts::MAX_IMAGE_BYTES {
            return None;
        }
        let bytes = std::fs::read(found).ok()?;
        (bytes.len() as u64 <= kentos_contracts::MAX_IMAGE_BYTES).then_some(bytes)
    }
}

impl ImageSource for Images<'_> {
    fn bitmap(&self, key: &str) -> Option<Arc<Bitmap>> {
        if let Some(found) = self.bitmaps.borrow().get(key) {
            return found.clone();
        }
        let read = self
            .bitmap_bytes(key)
            .and_then(|b| decode(&b))
            .and_then(|p| match p {
                Picture::Bitmap {
                    width,
                    height,
                    rgba,
                } => Some(Arc::new(Bitmap {
                    width,
                    height,
                    rgba,
                })),
                _ => None,
            });
        if read.is_none() {
            self.missing.borrow_mut().insert(key.into());
        }
        self.bitmaps.borrow_mut().insert(key.into(), read.clone());
        read
    }
    fn picture(&self, image: &AtlasImage) -> Option<Arc<Picture>> {
        if let Some(found) = self.pictures.borrow().get(image.key()) {
            return found.clone();
        }
        let read = match image {
            AtlasImage::Svg { svg, .. } => {
                if let Err(why) = crate::svg_source::vector_compatible(svg) {
                    self.missing
                        .borrow_mut()
                        .insert(format!("{}: {why}", image.key()));
                    return None;
                }
                kentos_native_style::svg::picture(svg, &|t, f, w, i| self.text(t, f, w, i))
            }
            AtlasImage::Raster { url, .. } => {
                kentos_native_style::bitmap::data_url(url).and_then(|(_, b)| decode(&b))
            }
            _ => None,
        }
        .map(Arc::new);
        if read.is_none() {
            self.missing.borrow_mut().insert(image.key().into());
        }
        self.pictures
            .borrow_mut()
            .insert(image.key().into(), read.clone());
        read
    }
    fn text(
        &self,
        text: &str,
        font: Option<&str>,
        weight: f64,
        italic: bool,
    ) -> Option<Arc<TextOutline>> {
        outline(text, font, weight as u16, italic).map(Arc::new)
    }
}

fn decode(bytes: &[u8]) -> Option<Picture> {
    if bytes.starts_with(b"\x89PNG") {
        kentos_native_style::bitmap::png(bytes)
    } else {
        kentos_native_style::bitmap::jpeg(bytes)
    }
}

/// A glyph outline in a bundled face. Rustybuzz provides the same OpenType advances/kerning used for text shaping.
pub(crate) fn outline(
    text: &str,
    font: Option<&str>,
    weight: u16,
    italic: bool,
) -> Option<TextOutline> {
    let family = font.unwrap_or("Arimo").to_lowercase();
    let id = if family.contains("plex") || family.contains("monospace") {
        "plex-mono"
    } else if family.contains("barlow") {
        "barlow"
    } else if family.contains("architect") {
        "architects-daughter"
    } else if family.contains("overpass") {
        "overpass"
    } else if family.contains("quicksand") {
        "quicksand"
    } else if family.contains("courier") {
        "courier-prime"
    } else {
        "arimo"
    };
    let index = crate::fonts::FACE_KEYS
        .iter()
        .enumerate()
        .filter(|(_, f)| f.0 == id)
        .min_by_key(|(_, f)| (u16::from(f.2 != italic) * 1000) + f.1.abs_diff(weight))
        .map(|(i, _)| i)?;
    let data = crate::fonts::FACES[index];
    let face = rustybuzz::Face::from_slice(data, 0)?;
    let ttf = ttf_parser::Face::parse(data, 0).ok()?;
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let glyphs = rustybuzz::shape(&face, &[], buffer);
    let k = 100. / f32::from(ttf.units_per_em());
    let mut builder = Outline {
        segments: Vec::new(),
        x: 0.,
        y: 0.,
        k,
    };
    let mut all = Vec::new();
    let mut advance = 0.;
    for (glyph, pos) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
        builder.x = advance + pos.x_offset as f32 * k;
        builder.y = -(pos.y_offset as f32) * k;
        ttf.outline_glyph(ttf_parser::GlyphId(glyph.glyph_id as u16), &mut builder);
        all.push(std::mem::take(&mut builder.segments));
        advance += pos.x_advance as f32 * k;
    }
    Some(TextOutline {
        size: 100.,
        advance,
        glyphs: all,
    })
}
struct Outline {
    segments: Vec<Segment>,
    x: f32,
    y: f32,
    k: f32,
}
impl Outline {
    fn p(&self, x: f32, y: f32) -> [f32; 2] {
        [self.x + x * self.k, self.y - y * self.k]
    }
}
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.segments.push(Segment::Move(self.p(x, y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.segments.push(Segment::Line(self.p(x, y)));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.segments
            .push(Segment::Quad(self.p(x1, y1), self.p(x, y)));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.segments
            .push(Segment::Cubic(self.p(x1, y1), self.p(x2, y2), self.p(x, y)));
    }
    fn close(&mut self) {
        self.segments.push(Segment::Close);
    }
}

/// SVG assets at the PDF's physical resolution, with the application's bundled drawing faces.
pub(crate) fn svg_assets(
    book: &kentos_sheet::model::SheetBook,
    inputs: &mut kentos_sheet::pdf::PdfInputs,
    options: &kentos_sheet::pdf::PdfOptions,
    dpi: u16,
) -> Result<(), String> {
    let sizes = kentos_sheet::pdf::svg_sizes(book, inputs, options, dpi).map_err(|e| e.message)?;
    if sizes.is_empty() {
        return Ok(());
    }
    let mut db = resvg::usvg::fontdb::Database::new();
    for face in crate::fonts::FACES {
        db.load_font_data(face.to_vec());
    }
    let options = resvg::usvg::Options {
        font_family: "Barlow".into(),
        fontdb: Arc::new(db),
        image_href_resolver: resvg::usvg::ImageHrefResolver {
            resolve_string: Box::new(|_, _| None),
            ..Default::default()
        },
        ..Default::default()
    };
    for size in sizes {
        let Some(asset) = inputs.assets.iter_mut().find(|a| a.sha256 == size.sha256) else {
            return Err(format!("{} SVG resminin baytları yok.", size.sha256));
        };
        let svg = crate::svg_source::prepare(&asset.data)?;
        let tree = resvg::usvg::Tree::from_data(&svg, &options).map_err(|e| e.to_string())?;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width, size.height)
            .ok_or("SVG resmi için bellek ayrılamadı.")?;
        let s = tree.size();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(
                size.width as f32 / s.width(),
                size.height as f32 / s.height(),
            ),
            &mut pixmap.as_mut(),
        );
        let rgba: Vec<_> = pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let p = p.demultiply();
                [p.red(), p.green(), p.blue(), p.alpha()]
            })
            .collect();
        asset.raster = Some(crate::raster::png(&rgba, size.width, size.height)?);
    }
    Ok(())
}

#[cfg(test)]
mod preflight_tests {
    use super::*;
    #[test]
    fn sheet_preflight_cpu_symbols_report_omitted_svg_content() {
        let fx: serde_json::Value =
            serde_json::from_str(include_str!("../../../../fixtures/sheet/v1/delivery.json"))
                .unwrap();
        let doc =
            Document::from_snapshot(serde_json::from_value(fx["model"].clone()).unwrap()).unwrap();
        for body in [
            "<image href='gone.png'/>",
            "<defs><linearGradient id='p'/></defs><rect width='10' height='10' fill='url(#p)'/>",
            "<style>rect{fill:red}</style><rect width='10' height='10'/>",
        ] {
            let images = Images::new(&doc, None);
            let source = AtlasImage::Svg {
                key: "symbol".into(),
                svg: format!(
                    "<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'>{body}</svg>"
                ),
                width: 10.,
                height: 10.,
            };
            assert!(images.picture(&source).is_none());
            assert!(!images.missing.borrow().is_empty());
        }
    }
}
