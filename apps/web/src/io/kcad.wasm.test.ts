import { describe, expect, it } from 'vitest';
import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import { ColumnsReader, packDrawing, unpackSnapshot, type DrawingColumns, type PackedDrawing } from './columns';
import { KcadError, decodeWith, difference, encodeWith, exactJson, sniffDrawing, type KcadProgress } from './kcad';
import { formatsBuilt, formatsModule } from './testFormats';

/**
 * `.kcad` v2 in the browser (docs/specs/kcad-v2.md): the shared Rust codec in
 * the formats WASM module, with what surrounds it in the worker (io/kcad.ts)
 * and the page's typed columns (io/columns.ts, docs/adr/0030), on the
 * byte-level fixtures the independent Python writer made (fixtures/kcad/v2,
 * expected.json written by hand). The native codec (crates/shared/kcad/tests)
 * and tools/kcad/kcad.py read the same files.
 */

interface Case {
  file: string;
  sniff: string;
  content?: string;
  entities?: number;
  error?: string;
  rewrite?: boolean;
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');
/** A fixture's bytes (a plain Uint8Array: Node gives a Buffer). */
const read = (name: string) => new Uint8Array(fs.readFileSync(new URL(`../../../../fixtures/kcad/v2/${name}`, import.meta.url)));
const text = (b: Uint8Array) => new TextDecoder().decode(b);
const cases = (JSON.parse(text(read('expected.json'))) as { files: Case[] }).files;
const drawing = (name: string) => JSON.parse(text(read(name))) as DocumentSnapshotV2;

/** A drawing in the contract's JSON form, packed as the page packs one. */
function pack(d: DocumentSnapshotV2): PackedDrawing {
  const { entities, uids, ...head } = structuredClone(d);
  return packDrawing(
    head,
    entities.map((e, i) => ({ ...e, uid: uids[i] })),
  ).drawing;
}

/** Whether two sets of columns are the same, every float bit for bit. */
function sameColumns(a: DrawingColumns, b: DrawingColumns): boolean {
  const bits = (f: Float64Array) => new Uint32Array(f.buffer, f.byteOffset, f.length * 2);
  const eq = (x: ArrayLike<number>, y: ArrayLike<number>) => x.length === y.length && Array.prototype.every.call(x, (v: number, i: number) => v === y[i]);
  return eq(a.kinds, b.kinds) && eq(a.uids, b.uids) && eq(a.ints, b.ints) && eq(bits(a.floats), bits(b.floats)) && eq(a.text, b.text) && eq(a.textLengths, b.textLengths);
}

describe('what a file is, by content (spec §8), as the page tells before choosing a reader', () => {
  it('agrees with expected.json on every fixture', () => {
    expect(cases.length).toBeGreaterThanOrEqual(50);
    for (const c of cases) expect(sniffDrawing(read(c.file)), c.file).toBe(c.sniff);
  });

  it('skips a byte order mark and white space before JSON; nothing else is empty', () => {
    const bytes = (s: string) => new TextEncoder().encode(s);
    expect(sniffDrawing(bytes('﻿ \r\n\t{"format":1}'))).toBe('json');
    expect(sniffDrawing(bytes('﻿  \n'))).toBe('empty');
    expect(sniffDrawing(new Uint8Array())).toBe('empty');
    expect(sniffDrawing(bytes('[1]'))).toBe('foreign');
    expect(sniffDrawing(new Uint8Array([0x89, 0x4b, 0x43]))).toBe('foreign');
  });
});

describe('JSON the Rust contracts read bit for bit', () => {
  it('writes −0 as -0.0 and everything else as JSON.stringify does', () => {
    const v = { a: -0, b: [0, -0, 1.5e-7, 486512.34], c: 'x', d: { e: -0 }, f: undefined };
    expect(exactJson(v)).toBe('{"a":-0.0,"b":[0,-0.0,1.5e-7,486512.34],"c":"x","d":{"e":-0.0}}');
    expect(exactJson({ a: 1, b: 'y' })).toBe(JSON.stringify({ a: 1, b: 'y' }));
    // A string can never become a number, whatever it holds.
    expect(JSON.parse(exactJson({ s: '\u0000kcad-negative-zero-\u0000', z: -0 }))).toEqual({ s: '\u0000kcad-negative-zero-\u0000', z: -0 });
  });

  it('refuses NaN and infinities, saying where', () => {
    expect(() => exactJson({ entities: [{ p: { x: Number.NaN } }] })).toThrow(/entities\/0\/p\/x: sayı NaN ya da sonsuz/);
    expect(() => exactJson([Infinity])).toThrow(KcadError);
  });
});

describe('the drawing packed for the worker', () => {
  it('keeps every field of every kind and counts what the contract does not know', () => {
    const all = drawing('drawing.json');
    const clean = pack(all);
    expect(difference(unpackSnapshot(clean), all)).toBeNull();
    const extra = structuredClone(all) as unknown as { entities: Record<string, unknown>[]; layers: { style: Record<string, unknown> }[] };
    extra.entities[2].note = 'bilinmeyen';
    extra.entities[3].note = 'bilinmeyen';
    (extra.entities[0].p as Record<string, unknown>).w = 1;
    extra.layers[1].style.glow = true;
    const { entities, uids, ...head } = extra as unknown as DocumentSnapshotV2;
    const out = packDrawing(
      head,
      entities.map((e, i) => ({ ...e, uid: uids[i] })),
    );
    expect(out.dropped).toEqual({ 'polyline.note': 1, 'polygon.note': 1, 'point.p.w': 1, 'belge.layers.style.glow': 1 });
    expect(difference(unpackSnapshot(out.drawing), all)).toBeNull();
  });
});

describe.skipIf(!formatsBuilt)('KCAD v2 in the browser (formats WASM module)', () => {
  it('reads every fixture as expected.json says: the error codes, and the drawings number for number', async () => {
    const m = await formatsModule();
    let valid = 0;
    for (const c of cases) {
      const data = read(c.file);
      if (c.error) {
        let error: unknown = null;
        try {
          decodeWith(m, data);
        } catch (e) {
          error = e;
        }
        expect(error, c.file).toBeInstanceOf(KcadError);
        expect((error as KcadError).code, c.file).toBe(c.error);
        expect((error as KcadError).message.length).toBeGreaterThan(20);
      } else if (c.content) {
        // Every number with Object.is: −0 is not 0.
        expect(difference(unpackSnapshot(decodeWith(m, data)), drawing(c.content)), c.file).toBeNull();
        valid++;
      } else {
        // A file the apps wrote (exchange/): read, and written again to the same bytes here too.
        const doc = unpackSnapshot(decodeWith(m, data));
        expect(doc.entities.length, c.file).toBe(c.entities);
        expect(encodeWith(m, pack(doc)), c.file).toEqual(data);
        valid++;
      }
    }
    expect(valid).toBe(6);
  });

  it('packs every file as the Rust codec does: the page and the module lay the columns out the same', async () => {
    const m = await formatsModule();
    for (const c of cases.filter((c) => !c.error && c.sniff === 'kcad')) {
      const rust = decodeWith(m, read(c.file));
      const page = pack(unpackSnapshot(rust));
      expect(sameColumns(page.columns, rust.columns), c.file).toBe(true);
    }
  });

  it('writes the reference files byte for byte from their drawings', async () => {
    const m = await formatsModule();
    for (const [content, file] of [
      ['minimal.json', 'minimal.kcad'],
      ['migrated.json', 'migrated.kcad'],
    ]) {
      expect(encodeWith(m, pack(drawing(content))), file).toEqual(read(file));
    }
  });

  it('keeps −0, the extremes, every kind and every optional field through a write and a read', async () => {
    const m = await formatsModule();
    // drawing.json's opaque parts hold whole numbers written as floats (2.0), which JavaScript cannot
    // tell from integers: the browser writes them as integers, so the bytes differ from drawing.kcad
    // there (ADR 0025) while every value reads back the same.
    const all = drawing('drawing.json');
    const back = unpackSnapshot(decodeWith(m, encodeWith(m, pack(all))));
    expect(difference(back, all)).toBeNull();
    const text = back.entities.find((e) => e.kind === 'text');
    expect(text?.kind === 'text' && Object.is(text.rotation, -0)).toBe(true);
    const extreme = back.entities[13];
    expect(extreme.kind === 'polyline' && extreme.pts[0].x === 5e-324 && extreme.pts[1].x === 1.7976931348623157e308).toBe(true);
  });

  it('refuses a drawing the file cannot hold, with the reason', async () => {
    const m = await formatsModule();
    const base = drawing('minimal.json');
    const refuse = (change: (d: DocumentSnapshotV2) => void) => {
      const d = structuredClone(base);
      change(d);
      try {
        encodeWith(m, pack(d));
      } catch (e) {
        return (e as Error).message;
      }
      return null;
    };
    expect(refuse((d) => (d.origin.x = Number.NaN))).toMatch(/origin\/x: sayı NaN ya da sonsuz/);
    expect(refuse((d) => ((d.entities[0] as { p: { x: number } }).p.x = Number.POSITIVE_INFINITY))).toMatch(/entities\/0\/p\/x: sayı NaN ya da sonsuz/);
    expect(
      refuse((d) => {
        d.entities.push({ ...d.entities[0], id: 2 });
        d.uids.push(d.uids[0]);
      }),
    ).toMatch(/kimlik/);
    expect(refuse((d) => (d.uids[0] = 'BÜYÜK'))).toMatch(/UUID değil/);
    expect(refuse((d) => (d.migratedFrom = { format: 'kentos.document', version: 1, sourceSha256: 'ab' }))).toMatch(/göç kaynağı/);
    // A lone UTF-16 surrogate (JavaScript allows it, UTF-8 does not) is refused with its place, not replaced.
    expect(refuse((d) => (d.entities[0].attrs = { Ad: 'P\ud8001' }))).toMatch(/entities\/0 \(point\) › attrs\/Ad: .*vekili/);
  });

  it('refuses bytes whose head reads back otherwise than it was sent: nothing unverified leaves the worker', async () => {
    const m = await formatsModule();
    const packed = pack(drawing('minimal.json'));
    // A field the contract does not read, past the page's projection (io/kcad.ts `projectHead`): the file
    // would not hold it, so the bytes are refused with the place, not handed out.
    const head = JSON.parse(packed.head) as { settings: Record<string, unknown> };
    head.settings.gridSize = 5;
    let error: unknown = null;
    try {
      encodeWith(m, { ...packed, head: JSON.stringify(head) });
    } catch (e) {
      error = e;
    }
    expect(error).toBeInstanceOf(KcadError);
    expect((error as KcadError).code).toBe('verify_failed');
    expect((error as KcadError).message).toMatch(/^KCAD v2 baytları geri okununca çizimle aynı çıkmadı \(belge\/settings\/gridSize\); dosya yazılmadı/);
  });

  it('reports the project before its objects, and the objects as they are read and written', async () => {
    const m = await formatsModule();
    const d = drawing('drawing.json');
    const seen: KcadProgress[] = [];
    const bytes = encodeWith(m, pack(d), (p) => seen.push(p));
    expect(seen.map((p) => p.stage).filter((s, i, a) => s !== a[i - 1])).toEqual(['writing', 'verifying', 'checking', 'project', 'reading']);
    seen.length = 0;
    const back = decodeWith(m, bytes, (p) => seen.push(p));
    expect(seen.find((p) => p.stage === 'project')).toEqual({ stage: 'project', name: d.name, layers: d.layers.length, objects: d.entities.length });
    expect(seen.at(-1)).toEqual({ stage: 'reading', done: d.entities.length, total: d.entities.length });
    expect(new ColumnsReader(back.columns).count).toBe(d.entities.length);
  });
});
