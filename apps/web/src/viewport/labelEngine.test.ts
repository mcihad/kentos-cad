import { describe, expect, it } from 'vitest';
import { CoreStore } from '../wasm/core';
import { LABEL, LABEL_STRIDE } from './storeRecords';

/**
 * The label engine (docs/adr/0212) through the WASM core: each case of fixtures/labels/v1/cases.json (written by
 * scripts/fixtures/label_engine_cases.py from the ADR, no KentOS code) placed by the geometry store as the desktop's
 * crates/shared/geometry-core/tests/all/label_engine.rs places it: the same labels in the same order, positions and
 * sizes within 1e-6.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Num = number;
interface Expected {
  id: Num;
  class: Num;
  state: Num;
  at: [Num, Num];
  angle: Num;
  w: Num;
  h: Num;
  lines?: [Num, Num, Num, Num, string, Num][];
  text?: string;
  letters?: Num[][];
  callout?: [[Num, Num], [Num, Num]];
}
interface Case {
  name: string;
  window: [Num, Num, Num, Num];
  scale: Num;
  layers: unknown[];
  defaults?: unknown;
  objects: unknown[];
  texts: { id: Num; texts: [Num, string][]; z?: Num }[];
  pins?: unknown[];
  fixed?: Num[][][];
  options?: { unplaced?: boolean; hidden?: boolean };
  expect: Expected[];
}

const close = (a: number, b: number) => Math.abs(a - b) <= 1e-6;

/** The records as the cases write them: each frame with its lines, letters and callout. */
function labels(records: Float64Array, texts: readonly string[]): Expected[] {
  const out: Expected[] = [];
  for (let i = 0; i + LABEL_STRIDE <= records.length; i += LABEL_STRIDE) {
    const r = records.subarray(i, i + LABEL_STRIDE);
    if (r[1] === LABEL.placed) {
      out.push({ id: r[0], class: r[7], state: r[8], at: [r[2], r[3]], angle: r[4], w: r[5], h: r[6] });
      continue;
    }
    const l = out[out.length - 1];
    if (r[1] === LABEL.placedLine) (l.lines ??= []).push([r[2], r[3], r[4], r[5], texts[r[6]], r[7]]);
    else if (r[1] === LABEL.placedLetter) {
      l.text = texts[r[6]];
      (l.letters ??= []).push([r[2], r[3], r[4], r[5], r[7], r[8]]);
    } else if (r[1] === LABEL.placedCallout) l.callout = [[r[2], r[3]], [r[4], r[5]]];
  }
  return out;
}

function differ(got: Expected, want: Expected, at: string): string | null {
  if (got.id !== want.id || got.class !== want.class || got.state !== want.state) return `${at}: ${got.id} ${got.class} ${got.state} ≠ ${want.id} ${want.class} ${want.state}`;
  const nums: [string, number, number][] = [
    ['x', got.at[0], want.at[0]],
    ['y', got.at[1], want.at[1]],
    ['açı', got.angle, want.angle],
    ['genişlik', got.w, want.w],
    ['yükseklik', got.h, want.h],
  ];
  for (const [name, a, b] of nums) if (!close(a, b)) return `${at} ${name}: ${a} ≠ ${b}`;
  const gl = got.lines ?? [];
  const wl = want.lines ?? [];
  if (gl.length !== wl.length) return `${at}: ${gl.length} satır ≠ ${wl.length}`;
  for (const [k, l] of gl.entries()) {
    const e = wl[k];
    if (l[4] !== e[4]) return `${at} satır ${k}: “${l[4]}” ≠ “${e[4]}”`;
    for (const j of [0, 1, 2, 3, 5]) if (!close(l[j] as number, e[j] as number)) return `${at} satır ${k}[${j}]: ${l[j]} ≠ ${e[j]}`;
  }
  if ((got.text ?? null) !== (want.text ?? null)) return `${at}: kıvrık metin “${got.text}” ≠ “${want.text}”`;
  const gs = got.letters ?? [];
  const ws = want.letters ?? [];
  if (gs.length !== ws.length) return `${at}: ${gs.length} harf ≠ ${ws.length}`;
  for (const [k, l] of gs.entries()) for (const j of [0, 1, 2, 3, 4, 5]) if (!close(l[j], ws[k][j])) return `${at} harf ${k}[${j}]: ${l[j]} ≠ ${ws[k][j]}`;
  if (!!got.callout !== !!want.callout) return `${at}: çağrı çizgisi ${!!got.callout} ≠ ${!!want.callout}`;
  if (got.callout && want.callout)
    for (const j of [0, 1]) for (const c of [0, 1]) if (!close(got.callout[j][c], want.callout[j][c])) return `${at} çağrı çizgisi: ${got.callout} ≠ ${want.callout}`;
  return null;
}

describe('the label engine', () => {
  it('places every shared case as the reference places it', () => {
    const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/labels/v1/cases.json', import.meta.url), 'utf8')) as { format: string; cases: Case[] };
    expect(file.format).toBe('kentos.labels-cases');
    expect(file.cases.length).toBeGreaterThanOrEqual(40);
    const off: string[] = [];
    for (const c of file.cases) {
      const s = new CoreStore();
      s.put(JSON.stringify(c.objects));
      s.setLabelLayers(JSON.stringify(c.layers));
      if (c.defaults) s.setLabelDefaults(JSON.stringify(c.defaults));
      const all = c.texts.flatMap((t) => t.texts);
      const from = Uint32Array.from([0, ...c.texts.map((_, i) => c.texts.slice(0, i + 1).reduce((n, t) => n + t.texts.length, 0))]);
      s.setObjectLabels(
        Float64Array.from(c.texts.map((t) => t.id)),
        from,
        Uint16Array.from(all.map(([cls]) => cls)),
        all.map(([, t]) => t).join(''),
        Uint32Array.from(all.map(([, t]) => t.length)),
        Float64Array.from(c.texts.map((t) => t.z ?? NaN)),
      );
      if (c.pins) s.setLabelPins(JSON.stringify(c.pins));
      const flags = (c.options?.unplaced ? 1 : 0) | (c.options?.hidden ? 2 : 0);
      const records = s.placeLabels(...c.window, c.scale, JSON.stringify(c.fixed ?? []), flags);
      const got = labels(records, s.placedTextsOf());
      if (got.length !== c.expect.length) {
        off.push(`${c.name}: ${got.length} etiket ≠ ${c.expect.length}`);
        continue;
      }
      for (const [k, g] of got.entries()) {
        const d = differ(g, c.expect[k], `${c.name} #${k}`);
        if (d) off.push(d);
      }
    }
    expect(off).toEqual([]);
  });
});
