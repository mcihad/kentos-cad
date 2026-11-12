//! Model tool aids on the active viewport. The session, selection and ids are the model's own.
use iced::{Point, Color};
use iced::widget::canvas::{Frame, Path, Stroke};
use kentos_sheet_ui::MapRequest;
use kentos_render_wgpu::{scene, Vec2};
use crate::app::App;
pub(crate) fn paint(app: Option<&App>, r: &MapRequest<'_>, frame: &mut Frame) {
    let Some(app) = app.filter(|a| a.sheets.model_scope() == Some(r.item)) else {
        return;
    };
    let Some(prim) = r.primitive else {
        return;
    };
    let accent = Color::from_rgb8(0x38, 0x73, 0xd8);
    let mut line = |a: Point, b: Point, width: f32| {
        let norm = |p: Point| {
            [
                f64::from(p.x / r.size.width),
                f64::from(p.y / r.size.height),
            ]
        };
        for [a, b] in prim.clip_shape.segments(norm(a), norm(b)) {
            let px =
                |p: [f64; 2]| Point::new(p[0] as f32 * r.size.width, p[1] as f32 * r.size.height);
            frame.stroke(
                &Path::line(px(a), px(b)),
                Stroke::default().with_color(accent).with_width(width),
            );
        }
    };
    let point = |p: Vec2| r.to_px(p.x, p.y);
    let mut path = |pts: &[Vec2], closed: bool, width: f32| {
        for pair in pts.windows(2) {
            line(point(pair[0]), point(pair[1]), width);
        }
        if closed && let (Some(a), Some(b)) = (pts.first(), pts.last()) {
            line(point(*a), point(*b), width);
        }
    };
    if let Some(doc) = app.document.as_ref() {
        let mut pieces = Vec::new();
        let mut ids = app.selection.ids().iter().copied().chain(
            app.selection
                .hover()
                .filter(|h| !app.selection.contains(*h)),
        );
        let objects =
            crate::viewport::highlight::objects_of(doc, Some(&app.spatial), &mut ids, &mut pieces);
        let extent = r.extent.unwrap_or([0., 0., 1., 1.]);
        let bounds = kentos_geometry_core::geometry::Bounds {
            min_x: extent[0],
            min_y: extent[1],
            max_x: extent[2],
            max_y: extent[3],
        };
        let origin = r
            .view
            .center
            .map_or(Vec2::new(0., 0.), |p| Vec2::new(p.x, p.y));
        let highlighted = scene::build_highlight(
            objects,
            &scene::Highlight {
                color: kentos_render_wgpu::Rgba8::rgb(56, 115, 216),
                fill: None,
                mark_size: 7.,
                mark_shape: 0,
            },
            origin,
            0.5 / r.pixels_per_metre,
            0,
            &bounds,
        );
        for seg in &highlighted.segments {
            let a = kentos_render_wgpu::precision::join(seg.a_hi, seg.a_lo);
            let b = kentos_render_wgpu::precision::join(seg.b_hi, seg.b_lo);
            path(
                &[
                    Vec2::new(origin.x + a[0], origin.y + a[1]),
                    Vec2::new(origin.x + b[0], origin.y + b[1]),
                ],
                false,
                2.,
            );
        }
    }
    if let Some(p) = app.session.preview(&app.format()) {
        path(&p.path, false, 1.5);
        if let Some(ring) = &p.ring {
            path(ring, true, 1.5);
        }
        for s in &p.strokes {
            path(&s.pts, s.closed, s.width);
        }
        for [a, b] in p.guides.iter().chain(p.hatch.iter()) {
            path(&[*a, *b], false, 1.);
        }
    }
    if let Some(band) = app.session.select_box()
        && let Some((a, b)) = band.worlds
    {
        let a = app.viewport.camera.world_to_screen(a);
        let b = app.viewport.camera.world_to_screen(b);
        let pts = [[a[0], a[1]], [b[0], a[1]], [b[0], b[1]], [a[0], b[1]]]
            .map(|p| app.viewport.camera.screen_to_world(p[0], p[1]));
        path(&pts, true, 1.);
    }
    if let Some(snap) = app.snap {
        let p = point(snap.point);
        let d = 4.;
        for (a, b) in [
            (Point::new(p.x - d, p.y - d), Point::new(p.x + d, p.y - d)),
            (Point::new(p.x + d, p.y - d), Point::new(p.x + d, p.y + d)),
            (Point::new(p.x + d, p.y + d), Point::new(p.x - d, p.y + d)),
            (Point::new(p.x - d, p.y + d), Point::new(p.x - d, p.y - d)),
        ] {
            line(a, b, 1.5);
        }
    }
    if let Some(p) = app.viewport.cursor {
        let p = point(p);
        line(Point::new(p.x - 10., p.y), Point::new(p.x + 10., p.y), 1.);
        line(Point::new(p.x, p.y - 10.), Point::new(p.x, p.y + 10.), 1.);
    }
}
