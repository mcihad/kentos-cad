import { describe, expect, it } from 'vitest';
import { alongTrack, trackAngles, trackPoint } from './objectTracking';

const v = (x: number, y: number) => ({ x, y });
const ORTHO = trackAngles(null);

describe('trackPoint', () => {
  it('locks onto the vertical through an acquired point', () => {
    const hit = trackPoint(v(100.3, 250), [v(100, 0)], null, ORTHO, 1)!;
    expect(hit.point).toEqual(v(100, 250));
    expect(hit.lines).toEqual([{ origin: v(100, 0), angle: 90 }]);
  });
  it('ignores the cursor when it is off every line', () => {
    expect(trackPoint(v(105, 250), [v(100, 0)], null, ORTHO, 1)).toBeNull();
  });
  it('prefers the crossing of two alignments', () => {
    // Above A (x = 10) and level with B (y = 50).
    const hit = trackPoint(v(10.4, 49.7), [v(10, 0), v(80, 50)], null, ORTHO, 1)!;
    expect(hit.point.x).toBeCloseTo(10, 12);
    expect(hit.point.y).toBeCloseTo(50, 12);
    expect(hit.lines).toHaveLength(2);
  });
  it('crosses an acquired alignment with a line from the last point', () => {
    // From the last point (0,0) straight up until level with A (y = 30).
    const hit = trackPoint(v(0.2, 29.6), [v(50, 30)], v(0, 0), ORTHO, 1)!;
    expect(hit.point.x).toBeCloseTo(0, 12);
    expect(hit.point.y).toBeCloseTo(30, 12);
  });
  it('never tracks from the last point alone', () => {
    expect(trackPoint(v(0.2, 29.6), [v(50, 90)], v(0, 0), ORTHO, 1)).toBeNull();
  });
  it('uses polar steps when polar tracking is on', () => {
    const hit = trackPoint(v(10.2, 9.9), [v(0, 0)], null, trackAngles(45), 1)!;
    expect(hit.lines[0].angle).toBe(45);
    expect(hit.point.x).toBeCloseTo(hit.point.y, 12);
  });
  it('places a typed distance along the tracking line', () => {
    const hit = trackPoint(v(100.3, 250), [v(100, 0)], null, ORTHO, 1)!;
    expect(alongTrack(hit, 12.5)).toEqual(v(100, 12.5));
  });
});
