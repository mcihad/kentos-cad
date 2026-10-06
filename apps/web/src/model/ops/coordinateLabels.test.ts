import { describe, expect, it } from 'vitest';
import { coordinateLabels, coordinatePlaces, type LabelOptions, type LabelPlace, type LabelUnits } from './coordinateLabels';
import type { ListedObject } from './table';

/**
 * Koordinat yaz (docs/adr/0185) through WASM against the shared cases (fixtures/coordinate-labels/v1/cases.json,
 * written by scripts/fixtures/coordinate_label_cases.py from the ADR, not KentOS code); the core runs them natively
 * (crates/shared/geometry-core/src/ops/coordinate_labels.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  places: { name: string; objects: ListedObject[]; want: unknown }[];
  labels: { name: string; places: LabelPlace[]; options: LabelOptions; units: LabelUnits; want: unknown }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/coordinate-labels/v1/cases.json', import.meta.url), 'utf8')) as File;

/** `got` is `want`: the same fields, the same words, numbers within 0.1 µm (the cases are exact, f64 a few ulps off). */
function same(got: unknown, want: unknown, at: string): void {
  if (typeof want === 'number') {
    expect(typeof got, at).toBe('number');
    expect(Math.abs((got as number) - want) <= 1e-7, `${at}: ${String(got)} ≠ ${want}`).toBe(true);
    return;
  }
  if (Array.isArray(want)) {
    expect(Array.isArray(got) && got.length === want.length, `${at}: ${JSON.stringify(got)} ≠ ${JSON.stringify(want)}`).toBe(true);
    want.forEach((w, i) => same((got as unknown[])[i], w, `${at}[${i}]`));
    return;
  }
  if (want && typeof want === 'object') {
    const g = got as Record<string, unknown>;
    expect(Object.keys(g).sort(), at).toEqual(Object.keys(want).sort());
    for (const [k, w] of Object.entries(want)) same(g[k], w, `${at}.${k}`);
    return;
  }
  expect(got, at).toEqual(want);
}

describe('Koordinat yaz (docs/adr/0185)', () => {
  it('finds and names the places as the shared cases say', () => {
    expect(file.format).toBe('kentos.coordinate-label-cases');
    for (const c of file.places) same(coordinatePlaces(c.objects), c.want, c.name);
  });

  it('writes and lays out the labels as the shared cases say', () => {
    for (const c of file.labels) same(coordinateLabels(c.places, c.options, c.units), c.want, c.name);
  });
});
