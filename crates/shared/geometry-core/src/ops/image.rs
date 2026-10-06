//! What the picture tools compute (docs/adr/0192 §5), for both platforms
//! (the web through the op table): Resim ekle's frame from its lower left
//! corner and a second point, and Resmi kırp's boundary from world points.

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::image::{Frame, clipped};
use crate::jsmath::{atan2, js_hypot};
use crate::op;
use crate::vec2::Vec2;

/// Why a clip gives nothing, in the tool's words.
pub const CLIP_OUTSIDE: &str =
    "Kırpma sınırı resmin dışında kalıyor; resmin üzerinde bir sınır çizin.";
/// Why a second point gives no frame.
pub const NO_SIZE: &str =
    "İkinci nokta birinciyle aynı yerde; resmin genişliği için başka bir noktaya tıklayın.";

/// A picture's frame placed by its lower left corner `p` and a second point
/// `q`: its width the distance between them, its turn their direction, its
/// height the width times `aspect` (the picture's height over its width).
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
}

crate::json_struct!(out Placed {
    width,
    height,
    rotation
});

pub fn placed(p: Vec2, q: Vec2, aspect: f64) -> Result<Placed, String> {
    let width = js_hypot(q.x - p.x, q.y - p.y);
    if !(width > 0.0 && width.is_finite() && aspect > 0.0 && aspect.is_finite()) {
        return Err(NO_SIZE.to_owned());
    }
    Ok(Placed {
        width,
        height: width * aspect,
        rotation: atan2(q.y - p.y, q.x - p.x),
    })
}

/// A boundary drawn in the world (at least three corners) as a picture's
/// clip: in its own fractions, cut to the picture, counter-clockwise; why
/// not when it misses the picture.
pub fn clip_of(shape: &Shape, world: &[Vec2]) -> Result<Vec<Vec2>, String> {
    let Some(frame) = Frame::of(shape) else {
        return Err(CLIP_OUTSIDE.to_owned());
    };
    let fractions: Option<Vec<Vec2>> = world
        .iter()
        .map(|q| frame.picture_fractions(*q).map(|(s, t)| Vec2::new(s, t)))
        .collect();
    fractions
        .filter(|f| f.len() >= 3)
        .and_then(|f| clipped(&f))
        .ok_or_else(|| CLIP_OUTSIDE.to_owned())
}

/// A clip as the op answers it.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipAnswer {
    pub clip: Option<Vec<Vec2>>,
    pub problem: Option<String>,
}

crate::json_struct!(out ClipAnswer { clip, problem });

/// A placement as the op answers it.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedAnswer {
    pub placed: Option<Placed>,
    pub problem: Option<String>,
}

crate::json_struct!(out PlacedAnswer { placed, problem });

pub(crate) static OPS: &[Op] = &[
    op!(
        "imagePlaced",
        |p: Vec2, q: Vec2, aspect: f64| match placed(p, q, aspect) {
            Ok(placed) => PlacedAnswer {
                placed: Some(placed),
                problem: None,
            },
            Err(why) => PlacedAnswer {
                placed: None,
                problem: Some(why),
            },
        }
    ),
    op!(
        "imageClip",
        |e: Entity, world: Vec<Vec2>| match clip_of(&e.shape, &world) {
            Ok(clip) => ClipAnswer {
                clip: Some(clip),
                problem: None,
            },
            Err(why) => ClipAnswer {
                clip: None,
                problem: Some(why),
            },
        }
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(rotation: f64, mirror: bool) -> Shape {
        Shape::Image {
            p: Vec2::new(100.0, 200.0),
            width: 40.0,
            height: 20.0,
            rotation,
            mirror: mirror.then_some(true),
            asset: Some("resim-0".into()),
            file: None,
            clip: None,
            opacity: None,
        }
    }

    #[test]
    fn a_second_point_gives_the_width_and_the_turn() {
        let got = placed(Vec2::new(0.0, 0.0), Vec2::new(0.0, 30.0), 0.5).expect("placed");
        assert_eq!(got.width, 30.0);
        assert_eq!(got.height, 15.0);
        assert!((got.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
        assert!(placed(Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0), 1.0).is_err());
    }

    #[test]
    fn a_boundary_is_cut_to_the_picture_in_its_own_fractions() {
        // The picture's left half and beyond: cut at its edges.
        let world = [
            Vec2::new(90.0, 190.0),
            Vec2::new(120.0, 190.0),
            Vec2::new(120.0, 230.0),
            Vec2::new(90.0, 230.0),
        ];
        let clip = clip_of(&picture(0.0, false), &world).expect("clip");
        assert_eq!(
            clip,
            [
                Vec2::new(0.0, 0.0),
                Vec2::new(0.5, 0.0),
                Vec2::new(0.5, 1.0),
                Vec2::new(0.0, 1.0)
            ]
        );
        // Upside down in its frame: the frame's bottom is the picture's top.
        let low = [
            Vec2::new(100.0, 200.0),
            Vec2::new(140.0, 200.0),
            Vec2::new(140.0, 205.0),
            Vec2::new(100.0, 205.0),
        ];
        let clip = clip_of(&picture(0.0, true), &low).expect("clip");
        assert!(clip.iter().all(|q| q.y >= 0.75 - 1e-12));
        // Beside it: nothing.
        let away = [
            Vec2::new(300.0, 300.0),
            Vec2::new(310.0, 300.0),
            Vec2::new(310.0, 310.0),
        ];
        assert_eq!(
            clip_of(&picture(0.0, false), &away),
            Err(CLIP_OUTSIDE.to_owned())
        );
    }
}
