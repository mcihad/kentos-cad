/**
 * Where a card that follows the pointer goes (the information card, the
 * value field beside the cursor): it never leaves the area it is drawn in
 * (DESIGN.md §7.4.2). Pure, so the rule is tested here and the desktop can
 * keep the same one.
 */

export interface Point {
  readonly x: number;
  readonly y: number;
}

/** A width and a height, CSS pixels. */
export interface Size {
  readonly w: number;
  readonly h: number;
}

/** Space kept between a card and the edge of its area, CSS pixels. */
export const EDGE_MARGIN = 8;

/** `v` held between `lo` and `hi`; `lo` when the two cross (a card larger than its area keeps its top-left corner in). */
const hold = (v: number, lo: number, hi: number): number => Math.max(lo, Math.min(v, hi));

/**
 * The top-left corner of a `card` beside the pointer `at`, inside an `area`
 * whose top-left is (0, 0). `offset` is the card's corner from the pointer
 * where it prefers to be (right and below: positive). A card that would
 * cross the right edge goes to the left of the pointer, the same distance
 * away; one below the pointer that would cross the bottom edge goes above
 * it. Then the card is held `EDGE_MARGIN` inside the area on every side.
 */
export function besidePointer(at: Point, card: Size, area: Size, offset: Point = { x: 18, y: 20 }): Point {
  let x = at.x + offset.x;
  if (x + card.w > area.w - EDGE_MARGIN) x = at.x - offset.x - card.w;
  let y = at.y + offset.y;
  if (offset.y >= 0 && y + card.h > area.h - EDGE_MARGIN) y = at.y - offset.y - card.h;
  return {
    x: Math.round(hold(x, EDGE_MARGIN, area.w - EDGE_MARGIN - card.w)),
    y: Math.round(hold(y, EDGE_MARGIN, area.h - EDGE_MARGIN - card.h)),
  };
}
