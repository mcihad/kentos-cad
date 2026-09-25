//! Float64 world coordinates → float32 GPU values (TODOS.md REN-07,
//! CLAUDE.md §4.9).
//!
//! The CPU keeps every coordinate in float64. The GPU never sees an absolute
//! world coordinate: it gets each point's offset from the scene's local origin
//! as two float32 parts, `hi = f32(d)` and `lo = f32(d − hi)`, and the camera
//! centre the same way. The vertex shader (`world_px`,
//! `shaders/wgsl/common/frame.wgsl`) subtracts high from high and low from
//! low: both differences are small for anything near the camera, so the
//! float32 it scales to pixels is the offset from the camera, good to about
//! 2⁻²⁴ of the distance from the view's centre. At Turkish TM coordinates
//! (E ≈ 487 000, N ≈ 4 420 000 m) that stays far below a millimetre at any
//! zoom and wherever the origin lies (`tests/precision.rs`).
//!
//! The functions here mirror the shader's float32 steps one for one, so tests
//! can measure on the CPU what the GPU computes; `tests/gpu.rs` checks the
//! same on a real device.

use crate::Vec2;
use crate::layout::FrameUniform;

/// The longest straight piece the scene hands the GPU, in world units. A
/// longer segment is cut into collinear pieces (display only, the document
/// is untouched). The piece under the camera then has both ends near it, so
/// its position on screen does not depend on how far away the segment's own
/// ends lie: at the deepest zoom (5000 px/m) an end 128 m away is still placed
/// to about 0.04 px.
pub const MAX_SEGMENT_LENGTH: f64 = 128.0;

/// Most pieces one segment is cut into: a segment kilometres long in absurd
/// data does not multiply without bound; its pieces are longer then.
pub const MAX_SEGMENT_PIECES: usize = 4096;

/// The float32 high and low parts of `v`: `hi + lo` equals `v` to about 2⁻⁴⁸ of its size.
pub fn split(v: f64) -> (f32, f32) {
    let hi = v as f32;
    let lo = (v - f64::from(hi)) as f32;
    (hi, lo)
}

/// High and low parts of `p − origin`, the difference taken in float64.
pub fn split_offset(p: Vec2, origin: Vec2) -> ([f32; 2], [f32; 2]) {
    let (xh, xl) = split(p.x - origin.x);
    let (yh, yl) = split(p.y - origin.y);
    ([xh, yh], [xl, yl])
}

/// The value the parts stand for, in float64.
pub fn join(hi: [f32; 2], lo: [f32; 2]) -> [f64; 2] {
    [
        f64::from(hi[0]) + f64::from(lo[0]),
        f64::from(hi[1]) + f64::from(lo[1]),
    ]
}

/// Device pixels from the camera centre (y up) of a point given as parts:
/// `world_px` of `common/frame.wgsl`, step for step in float32.
pub fn world_px(hi: [f32; 2], lo: [f32; 2], frame: &FrameUniform) -> [f32; 2] {
    let dx = (hi[0] - frame.center_hi[0]) + (lo[0] - frame.center_lo[0]);
    let dy = (hi[1] - frame.center_hi[1]) + (lo[1] - frame.center_lo[1]);
    [dx * frame.px_per_unit, dy * frame.px_per_unit]
}

/// The four corners (device pixels from the camera centre, y up) of the box
/// `line_vs` (`cad2d/line.wgsl`) puts around segment a → b, in float32 as the
/// shader computes them: corner bit 0 picks the side, bit 1 the end.
pub fn line_corners(
    a_hi: [f32; 2],
    a_lo: [f32; 2],
    b_hi: [f32; 2],
    b_lo: [f32; 2],
    frame: &FrameUniform,
) -> [[f32; 2]; 4] {
    const FEATHER: f32 = 1.0;
    let a = world_px(a_hi, a_lo, frame);
    let b = world_px(b_hi, b_lo, frame);
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let dir = if len > 1e-6 {
        [d[0] / len, d[1] / len]
    } else {
        [1.0, 0.0]
    };
    let normal = [-dir[1], dir[0]];
    let half_width = 0.5 * frame.line_width * frame.dpi;
    let reach = half_width + FEATHER;
    let mut out = [[0.0f32; 2]; 4];
    for (corner, point) in out.iter_mut().enumerate() {
        let along = if corner & 2 != 0 { len + reach } else { -reach };
        let across = if corner & 1 != 0 { reach } else { -reach };
        *point = [
            a[0] + dir[0] * along + normal[0] * across,
            a[1] + dir[1] * along + normal[1] * across,
        ];
    }
    out
}
