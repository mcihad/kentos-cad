//! Window-free print preview: the PDF's scene, profiles, physical transform and bundled faces.
use kentos_domain::Document;
use kentos_sheet::{
    display::{self, Prim},
    pdf::MapContent,
    svg::{AssetHref, MapImage, SvgOptions},
    template::base64_encode,
};
use crate::book::PlotError;

pub struct PageImage {
    pub svg: String,
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A single printed page; raster memory is bounded, never silently reduced in resolution.
pub fn page(
    doc: &Document,
    sheet: &str,
    user: &str,
    date: &str,
    drawing: Option<&str>,
    dpi: u16,
) -> Result<PageImage, PlotError> {
    if !(36..=1200).contains(&dpi) {
        return Err(PlotError::new("bad_dpi", "Çözünürlük 36–1200 dpi olmalı."));
    }
    let (book, inputs, _, _) =
        crate::book::prepare(doc, &[sheet.into()], user, date, drawing, true)?;
    let list = display::display_list(&book, sheet, &inputs.render)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    let mut options = SvgOptions {
        placeholders: false,
        ..Default::default()
    };
    for map in inputs.maps {
        if let MapContent::Raster(r) = map.content {
            options.maps.push(MapImage {
                item: map.item,
                href: format!("data:image/png;base64,{}", base64_encode(&r.png)),
            });
        }
    }
    for a in inputs.assets {
        let mime = if a.data.starts_with(&[0xff, 0xd8]) {
            "image/jpeg"
        } else if a.data.starts_with(b"\x89PNG") {
            "image/png"
        } else {
            "image/svg+xml"
        };
        options.assets.push(AssetHref {
            sha256: a.sha256,
            href: format!("data:{mime};base64,{}", base64_encode(&a.data)),
        });
    }
    let plan = kentos_sheet::print_setup::plan_for(&book, sheet)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    let mut svg = kentos_sheet::svg::to_svg_plotted(&list, &options, plan.as_ref());
    // usvg 0.45 consumes SVG 1.1 image references. Keep external SVG portable too.
    svg = svg
        .replace(
            "xmlns=\"http://www.w3.org/2000/svg\"",
            "xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\"",
        )
        .replace(" href=\"", " xlink:href=\"");
    // SVG remains portable outside KentOS; the rasterizer also loads these exact faces directly.
    let mut fonts = String::from("<style>");
    for ((font, weight, italic), bytes) in
        crate::fonts::FACE_KEYS.into_iter().zip(crate::fonts::FACES)
    {
        if !list
            .prims
            .iter()
            .any(|p| matches!(p,Prim::Text(t) if t.font==font))
        {
            continue;
        }
        let family = kentos_sheet::svg::css_family(font)
            .split(',')
            .next()
            .unwrap_or("Barlow");
        fonts.push_str(&format!("@font-face{{font-family:{family};font-weight:{weight};font-style:{};src:url(data:font/ttf;base64,{})}}",if italic {"italic"} else {"normal"},base64_encode(bytes)));
    }
    fonts.push_str("</style>");
    if let Some(i) = svg.find("<rect") {
        svg.insert_str(i, &fonts);
    }
    let width = (f64::from(list.size.width) * f64::from(dpi) / 25400.)
        .ceil()
        .max(1.) as u32;
    let height = (f64::from(list.size.height) * f64::from(dpi) / 25400.)
        .ceil()
        .max(1.) as u32;
    if width > 16384 || height > 16384 || u64::from(width) * u64::from(height) > 120_000_000 {
        return Err(PlotError::new(
            "page_too_large",
            "Pafta 16384 piksel/120 MP sınırını aşıyor; çözünürlüğü düşürün.",
        ));
    }
    let mut db = resvg::usvg::fontdb::Database::new();
    for bytes in crate::fonts::FACES {
        db.load_font_data(bytes.to_vec());
    }
    let opts = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(db),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &opts)
        .map_err(|e| PlotError::new("preview_unwritable", e.to_string()))?;
    let mut pix = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| PlotError::new("preview_unwritable", "Önizleme için bellek ayrılamadı."))?;
    let size = tree.size();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            width as f32 / size.width(),
            height as f32 / size.height(),
        ),
        &mut pix.as_mut(),
    );
    let rgba = pix
        .pixels()
        .iter()
        .flat_map(|p| {
            let p = p.demultiply();
            [p.red(), p.green(), p.blue(), p.alpha()]
        })
        .collect::<Vec<_>>();
    let png = crate::raster::png(&rgba, width, height)
        .map_err(|e| PlotError::new("preview_unwritable", e))?;
    Ok(PageImage {
        svg,
        png,
        width,
        height,
        rgba,
    })
}
