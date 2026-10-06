//! A picture's frame (docs/adr/0192): the rectangle its lower left corner,
//! width, height and turn give; a point of the frame by its fractions (0,0
//! the lower left, 1,1 the upper right) and back; the part its clip shows.
//!
//! The clip is in the picture's own fractions, so no move, turn, scale or
//! mirror changes it. A mirrored picture is turned upside down inside its
//! frame (as an insert is mirrored in its x axis before its turn): the
//! picture's own fraction up the frame is one less the frame's.

use crate::entity::Shape;
use crate::jsmath::{atan2, cos, js_hypot, sin};
use crate::vec2::Vec2;

/// A picture's frame: its lower left corner and its two sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub p: Vec2,
    /// Along its bottom edge, its width long.
    pub u: Vec2,
    /// Up its left edge, its height long.
    pub v: Vec2,
    /// The picture upside down inside it.
    pub mirror: bool,
}

impl Frame {
    /// The frame of a picture; none for any other shape.
    pub fn of(shape: &Shape) -> Option<Frame> {
        match shape {
            Shape::Image {
                p,
                width,
                height,
                rotation,
                mirror,
                ..
            } => Some(Frame::new(
                *p,
                *width,
                *height,
                *rotation,
                mirror.unwrap_or(false),
            )),
            _ => None,
        }
    }

    pub fn new(p: Vec2, width: f64, height: f64, rotation: f64, mirror: bool) -> Frame {
        let (c, s) = (cos(rotation), sin(rotation));
        Frame {
            p,
            u: Vec2::new(width * c, width * s),
            v: Vec2::new(-height * s, height * c),
            mirror,
        }
    }

    /// The point of the frame at its fractions `s` along and `t` up.
    pub fn at(&self, s: f64, t: f64) -> Vec2 {
        Vec2::new(
            self.p.x + self.u.x * s + self.v.x * t,
            self.p.y + self.u.y * s + self.v.y * t,
        )
    }

    /// Its corners, counter-clockwise from the lower left.
    pub fn corners(&self) -> [Vec2; 4] {
        [
            self.at(0.0, 0.0),
            self.at(1.0, 0.0),
            self.at(1.0, 1.0),
            self.at(0.0, 1.0),
        ]
    }

    /// A point's fractions along and up the frame; none for a frame without area.
    pub fn fractions(&self, q: Vec2) -> Option<(f64, f64)> {
        let det = self.u.x * self.v.y - self.u.y * self.v.x;
        if !(det.abs() > 0.0) {
            return None;
        }
        let d = Vec2::new(q.x - self.p.x, q.y - self.p.y);
        Some((
            (d.x * self.v.y - d.y * self.v.x) / det,
            (self.u.x * d.y - self.u.y * d.x) / det,
        ))
    }

    /// The point of the frame at the picture's own fractions (a clip's corner).
    pub fn at_picture(&self, s: f64, t: f64) -> Vec2 {
        self.at(s, if self.mirror { 1.0 - t } else { t })
    }

    /// A point in the picture's own fractions.
    pub fn picture_fractions(&self, q: Vec2) -> Option<(f64, f64)> {
        let (s, t) = self.fractions(q)?;
        Some((s, if self.mirror { 1.0 - t } else { t }))
    }

    /// Its width, height and turn (radians, counter-clockwise).
    pub fn size_and_turn(&self) -> (f64, f64, f64) {
        (
            js_hypot(self.u.x, self.u.y),
            js_hypot(self.v.x, self.v.y),
            atan2(self.u.y, self.u.x),
        )
    }
}

/// The part of a picture shown, in the world, counter-clockwise: its clip's
/// corners, or its frame's corners; empty for any other shape.
pub fn shown(shape: &Shape) -> Vec<Vec2> {
    let Some(frame) = Frame::of(shape) else {
        return Vec::new();
    };
    match shape {
        Shape::Image {
            clip: Some(clip), ..
        } if clip.len() >= 3 => {
            let mut ring: Vec<Vec2> = clip.iter().map(|q| frame.at_picture(q.x, q.y)).collect();
            if crate::geometry::signed_area(&ring) < 0.0 {
                ring.reverse();
            }
            ring
        }
        _ => frame.corners().to_vec(),
    }
}

/// The picture's own shape with another frame (its source, clip and opacity kept).
pub fn with_frame(shape: &Shape, frame: &Frame) -> Shape {
    match shape {
        Shape::Image {
            asset,
            file,
            clip,
            opacity,
            ..
        } => {
            let (width, height, rotation) = frame.size_and_turn();
            Shape::Image {
                p: frame.p,
                width,
                height,
                rotation,
                mirror: frame.mirror.then_some(true),
                asset: asset.clone(),
                file: file.clone(),
                clip: clip.clone(),
                opacity: *opacity,
            }
        }
        _ => shape.clone(),
    }
}

/// The frame a reflection makes of `frame`, given its corners reflected
/// (lower left, lower right, upper right, upper left): the reflected upper
/// left is the new lower left and the picture turns over (docs/adr/0192 §4).
pub fn reflected(frame: &Frame, corners: [Vec2; 4]) -> Frame {
    let [lower_left, _, upper_right, upper_left] = corners;
    Frame {
        p: upper_left,
        u: Vec2::new(upper_right.x - upper_left.x, upper_right.y - upper_left.y),
        v: Vec2::new(lower_left.x - upper_left.x, lower_left.y - upper_left.y),
        mirror: !frame.mirror,
    }
}

/// A clip boundary in the picture's own fractions cut to the picture (the
/// unit square; Sutherland–Hodgman, the square being convex), counter-
/// clockwise; none when nothing of it is on the picture (docs/adr/0192 §5).
pub fn clipped(ring: &[Vec2]) -> Option<Vec<Vec2>> {
    // Each side of the square as a test (inside when ≥ 0) and where a segment crosses it.
    type Side = (fn(Vec2) -> f64, fn(Vec2, Vec2) -> Vec2);
    fn cross_x(a: Vec2, b: Vec2, x: f64) -> Vec2 {
        let t = (x - a.x) / (b.x - a.x);
        Vec2::new(x, a.y + (b.y - a.y) * t)
    }
    fn cross_y(a: Vec2, b: Vec2, y: f64) -> Vec2 {
        let t = (y - a.y) / (b.y - a.y);
        Vec2::new(a.x + (b.x - a.x) * t, y)
    }
    let sides: [Side; 4] = [
        (|p| p.x, |a, b| cross_x(a, b, 0.0)),
        (|p| 1.0 - p.x, |a, b| cross_x(a, b, 1.0)),
        (|p| p.y, |a, b| cross_y(a, b, 0.0)),
        (|p| 1.0 - p.y, |a, b| cross_y(a, b, 1.0)),
    ];
    let mut out: Vec<Vec2> = ring.to_vec();
    for (inside, meet) in sides {
        let input = std::mem::take(&mut out);
        let n = input.len();
        for i in 0..n {
            let (a, b) = (input[i], input[(i + 1) % n]);
            let (ia, ib) = (inside(a) >= 0.0, inside(b) >= 0.0);
            if ia {
                out.push(a);
            }
            if ia != ib {
                out.push(meet(a, b));
            }
        }
    }
    // Corners a side met twice go once.
    out.dedup_by(|a, b| (a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12);
    if out.len() > 1
        && let (Some(f), Some(l)) = (out.first(), out.last())
        && (f.x - l.x).abs() < 1e-12
        && (f.y - l.y).abs() < 1e-12
    {
        out.pop();
    }
    let area = crate::geometry::signed_area(&out);
    if out.len() < 3 || area.abs() < 1e-9 {
        return None;
    }
    if area < 0.0 {
        out.reverse();
    }
    // From its lowest corner (the leftmost of them), so that a boundary is written one way.
    let first = (0..out.len())
        .min_by(|&i, &j| {
            let (a, b) = (out[i], out[j]);
            a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x))
        })
        .unwrap_or(0);
    out.rotate_left(first);
    Some(out)
}
