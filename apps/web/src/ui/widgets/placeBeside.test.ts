import { describe, expect, it } from 'vitest';
import { besidePointer, EDGE_MARGIN } from './placeBeside';

// The rule the desktop keeps too (DESIGN.md §7.4.2): right of and below the pointer, the other side near an
// edge, never outside the area.
describe('besidePointer', () => {
  const area = { w: 800, h: 600 };
  const card = { w: 280, h: 120 };

  it('puts the card right of and below the pointer where it fits', () => {
    expect(besidePointer({ x: 100, y: 100 }, card, area)).toEqual({ x: 118, y: 120 });
  });

  it('goes left of the pointer near the right edge, the same distance away', () => {
    expect(besidePointer({ x: 700, y: 100 }, card, area)).toEqual({ x: 700 - 18 - 280, y: 120 });
  });

  it('goes above the pointer near the bottom edge', () => {
    expect(besidePointer({ x: 100, y: 560 }, card, area)).toEqual({ x: 118, y: 560 - 20 - 120 });
  });

  it('goes left and above in the bottom-right corner', () => {
    expect(besidePointer({ x: 790, y: 590 }, card, area)).toEqual({ x: 790 - 18 - 280, y: 590 - 20 - 120 });
  });

  it('is held inside the area when neither side has room', () => {
    // Left of the pointer it would start before the area: held at the margin.
    expect(besidePointer({ x: 290, y: 100 }, card, { w: 300, h: 600 })).toEqual({ x: EDGE_MARGIN, y: 120 });
    // Above the pointer it would start above the area: held at the margin.
    expect(besidePointer({ x: 100, y: 130 }, card, { w: 800, h: 200 })).toEqual({ x: 118, y: EDGE_MARGIN });
  });

  it('keeps its top-left corner in when it is larger than the area', () => {
    expect(besidePointer({ x: 50, y: 50 }, { w: 500, h: 400 }, { w: 300, h: 200 })).toEqual({ x: EDGE_MARGIN, y: EDGE_MARGIN });
  });

  it('never flips a card placed above the pointer; near the top it slides down', () => {
    // The value field beside the cursor: 58 px above it.
    const field = { w: 190, h: 50 };
    expect(besidePointer({ x: 100, y: 300 }, field, area, { x: 18, y: -58 })).toEqual({ x: 118, y: 242 });
    expect(besidePointer({ x: 100, y: 20 }, field, area, { x: 18, y: -58 })).toEqual({ x: 118, y: EDGE_MARGIN });
    expect(besidePointer({ x: 780, y: 300 }, field, area, { x: 18, y: -58 })).toEqual({ x: 780 - 18 - 190, y: 242 });
  });
});
