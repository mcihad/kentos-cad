//! A picture tile cut for the drawing's raster atlas (docs/adr/0208 §3), the
//! same on both apps: the atlas holds 256-pixel tiles in 258-pixel slots,
//! each with an apron of its neighbours' pixels (the tile's edge repeated
//! outside it) so bilinear sampling shows no seam inside a tile, its colours
//! premultiplied. A tile of 512 pixels takes four slots; one of another size
//! is resampled to the grid's first. A tile clear everywhere is empty.

/// A slot's side: a tile's 256 pixels and a pixel round them (the raster atlas's).
pub const SLOT: u32 = 258;
/// A slot's picture side.
pub const PIECE: usize = (SLOT - 2) as usize;

/// Slots a picture tile of `px` pixels takes across (or down): its own
/// 256-pixel pieces up to four, else one up to 384 pixels and two beyond
/// (resampled). The view's slots and a tile's cutting agree by this.
pub fn slots_across(px: u32) -> u32 {
    if px.is_multiple_of(PIECE as u32) && px <= 4 * PIECE as u32 && px > 0 {
        px / PIECE as u32
    } else if px <= PIECE as u32 * 3 / 2 {
        1
    } else {
        2
    }
}

/// A tile cut into slots: `across` × `down`, row by row, each `SLOT` × `SLOT`
/// premultiplied RGBA.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub across: u32,
    pub down: u32,
    pub slots: Vec<Vec<u8>>,
}

/// `rgba` (straight alpha) cut into premultiplied slots: as many as a tile
/// of `want` pixels takes (its own size's when none), resampled when its
/// size is another; none when nothing of it shows.
pub fn slots(
    width: usize,
    height: usize,
    mut rgba: Vec<u8>,
    want: Option<(u32, u32)>,
) -> Option<Cut> {
    if width == 0 || height == 0 || rgba.len() != width * height * 4 {
        return None;
    }
    if rgba.chunks_exact(4).all(|p| p[3] == 0) {
        return None;
    }
    let (ww, wh) = want.unwrap_or((width as u32, height as u32));
    let (tw, th) = (
        slots_across(ww) as usize * PIECE,
        slots_across(wh) as usize * PIECE,
    );
    let (mut w, mut h) = (width, height);
    if (w, h) != (tw, th) {
        rgba = resample(&rgba, w, h, tw, th);
        (w, h) = (tw, th);
    }
    for p in rgba.chunks_exact_mut(4) {
        let a = u16::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
    let (across, down) = (w / PIECE, h / PIECE);
    let side = SLOT as usize;
    let mut out = Vec::with_capacity(across * down);
    for sy in 0..down {
        for sx in 0..across {
            let mut slot = vec![0u8; side * side * 4];
            for y in 0..side {
                // The apron: the row above and below, clamped to the tile.
                let ty = (sy * PIECE + y).saturating_sub(1).min(h - 1);
                for x in 0..side {
                    let tx = (sx * PIECE + x).saturating_sub(1).min(w - 1);
                    let from = (ty * w + tx) * 4;
                    let to = (y * side + x) * 4;
                    slot[to..to + 4].copy_from_slice(&rgba[from..from + 4]);
                }
            }
            out.push(slot);
        }
    }
    Some(Cut {
        across: across as u32,
        down: down as u32,
        slots: out,
    })
}

/// Bilinear resampling of straight-alpha RGBA.
fn resample(rgba: &[u8], w: usize, h: usize, to_w: usize, to_h: usize) -> Vec<u8> {
    let mut out = vec![0u8; to_w * to_h * 4];
    let (sx, sy) = (w as f64 / to_w as f64, h as f64 / to_h as f64);
    for y in 0..to_h {
        let fy = ((y as f64 + 0.5) * sy - 0.5).clamp(0.0, (h - 1) as f64);
        let (y0, ty) = (fy.floor() as usize, fy - fy.floor());
        let y1 = (y0 + 1).min(h - 1);
        for x in 0..to_w {
            let fx = ((x as f64 + 0.5) * sx - 0.5).clamp(0.0, (w - 1) as f64);
            let (x0, tx) = (fx.floor() as usize, fx - fx.floor());
            let x1 = (x0 + 1).min(w - 1);
            for c in 0..4 {
                let at = |xx: usize, yy: usize| f64::from(rgba[(yy * w + xx) * 4 + c]);
                let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
                let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
                out[(y * to_w + x) * 4 + c] = (top * (1.0 - ty) + bottom * ty).round() as u8;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_cut_into_slots_with_its_edges_repeated_and_resampled_when_odd() {
        // 512 × 512: four slots; red on the left half, blue on the right.
        let (w, h) = (512usize, 512usize);
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let p = (y * w + x) * 4;
                rgba[p..p + 4].copy_from_slice(if x < 256 {
                    &[255, 0, 0, 255]
                } else {
                    &[0, 0, 255, 255]
                });
            }
        }
        let img = slots(w, h, rgba, None).expect("a picture");
        assert_eq!((img.across, img.down, img.slots.len()), (2, 2, 4));
        let side = SLOT as usize;
        let px =
            |s: &[u8], x: usize, y: usize| s[(y * side + x) * 4..(y * side + x) * 4 + 4].to_vec();
        // The first slot's apron left of its first pixel repeats the tile's edge (red);
        // its apron right of its last pixel is its neighbour's first (blue).
        assert_eq!(px(&img.slots[0], 0, 5), vec![255, 0, 0, 255]);
        assert_eq!(px(&img.slots[0], side - 1, 5), vec![0, 0, 255, 255]);
        assert_eq!(px(&img.slots[1], 0, 5), vec![255, 0, 0, 255]);
        // A clear tile is empty; a 300-pixel one becomes one slot.
        assert!(slots(256, 256, vec![0; 256 * 256 * 4], None).is_none());
        let odd = slots(300, 300, vec![200; 300 * 300 * 4], None).expect("a picture");
        assert_eq!((odd.across, odd.down), (1, 1));
        // A 256-pixel answer of a grid of 512-pixel tiles is cut as the grid's.
        let up = slots(256, 256, vec![200; 256 * 256 * 4], Some((512, 512))).expect("a picture");
        assert_eq!((up.across, up.down, up.slots.len()), (2, 2, 4));
        assert_eq!(
            (
                slots_across(256),
                slots_across(512),
                slots_across(300),
                slots_across(500),
                slots_across(768)
            ),
            (1, 2, 1, 2, 3)
        );
        // Premultiplied: 200 at alpha 200 is 157.
        assert_eq!(odd.slots[0][(5 * side + 5) * 4], 157);
    }
}
