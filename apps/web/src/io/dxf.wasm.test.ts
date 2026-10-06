import { describe, expect, it } from 'vitest';
import type { DxfWriteInput } from '../contracts/generated/DxfWriteInput';
import type { Entity } from '../contracts/generated/Entity';
import type { ExportReport } from '../contracts/generated/ExportReport';
import type { ImportResult } from '../contracts/generated/ImportResult';
import { ColumnsReader } from './columns';
import { FORMATS_VERSION } from './version';

/**
 * DXF as the browser reads and writes it: the DXF WASM module
 * (crates/wasm/dxf-wasm → src/io/dxf/pkg, built by `pnpm wasm`; the formats
 * worker loads it the first time a DXF is read or written, docs/adr/0138) on
 * the shared fixtures (fixtures/formats/v1, which crates/shared/formats/tests
 * reads natively too). A read's objects cross as typed columns (io/columns.ts)
 * and its progress as steps. Skipped only when the package has not been built.
 */

interface Imported {
  readonly ok: boolean;
  readonly message: string;
  takeHead(): string;
  takeKinds(): Uint8Array;
  takeUids(): Uint8Array;
  takeInts(): Uint32Array;
  takeFloats(): Float64Array;
  takeText(): Uint16Array;
  takeTextLengths(): Uint32Array;
  free(): void;
}

interface Dxf {
  initSync(o: { module: BufferSource }): unknown;
  dxfVersion(): number;
  readDxf(bytes: Uint8Array, options: string, progress: { step(done: number, total: number): void }): Imported;
  writeDxf(input: string): { takeBytes(): Uint8Array; readonly report: string; free(): void };
}

const glue = import.meta.glob<Dxf>('./dxf/pkg/kentos_dxf_wasm.js');
const loader = Object.values(glue)[0];
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');
const fixture = (name: string) => fs.readFileSync(new URL(`../../../../fixtures/formats/v1/${name}`, import.meta.url));

let loaded: Dxf | undefined;
async function load(): Promise<Dxf> {
  if (loaded) return loaded;
  const w = await loader!();
  w.initSync({ module: fs.readFileSync(new URL('./dxf/pkg/kentos_dxf_wasm_bg.wasm', import.meta.url)) });
  return (loaded = w);
}

const quiet = { step: () => {} };

/** A read's result as the contract's JSON form has it: the head, and the objects read back from the columns. */
function imported(r: Imported): ImportResult {
  try {
    if (!r.ok) throw new Error(r.message);
    const columns = { kinds: r.takeKinds(), uids: r.takeUids(), ints: r.takeInts(), floats: r.takeFloats(), text: r.takeText(), textLengths: r.takeTextLengths() };
    const head = JSON.parse(r.takeHead()) as ImportResult;
    const reader = new ColumnsReader(columns);
    const entities: Entity[] = [];
    for (let i = 0; i < reader.count; i++) {
      const { uid: _placeholder, ...e } = reader.next();
      entities.push(e as Entity);
    }
    expect(reader.done).toBe(true);
    return { ...head, entities };
  } finally {
    r.free();
  }
}

describe.skipIf(!loader)('DXF WASM module', () => {
  it('speaks the version of the contracts', async () => {
    expect((await load()).dxfVersion()).toBe(FORMATS_VERSION);
  });

  it('reads DXF: layers from the table, blocks kept or exploded with exact coordinates, the report', async () => {
    const w = await load();
    const read = (name: string, explodeBlocks = false) => imported(w.readDxf(fixture(name), JSON.stringify({ maxEntities: 0, explodeBlocks }), quiet));
    const e = read('entities.dxf');
    expect(e.layers.map((l) => [l.name, l.color, l.visible, l.locked])).toEqual([
      ['0', 'ink', true, false],
      ['PARSEL', '#FF0000', true, false],
      ['Yol ekseni', '#0000FF', false, false],
      ['Kot', '#00FF00', true, true],
      ['Yazı', '#FF00FF', false, false],
    ]);
    const line = e.entities[0];
    expect(line.kind === 'line' && [line.a, line.b]).toEqual([
      { x: 452000.125, y: 4412000.5 },
      { x: 452010.25, y: 4412005.75 },
    ]);
    expect(e.report.skipped.map((s) => s.what)).toEqual(['IMAGE', 'Kâğıt uzayı nesnesi']);
    // Blocks kept (docs/adr/0144 §5): the definitions in the file's order, numbered 1, 2, …; inserts place them.
    const kept = read('blocks.dxf');
    const id = (n: number) => `00000000-0000-0000-0000-${n.toString(16).padStart(12, '0')}`;
    expect(kept.blocks?.map((d) => [d.name, d.id])).toEqual([
      ['NO', id(1)],
      ['KAPI', id(2)],
      ['DAIRE', id(3)],
      ['KENDI', id(4)],
    ]);
    const inserts = kept.entities.flatMap((x) => (x.kind === 'insert' ? [[x.block, x.p, x.scale, x.rotation, x.layerId]] : []));
    expect(inserts).toEqual([
      [id(2), { x: 1000, y: 2000 }, 1, Math.PI / 2, 'KAPILAR'],
      [id(4), { x: 0, y: 0 }, 1, 0, '0'],
    ]);
    expect(kept.report.source).toContainEqual({ label: 'Blok', value: '4 tanım (1 tanesi yerleştirilmemiş)' });
    // Attribute definitions (docs/adr/0144 §7): the block's visible ones in the file's order; the insert holds the values
    // and shows the defined ones itself (crates/shared/formats/tests/all/dxf.rs has the whole file).
    const att = read('attributes.dxf');
    expect(att.blocks?.[0].attributes?.map((a) => [a.tag, a.prompt, a.value ?? null])).toEqual([
      ['NO', 'Rögar numarası', 'R-?'],
      ['KOT', 'Kapak kotu', null],
      ['ORTA', 'Orta', 'MERKEZ'],
    ]);
    expect(att.entities.flatMap((x): unknown[] => (x.kind === 'insert' ? [x.attrs] : x.kind === 'text' ? [x.text] : []))).toEqual([
      { EK: 'ekstra', GIZLI: 'secret', KOT: '101.35', NO: 'R-12' },
      'ekstra',
      {},
      'MERKEZ',
    ]);
    // Blokları patlat: every insert opened into its objects.
    const b = read('blocks.dxf', true);
    expect(b.blocks).toBeUndefined();
    const circle = b.entities.find((x) => x.kind === 'circle');
    expect(circle?.kind === 'circle' && [circle.c, circle.r, circle.layerId]).toEqual([{ x: 1000, y: 2010 }, 1, 'KAPILAR']);
    const point = b.entities.find((x) => x.kind === 'point');
    expect(point?.kind === 'point' && [point.p, point.z]).toEqual([{ x: 996, y: 2003 }, 101.5]);
    const dwg = w.readDxf(new TextEncoder().encode('AC1032\u0000binary'), '{"maxEntities":0}', quiet);
    expect([dwg.ok, dwg.message]).toEqual([false, expect.stringMatching(/DWG dosyası/)]);
    dwg.free();
  });

  it('writes an AutoCAD 2007 DXF that reads back as the same objects, bit for bit', async () => {
    const w = await load();
    const P = (x: number, y: number) => ({ x: 452345.123 + x, y: 4412345.678 + y });
    const entities: Entity[] = [
      { kind: 'polygon', id: 1, layerId: 'parsel', attrs: { Ada: '101', Parsel: '12' }, label: '12', pts: [P(0, 0), P(20, 0), P(20, 20), P(0, 20)], bulges: [0, 0.2, 0, 0], holes: [{ pts: [P(2, 2), P(4, 2), P(4, 4)] }] },
      { kind: 'arc', id: 2, layerId: 'yapi', attrs: {}, c: P(40, 30), r: 3, a0: 0.1, a1: 2.2 },
      { kind: 'spline', id: 3, layerId: 'yapi', attrs: {}, pts: [P(0, 40), P(5, 45), P(10, 40)], closed: true },
      { kind: 'text', id: 4, layerId: 'yazi', attrs: {}, color: '#FF0000', p: P(5, 60), text: 'Çınar ağacı ^ 100%', height: 2.5, rotation: 30 },
      { kind: 'point', id: 5, layerId: 'yazi', attrs: { Ad: 'P7' }, label: 'P7', p: P(1 / 3, 0.1 + 0.2), z: 0 },
      { kind: 'hatch', id: 6, layerId: 'parsel', attrs: {}, ring: [P(40, 0), P(60, 0), P(60, 20), P(40, 20)], holes: [[P(45, 5), P(50, 5), P(50, 10)]], pattern: { type: 'cross', angle: 30, spacing: 2 } },
      { kind: 'dimension', id: 7, layerId: 'yazi', attrs: {}, a: P(0, 70), b: P(20, 75), offset: 3, height: 2, style: 'linear', angle: 0 },
      { kind: 'dimension', id: 8, layerId: 'yazi', attrs: {}, a: P(30, 70), b: P(34, 73), offset: 1.5, height: 2, text: '%%c {5} \\ ^', style: 'diameter' },
    ];
    const input: DxfWriteInput = {
      entities,
      layers: [
        { id: 'parsel', name: 'Parsel sınırı', path: ['Kadastro'], color: 'fg-dim', visible: true, locked: false, lineType: 'continuous', lineWeight: 0.18 },
        { id: 'yapi', name: 'Yapı', path: [], color: '#7FB2E5', visible: false, locked: true, lineType: 'dashed', lineWeight: 0.35 },
        { id: 'yazi', name: 'Yazılar', path: ['Pafta'], color: 'ink', visible: true, locked: false, lineType: 'continuous', lineWeight: 0.25 },
      ],
      scale: 1000,
      lengthDecimals: 3,
      grads: true,
      dimensionValues: { 7: '20.000' },
    };
    const out = w.writeDxf(JSON.stringify(input));
    const bytes = out.takeBytes();
    const report = JSON.parse(out.report) as ExportReport;
    out.free();
    const head = '  0\r\nSECTION\r\n  2\r\nHEADER\r\n  9\r\n$ACADVER\r\n  1\r\nAC1021\r\n';
    expect(new TextDecoder().decode(bytes.subarray(0, head.length))).toBe(head);
    expect(report.counts).toEqual({ polygon: 1, arc: 1, spline: 1, text: 1, point: 1, hatch: 1, dimension: 2 });
    expect(report.notes.map((n) => n.what)).toEqual(['Katman rengi', 'Adalı alan']);
    const back = imported(w.readDxf(bytes, JSON.stringify({ maxEntities: 0 }), quiet));
    // The objects come numbered in file order and name layers as the file does; everything else is the drawing's own.
    const layerName = new Map(input.layers.map((l) => [l.id, l.name]));
    expect(back.entities).toEqual(entities.map((e, i) => ({ ...e, id: i + 1, layerId: layerName.get(e.layerId) })));
    expect(back.layers.map((l) => [l.name, l.color, l.visible, l.locked, l.lineType, l.lineWeight])).toEqual([
      ['Parsel sınırı', 'fg-dim', true, false, 'continuous', 0.18],
      ['Yapı', '#7FB2E5', false, true, 'dashed', 0.35],
      ['Yazılar', 'ink', true, false, 'continuous', 0.25],
    ]);
    expect(back.report.skipped).toEqual([]);
  });

  // The web's drawing as it goes to the worker (docs/adr/0144 §5): its blocks with the objects, an insert placing
  // one that holds another; written as BLOCKs and INSERTs, and read back as the same blocks and inserts.
  it('writes the blocks the objects place as blocks, and reads them back', async () => {
    const w = await load();
    const LAMBA = '018f3a2b-0000-7000-8000-000000000002';
    const DIREK = '018f3a2b-0000-7000-8000-000000000001';
    const input: DxfWriteInput = {
      entities: [
        { kind: 'insert', id: 1, layerId: 'a', attrs: { No: '7' }, block: DIREK, p: { x: 452345.5, y: 4412345.25 }, scale: 2.5, rotation: 0.1, mirror: true },
        { kind: 'line', id: 2, layerId: 'a', attrs: {}, a: { x: 452340, y: 4412340 }, b: { x: 452350, y: 4412340 } },
      ],
      layers: [{ id: 'a', name: 'Aydınlatma', path: [], color: 'ink', visible: true, locked: false, lineType: 'continuous', lineWeight: 0.25 }],
      scale: 1000,
      lengthDecimals: 3,
      grads: true,
      dimensionValues: {},
      blocks: [
        {
          id: DIREK,
          name: 'Direk',
          base: { x: 1, y: 2 },
          entities: [
            { kind: 'line', id: 1, layerId: '', attrs: {}, a: { x: 1, y: 2 }, b: { x: 1, y: 8 } },
            { kind: 'insert', id: 2, layerId: '', attrs: {}, block: LAMBA, p: { x: 1, y: 8 }, scale: 1, rotation: Math.PI / 2 },
          ],
        },
        { id: LAMBA, name: 'Lamba', base: { x: 0, y: 0 }, entities: [{ kind: 'circle', id: 1, layerId: '', attrs: {}, color: '#F5D90A', c: { x: 0, y: 0 }, r: 0.4 }] },
      ],
    };
    const out = w.writeDxf(JSON.stringify(input));
    const bytes = out.takeBytes();
    const report = JSON.parse(out.report) as ExportReport;
    out.free();
    expect(report.counts).toEqual({ insert: 1, line: 1 });
    expect(report.skipped).toEqual([]);
    const back = imported(w.readDxf(bytes, JSON.stringify({ maxEntities: 0 }), quiet));
    expect(back.blocks?.map((b) => [b.name, b.base, b.entities.map((e) => e.kind)])).toEqual([
      ['Lamba', { x: 0, y: 0 }, ['circle']],
      ['Direk', { x: 1, y: 2 }, ['line', 'insert']],
    ]);
    const insert = back.entities.find((e) => e.kind === 'insert');
    expect(insert?.kind === 'insert' && [insert.block, insert.p, insert.scale, insert.rotation, insert.mirror, insert.attrs, insert.layerId]).toEqual([
      back.blocks?.[1].id,
      { x: 452345.5, y: 4412345.25 },
      2.5,
      0.1,
      true,
      { No: '7' },
      'Aydınlatma',
    ]);
  });

  // Both platforms write the same bytes (crates/shared/formats/tests/dxf_write.rs; scripts/fixtures/dxf_write_reference.py checks them).
  it.each(['blocks', 'texts', 'leaders', 'dimensions'])('writes the %s fixture to its committed bytes, as the native writer does', async (name) => {
    const w = await load();
    const out = w.writeDxf(new TextDecoder().decode(fixture(`dxf-write/${name}.input.json`)));
    const bytes = out.takeBytes();
    out.free();
    expect(new TextDecoder().decode(bytes)).toBe(new TextDecoder().decode(fixture(`dxf-write/${name}.dxf`)));
  });

  // docs/adr/0145 §7: a text's alignment, width factor and mask cross the columns (crates/shared/formats/tests/all/dxf.rs has the whole file).
  it('reads texts with their alignment, width factor and mask', async () => {
    const w = await load();
    const r = imported(w.readDxf(fixture('texts.dxf'), JSON.stringify({ maxEntities: 0 }), quiet));
    const texts = r.entities.flatMap((x) => (x.kind === 'text' ? [[x.text, x.p, x.align ?? null, x.widthFactor ?? null, x.mask ?? false]] : []));
    expect(texts[1]).toEqual(['ORTA', { x: 30, y: 0 }, 'baselineCenter', null, false]);
    expect(texts[13]).toEqual(['GENIS', { x: 100, y: 10 }, null, 0.8, false]);
    expect(texts[18]).toEqual(['ZEMINLI', { x: 160, y: 10 }, null, null, true]);
    // An MTEXT is one multi-line text at its attachment point (docs/adr/0182 §5).
    expect(texts[19]).toEqual(['UST SOL\nIKINCI', { x: 0, y: 50 }, 'topLeft', null, false]);
    expect(texts[20]).toEqual(['MERKEZ', { x: 30, y: 50 }, 'middleCenter', null, true]);
    expect(texts[21]).toEqual(['A\nB\nC', { x: 60, y: 50 }, 'bottomRight', null, false]);
    expect(texts[23]).toEqual(['OLCEK', { x: 200, y: 20 }, null, 1.5, false]);
    const etiket = r.blocks?.find((b) => b.name === 'ETIKET');
    expect(etiket?.attributes?.map((a) => [a.tag, a.p, a.align, a.widthFactor])).toEqual([['NO', { x: 1.5, y: 0 }, 'middleCenter', 0.9]]);
  });

  // docs/adr/0165 §2: a file in inches read into a local project in millimetres comes in metres, the insert keeping its
  // own scale; read into a project with a coordinate system, as it is (crates/shared/formats/tests/all/dxf_units.rs has more).
  it('turns a declared unit into metres for a local project', async () => {
    const w = await load();
    const local = imported(w.readDxf(fixture('units.dxf'), JSON.stringify({ maxEntities: 0, unit: 'mm' }), quiet));
    const line = local.entities.find((e) => e.kind === 'line');
    expect(line?.kind === 'line' && [line.b, line.za]).toEqual([{ x: 0.254, y: 0 }, 0.0508]);
    const circle = local.entities.find((e) => e.kind === 'circle');
    expect(circle?.kind === 'circle' && [circle.c, circle.r]).toEqual([{ x: 0.127, y: 0.1016 }, 0.0508]);
    const insert = local.entities.find((e) => e.kind === 'insert');
    expect(insert?.kind === 'insert' && [insert.p, insert.scale]).toEqual([{ x: 0.0762, y: 0.1016 }, 2]);
    expect(local.blocks?.[0]?.base).toEqual({ x: 0.0254, y: 0.0254 });
    expect(local.report.notes.find((n) => n.what === 'Birim')?.reason).toBe('dosya inç biriminde; değerler çizimin birimine, milimetreye çevrildi');
    const placed = imported(w.readDxf(fixture('units.dxf'), JSON.stringify({ maxEntities: 0 }), quiet));
    const asIs = placed.entities.find((e) => e.kind === 'line');
    expect(asIs?.kind === 'line' && asIs.b).toEqual({ x: 10, y: 0 });
  });

  // docs/adr/0146 §8: a LEADER and its MTEXT, in either order, cross the columns as one leader; a MULTILEADER too
  // (crates/shared/formats/tests/all/dxf.rs has the whole file).
  it('reads leaders with their notes, arrowheads and heights', async () => {
    const w = await load();
    const r = imported(w.readDxf(fixture('leaders.dxf'), JSON.stringify({ maxEntities: 0 }), quiet));
    const leaders = r.entities.flatMap((x) => (x.kind === 'leader' ? [[x.pts, x.text ?? null, x.height, x.arrow ?? null, x.mask ?? false]] : []));
    expect(leaders).toHaveLength(9);
    expect(leaders[0]).toEqual([[{ x: 0, y: 0 }, { x: 6, y: 5 }], 'Mevcut bina', 2.5, null, false]);
    expect(leaders[1]).toEqual([[{ x: 40, y: -10 }, { x: 36, y: -6 }], 'Ø150 PVC', 2, 'dot', true]);
    expect(leaders[2]).toEqual([[{ x: 60, y: 0 }, { x: 64, y: 4 }], null, 3.6, 'none', false]);
    expect(leaders[7]).toEqual([[{ x: 100, y: 0 }, { x: 104, y: 4 }, { x: 106, y: 6 }], 'Ada 101', 2, 'open', true]);
    expect(r.entities.flatMap((x) => (x.kind === 'text' ? [x.text] : []))).toEqual(['DN 150', 'Parsel 5']);
    const vana = r.blocks?.find((b) => b.name === 'VANA');
    expect(vana?.entities.flatMap((x) => (x.kind === 'leader' ? [x.text] : []))).toEqual(['V']);
  });

  // docs/adr/0147 §8: another program's ordinate, arc length and jogged radius come in as KentOS's own, with their
  // points, heights and masks; what has no KentOS form, as its block's lines and values (crates/shared/formats/tests/all/dxf.rs
  // has the whole file).
  it('reads the new dimension kinds as KentOS dimensions', async () => {
    const w = await load();
    const r = imported(w.readDxf(fixture('dimension-kinds.dxf'), JSON.stringify({ maxEntities: 0 }), quiet));
    const dims = r.entities.flatMap((x) => (x.kind === 'dimension' ? [x] : []));
    expect(dims.map((d) => [d.style, d.angle ?? null, d.text ?? null, d.mask ?? false, d.layerId])).toEqual([
      ['ordinate', 0, null, false, 'OLCU'],
      ['ordinate', 90, 'X=4412320.00', false, 'OLCU'],
      ['arcLength', null, null, true, 'OLCU'],
      ['jogged', null, null, false, 'OLCU'],
    ]);
    const [east, north, arc, jogged] = dims;
    expect([east.a, east.b, east.offset, east.height]).toEqual([{ x: 452310, y: 4412320 }, { x: 452310, y: 4412345 }, 0, 3]);
    expect(north.b).toEqual({ x: 452275, y: 4412320 });
    expect([arc.a, arc.b, arc.c]).toEqual([{ x: 452360, y: 4412300 }, { x: 452350, y: 4412310 }, { x: 452350, y: 4412300 }]);
    expect(Math.abs(arc.offset - 3)).toBeLessThan(1e-9);
    expect([jogged.a, jogged.b, jogged.c]).toEqual([{ x: 452400, y: 4412100 }, { x: 452475.244432, y: 4412306.732377 }, { x: 452465.584951, y: 4412288.964585 }]);
    expect(Math.abs(jogged.offset - 8)).toBeLessThan(1e-5);
    expect(r.entities.filter((x) => x.kind === 'line')).toHaveLength(4);
    expect(r.entities.filter((x) => x.kind === 'text')).toHaveLength(2);
    expect(r.report.notes.filter((n) => n.what === 'Ölçü (DIMENSION)' && n.reason.startsWith('koordinat ölçüsünün başlangıcı (0, 0) değil')).map((n) => n.count)).toEqual([1]);
  });

  // The writer's dimensions come back as they went, on the web's columns too (dxf_write.rs reads them natively).
  it('reads its own new dimensions back as they were written', async () => {
    const w = await load();
    const input = JSON.parse(new TextDecoder().decode(fixture('dxf-write/dimensions.input.json'))) as DxfWriteInput;
    const r = imported(w.readDxf(fixture('dxf-write/dimensions.dxf'), JSON.stringify({ maxEntities: 0 }), quiet));
    const bare = (entities: readonly Entity[]) =>
      entities.flatMap((x) => {
        if (x.kind !== 'dimension') return [];
        const { id: _id, layerId: _layer, ...rest } = x;
        return [rest];
      });
    expect(bare(r.entities)).toHaveLength(7);
    expect(bare(r.entities)).toEqual(bare(input.entities));
    expect(r.report.notes).toEqual([]);
  });

  it('says how far a read is, to its end', async () => {
    const w = await load();
    const heard: [number, number][] = [];
    const r = w.readDxf(fixture('entities.dxf'), JSON.stringify({ maxEntities: 0 }), { step: (done, total) => heard.push([done, total]) });
    expect(r.ok).toBe(true);
    r.free();
    expect(heard.length).toBeGreaterThan(1);
    expect(heard.every(([d], i) => i === 0 || d >= heard[i - 1][0])).toBe(true);
    expect(heard.at(-1)).toEqual([950, 1000]);
  });
});
