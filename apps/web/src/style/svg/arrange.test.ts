import { describe, expect, it } from 'vitest';
import { alignMoves, anchorPoint, copiesOf, distributeMoves, mirrorMatrix, polarArray, rectArray, restack, skewAbout, transformMoves, unitsOf, type Unit } from './arrange';
import { apply, type Box } from './pathData';
import { shapeBox, type SvgShape } from './svgModel';

const base = { fill: 'fill', stroke: 'none', strokeWidth: 1 } as const;
const rect = (id: string, x: number, y: number, w: number, h: number, group?: string): SvgShape => ({ ...base, id, kind: 'rect', x, y, w, h, group });
const page = { width: 100, height: 100 };
const moved = (u: Unit, m: readonly number[]) => apply(m as [number, number, number, number, number, number], u.box.minX, u.box.minY);
const close = (p: readonly number[], q: readonly number[]) => p.every((v, i) => Math.abs(v - q[i]) < 1e-9);

describe('units', () => {
  it('keeps the choice order and treats a group as one', () => {
    const shapes = [rect('a', 0, 0, 10, 10, 'g'), rect('b', 20, 0, 10, 10, 'g'), rect('c', 50, 50, 5, 5)];
    const units = unitsOf(shapes, ['c', 'a', 'b']);
    expect(units.map((u) => u.ids)).toEqual([['c'], ['a', 'b']]);
    expect(units[1].box).toEqual({ minX: 0, minY: 0, maxX: 30, maxY: 10 });
  });
});

describe('align and distribute', () => {
  const units = unitsOf([rect('a', 0, 0, 10, 10), rect('b', 20, 30, 20, 20), rect('c', 60, 10, 5, 5)], ['a', 'b', 'c']);

  it('aligns left edges to the selection, the first, the last, the biggest and the canvas', () => {
    expect(alignMoves(units, 'left', 'selection', page).map((m, i) => moved(units[i], m)[0])).toEqual([0, 0, 0]);
    expect(alignMoves(units, 'right', 'first', page).map((m, i) => apply(m, units[i].box.maxX, 0)[0])).toEqual([10, 10, 10]);
    expect(alignMoves(units, 'top', 'last', page).map((m, i) => moved(units[i], m)[1])).toEqual([10, 10, 10]);
    expect(alignMoves(units, 'bottom', 'biggest', page).map((m, i) => apply(m, 0, units[i].box.maxY)[1])).toEqual([50, 50, 50]);
    expect(alignMoves(units, 'hcenter', 'canvas', page).map((m, i) => apply(m, (units[i].box.minX + units[i].box.maxX) / 2, 0)[0])).toEqual([50, 50, 50]);
    // One shape alone goes to the canvas; as one block the spacing stays.
    expect(moved(units[0], alignMoves([units[0]], 'right', 'selection', page)[0])[0]).toBe(90);
    const block = alignMoves(units, 'left', 'canvas', page, true);
    expect(block.every((m) => m[4] === 0)).toBe(true);
  });

  it('spaces centres evenly and makes gaps equal', () => {
    const c = distributeMoves(units, 'hcenter');
    const centres = units.map((u, i) => apply(c[i], (u.box.minX + u.box.maxX) / 2, 0)[0]);
    expect(centres).toEqual([5, 33.75, 62.5]);
    const g = distributeMoves(units, 'hgap');
    const boxes = units.map((u, i) => [apply(g[i], u.box.minX, 0)[0], apply(g[i], u.box.maxX, 0)[0]]);
    expect(boxes[1][0] - boxes[0][1]).toBeCloseTo(boxes[2][0] - boxes[1][1], 9);
    expect(boxes[0][0]).toBe(0);
    expect(boxes[2][1]).toBe(65);
    // Two units: nothing to spread.
    expect(distributeMoves(units.slice(0, 2), 'hcenter').every((m) => m[4] === 0 && m[5] === 0)).toBe(true);
  });
});

describe('numeric transforms', () => {
  const units = unitsOf([rect('a', 0, 0, 10, 10), rect('b', 20, 0, 10, 20)], ['a', 'b']);

  it('moves relatively, absolutely, and step by step when separate', () => {
    expect(transformMoves(units, { kind: 'move', x: 5, y: 0, relative: true }, false).map((m) => m[4])).toEqual([5, 5]);
    expect(transformMoves(units, { kind: 'move', x: 5, y: 0, relative: true }, true).map((m) => m[4])).toEqual([5, 10]);
    const abs = transformMoves(units, { kind: 'move', x: 50, y: 50, relative: false }, false);
    expect(apply(abs[0], 0, 0)).toEqual([50, 50]);
  });

  it('scales about an anchor, together or each on its own', () => {
    const together = transformMoves(units, { kind: 'scale', sx: 200, sy: 200, anchor: 'tl' }, false);
    expect(apply(together[1], 20, 0)).toEqual([40, 0]);
    const own = transformMoves(units, { kind: 'scale', sx: 50, sy: 50, anchor: 'c' }, true);
    expect(apply(own[1], 25, 10)).toEqual([25, 10]);
  });

  it('rotates about the centre, a corner or a point, in either direction', () => {
    const [m] = transformMoves([units[0]], { kind: 'rotate', deg: 90, ccw: false, about: 'c' }, false);
    expect(close(apply(m, 10, 5), [5, 10])).toBe(true);
    const [p] = transformMoves([units[0]], { kind: 'rotate', deg: 90, ccw: true, about: [0, 0] }, false);
    expect(close(apply(p, 10, 0), [0, -10])).toBe(true);
  });

  it('skews about an anchor and applies a matrix per box when separate', () => {
    const s = skewAbout(45, 0, [0, 0]);
    expect(close(apply(s, 0, 10), [10, 10])).toBe(true);
    const flip = [-1, 0, 0, 1, 0, 0] as const;
    const own = transformMoves(units, { kind: 'matrix', m: flip }, true);
    // Each box flips about its own left edge.
    expect(close(apply(own[1], 30, 0), [10, 0])).toBe(true);
    expect(anchorPoint({ minX: 0, minY: 0, maxX: 10, maxY: 20 }, 'br')).toEqual([10, 20]);
  });
});

describe('arrays', () => {
  const box: Box = { minX: 0, minY: 0, maxX: 10, maxY: 10 };

  it('lays out rows and columns by step or by gap, without the original', () => {
    const step = rectArray(box, { rows: 2, cols: 3, dx: 15, dy: 20, mode: 'step' });
    expect(step).toHaveLength(5);
    expect(step.map((m) => [m[4], m[5]])).toContainEqual([30, 20]);
    const gap = rectArray(box, { rows: 1, cols: 3, dx: 2, dy: 0, mode: 'gap' });
    expect(gap.map((m) => m[4])).toEqual([12, 24]);
  });

  it('turns copies round a centre, spread over a full turn or an arc', () => {
    const wheel = polarArray(box, { count: 4, angle: 360, centre: [5, 25], rotate: true, ccw: false });
    expect(wheel).toHaveLength(3);
    // The second copy's centre: a quarter turn clockwise on screen.
    expect(close(apply(wheel[0], 5, 5), [25, 25])).toBe(true);
    const arc = polarArray(box, { count: 3, angle: 90, centre: [5, 25], rotate: false, ccw: true });
    // Only moved: the matrix is a translation; the last copy ends a quarter turn round.
    expect(arc[1].slice(0, 4)).toEqual([1, 0, 0, 1]);
    expect(close(apply(arc[1], 5, 5), [-15, 25])).toBe(true);
  });

  it('mirrors across vertical, horizontal and slanted lines', () => {
    expect(close(apply(mirrorMatrix('v', [10, 0]), 4, 3), [16, 3])).toBe(true);
    expect(close(apply(mirrorMatrix('h', [0, 10]), 4, 3), [4, 17])).toBe(true);
    expect(close(apply(mirrorMatrix('angle', [0, 0], 45), 4, 0), [0, 4])).toBe(true);
  });

  it('copies shapes with new ids, each copy with groups of its own', () => {
    const shapes = [rect('a', 0, 0, 10, 10, 'g'), rect('b', 20, 0, 10, 10, 'g')];
    const copies = copiesOf(shapes, rectArray(box, { rows: 1, cols: 3, dx: 40, dy: 0, mode: 'step' }));
    expect(copies).toHaveLength(4);
    expect(new Set(copies.map((c) => c.id)).size).toBe(4);
    expect(copies[0].group).toBe(copies[1].group);
    expect(copies[0].group).not.toBe(copies[2].group);
    expect(copies[0].group).not.toBe('g');
    expect(shapeBox(copies[2]).minX).toBe(80);
  });
});

describe('stacking order', () => {
  it('raises, lowers, and sends to top and bottom', () => {
    const list = ['a', 'b', 'c', 'd'].map((id) => rect(id, 0, 0, 1, 1));
    const ids = (l: SvgShape[]) => l.map((s) => s.id).join('');
    expect(ids(restack(list, new Set(['b']), 'raise'))).toBe('acbd');
    expect(ids(restack(list, new Set(['b']), 'lower'))).toBe('bacd');
    expect(ids(restack(list, new Set(['a', 'c']), 'top'))).toBe('bdac');
    expect(ids(restack(list, new Set(['d']), 'bottom'))).toBe('dabc');
    expect(ids(restack(list, new Set(['d']), 'raise'))).toBe('abcd');
  });
});
