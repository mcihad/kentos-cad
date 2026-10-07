import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/text/v1/along.json?raw';
import type { Entity, TextEntity } from '../model/entities';
import { textLines } from '../model/paragraph';
import { textAlongPiece, textAlongReadable, textAlongStraight, textAlongTurn } from '../model/textAlong';

/**
 * Eğri boyunca yazı (docs/adr/0196) through the WASM core, on the independent reference's cases
 * (fixtures/text/v1/along.json, scripts/fixtures/text_along_cases.py): a text's records letter by letter, the piece
 * of a curve a tool cuts, Okunur yap, Düzleştir and Doğrultuya döndür. The core runs the same file natively
 * (crates/shared/geometry-core/tests/all/text_along.rs). The reference works in the world, the core in the text's
 * frame: they meet within 1e-9 of the coordinates' size.
 */
const f = JSON.parse(text);
const near = (got: number, want: number, what: string) => expect(Math.abs(got - want) <= 1e-9 * Math.max(1, Math.abs(want)), `${what}: ${got} ≠ ${want}`).toBe(true);
/** Angles in degrees, equal round the circle. */
const nearTurn = (got: number, want: number, what: string) => expect(Math.abs((((got - want) % 360) + 540) % 360 - 180) <= 1e-9, `${what}: ${got} ≠ ${want}`).toBe(true);
/** The text as an object of the drawing, measured in its typeface or the case's drawing font. */
const asText = (t: Record<string, unknown>) => ({ kind: 'text', id: 1, layerId: 'yazi', attrs: {}, ...t, font: (t.font as string | undefined) ?? (t.drawingFont as string | undefined) ?? 'barlow' }) as unknown as TextEntity;

describe('Eğri boyunca yazı (fixtures/text/v1/along.json)', () => {
  it('gives each letter its record where the reference puts it', () => {
    for (const c of f.layout) {
      const got = textLines(asText(c.text));
      const want: number[] = c.want.records.flat();
      expect(got.length, c.name).toBe(want.length);
      got.forEach((g, i) => (i % 9 === 4 ? nearTurn(g, want[i], `${c.name} #${i}`) : near(g, want[i], `${c.name} #${i}`)));
    }
  });

  it('cuts the piece of a curve the reference cuts', () => {
    for (const c of f.piece) {
      const got = textAlongPiece(c.curve as Entity, c.click, c.length, c.share);
      expect(got, c.name).not.toBeNull();
      near(got!.p.x, c.want.p.x, `${c.name} p.x`);
      near(got!.p.y, c.want.p.y, `${c.name} p.y`);
      nearTurn(got!.rotation, c.want.rotation, `${c.name} rotation`);
      expect(got!.path.pts.length, c.name).toBe(c.want.path.pts.length);
      got!.path.pts.forEach((q, i) => {
        near(q.x, c.want.path.pts[i].x, `${c.name} vertex ${i} x`);
        near(q.y, c.want.path.pts[i].y, `${c.name} vertex ${i} y`);
      });
      expect(got!.path.bulges === undefined, c.name).toBe(c.want.path.bulges === undefined);
    }
  });

  it('turns an upside-down text the other way, straightens, and reads a direction', () => {
    for (const c of f.readable) {
      const got = textAlongReadable(asText(c.text));
      expect(got === null, c.name).toBe(c.want === null);
      if (!got) continue;
      near(got.p.x, c.want.p.x, `${c.name} p.x`);
      near(got.p.y, c.want.p.y, `${c.name} p.y`);
      nearTurn(got.rotation, c.want.rotation, `${c.name} rotation`);
      expect(got.align, c.name).toBe(c.want.align ?? null);
    }
    for (const c of f.straight) {
      const got = textAlongStraight(asText(c.text))!;
      near(got.p.x, c.want.p.x, `${c.name} p.x`);
      near(got.p.y, c.want.p.y, `${c.name} p.y`);
      nearTurn(got.rotation, c.want.rotation, `${c.name} rotation`);
    }
    for (const c of f.turn) nearTurn(textAlongTurn(c.direction), c.want, c.name);
  });
});
