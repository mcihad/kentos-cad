import { describe, expect, it } from 'vitest';
import type { PdfInputs } from '../../contracts/generated/sheet/PdfInputs';
import type { PdfOptions } from '../../contracts/generated/sheet/PdfOptions';
import { testEngine } from '../../product/sheet/engineTesting';
import { MemoryKeyValue, sha256Hex } from '../../product/sheet/store';
import { NO_SHAPE_PARAMS } from '../../style/primitives';
import type { MarkerBatch, PaintFillBatch, StrokeBatch } from '../../render/types';
import { fillPaths, markerPaths, strokePaths, triangleRings } from './mapVectors';
import { coreLayers, corePaths, exportName, faceFile, makePdf, pdfNotes, tmCrsOf } from './pdfExport';
import { FONT_FILES } from './pdfFonts';
import { SheetService } from './service';
import { fakeApp } from './sheetAppTesting';
import { sheetFromTemplate } from './templateActions';

/**
 * The PDF on the web (docs/sheet/design.md §9a): the drawing's batches as
 * vectors on the ground (a stroke batch's paths and rings, a fill's rings
 * with their holes, a hatch's lines clipped to its area, a shape marker's
 * outline where the shader puts it), our paths as the core's, the faces'
 * files the desktop has for every face the engine knows, the export's name
 * from the sheet's defaults through the engine, and what goes to the core's
 * writer: the sheets, the system's WKT for GeoPDF, the maps, the faces.
 */

const e = testEngine();
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array } } }).process.getBuiltinModule('node:fs');
const zlib = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:zlib'): { deflateSync(b: Uint8Array): Uint8Array; crc32(b: Uint8Array): number } } }).process.getBuiltinModule('node:zlib');
const O = { scale: 1000, origin: { x: 1000, y: 2000 } };

/** A plain black PNG of `w` × `h` pixels (8-bit RGB), as a browser's canvas would give one. */
function plainPng(w: number, h: number): Uint8Array {
  const chunk = (type: string, data: Uint8Array): Uint8Array => {
    const body = new Uint8Array(4 + data.length);
    body.set(new TextEncoder().encode(type));
    body.set(data, 4);
    const out = new Uint8Array(12 + data.length);
    const v = new DataView(out.buffer);
    v.setUint32(0, data.length);
    out.set(body, 4);
    v.setUint32(8 + data.length, zlib.crc32(body));
    return out;
  };
  const head = new Uint8Array(13);
  new DataView(head.buffer).setUint32(0, w);
  new DataView(head.buffer).setUint32(4, h);
  head[8] = 8;
  head[9] = 2;
  const parts = [new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', head), chunk('IDAT', zlib.deflateSync(new Uint8Array((w * 3 + 1) * h))), chunk('IEND', new Uint8Array())];
  const png = new Uint8Array(parts.reduce((n, x) => n + x.length, 0));
  parts.reduce((at, x) => (png.set(x, at), at + x.length), 0);
  return png;
}
const ext = { reach: 0, reachUnit: 'world' as const, bounds: [0, 0, 10, 10] as const };

describe('map vectors', () => {
  it('joins a stroke batch’s segments into its paths, a ring closed, widths and dashes in paper mm', () => {
    // A ring of four segments (path ends marked at its start and end) and a two-segment line.
    const s = Float32Array.from([0, 0, 10, 0, 0, 1, 10, 0, 10, 10, 10, 0, 10, 10, 0, 10, 20, 0, 0, 10, 0, 0, 30, 2, 20, 0, 25, 0, 0, 1, 25, 0, 30, 5, 5, 2]);
    const b: StrokeBatch = { kind: 'stroke', ...ext, segments: s, color: [1, 0, 0, 1], width: 0.35, unit: 'world', dash: [2, 1], dashOffset: 0, cap: 'round', blur: 0 };
    const p = strokePaths(b, O)!;
    expect(p.parts).toEqual([
      { points: [1000, 2000, 1010, 2000, 1010, 2010, 1000, 2010], closed: true },
      { points: [1020, 2000, 1025, 2000, 1030, 2005], closed: false },
    ]);
    // 0.35 m at 1/1000 is 0.35 mm on the paper.
    expect(p.stroke).toMatchObject({ color: '#ff0000', width: 0.35, dash: [2, 1], cap: 'round' });
  });

  it('turns a fill’s triangles into the rings bounding them: an outer one counter-clockwise, a hole clockwise', () => {
    // A 4 × 4 square with a 2 × 2 hole in its middle, as eight triangles.
    const tri = [0, 0, 4, 0, 3, 1, 0, 0, 3, 1, 1, 1, 4, 0, 4, 4, 3, 3, 4, 0, 3, 3, 3, 1, 4, 4, 0, 4, 1, 3, 4, 4, 1, 3, 3, 3, 0, 4, 0, 0, 1, 1, 0, 4, 1, 1, 1, 3];
    const rings = triangleRings(tri, 0, 0);
    const area = (p: number[]) => p.reduce((a, _, i) => (i % 2 ? a : a + p[i] * p[(i + 3) % p.length] - p[(i + 2) % p.length] * p[i + 1]), 0) / 2;
    expect(rings.map((r) => area(r.points)).sort((a, b) => a - b)).toEqual([-4, 16]);
    const solid: PaintFillBatch = { kind: 'fill', ...ext, positions: Float32Array.from(tri), paint: { kind: 'solid', color: [0, 0, 1, 0.5] } };
    const r = fillPaths(solid, { scale: 1000, origin: { x: 0, y: 0 } });
    if ('why' in r) throw new Error(r.why);
    const core = corePaths(r.paths[0]);
    expect(core).toHaveLength(1);
    expect(core[0].holes).toHaveLength(1);
    expect(core[0].fill).toBe('#0000ff80');
  });

  it('draws a hatch as its lines clipped to the area, and names a pattern fill it cannot draw', () => {
    const square = Float32Array.from([0, 0, 10, 0, 10, 10, 0, 0, 10, 10, 0, 10]);
    // Horizontal lines (angle 0: normal pointing north) every 2 m.
    const hatch: PaintFillBatch = { kind: 'fill', ...ext, positions: square, paint: { kind: 'hatch', color: [0, 0, 0, 1], angle: 0, spacing: 2, width: 0.18, offset: 1, dash: null, dashOffset: 0, unit: 'world' } };
    const r = fillPaths(hatch, { scale: 1000, origin: { x: 0, y: 0 } });
    if ('why' in r) throw new Error(r.why);
    const ys = r.paths[0].parts.map((p) => p.points[1]).sort((a, b) => a - b);
    expect(ys).toEqual([1, 3, 5, 7, 9]);
    expect(r.paths[0].parts.every((p) => Math.abs(p.points[0] - 0) < 1e-9 && Math.abs(p.points[2] - 10) < 1e-9)).toBe(true);
    const pattern: PaintFillBatch = { ...hatch, paint: { kind: 'pattern', shape: 'circle', fill: [0, 0, 0, 1], stroke: null, strokeWidth: 0, half: [1, 1], markOffset: [0, 0], markRotation: 0, params: NO_SHAPE_PARAMS, size: [2, 2], stagger: false, angle: 0, offset: [0, 0], jitter: 0, coverage: 1, seed: 0, tint: 0.3, opacity: 1, unit: 'world' } };
    expect(fillPaths(pattern, O)).toEqual({ why: 'desen dolgusu' });
  });

  it('puts a shape marker’s outline where the shader does: anchored, offset and turned with it', () => {
    const b: MarkerBatch = {
      kind: 'marker',
      ...ext,
      instances: Float32Array.from([5, 5, Math.PI / 2, 2, 2]),
      unit: 'world',
      look: { kind: 'shape', shape: 'square', fill: [0, 1, 0, 1], stroke: null, strokeWidth: 0, params: NO_SHAPE_PARAMS },
      offset: [1, 0],
      anchor: [0, 0],
      opacity: 1,
      extent: [2, 2],
    };
    const r = markerPaths(b, { scale: 1000, origin: { x: 0, y: 0 } });
    if ('why' in r) throw new Error(r.why);
    const pts = r.paths[0].parts[0].points;
    // Its centre: the point moved 1 along the marker's x, turned a quarter: (5, 6).
    const cx = (pts[0] + pts[2] + pts[4] + pts[6]) / 4;
    const cy = (pts[1] + pts[3] + pts[5] + pts[7]) / 4;
    expect([cx, cy].map((v) => Math.round(v * 1e6) / 1e6)).toEqual([5, 6]);
    expect(r.paths[0].fill).toMatchObject({ color: '#00ff00', rule: 'evenodd' });
    expect(markerPaths({ ...b, look: { kind: 'image', image: { key: 'x', kind: 'raster', url: '', width: 1, height: 1 }, fit: 'width' } }, O)).toEqual({ why: 'resimli simge' });
  });
});

describe('map outlines', () => {
  const circles = (instances: number[], params = NO_SHAPE_PARAMS): MarkerBatch => ({
    kind: 'marker',
    ...ext,
    instances: Float32Array.from(instances),
    unit: 'world',
    look: { kind: 'shape', shape: 'circle', fill: [1, 0, 0, 1], stroke: null, strokeWidth: 0, params },
    offset: [0, 0],
    anchor: [0.5, 0.5],
    opacity: 1,
    extent: [2, 2],
  });
  const at = { scale: 1000, origin: { x: 0, y: 0 } };
  const core = (b: MarkerBatch) => {
    const r = markerPaths(b, at);
    if ('why' in r) throw new Error(r.why);
    return r.paths.flatMap(corePaths);
  };

  it('closes a circle (an arc all round), so its fill is kept', () => {
    const [c] = core(circles([5, 5, 0, 2, 2]));
    expect(c).toMatchObject({ closed: true, fill: '#ff0000', holes: [] });
    // No point twice: the arc's last point, the same as its first, is left out.
    expect(c.points[0]).not.toEqual(c.points[c.points.length - 1]);
  });

  it('leaves an even-odd hole empty: a marker’s hole is its path’s hole, two markers stay two filled paths', () => {
    const holed: typeof NO_SHAPE_PARAMS = [0.5, NO_SHAPE_PARAMS[1], NO_SHAPE_PARAMS[2], NO_SHAPE_PARAMS[3]];
    const [washer] = core(circles([5, 5, 0, 2, 2], holed));
    expect(washer.holes).toHaveLength(1);
    expect(washer.fill).toBe('#ff0000');
    // Two markers overlapping: each its own filled path (an even-odd fill of both would empty the overlap).
    const two = core(circles([5, 5, 0, 2, 2, 5.5, 5, 0, 2, 2]));
    expect(two.map((x) => [x.closed, x.holes.length])).toEqual([
      [true, 0],
      [true, 0],
    ]);
  });
});

describe('PDF from the web', () => {
  it('writes every layer’s lines first and every layer’s texts over them, a layer one group in the drawing’s order', () => {
    const font = { family: 'barlow' as const, weight: 400, italic: false };
    const text = (s: string) => ({ text: s, x: 0, y: 0, size: 2.5, rotation: 0, align: 'left' as const, baseline: 'alphabetic' as const, font, color: '#000000', halo: null, widthFactor: 1, layer: '' });
    const line = { parts: [{ points: [0, 0, 1, 1], closed: false }], stroke: { color: '#000000', opacity: 1, width: 0.18, dash: null, dashOffset: 0, cap: 'butt' as const, join: 'miter' as const } };
    const mask = { parts: [{ points: [0, 0, 1, 0, 1, 1], closed: true }], fill: { color: '#ffffff', opacity: 1, rule: 'nonzero' as const } };
    const out = coreLayers([
      { id: 'yazi', name: 'Yazılar', paths: [], masks: [mask], texts: [text('Kızılırmak Caddesi')] },
      { id: 'parsel', name: 'Parsel sınırı', paths: [line], masks: [], texts: [text('7')] },
      { id: 'ada', name: 'Ada sınırı', paths: [line], masks: [], texts: [] },
    ]);
    expect(out.map((l) => `${l.id}:${l.paths.length}/${l.texts.length}`)).toEqual(['yazi:0/0', 'parsel:1/0', 'ada:1/0', 'yazi:1/1', 'parsel:0/1']);
    // The mask goes with its text, over every layer's lines.
    expect(out[3].paths[0].fill).toBe('#ffffff');
  });

  it('has the desktop’s file for every face the engine knows (ADR 0055)', () => {
    expect(FONT_FILES.length).toBe(22);
    for (const font of e.info.fonts) expect(FONT_FILES, font).toContain(faceFile({ font, weight: 400, italic: false }));
    expect(faceFile({ font: 'barlow', weight: 400, italic: true })).toBe('Barlow-400-italic.ttf');
    expect(faceFile({ font: 'plex-mono', weight: 500, italic: false })).toBe('IBMPlexMono-500.ttf');
  });

  it('names the file from the sheet’s export defaults, written by the engine', async () => {
    const { app } = fakeApp();
    const s = new SheetService(app, new MemoryKeyValue(), () => Promise.resolve(e));
    await s.ensureEngine();
    const t = e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi')!;
    const id = (await sheetFromTemplate(s, t, null, undefined, [{ name: 'ada', value: '1245' }]))!;
    const book = s.book()!;
    const sheet = book.book.sheets.find((x) => x.id === id)!;
    const inputs = s.paint.inputs(e, book, sheet, 'export');
    expect(exportName(s, e, book, id, inputs)).toBe(sheet.name);
    s.apply([{ op: 'setExport', sheet: id, export: { ...sheet.export, fileName: '[% @ada %]-pafta' } }]);
    expect(exportName(s, e, s.book()!, id, inputs)).toBe('1245-pafta');
    s.dispose();
  });

  it('gives the core’s writer the sheets, the system as WKT, each map’s content and the faces it asks for', async () => {
    const { app } = fakeApp();
    const s = new SheetService(app, new MemoryKeyValue(), () => Promise.resolve(e));
    const engine = await s.ensureEngine();
    const t = e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi')!;
    const id = (await sheetFromTemplate(s, t, null))!;
    let seen: { inputs: PdfInputs; options: PdfOptions } | null = null;
    const write = engine.toPdf.bind(engine);
    engine.toPdf = (_book, inputs, options) => {
      seen = { inputs, options };
      return new TextEncoder().encode('%PDF-1.7');
    };
    const asked: string[] = [];
    const bytes = await makePdf(app, s, { sheets: [id], dpi: 300, geo: true, layers: true }, async (names) => {
      asked.push(...names);
      return new Map(names.map((n) => [n, new Uint8Array([0, 1, 0, 0])]));
    });
    engine.toPdf = write;
    expect(new TextDecoder().decode(bytes)).toBe('%PDF-1.7');
    const got = seen!;
    expect(got.options).toEqual({ sheets: [id], geo: true, layers: true });
    expect(got.inputs.crs?.wkt).toMatch(/^PROJCS\["TUREF \/ TM36",GEOGCS\["TUREF"/);
    expect(got.inputs.crs?.epsg).toBe(5256);
    expect(got.inputs.maps).toHaveLength(1);
    expect(got.inputs.maps[0].content).toEqual({ type: 'vector', layers: [] });
    // The faces the engine asked for, each as the desktop's file.
    expect(asked.length).toBeGreaterThan(0);
    expect(asked.every((n) => FONT_FILES.includes(n))).toBe(true);
    expect(got.inputs.fonts.map((f) => f.data)).toEqual(asked.map(() => 'AAEAAA=='));
    expect(got.inputs.render.project.name).toBe('Yeni çizim');
    s.dispose();
  });

  it('writes a PDF through the core with the desktop’s own font files', async () => {
    const { app } = fakeApp();
    const s = new SheetService(app, new MemoryKeyValue(), () => Promise.resolve(e));
    await s.ensureEngine();
    const t = e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi')!;
    const id = (await sheetFromTemplate(s, t, null))!;
    const disk = async (names: readonly string[]) => new Map(names.map((n) => [n, new Uint8Array(fs.readFileSync(new URL(`../../../../desktop/assets/fonts/drawing/${n}`, import.meta.url)))]));
    const bytes = await makePdf(app, s, { sheets: [id], dpi: 300, geo: true, layers: true }, disk);
    const head = new TextDecoder('latin1').decode(bytes.subarray(0, 8));
    expect(head).toMatch(/^%PDF-1\.\d/);
    const text = new TextDecoder('latin1').decode(bytes);
    // A3 landscape in points, and the map's georeference (design §9a).
    expect(text).toMatch(/\/MediaBox \[0 0 1190\.55\d* 841\.8\d*\]/);
    expect(text).toContain('/Measure');
    // Two of the same sheet's PDFs are the same bytes (the document id is the content's hash).
    expect(await makePdf(app, s, { sheets: [id], dpi: 300, geo: true, layers: true }, disk)).toEqual(bytes);
    // Without GeoPDF no viewport is written; without layers no optional content.
    const plain = new TextDecoder('latin1').decode(await makePdf(app, s, { sheets: [id], dpi: 300, geo: false, layers: false }, disk));
    expect(plain).not.toContain('/Measure');
    expect(plain).not.toContain('/OCProperties');
    // A face the core asks for and does not get is said by name.
    await expect(makePdf(app, s, { sheets: [id], dpi: 300, geo: true, layers: true }, async () => new Map())).rejects.toMatchObject({ code: 'pdf_font_missing' });
    s.dispose();
  });

  it('sends an SVG picture as the PNG drawn at the pixels the core asks, says so in the window first and in the log after', async () => {
    const { app } = fakeApp();
    const s = new SheetService(app, new MemoryKeyValue(), () => Promise.resolve(e));
    const engine = await s.ensureEngine();
    const t = e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi')!;
    const id = (await sheetFromTemplate(s, t, null))!;
    // A 2:1 SVG in a 60 × 30 mm picture frame.
    const svg = new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="200" height="100" fill="#e01010"/></svg>');
    const meta = { sha256: await sha256Hex(svg), kind: 'svg' as const, name: 'isaret.svg', width: 200, height: 100, bytes: svg.length };
    await s.assets.put(meta, svg);
    const item = engine.newItem(s.workspace(), s.capabilities(), { tool: 'picture', id: 'svg-resim', name: 'İşaret', frame: { left: 40_000, top: 190_000, width: 60_000, height: 30_000 } });
    s.apply([
      { op: 'addAssets', assets: [meta] },
      { op: 'addItems', to: { kind: 'sheet', id }, items: [{ ...item, kind: { ...item.kind, asset: meta.sha256 } as typeof item.kind }] },
    ]);
    const seen: PdfInputs[] = [];
    const write = engine.toPdf.bind(engine);
    engine.toPdf = (_book, inputs) => {
      seen.push(structuredClone(inputs));
      return new TextEncoder().encode('%PDF-1.7');
    };
    const fonts = async (names: readonly string[]) => new Map(names.map((n) => [n, new Uint8Array([0, 1, 0, 0])]));
    const drawn: [string, number, number][] = [];
    const choice = { sheets: [id], dpi: 300, geo: false, layers: false };
    const draw = async (bytes: Uint8Array, w: number, h: number) => {
      drawn.push([new TextDecoder().decode(bytes), w, h]);
      return plainPng(w, h);
    };
    // The window's notes first (the core's words), then the PDF with the same PNG.
    const notes = await pdfNotes(s, choice, draw);
    expect(notes.map((f) => [f.severity, f.code, f.item])).toEqual([['info', 'svg_as_picture', 'svg-resim']]);
    expect(notes[0].message).toMatch(/SVG resim PDF'e resim olarak gömülür \(709 × 355 piksel\)/);
    await makePdf(app, s, choice, fonts, draw);
    // One the browser cannot read goes without, and the window says what the PDF prints instead.
    const broken = async () => {
      throw new Error('okunamadı');
    };
    expect((await pdfNotes(s, choice, broken)).map((f) => [f.severity, f.code])).toEqual([['warning', 'svg_not_in_pdf']]);
    await makePdf(app, s, choice, fonts, broken);
    engine.toPdf = write;
    // Drawn once for the window and the PDF, at its frame at 300 dpi: 60 mm → 708.7 → 709 pixels, 30 mm → 354.3 → 355.
    expect(drawn).toEqual([[new TextDecoder().decode(svg), 709, 355]]);
    const rasters = (i: PdfInputs) => i.assets.filter((a) => a.raster !== undefined).map((a) => [a.sha256, a.raster]);
    expect(rasters(seen[0])).toEqual([[meta.sha256, expect.stringMatching(/^iVBORw0KGgo/)]]);
    expect(rasters(seen[1])).toEqual([]);
    // The core's findings in the log, as the desktop says them: a note as information, a warning with its fix.
    const log = app.log.entries.value.map((x) => [x.level, x.text]);
    expect(log).toContainEqual(['info', expect.stringMatching(/İşaret.*SVG resim PDF'e resim olarak gömülür \(709 × 355 piksel\)/)]);
    expect(log).toContainEqual(['warn', expect.stringMatching(/İşaret.*SVG resim PDF'e çizilemedi.*PNG ya da JPEG/)]);
    s.dispose();
  });

  it('leaves GeoPDF out without a transverse Mercator system', () => {
    expect(tmCrsOf({ srid: 5252, name: 'TUREF', kind: 'geographic', datum: 'TUREF', ellipsoid: 'GRS80', unit: 'degree', area: '' } as never)).toBeNull();
    expect(tmCrsOf({ srid: 32636, name: 'WGS 84 / UTM 36N', kind: 'projected', datum: 'WGS84', ellipsoid: 'WGS84', unit: 'metre', projection: 'UTM', centralMeridian: 33, scaleFactor: 0.9996, falseEasting: 500000, falseNorthing: 0, area: '' } as never)).toMatchObject({ datum: 'WGS 84', ellipsoid: 'WGS 84', epsg: 32636 });
  });
});
