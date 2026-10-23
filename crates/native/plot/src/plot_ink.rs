//! Plot profiles on raster fallback: vector ink through tiny-skia, opaque image/pattern batches through the CPU shader.
use kentos_native_style::{
    StyledLayer,
    batches::{BatchKind, FillPaintBatch, MarkerLook, Unit, Cap},
    cpu::{CpuView, paint},
};
use kentos_sheet::{
    plot_profile::{
        PlotProfile, PlotStyle,
        apply::{self, Resolved},
    },
    pdf::MapPath,
    LineCap, LineJoin,
};
use resvg::tiny_skia::{
    self, Pixmap, PixmapPaint, PathBuilder, FillRule, Transform, Stroke, StrokeDash,
};
use crate::{
    images::Images,
    vectors::{VectorScale, layer_paths},
};
fn rgba(c: &mut [f64; 4], s: &PlotStyle) {
    let source = format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        (c[0] * 255.).round().clamp(0., 255.) as u8,
        (c[1] * 255.).round().clamp(0., 255.) as u8,
        (c[2] * 255.).round().clamp(0., 255.) as u8,
        (c[3] * 255.).round().clamp(0., 255.) as u8
    );
    *c = kentos_native_style::color::parse_hex(&apply::color(&source, s), 1.);
}
fn length(um: i32, unit: Unit, scale: f64) -> f64 {
    match unit {
        Unit::World => f64::from(um) * scale / 1e6,
        Unit::Px => f64::from(um) / 1000. / crate::maps::PX_MM,
    }
}
fn cpu_style(layer: &mut StyledLayer, s: &PlotStyle, scale: f64) {
    for b in &mut layer.batches {
        match &mut b.kind {
            BatchKind::Stroke {
                color,
                width,
                unit,
                dash,
                dash_offset,
                cap,
                ..
            } => {
                rgba(color, s);
                if let Some(w) = s.line_weight {
                    *width = length(w, *unit, scale);
                }
                if let Some(d) = &s.dash {
                    *dash = (!d.is_empty()).then(|| {
                        apply::dash_um(d)
                            .iter()
                            .map(|v| length(*v, *unit, scale))
                            .collect()
                    });
                    *dash_offset = 0.;
                }
                if let Some(c) = s.cap {
                    *cap = match c {
                        LineCap::Butt => Cap::Butt,
                        LineCap::Round => Cap::Round,
                        LineCap::Square => Cap::Square,
                    };
                }
            }
            BatchKind::Fill { paint } => match paint {
                FillPaintBatch::Solid { color } => rgba(color, s),
                FillPaintBatch::Hatch {
                    color,
                    width,
                    unit,
                    dash,
                    dash_offset,
                    ..
                } => {
                    rgba(color, s);
                    if let Some(w) = s.line_weight {
                        *width = length(w, *unit, scale);
                    }
                    if let Some(d) = &s.dash {
                        *dash = (!d.is_empty()).then(|| {
                            apply::dash_um(d)
                                .iter()
                                .map(|v| length(*v, *unit, scale))
                                .collect()
                        });
                        *dash_offset = 0.;
                    }
                }
                FillPaintBatch::Gradient { color, color2, .. } => {
                    rgba(color, s);
                    rgba(color2, s);
                }
                FillPaintBatch::Pattern {
                    fill,
                    stroke,
                    stroke_width,
                    opacity,
                    unit,
                    ..
                } => {
                    if let Some(c) = fill {
                        rgba(c, s);
                    }
                    if let Some(c) = stroke {
                        rgba(c, s);
                    }
                    if let Some(w) = s.line_weight {
                        *stroke_width = length(w, *unit, scale);
                    }
                    if s.opacity.is_some() {
                        *opacity = 1.;
                    }
                }
                FillPaintBatch::Tile { opacity, .. }
                | FillPaintBatch::Image { opacity, .. }
                | FillPaintBatch::Raster { opacity, .. }
                | FillPaintBatch::PointCloud { opacity, .. }
                | FillPaintBatch::Service { opacity, .. } => {
                    if let Some(a) = s.opacity {
                        *opacity = f64::from(a) / 100.;
                    }
                }
            },
            BatchKind::Marker {
                look,
                opacity,
                unit,
                ..
            } => {
                if let MarkerLook::Shape {
                    fill,
                    stroke,
                    stroke_width,
                    ..
                } = look
                {
                    if let Some(c) = fill {
                        rgba(c, s);
                    }
                    if let Some(c) = stroke {
                        rgba(c, s);
                    }
                    if let Some(w) = s.line_weight {
                        *stroke_width = length(w, *unit, scale);
                    }
                }
                if let Some(a) = s.opacity {
                    *opacity = if matches!(look, MarkerLook::Shape { .. }) {
                        1.
                    } else {
                        f64::from(a) / 100.
                    };
                }
            }
        }
    }
}
fn over(target: &mut Pixmap, rgba: &[u8], w: u32, h: u32) -> Result<(), String> {
    let source = super::raster::tiny(rgba, w, h).ok_or("Baskı resmi için bellek ayrılamadı.")?;
    target.draw_pixmap(
        0,
        0,
        source.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    Ok(())
}
fn path(p: &MapPath, view: &CpuView, origin: [f64; 2]) -> Option<tiny_skia::Path> {
    let mut b = PathBuilder::new();
    for ring in std::iter::once(&p.points).chain(p.holes.iter()) {
        for (i, p) in ring.iter().enumerate() {
            let (x, y) = (p[0] - origin[0], p[1] - origin[1]);
            let q = [
                (view.m[0] * x + view.m[1] * y + view.t[0]) as f32,
                (view.m[2] * x + view.m[3] * y + view.t[1]) as f32,
            ];
            if i == 0 {
                b.move_to(q[0], q[1]);
            } else {
                b.line_to(q[0], q[1]);
            }
        }
        if p.closed {
            b.close();
        }
    }
    b.finish()
}
fn draw(
    target: &mut Pixmap,
    paths: &mut [MapPath],
    view: &CpuView,
    origin: [f64; 2],
    style: Option<&PlotStyle>,
) {
    let px_mm = view.dpr / crate::maps::PX_MM;
    for p in paths {
        if let Some(s) = style {
            if let Some(c) = &mut p.fill {
                *c = apply::color(c, s);
            }
            if let Some(stroke) = &mut p.stroke {
                apply::map_stroke(stroke, s);
            }
        }
        let Some(shape) = path(p, view, origin) else {
            continue;
        };
        if let Some(fill) = &p.fill {
            target.fill_path(
                &shape,
                &super::raster::ink(fill),
                FillRule::EvenOdd,
                Transform::identity(),
                None,
            );
        }
        if let Some(s) = &p.stroke {
            target.stroke_path(
                &shape,
                &super::raster::ink(&s.color),
                &Stroke {
                    width: if s.width == 0. {
                        1.
                    } else {
                        (s.width * px_mm) as f32
                    },
                    line_cap: match s.cap {
                        LineCap::Butt => tiny_skia::LineCap::Butt,
                        LineCap::Round => tiny_skia::LineCap::Round,
                        LineCap::Square => tiny_skia::LineCap::Square,
                    },
                    line_join: match s.join {
                        LineJoin::Miter => tiny_skia::LineJoin::Miter,
                        LineJoin::Round => tiny_skia::LineJoin::Round,
                        LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
                    },
                    miter_limit: 10.,
                    dash: StrokeDash::new(s.dash.iter().map(|d| (d * px_mm) as f32).collect(), 0.),
                },
                Transform::identity(),
                None,
            );
        }
    }
}
pub(crate) fn render(
    w: u32,
    h: u32,
    view: &CpuView,
    built: &mut [(String, String, StyledLayer)],
    images: &Images<'_>,
    profile: &PlotProfile,
    origin: [f64; 2],
) -> Result<Pixmap, String> {
    let scale = view.scale;
    let mut target = Pixmap::new(w, h).ok_or("Baskı resmi için bellek ayrılamadı.")?;
    let resolved = Resolved::new(profile);
    let vector = VectorScale { scale, origin };
    for (id, _, layer) in built {
        let style = resolved.model(id);
        if layer.batches.iter().any(|b| b.effect.is_some()) {
            if style.is_some_and(|s| s.join.is_some_and(|j| j != LineJoin::Round))
                && layer
                    .batches
                    .iter()
                    .any(|b| matches!(b.kind, BatchKind::Stroke { .. }))
            {
                return Err("plot_profile_effect_join: Efektli çizginin baskı birleşimini değiştirmek desteklenmiyor; bu katmanın birleşimini Nesneden al yapın veya efekti kaldırın.".into());
            }
            if let Some(s) = style {
                cpu_style(layer, s, scale);
            }
            let rgba = paint(w, h, view, &[layer], images)
                .ok_or("Baskı efekti için bellek ayrılamadı.")?;
            over(&mut target, &rgba, w, h)?;
            continue;
        }
        let batches = std::mem::take(&mut layer.batches);
        for batch in batches {
            layer.batches = vec![batch];
            let (mut paths, missing) = layer_paths(layer, &vector, None);
            if missing.is_empty() {
                draw(&mut target, &mut paths, view, origin, style);
            } else {
                if let Some(s) = style {
                    cpu_style(layer, s, scale);
                }
                let rgba = paint(w, h, view, &[layer], images)
                    .ok_or("Baskı resmi için bellek ayrılamadı.")?;
                over(&mut target, &rgba, w, h)?;
            }
        }
    }
    Ok(target)
}
