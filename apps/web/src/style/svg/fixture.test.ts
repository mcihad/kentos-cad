import { describe, expect, it } from 'vitest';
import { writeArgs } from '../../wasm/core';
import { SnapIndex, callOp, crc32, inkMask, inkMaskNumbers, opId, traceBitmap, traceBitmapNumbers, traceContours, withPngDpi } from './pkg/kentos_svg_wasm.js';

/**
 * The frozen answers of the SVG editor's core (fixtures/svg/v1/cases.json,
 * scripts/fixtures/record-svg.test.ts) through the WASM package the editor
 * loads: every operation of the table from the page's arguments, key for
 * key and number for number, and the typed entries (the snap index, the
 * picture's bytes, the PNG's). The Rust core checks the same file natively
 * (crates/shared/svg-core/tests/cases.rs).
 */

type Case = Record<string, unknown> & { fn: string };

const [text] = Object.values(import.meta.glob<string>('../../../../../fixtures/svg/v1/cases.json', { query: '?raw', import: 'default', eager: true }));
const file = JSON.parse(text) as { format: string; version: number; cases: Case[] };

/** The core's answer to a table operation as its JSON text reads, or the message it refused the call with. */
function run(c: Case): string {
  try {
    return JSON.stringify({ expect: JSON.parse(callOp(opId(c.fn), writeArgs(c.args as unknown[]))) as unknown });
  } catch (e) {
    return JSON.stringify({ throws: e instanceof Error ? e.message : String(e) });
  }
}

const same = (got: unknown, want: unknown) => JSON.stringify(got) === JSON.stringify(want);
const data = (c: Case) => c.data as number[];

/** A typed entry's answer against the file's; the case's name when they differ. */
const TYPED: Record<string, (c: Case) => boolean> = {
  SnapIndex: (c) => {
    const index = SnapIndex.of(writeArgs(c.source));
    const got = (c.queries as [number, number, number, boolean, number, number][]).map(([x, y, r, has, fx, fy]) => JSON.parse(index.query(x, y, r, has, fx, fy)) as unknown);
    index.free();
    return same(got, c.expect);
  },
  inkMask: (c) => {
    const [w, h, t, inv] = [c.width as number, c.height as number, c.threshold as number, !!c.invert];
    const mask = c.numbers ? inkMaskNumbers(w, h, Float64Array.from(data(c)), t, inv) : inkMask(w, h, Uint8Array.from(data(c)), t, inv);
    return same([...mask], c.expect);
  },
  traceContours: (c) => same(JSON.parse(traceContours(Uint8Array.from(c.mask as number[]), c.width as number, c.height as number)), c.expect),
  traceBitmap: (c) => {
    const [w, h, o] = [c.width as number, c.height as number, writeArgs(c.options)];
    return same(JSON.parse(c.numbers ? traceBitmapNumbers(w, h, Float64Array.from(data(c)), o) : traceBitmap(w, h, Uint8Array.from(data(c)), o)), c.expect);
  },
  crc32: (c) => crc32(Uint8Array.from(c.bytes as number[])) === c.expect,
  withPngDpi: (c) => {
    const out = withPngDpi(Uint8Array.from(c.bytes as number[]), c.dpi as number);
    return same(out ? [...out] : null, c.expect);
  },
};

describe('SVG core: frozen answers through WASM', () => {
  it('is the SVG core’s case file', () => {
    expect(file.format).toBe('kentos.svg-cases');
    expect(file.version).toBe(1);
    expect(file.cases.length).toBeGreaterThan(1000);
  });

  it('every table operation answers as frozen', () => {
    const table = file.cases.filter((c) => !(c.fn in TYPED));
    const failures = table
      .map((c, i) => [c, i] as const)
      .filter(([c]) => run(c) !== JSON.stringify('throws' in c ? { throws: c.throws } : { expect: c.expect }))
      .map(([c, i]) => `${i}. ${c.fn}: ${run(c).slice(0, 300)}`);
    expect(failures.slice(0, 5).join('\n')).toBe('');
    expect(new Set(table.map((c) => c.fn)).size).toBeGreaterThan(90);
  });

  it('the snap index, the pictures and the PNG bytes answer as frozen', () => {
    const typed = file.cases.filter((c) => c.fn in TYPED);
    const failures = typed.filter((c) => !TYPED[c.fn](c)).map((c) => c.fn);
    expect(failures).toEqual([]);
    expect(new Set(typed.map((c) => c.fn))).toEqual(new Set(Object.keys(TYPED)));
  });
});
