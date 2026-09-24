import { describe, expect, it } from 'vitest';
import { batchLegible, MIN_TEXT_PX, type MarkerBatch } from './types';

const textBatch = (height: number, unit: 'world' | 'px'): MarkerBatch => ({
  kind: 'marker',
  instances: new Float32Array(0),
  unit,
  look: { kind: 'image', image: { key: 't', kind: 'text', text: 'SEG', font: 'sans-serif', weight: 400, italic: false, color: '#000', halo: null }, fit: 'height' },
  offset: [0, 0],
  anchor: [0, 0],
  opacity: 1,
  extent: [0, height],
  bounds: [0, 0, 0, 0],
  reach: 0,
  reachUnit: unit,
});

describe('far-view legibility', () => {
  it('leaves out text too small to read and keeps everything else', () => {
    // 2.5 m tall text: readable at 2 device px per metre (5 px), not at 1 (2.5 px).
    expect(batchLegible(textBatch(2.5, 'world'), 2, 1)).toBe(true);
    expect(batchLegible(textBatch(2.5, 'world'), 1, 1)).toBe(false);
    // On a 2× screen the threshold is in CSS px.
    expect(batchLegible(textBatch(2.5, 'world'), 2 * MIN_TEXT_PX, 2)).toBe(true);
    // Screen-sized text keeps its size at any zoom.
    expect(batchLegible(textBatch(10, 'px'), 1e-6, 1)).toBe(true);
    const shape: MarkerBatch = { ...textBatch(0.1, 'world'), look: { kind: 'shape', shape: 'circle', fill: [0, 0, 0, 1], stroke: null, strokeWidth: 0, params: [0, 12, Math.PI, 0.2] } };
    expect(batchLegible(shape, 1e-3, 1)).toBe(true);
  });
});
