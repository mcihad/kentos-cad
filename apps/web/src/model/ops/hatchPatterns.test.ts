import { describe, expect, it } from 'vitest';
import type { Entity, HatchPattern } from '../entities';
import type { Vec2 } from '../geometry';
import { hatchChoiceNamed, hatchChoices, hatchColour, hatchLibrary, hatchPaints, hatchPatternPieces, hatchRegion } from './hatchPatterns';

/**
 * Hatch patterns (docs/adr/0186) through WASM against the shared cases (fixtures/hatch/v1/cases.json, written by
 * scripts/fixtures/hatch_pattern_cases.py from the ADR, not KentOS code); the core runs them natively
 * (crates/shared/geometry-core/tests/all/hatch.rs): the library, a pattern's paints, its lines and dots cut to a
 * region and a hatch's region. Numbers within 1e-9 (relative for the larger).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type XY = [number, number];
interface File {
  library: unknown[];
  paints: { note: string; pattern: HatchPattern; expect: unknown }[];
  pieces: { note: string; ring: XY[]; holes: XY[][]; pattern: HatchPattern; budget: number; expect: unknown }[];
  regions: { note: string; outer: Entity; islands: Entity[]; cutouts: XY[][]; seed: Vec2; expect: { area: number; holes: number; islands: number[]; cutouts: number[] } | null }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/hatch/v1/cases.json', import.meta.url), 'utf8')) as File;

/** `got` agrees with `want`: the same fields and words, numbers within 1e-9 (relative above 1); a null dash none. */
function close(got: unknown, want: unknown, at: string): void {
  if (typeof want === 'number') {
    expect(typeof got, at).toBe('number');
    const g = got as number;
    expect(Math.abs(g - want) <= 1e-9 * Math.max(Math.abs(g), Math.abs(want), 1), `${at}: ${g} ≠ ${want}`).toBe(true);
    return;
  }
  if (Array.isArray(want)) {
    expect(Array.isArray(got) && got.length === want.length, `${at}: ${JSON.stringify(got)} ≠ ${JSON.stringify(want)}`).toBe(true);
    want.forEach((w, i) => close((got as unknown[])[i], w, `${at}[${i}]`));
    return;
  }
  if (want && typeof want === 'object') {
    const g = Object.fromEntries(Object.entries(got as Record<string, unknown>).filter(([, v]) => v !== null && v !== undefined));
    expect(Object.keys(g).sort(), at).toEqual(Object.keys(want).sort());
    for (const [k, w] of Object.entries(want)) close(g[k], w, `${at}.${k}`);
    return;
  }
  expect(got, at).toEqual(want);
}

const xy = ([x, y]: XY): Vec2 => ({ x, y });
const signed = (ring: readonly Vec2[]) => ring.reduce((s, a, i) => s + a.x * ring[(i + 1) % ring.length].y - ring[(i + 1) % ring.length].x * a.y, 0) / 2;

describe('hatch patterns through WASM (fixtures/hatch/v1)', () => {
  it('the library is the ADR’s', () => {
    close(hatchLibrary(), file.library, 'library');
  });

  it('a pattern’s families are its paints', () => {
    for (const c of file.paints) close(hatchPaints(c.pattern), c.expect, c.note);
  });

  it('a pattern’s lines and dots are cut to the region', () => {
    for (const c of file.pieces) {
      const got = hatchPatternPieces(c.ring.map(xy), c.holes.map((h) => h.map(xy)), c.pattern, c.budget);
      const pt = (p: Vec2): XY => [p.x, p.y];
      close({ segments: got.segments.map(([a, b]) => [pt(a), pt(b)]), dots: got.dots.map(pt), capped: got.capped }, c.expect, c.note);
    }
  });

  it('a hatch’s region is its outer less what reaches in', () => {
    for (const c of file.regions) {
      const got = hatchRegion(c.outer, c.islands, c.cutouts.map((r) => r.map(xy)), c.seed);
      if (c.expect === null) {
        expect(got, c.note).toBeNull();
        continue;
      }
      expect(got, c.note).not.toBeNull();
      const area = Math.abs(signed(got!.ring)) - got!.holes.reduce((s, h) => s + Math.abs(signed(h)), 0);
      expect(Math.abs(area - c.expect.area) < 1e-6, `${c.note}: ${area}`).toBe(true);
      expect([got!.holes.length, got!.islands, got!.cutouts], c.note).toEqual([c.expect.holes, c.expect.islands, c.expect.cutouts]);
    }
  });

  it('Desen’s choices, typed names and colours are the core’s', () => {
    const list = hatchChoices();
    expect(list[3].name).toBe('ANSI31');
    expect(hatchChoiceNamed('ansı31')).toBe(3);
    // A gradient by its shape's word: the space would be the command line's Enter.
    expect(hatchChoiceNamed('küre')).toBe(list.length - 1);
    expect(hatchChoiceNamed('degrade')).toBeNull();
    expect(hatchColour('Kırmızı')).toBe('#E5484D');
    expect(hatchColour('#7fb2e5')).toBe('#7FB2E5');
    expect(hatchColour('mor')).toBeNull();
  });
});
