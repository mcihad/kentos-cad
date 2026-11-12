//! Local advisory only: never move a dimension or change its measured value to make room.
use crate::tool::{Context, Preview, Stroke, TextGhost, Tone};
use kentos_commands::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::{
    Vec2,
    entity::TextPlace,
    geometry::{Bounds, empty_bounds, extend_bounds},
};

fn bounds(points: &[Vec2]) -> Bounds {
    let mut b = empty_bounds();
    for &p in points {
        extend_bounds(&mut b, p, 0.);
    }
    b
}
fn crosses(a: Bounds, b: Bounds) -> bool {
    a.min_x < b.max_x && b.min_x < a.max_x && a.min_y < b.max_y && b.min_y < a.max_y
}
fn outline(ghost: &TextGhost, font: kentos_geometry_core::text::Font) -> Vec<Vec2> {
    let mut p = TextPlace::line(
        ghost.p,
        &ghost.text,
        ghost.height,
        ghost.rotation,
        ghost.align,
        Some(ghost.width_factor),
    );
    p.font = ghost
        .face
        .font
        .map(|f| kentos_native_application::geometry::drawing_font(Some(f)));
    p.bold = ghost.face.bold;
    p.outline(font)
}

/// Inspect at most 128 nearby objects in the maintained spatial index. Rotated boxes use a
/// conservative axis-aligned intersection, so the visible wording deliberately says “may”.
pub(crate) fn overlap(cx: &Context<'_>, ghost: &TextGhost) -> Option<Vec<Vec2>> {
    let font = kentos_native_application::geometry::drawing_font(cx.doc.settings().drawing_font);
    let ring = outline(ghost, font);
    let b = bounds(&ring);
    let format = cx.format();
    let store = cx.spatial.store();
    for id in store.reaching(&b).into_iter().take(128) {
        let Some(it) = store.get(id).filter(|it| store.shown(it)) else {
            continue;
        };
        let other = match cx.doc.get(Slot(id as u32)) {
            Some(Entity::Dimension(d)) => {
                let Some(l) = crate::spatial::dimension_layout(&Entity::Dimension(d.clone()))
                else {
                    continue;
                };
                outline(
                    &TextGhost {
                        p: l.text_at,
                        text: format.dimension_value(
                            l.prefix,
                            l.unit,
                            l.value,
                            &d.look,
                            d.text.as_deref(),
                        ),
                        height: d.height,
                        rotation: l.rotation,
                        align: Some(kentos_geometry_core::text::TextAlign::MiddleCenter),
                        mask: false,
                        face: kentos_commands::TextFace {
                            font: d.look.font,
                            ..Default::default()
                        },
                        width_factor: 1.,
                    },
                    font,
                )
            }
            Some(Entity::Text(_)) => {
                let Some(place) = TextPlace::of(&it.shape) else {
                    continue;
                };
                place.outline(font)
            }
            _ => continue,
        };
        if crosses(b, bounds(&other)) {
            return Some(ring);
        }
    }
    None
}

pub(crate) fn warn(preview: &mut Preview, ring: &Option<Vec<Vec2>>) {
    if let Some(ring) = ring {
        preview
            .strokes
            .push(Stroke::solid(ring.clone(), true).tone(Tone::Danger));
        preview.tag_tone = Tone::Danger;
        if let Some(tag) = &mut preview.tag {
            tag.lines
                .push("Yazı kutusu başka bir açıklamayla çakışabilir".into());
        }
    }
}
