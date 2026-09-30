import { describe, expect, it } from 'vitest';
import increment from '../../../../fixtures/text/v1/increment.json?raw';
import pattern from '../../../../fixtures/text/v1/pattern.json?raw';
import readable from '../../../../fixtures/text/v1/readable.json?raw';
import realign from '../../../../fixtures/text/v1/realign.json?raw';
import type { TextAlign } from './entities';
import type { Vec2 } from './geometry';
import { textIncrement, textReadable, textRealign, textReplace } from './textEdit';

/**
 * A text's editing rules (docs/adr/0145 §3, §6) through WASM against the shared
 * cases (fixtures/text/v1, written from the rules by
 * scripts/fixtures/text_cases.py); natively crates/shared/geometry-core/tests/text.rs.
 */

interface File<C> {
  format: string;
  version: number;
  cases: C[];
}

const read = <C>(text: string): File<C> => JSON.parse(text) as File<C>;

describe('text editing rules (fixtures/text/v1)', () => {
  it('Artır gives the next number', () => {
    const file = read<{ name: string; text: string; next: string | null }>(increment);
    expect([file.format, file.version]).toEqual(['kentos.text-cases', 1]);
    for (const c of file.cases) expect(textIncrement(c.text), c.name).toBe(c.next);
  });

  it('Bul ve değiştir matches as the rules say, many texts in one call', () => {
    const file = read<{ name: string; text: string; find: string; replace: string; wildcard: boolean; caseless: boolean; wholeWord: boolean; expect: string | null }>(pattern);
    for (const c of file.cases) expect(textReplace([c.text], c.find, c.replace, c), c.name).toEqual([c.expect]);
    // Several texts in one call, each answered in its place.
    expect(textReplace(['Ada 1', 'Pafta', 'Ada 2'], 'Ada *', 'Parsel *', { wildcard: true, caseless: false, wholeWord: false })).toEqual(['Parsel 1', null, 'Parsel 2']);
  });

  it('Okunur yap turns what reads upside down about its box', () => {
    const file = read<{ name: string; p: Vec2; height: number; rotation: number; align?: TextAlign; width: number; expect: { p: Vec2; rotation: number } | null }>(readable);
    for (const c of file.cases) {
      const got = textReadable(c);
      if (!c.expect) {
        expect(got, c.name).toBeNull();
        continue;
      }
      // The point to a nanometre (float64 at 10⁶ m is a tenth of that); the turn exactly.
      expect(Math.abs(got!.p.x - c.expect.p.x), c.name).toBeLessThanOrEqual(1e-9);
      expect(Math.abs(got!.p.y - c.expect.p.y), c.name).toBeLessThanOrEqual(1e-9);
      expect(got!.rotation, c.name).toBe(c.expect.rotation);
    }
  });

  it('Hizayı değiştir keeps the text where it is', () => {
    const file = read<{ name: string; p: Vec2; height: number; rotation: number; align?: TextAlign; to: TextAlign | null; width: number; expect: Vec2 }>(realign);
    expect(file.cases.length).toBeGreaterThanOrEqual(5);
    for (const c of file.cases) {
      const got = textRealign(c, c.to);
      expect(Math.abs(got.x - c.expect.x), c.name).toBeLessThanOrEqual(1e-9);
      expect(Math.abs(got.y - c.expect.y), c.name).toBeLessThanOrEqual(1e-9);
    }
  });
});
