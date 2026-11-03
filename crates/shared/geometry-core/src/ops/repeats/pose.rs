//! Text values and block identities are semantic: a matching pose alone never proves equality.
use super::{Shape, Vec2, finite};
use crate::geometry::dist;

pub(super) fn supported(s: &Shape) -> bool {
    match s {
        Shape::Text {
            p,
            text,
            height,
            rotation,
            path,
            box_width,
            line_spacing,
            runs,
            flow,
            ..
        } => {
            finite(*p)
                && height.is_finite()
                && *height > 0.
                && rotation.is_finite()
                && text.len() <= 4096
                && !text.contains(['\n', '\r'])
                && path.is_none()
                && box_width.is_none()
                && line_spacing.is_none()
                && runs.as_ref().is_none_or(Vec::is_empty)
                && flow.is_none()
        }
        Shape::Insert {
            p,
            scale,
            y_scale,
            rotation,
            clip,
            params,
            attrs,
            ..
        } => {
            finite(*p)
                && scale.is_finite()
                && *scale > 0.
                && y_scale.is_none()
                && rotation.is_finite()
                && attrs.as_ref().is_none_or(|a| {
                    a.0.len() <= 128 && a.0.iter().all(|(k, v)| k.len() <= 128 && matches!(v, crate::api::json::Json::Str(s) if s.len() <= 4096))
                })
                && clip.is_none()
                && params.as_ref().is_none_or(|p| p.0.is_empty())
        }
        _ => false,
    }
}
pub(super) fn frame(s: &Shape) -> Option<(Vec2, f64, f64)> {
    match s {
        Shape::Text {
            p,
            height,
            rotation,
            ..
        } => Some((*p, *height, rotation.to_radians())),
        Shape::Insert {
            p, scale, rotation, ..
        } => Some((*p, *scale, *rotation)),
        _ => None,
    }
}
pub(super) fn work(s: &Shape) -> usize {
    match s {
        Shape::Text { text, .. } => 1 + text.len() / 128,
        Shape::Insert { attrs, .. } => {
            1 + attrs.as_ref().map_or(0, |a| {
                a.0.iter()
                    .map(|(k, v)| 1 + (k.len() + match v {crate::api::json::Json::Str(s) => s.len(), _ => 4096}) / 128)
                    .sum::<usize>()
            })
        }
        _ => 1,
    }
}
fn angle(a: f64, b: f64) -> bool {
    crate::jsmath::js_min((a - b).rem_euclid(std::f64::consts::TAU),
        (b - a).rem_euclid(std::f64::consts::TAU))
        <= 1e-10
}
pub(super) fn equal(
    a: &Shape,
    b: &Shape,
    t: f64,
    text_variants: bool,
    attribute_variants: bool,
) -> bool {
    if !supported(a) || !supported(b) {
        return false;
    }
    let mut normalized = b.clone();
    match (a, &mut normalized) {
        (
            Shape::Text {
                p,
                height,
                rotation,
                text,
                ..
            },
            Shape::Text {
                p: q,
                height: h,
                rotation: r,
                text: word,
                ..
            },
        ) => {
            if dist(*p, *q) > t
                || (height - *h).abs() > t
                || !angle(rotation.to_radians(), r.to_radians())
            {
                return false;
            }
            *q = *p;
            *h = *height;
            *r = *rotation;
            if text_variants {
                *word = text.clone();
            }
        }
        (
            Shape::Insert {
                p,
                scale,
                rotation,
                attrs,
                ..
            },
            Shape::Insert {
                p: q,
                scale: s,
                rotation: r,
                attrs: values,
                ..
            },
        ) => {
            if dist(*p, *q) > t || (scale - *s).abs() > scale.abs() * 1e-12 || !angle(*rotation, *r)
            {
                return false;
            }
            *q = *p;
            *s = *scale;
            *r = *rotation;
            if attribute_variants {
                *values = attrs.clone();
            }
        }
        _ => return false,
    }
    *a == normalized
}

#[cfg(test)]
mod tests {
    use super::super::{Options, find};
    use super::*;
    fn text(x: f64, word: &str) -> Shape {
        Shape::Text {
            p: Vec2::new(x, 1e6),
            text: word.into(),
            height: 0.003,
            rotation: 17.,
            align: None,
            width_factor: None,
            mask: None,
            box_width: None,
            line_spacing: None,
            runs: None,
            flow: None,
            face: Default::default(),
            path: None,
        }
    }
    #[test]
    fn text_values_are_separate_from_exact_poses_styles_and_large_world_coordinates() {
        let a = text(4e9, "A01");
        let b = text(4e9 + 0.3, "A02");
        let mut wrong = text(4e9 + 0.6, "A03");
        if let Shape::Text { face, .. } = &mut wrong {
            face.bold = true;
        }
        let scene = [a, b, wrong];
        let exact = find(&scene, &[0], Options::default(), |_, _| true, || false).unwrap();
        assert_eq!(exact.matches.len(), 1);
        let variants = find(
            &scene,
            &[0],
            Options {
                text_variants: true,
                ..Default::default()
            },
            |_, _| true,
            || false,
        )
        .unwrap();
        assert_eq!(variants.matches.len(), 2, "{variants:?}");
        assert_eq!(variants.matches[1].members, [1]);
        assert_eq!(variants.matches[1].rotation, 0.);
        assert_eq!(variants.matches[1].scale, 1.);
    }
    #[test]
    fn an_insert_requires_the_same_definition_mirror_values_and_exact_transform() {
        let a = Shape::Insert {
            block: "same-definition".into(),
            p: Vec2::new(0., 0.),
            scale: 1.,
            y_scale: None,
            rotation: 0.,
            mirror: None,
            attrs: None,
            places: None,
            params: None,
            clip: None,
        };
        let mut b = a.clone();
        if let Shape::Insert {
            p,
            rotation,
            mirror,
            ..
        } = &mut b
        {
            *p = Vec2::new(2., 3.);
            *rotation = 0.4;
            *mirror = Some(true);
        }
        let mut other = b.clone();
        if let Shape::Insert { block, .. } = &mut other {
            *block = "another-definition".into();
        }
        let scene = [a, b, other];
        assert_eq!(
            find(&scene, &[0], Options::default(), |_, _| true, || false)
                .unwrap()
                .matches
                .len(),
            1
        );
        let found = find(
            &scene,
            &[0],
            Options {
                mirrors: true,
                ..Default::default()
            },
            |_, _| true,
            || false,
        )
        .unwrap();
        assert_eq!(found.matches.len(), 2, "{found:?}");
        assert!(found.matches[1].mirror);
        assert_eq!(found.matches[1].members, [1]);
    }
}
