//! The sheet's frame (pafta çerçevesi): one or two lines; in CAD with zone
//! marks in the band between them (ISO 5457: columns 1, 2, 3 … from the
//! left, rows A, B, C … from the top, I and O left out, an even number of
//! zones about `size` long) and centring marks at the middle of each side.

use super::{Note, Pen};
use crate::kinds::*;
use crate::model::Item;
use crate::text;

/// Zone letters: the alphabet without I and O, then doubled.
pub fn zone_letter(i: usize) -> String {
    const L: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
    if i < L.len() {
        char::from(L[i]).to_string()
    } else {
        let a = char::from(L[(i / L.len() - 1) % L.len()]);
        let b = char::from(L[i % L.len()]);
        format!("{a}{b}")
    }
}

/// An even number of zones, at least two, about `size` long each.
fn zones_along(len: i64, size: i64) -> i64 {
    if size <= 0 {
        return 2;
    }
    let n = (len + size / 2) / size;
    let even = if n % 2 == 0 { n } else { n + 1 };
    even.max(2)
}

pub(crate) fn draw(it: &Item, b: &BorderItem, pen: &mut Pen, notes: &mut Vec<Note>) {
    let f = it.content_rect();
    let outer = f.inset(b.inset);
    let band = if b.style == BorderStyle::Double || b.zones.is_some() {
        b.gap
    } else {
        0
    };
    let inner = outer.inset(band);
    if b.centring_marks {
        // From the outer line to 5 mm inside the inner one, at the middle of each side.
        let (cx, cy) = (outer.center()[0], outer.center()[1]);
        let reach = 5_000.0;
        let (ox0, oy0, ox1, oy1) = (
            f64::from(outer.left),
            f64::from(outer.top),
            outer.right() as f64,
            outer.bottom() as f64,
        );
        let (ix0, iy0, ix1, iy1) = (
            f64::from(inner.left),
            f64::from(inner.top),
            inner.right() as f64,
            inner.bottom() as f64,
        );
        pen.lines(
            &[
                [[cx, oy0], [cx, iy0 + reach]],
                [[cx, oy1], [cx, iy1 - reach]],
                [[ox0, cy], [ix0 + reach, cy]],
                [[ox1, cy], [ix1 - reach, cy]],
            ],
            &b.stroke,
        );
    }
    if let Some(z) = &b.zones {
        let cols = zones_along(i64::from(inner.width), i64::from(z.size));
        let rows = zones_along(i64::from(inner.height), i64::from(z.size));
        let cap = text::cap_height(&z.text) as f64;
        if cap > f64::from(band) * 0.8 {
            notes.push(Note {
                item: it.id.clone(),
                code: "zone_text_too_large",
                detail: String::new(),
            });
        }
        let mut segs = Vec::new();
        let (ox0, oy0, ox1, oy1) = (
            f64::from(outer.left),
            f64::from(outer.top),
            outer.right() as f64,
            outer.bottom() as f64,
        );
        let (ix0, iy0, ix1, iy1) = (
            f64::from(inner.left),
            f64::from(inner.top),
            inner.right() as f64,
            inner.bottom() as f64,
        );
        let xs: Vec<f64> = (0..=cols)
            .map(|i| ix0 + (ix1 - ix0) * i as f64 / cols as f64)
            .collect();
        let ys: Vec<f64> = (0..=rows)
            .map(|i| iy0 + (iy1 - iy0) * i as f64 / rows as f64)
            .collect();
        for x in &xs[1..xs.len() - 1] {
            segs.push([[*x, oy0], [*x, iy0]]);
            segs.push([[*x, iy1], [*x, oy1]]);
        }
        for y in &ys[1..ys.len() - 1] {
            segs.push([[ox0, *y], [ix0, *y]]);
            segs.push([[ix1, *y], [ox1, *y]]);
        }
        pen.lines(&segs, &b.inner_stroke);
        let mid_top = (oy0 + iy0) / 2.0 + cap / 2.0;
        let mid_bottom = (iy1 + oy1) / 2.0 + cap / 2.0;
        for (i, w) in xs.windows(2).enumerate() {
            let t = (i + 1).to_string();
            let tw = text::text_width(&t, &z.text);
            let x = (w[0] + w[1]) / 2.0 - tw as f64 / 2.0;
            pen.text(&t, [x, mid_top], &z.text, z.text.size, tw, 0);
            pen.text(&t, [x, mid_bottom], &z.text, z.text.size, tw, 0);
        }
        let mid_left = (ox0 + ix0) / 2.0;
        let mid_right = (ix1 + ox1) / 2.0;
        for (i, w) in ys.windows(2).enumerate() {
            let t = zone_letter(i);
            let tw = text::text_width(&t, &z.text) as f64;
            let y = (w[0] + w[1]) / 2.0 + cap / 2.0;
            pen.text(
                &t,
                [mid_left - tw / 2.0, y],
                &z.text,
                z.text.size,
                tw as i64,
                0,
            );
            pen.text(
                &t,
                [mid_right - tw / 2.0, y],
                &z.text,
                z.text.size,
                tw as i64,
                0,
            );
        }
    }
    if b.style == BorderStyle::Double {
        pen.rect(inner, None, Some(&b.inner_stroke), 0);
    }
    pen.rect(outer, None, Some(&b.stroke), 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_are_even_and_skip_i_and_o() {
        assert_eq!(zones_along(400_000, 50_000), 8);
        assert_eq!(zones_along(190_000, 50_000), 4);
        assert_eq!(zones_along(277_000, 50_000), 6);
        assert_eq!(zones_along(40_000, 50_000), 2);
        let letters: String = (0..10).map(zone_letter).collect();
        assert_eq!(letters, "ABCDEFGHJK");
        assert_eq!(zone_letter(24), "AA");
    }
}
