//! Rich map frames through the same CPU shaders as the desktop, with bundled-font text (CAD-SHEET-06).
use crate::{
    images::Images,
    maps::{MapLabel, MapSource, built_layers_profile},
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
    profile: Option<&kentos_sheet::plot_profile::PlotProfile>,
) -> Result<RasterMap, String> {
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
        return Err(
            "Harita resmi 32 milyon pikseli aşıyor; paftanın dpi değerini düşürün.".into(),
        );
    }
    let image = render_image_profile(src, m, labels, [w, h], drawing, profile)?;
    Ok(RasterMap {
        png: png(&image.rgba, image.width, image.height)?,
    })
}

/// A straight RGBA map image, using the output style engine and bundled fonts.
pub struct MapImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
/// Shared by a polygon viewport's desktop preview and its headless raster output.
pub fn render_image(
    src: &MapSource<'_>,
    m: &MapPrim,
    labels: &[MapLabel],
    size: [u32; 2],
    drawing: Option<&str>,
) -> Result<MapImage, String> {
    render_image_profile(src, m, labels, size, drawing, None)
}
pub fn render_image_profile(
    src: &MapSource<'_>,
    m: &MapPrim,
    labels: &[MapLabel],
    size: [u32; 2],
    drawing: Option<&str>,
    profile: Option<&kentos_sheet::plot_profile::PlotProfile>,
) -> Result<MapImage, String> {
    let center = m.view.center.ok_or("Haritanın merkezi yok.")?;
    let extent = m.extent.ok_or("Haritanın kapsamı yok.")?;
    let [w, h] = size;
    if w == 0 || h == 0 || w > 8192 || h > 8192 || u64::from(w) * u64::from(h) > 32_100_000 {
        return Err("Harita resmi en çok 8192 px/32 MP olabilir.".into());
    }
    let scale = m.view.scale.max(1e-6);
    let physical = kentos_sheet::projection::Scale::for_rendering(scale);
    let ppm = physical.pixels_per_metre(f64::from(w) / f64::from(m.clip.width));
    let camera = kentos_sheet::projection::Transform::at_resolution(
        center,
        [f64::from(w) / 2., f64::from(h) / 2.],
        ppm,
        m.view.rotation,
    );
    let (matrix, t) = camera.relative_affine([src.origin.x, src.origin.y]);
    let view = CpuView {
        m: matrix,
        t,
        px_per_m: ppm,
        dpr: physical.paper_pixels_per_mm(ppm) * crate::maps::PX_MM,
        scale,
    };
    let mut built = built_layers_profile(
        src,
        &m.layers,
        &m.layer_overrides,
        m.view.scale,
        extent,
        profile,
        labels,
    )?;
    let images = Images::new(src.doc, drawing);
    for (_, _, layer) in &built {
        images.made(layer);
    }
    let mut pixmap = if let Some(profile) = profile {
        crate::plot_ink::render(
            w,
            h,
            &view,
            &mut built,
            &images,
            profile,
            [src.origin.x, src.origin.y],
        )?
    } else {
        let layers: Vec<_> = built.iter().map(|(_, _, l)| l).collect();
        let rgba = paint(w, h, &view, &layers, &images)
            .ok_or("Harita resmi için bellek ayrılamadı.")?;
        tiny(&rgba, w, h).ok_or("Harita resmi açılamadı.")?
    };
    let missing: Vec<_> = images.missing.borrow().iter().cloned().collect();
    if !missing.is_empty() {
        return Err(format!(
            "Haritanın resimleri okunamadı: {}. Bağlantıları düzeltin veya resimleri projeye gömün.",
            missing.join(", ")
        ));
    }
    let mut writing = labels.to_vec();
    if let Some(profile) = profile {
        let resolved = kentos_sheet::plot_profile::apply::Resolved::new(profile);
        for l in &mut writing {
            if let Some(s) = resolved.model(&l.layer) {
                l.color = kentos_sheet::plot_profile::apply::color(&l.color, s);
            }
        }
    }
    draw_labels(
        &mut pixmap,
        &view,
        [src.origin.x, src.origin.y],
        &writing,
        &images,
        scale,
        profile,
    );
    let straight: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let p = p.demultiply();
            [p.red(), p.green(), p.blue(), p.alpha()]
        })
        .collect();
    let mut rgba = straight;
    if !m.clip_shape.mask_rgba(&mut rgba, w, h) {
        return Err("Harita kırpma belleği geçersiz.".into());
    }
    Ok(MapImage {
        width: w,
        height: h,
        rgba,
    })
}

pub(crate) fn png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    png_at(rgba,w,h,None)
}

pub(crate) fn png_at(rgba: &[u8], w: u32, h: u32, dpi: Option<u16>) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        if let Some(dpi) = dpi {
            let ppm = (f64::from(dpi) / 0.0254).round() as u32;
            encoder.set_pixel_dims(Some(png::PixelDimensions {
                xppu: ppm,
                yppu: ppm,
                unit: png::Unit::Meter,
            }));
        }
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}

pub(crate) fn tiny(rgba: &[u8], w: u32, h: u32) -> Option<resvg::tiny_skia::Pixmap> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    for (dst, src) in pixmap.pixels_mut().iter_mut().zip(rgba.chunks_exact(4)) {
        *dst = resvg::tiny_skia::ColorU8::from_rgba(src[0], src[1], src[2], src[3]).premultiply();
    }
    Some(pixmap)
}

pub(crate) fn ink(color: &str) -> resvg::tiny_skia::Paint<'static> {
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
    profile: Option<&kentos_sheet::plot_profile::PlotProfile>,
) {
    use resvg::tiny_skia::{
        FillRule, PathBuilder, Stroke, Transform, StrokeDash, LineCap, LineJoin,
    };
    let styles = profile.map(kentos_sheet::plot_profile::apply::Resolved::new);
    let px_mm = view.dpr / crate::maps::PX_MM;
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
        let style = styles.as_ref().and_then(|s| s.model(&l.layer));
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
                    width: style
                        .and_then(|s| s.line_weight)
                        .map_or(view.dpr as f32, |w| {
                            if w == 0 {
                                1.
                            } else {
                                (f64::from(w) / 1000. * px_mm) as f32
                            }
                        }),
                    line_cap: match style
                        .and_then(|s| s.cap)
                        .unwrap_or(kentos_sheet::LineCap::Butt)
                    {
                        kentos_sheet::LineCap::Butt => LineCap::Butt,
                        kentos_sheet::LineCap::Round => LineCap::Round,
                        kentos_sheet::LineCap::Square => LineCap::Square,
                    },
                    line_join: match style
                        .and_then(|s| s.join)
                        .unwrap_or(kentos_sheet::LineJoin::Miter)
                    {
                        kentos_sheet::LineJoin::Miter => LineJoin::Miter,
                        kentos_sheet::LineJoin::Round => LineJoin::Round,
                        kentos_sheet::LineJoin::Bevel => LineJoin::Bevel,
                    },
                    miter_limit: 10.,
                    dash: style.and_then(|s| s.dash.as_ref()).and_then(|d| {
                        StrokeDash::new(
                            kentos_sheet::plot_profile::apply::dash_um(d)
                                .iter()
                                .map(|v| (f64::from(*v) / 1000. * px_mm) as f32)
                                .collect(),
                            0.,
                        )
                    }),
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
        let size = kentos_sheet::projection::Scale::for_rendering(scale)
            .model_metres_from_mm(l.size)
            / f64::from(outline.size);
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
