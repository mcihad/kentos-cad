//! Rich map frames through the same CPU shaders as the desktop, with bundled-font text (CAD-SHEET-06).
use crate::{
    images::Images,
    maps::{MapLabel, MapSource, built_layers},
};
use kentos_native_style::cpu::{CpuView, paint};
use kentos_sheet::{
    display::MapPrim,
    pdf::{RasterMap, TextAnchor},
};

pub(crate) fn map(
    src: &MapSource<'_>,
    m: &MapPrim,
    labels: &[MapLabel],
    dpi: u16,
    drawing: Option<&str>,
) -> Result<RasterMap, String> {
    let center = m.view.center.ok_or("Haritanın merkezi yok.")?;
    let extent = m.extent.ok_or("Haritanın kapsamı yok.")?;
    let raw = [
        f64::from(m.clip.width) * f64::from(dpi) / 25400.,
        f64::from(m.clip.height) * f64::from(dpi) / 25400.,
    ];
    // Same export bound as the desktop's CPU painter; preserve the aspect ratio.
    let cap = (8192. / raw[0].max(raw[1]).max(1.))
        .min((32_000_000. / (raw[0] * raw[1]).max(1.)).sqrt())
        .min(1.);
    let (w, h) = (
        (raw[0] * cap).ceil().max(1.) as u32,
        (raw[1] * cap).ceil().max(1.) as u32,
    );
    if u64::from(w) * u64::from(h) > 32_100_000 {
        return Err("Harita resmi 32 milyon pikseli aşıyor; paftanın dpi değerini düşürün.".into());
    }
    let scale = m.view.scale.max(1e-6);
    let ppm = f64::from(w) * 1_000_000. / (f64::from(m.clip.width) * scale);
    let (s, c) = kentos_sheet::units::sin_cos(m.view.rotation);
    let delta = [
        (src.origin.x - center.x) * ppm,
        (src.origin.y - center.y) * ppm,
    ];
    let view = CpuView {
        m: [c * ppm, s * ppm, s * ppm, -c * ppm],
        t: [
            f64::from(w) / 2. + delta[0] * c + delta[1] * s,
            f64::from(h) / 2. + delta[0] * s - delta[1] * c,
        ],
        px_per_m: ppm,
        dpr: ppm * scale / 1000. * crate::maps::PX_MM,
        scale,
    };
    let built = built_layers(src, &m.layers, &m.layer_overrides, m.view.scale, extent);
    let images = Images::new(src.doc, drawing);
    for (_, _, layer) in &built {
        images.made(layer);
    }
    let layers: Vec<_> = built.iter().map(|(_, _, l)| l).collect();
    let rgba =
        paint(w, h, &view, &layers, &images).ok_or("Harita resmi için bellek ayrılamadı.")?;
    let missing: Vec<_> = images.missing.borrow().iter().cloned().collect();
    if !missing.is_empty() {
        return Err(format!(
            "Haritanın resimleri okunamadı: {}. Bağlantıları düzeltin veya resimleri projeye gömün.",
            missing.join(", ")
        ));
    }
    let mut pixmap = tiny(&rgba, w, h).ok_or("Harita resmi açılamadı.")?;
    draw_labels(
        &mut pixmap,
        &view,
        [src.origin.x, src.origin.y],
        labels,
        &images,
        scale,
    );
    let straight: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let p = p.demultiply();
            [p.red(), p.green(), p.blue(), p.alpha()]
        })
        .collect();
    Ok(RasterMap {
        png: png(&straight, w, h)?,
    })
}

pub(crate) fn png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}

fn tiny(rgba: &[u8], w: u32, h: u32) -> Option<resvg::tiny_skia::Pixmap> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    for (dst, src) in pixmap.pixels_mut().iter_mut().zip(rgba.chunks_exact(4)) {
        *dst = resvg::tiny_skia::ColorU8::from_rgba(src[0], src[1], src[2], src[3]).premultiply();
    }
    Some(pixmap)
}

fn ink(color: &str) -> resvg::tiny_skia::Paint<'static> {
    let c = kentos_native_style::color::parse_hex(color, 1.);
    let mut p = resvg::tiny_skia::Paint {
        anti_alias: true,
        ..Default::default()
    };
    p.set_color_rgba8(
        (c[0] * 255.).round() as u8,
        (c[1] * 255.).round() as u8,
        (c[2] * 255.).round() as u8,
        (c[3] * 255.).round() as u8,
    );
    p
}

fn draw_labels(
    pixmap: &mut resvg::tiny_skia::Pixmap,
    view: &CpuView,
    origin: [f64; 2],
    labels: &[MapLabel],
    _images: &Images<'_>,
    scale: f64,
) {
    use resvg::tiny_skia::{FillRule, PathBuilder, Stroke, Transform};
    let pixel = |p: [f64; 2]| {
        let x = p[0] - origin[0];
        let y = p[1] - origin[1];
        [
            (view.m[0] * x + view.m[1] * y + view.t[0]) as f32,
            (view.m[2] * x + view.m[3] * y + view.t[1]) as f32,
        ]
    };
    let path = |ring: &[[f64; 2]], close: bool| {
        let mut b = PathBuilder::new();
        for (i, p) in ring.iter().enumerate() {
            let p = pixel(*p);
            if i == 0 {
                b.move_to(p[0], p[1]);
            } else {
                b.line_to(p[0], p[1]);
            }
        }
        if close {
            b.close();
        }
        b.finish()
    };
    for l in labels {
        for (ring, color) in [
            (l.fill.as_deref(), l.color.as_str()),
            (l.mask.as_deref(), "#ffffff"),
        ] {
            if let Some(p) = ring.and_then(|r| path(r, true)) {
                pixmap.fill_path(
                    &p,
                    &ink(color),
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
        if let Some((ring, closed)) = &l.stroke
            && let Some(p) = path(ring, *closed)
        {
            pixmap.stroke_path(
                &p,
                &ink(&l.color),
                &Stroke {
                    width: view.dpr as f32,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }
        if let Some(ring) = &l.underline
            && let Some(p) = path(ring, true)
        {
            pixmap.fill_path(
                &p,
                &ink(&l.color),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        if l.text.is_empty() {
            continue;
        }
        let font = l.font.map(crate::fonts::sheet_font);
        let Some(outline) = crate::images::outline(&l.text, font, l.weight, l.italic) else {
            continue;
        };
        let (s, c) = l.rotation.to_radians().sin_cos();
        let size = l.size * scale / 1000. / f64::from(outline.size);
        let along = match l.anchor {
            TextAnchor::CenterBaseline | TextAnchor::CenterMiddle => {
                -f64::from(outline.advance) / 2.
            }
            TextAnchor::RightBaseline | TextAnchor::RightMiddle => -f64::from(outline.advance),
            _ => 0.,
        };
        let down = match l.anchor {
            TextAnchor::LeftMiddle | TextAnchor::CenterMiddle | TextAnchor::RightMiddle => 35.,
            _ => 0.,
        };
        let w = l.width_factor;
        let a = c * w * size;
        let b = s * w * size;
        let cc = (-c * l.lean + s) * size;
        let d = (-s * l.lean - c) * size;
        let at = pixel([
            l.at.x + c * along * w * size + s * down * size,
            l.at.y + s * along * w * size - c * down * size,
        ]);
        let xf = Transform::from_row(
            (view.m[0] * a + view.m[1] * b) as f32,
            (view.m[2] * a + view.m[3] * b) as f32,
            (view.m[0] * cc + view.m[1] * d) as f32,
            (view.m[2] * cc + view.m[3] * d) as f32,
            at[0],
            at[1],
        );
        for glyph in &outline.glyphs {
            if let Some(p) = kentos_native_style::raster::path_of(glyph) {
                let halo = 0.75 * crate::maps::PX_MM * 100. / l.size.max(1e-12);
                pixmap.stroke_path(
                    &p,
                    &ink(&l.halo),
                    &Stroke {
                        width: halo as f32,
                        ..Default::default()
                    },
                    xf,
                    None,
                );
                pixmap.fill_path(&p, &ink(&l.color), FillRule::Winding, xf, None);
            }
        }
    }
}
