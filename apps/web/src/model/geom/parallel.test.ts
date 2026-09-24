import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../geometry';
import { cleanAxis, corridorArea, parallelSides } from './parallel';
import { netArea } from './region';

const v = (x: number, y: number): Vec2 => ({ x, y });
const close = (a: Vec2, b: Vec2) => {
  expect(a.x).toBeCloseTo(b.x, 9);
  expect(a.y).toBeCloseTo(b.y, 9);
};

describe('paralel çizgi', () => {
  it('a straight axis gets sides at the left and right distances', () => {
    const s = parallelSides([v(0, 0), v(10, 0)], 3, 5, false);
    close(s.left![0], v(0, 3));
    close(s.left![1], v(10, 3));
    close(s.right![0], v(0, -5));
    close(s.right![1], v(10, -5));
  });
  it('corners are mitred on both sides', () => {
    // East then north: the left side turns inside the corner, the right side outside.
    const s = parallelSides([v(0, 0), v(10, 0), v(10, 10)], 2, 2, false);
    close(s.left![1], v(8, 2));
    close(s.right![1], v(12, -2));
  });
  it('a side at 0 is the axis itself', () => {
    const s = parallelSides([v(0, 0), v(10, 0)], 0, 4, false);
    expect(s.left).toBeNull();
    expect(s.right).toHaveLength(2);
  });
  it('the corridor of an open axis is one area of length × width', () => {
    const a = corridorArea([v(0, 0), v(10, 0), v(10, 10)], 2, 3, false)!;
    // Outer rectangle 13×13 less the inner corner 8×8: 20 m × 5 m plus (3² − 2²) at the mitred turn.
    expect(netArea(a)).toBeCloseTo(13 * 13 - 8 * 8, 9);
    expect(a.holes).toHaveLength(0);
  });
  it('a closed axis gives a ring-shaped area', () => {
    const sq = [v(0, 0), v(10, 0), v(10, 10), v(0, 10)];
    const a = corridorArea(sq, 1, 1, true)!;
    expect(a.holes).toHaveLength(1);
    expect(netArea(a)).toBeCloseTo(12 * 12 - 8 * 8, 9);
  });
  it('repeated clicks do not make zero-length legs', () => {
    expect(cleanAxis([v(0, 0), v(0, 0), v(5, 0), v(5, 0)], false)).toHaveLength(2);
    expect(corridorArea([v(0, 0), v(0, 0)], 1, 1, false)).toBeNull();
  });
});
