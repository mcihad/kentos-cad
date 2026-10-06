//! Genel bakış and Büyüteç (docs/adr/0181 §3, §4): where their cards stand in
//! the drawing area, the side the magnifier takes as the pointer nears it,
//! and the overview's fit — the drawing's extent in its picture, the view's
//! frame on it, a press turned into the view's new centre. Both platforms
//! keep these rules (the web's `viewport/navigation.ts`), and play the cases
//! of `fixtures/navigation/v1/cases.json` (scripts/fixtures/navigation_cases.py).

use crate::geometry::Bounds;
use crate::jsmath::{js_max, js_min};

/// From the drawing area's edges, logical pixels.
pub const MARGIN: f64 = 8.0;
/// Between the two cards.
pub const GAP: f64 = 8.0;
/// How near the pointer comes before the magnifier moves to the other side.
pub const NEAR: f64 = 16.0;
/// Room under a GIS project's grid north at the top right.
pub const NORTH: f64 = 52.0;
/// The overview's picture, logical pixels.
pub const OVERVIEW: (f64, f64) = (240.0, 160.0);
/// The magnifier's picture.
pub const LENS: (f64, f64) = (220.0, 220.0);
/// The overview's margin inside its picture.
pub const PAD: f64 = 6.0;
/// The magnifier's steps.
pub const ZOOMS: [u32; 4] = [2, 4, 8, 16];

/// A rectangle: x, y, width, height.
pub type Rect = [f64; 4];

/// A card and its content (below its title row, inside its 1 px frame).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Card {
    pub card: Rect,
    pub content: Rect,
}

fn card(x: f64, y: f64, content: (f64, f64), header: f64) -> Card {
    Card {
        card: [x, y, content.0 + 2.0, header + content.1 + 2.0],
        content: [x + 1.0, y + 1.0 + header, content.0, content.1],
    }
}

/// Which side the magnifier is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Right,
    Left,
}

impl Side {
    pub fn word(self) -> &'static str {
        match self {
            Side::Right => "right",
            Side::Left => "left",
        }
    }

    fn other(self) -> Side {
        match self {
            Side::Right => Side::Left,
            Side::Left => Side::Right,
        }
    }
}

/// The cards in a drawing area of `area` logical pixels: the overview's when
/// it shows, and the magnifier's on either side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cards {
    pub overview: Option<Card>,
    pub right: Card,
    pub left: Card,
}

impl Cards {
    pub fn lens(&self, side: Side) -> Card {
        match side {
            Side::Right => self.right,
            Side::Left => self.left,
        }
    }
}

/// Where the cards stand (§4): the overview at the top left; the magnifier
/// at the top right (under a GIS project's grid north), or at the left under
/// the overview, beside it when the area is too low.
pub fn cards(area: (f64, f64), header: f64, gis: bool, overview: bool) -> Cards {
    let (w, h) = area;
    let lens = (LENS.0 + 2.0, header + LENS.1 + 2.0);
    let right = card(
        w - MARGIN - lens.0,
        MARGIN + if gis { NORTH } else { 0.0 },
        LENS,
        header,
    );
    let left = if overview {
        let below = MARGIN + header + OVERVIEW.1 + 2.0 + GAP;
        if below + lens.1 <= h - MARGIN {
            card(MARGIN, below, LENS, header)
        } else {
            card(MARGIN + OVERVIEW.0 + 2.0 + GAP, MARGIN, LENS, header)
        }
    } else {
        card(MARGIN, MARGIN, LENS, header)
    };
    Cards {
        overview: overview.then(|| card(MARGIN, MARGIN, OVERVIEW, header)),
        right,
        left,
    }
}

fn near(r: &Rect, p: (f64, f64)) -> bool {
    r[0] - NEAR <= p.0
        && p.0 <= r[0] + r[2] + NEAR
        && r[1] - NEAR <= p.1
        && p.1 <= r[1] + r[3] + NEAR
}

/// The magnifier's side once the pointer is at `pointer`: the other side when
/// the pointer nears its card, unless it is as near the other side's.
pub fn next_side(cards: &Cards, side: Side, pointer: (f64, f64)) -> Side {
    let other = side.other();
    if near(&cards.lens(side).card, pointer) && !near(&cards.lens(other).card, pointer) {
        other
    } else {
        side
    }
}

/// The extent in the overview's picture: its centre on the picture's and `k`
/// pixels per metre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub cx: f64,
    pub cy: f64,
    pub k: f64,
}

/// The extent fitted in a picture of `size` logical pixels, `PAD` inside its
/// edges; an extent under 1 m wide or high counts as 1 m about its centre.
pub fn fit(extent: &Bounds, size: (f64, f64)) -> Fit {
    let cx = (extent.min_x + extent.max_x) / 2.0;
    let cy = (extent.min_y + extent.max_y) / 2.0;
    let w = js_max(extent.max_x - extent.min_x, 1.0);
    let h = js_max(extent.max_y - extent.min_y, 1.0);
    Fit {
        cx,
        cy,
        k: js_min((size.0 - 2.0 * PAD) / w, (size.1 - 2.0 * PAD) / h),
    }
}

impl Fit {
    /// A drawing point in the picture of `size`, logical pixels from its top left.
    pub fn to_card(&self, size: (f64, f64), x: f64, y: f64) -> (f64, f64) {
        (
            size.0 / 2.0 + (x - self.cx) * self.k,
            size.1 / 2.0 - (y - self.cy) * self.k,
        )
    }

    /// A point of the picture in the drawing.
    pub fn to_world(&self, size: (f64, f64), u: f64, v: f64) -> (f64, f64) {
        (
            self.cx + (u - size.0 / 2.0) / self.k,
            self.cy - (v - size.1 / 2.0) / self.k,
        )
    }

    /// The view's frame in the picture (u0, v0, u1, v1) for a view centred on
    /// `center` at `metres_per_pixel` over `view_px`, and whether it is small
    /// enough to be drawn as a cross (under 6 pixels both ways).
    pub fn view_frame(
        &self,
        size: (f64, f64),
        center: (f64, f64),
        metres_per_pixel: f64,
        view_px: (f64, f64),
    ) -> ([f64; 4], bool) {
        let hw = view_px.0 * metres_per_pixel / 2.0;
        let hh = view_px.1 * metres_per_pixel / 2.0;
        let (u0, v0) = self.to_card(size, center.0 - hw, center.1 + hh);
        let (u1, v1) = self.to_card(size, center.0 + hw, center.1 - hh);
        ([u0, v0, u1, v1], u1 - u0 < 6.0 && v1 - v0 < 6.0)
    }
}
