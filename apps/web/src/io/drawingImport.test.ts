import { describe, expect, it } from 'vitest';
import type { ImportLayer } from '../contracts/generated/ImportLayer';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import type { LayerTarget } from './apply';
import type { ImportedDrawing } from './client';
import { packDrawing, type DrawingHead, type PageEntity } from './columns';
import { AT_ONCE, ProgressiveImport, importedEntities, viewOf } from './drawingImport';

/**
 * An imported drawing going into the drawing (io/drawingImport.ts,
 * docs/adr/0138): the objects of the chosen layers out of the columns, a
 * large import a slice at a time yet ONE undo step, Durdur taking back
 * everything it made, an object the drawing refuses taking it all back, and
 * the view of the chosen layers without a far stray.
 */

const head: DrawingHead = {
  format: 'kentos.document',
  version: 2,
  name: 'içe aktarılan',
  settings: { srid: 5257, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000 },
  origin: { x: 0, y: 0 },
  layers: [{ id: '0', name: '0', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, children: [] }],
  activeLayer: '0',
  styles: { items: [], categories: [] },
};

const ZERO = '00000000-0000-0000-0000-000000000000';

/** A drawing a reader imported: `n` points on PARSEL and one line on YOL, as the worker hands it over. */
function imported(n: number, extra: PageEntity[] = []): ImportedDrawing {
  const objects: PageEntity[] = [
    ...Array.from({ length: n }, (_, i): PageEntity => ({ kind: 'point', id: i + 1, uid: ZERO, layerId: 'PARSEL', attrs: {}, p: { x: 421000 + i, y: 4448000 } })),
    { kind: 'line', id: n + 1, uid: ZERO, layerId: 'YOL', attrs: {}, a: { x: 0, y: 0 }, b: { x: 1, y: 1 } },
    ...extra,
  ];
  const layer = (name: string, count: number): ImportLayer => ({ name, color: 'ink', visible: true, locked: false, lineType: 'continuous', count, kinds: {} });
  const { drawing } = packDrawing(head, objects);
  return {
    result: { entities: [], layers: [layer('PARSEL', n + extra.length), layer('YOL', 1)], report: { counts: {}, source: [], notes: [], skipped: [] } },
    columns: drawing.columns,
  };
}

const makeDoc = () => new CadDocument({ name: 't', layers: new LayerStore([{ id: 'a', name: 'Mevcut' }], 'a'), origin: { x: 0, y: 0 } });

const plan = (layers: [string, LayerTarget][]) => ({ label: 'NCZ: plan.ncz', layers: new Map(layers), group: 'plan.ncz' });
const fresh = (name: string): LayerTarget => ({ kind: 'new', name, style: {}, visible: true, locked: false });

describe('importedEntities', () => {
  it('reads the chosen layers’ objects out of the columns, in file order', () => {
    const got = importedEntities(imported(3), new Set(['PARSEL']));
    expect(got.map((e) => (e.kind === 'point' ? e.p.x : null))).toEqual([421000, 421001, 421002]);
    // The placeholder persistent id is not the drawing's: the document gives one.
    expect(got.every((e) => !('uid' in e))).toBe(true);
    expect(importedEntities(imported(3), new Set(['YOL'])).map((e) => e.kind)).toEqual(['line']);
  });
});

describe('ProgressiveImport', () => {
  it('writes a large import a slice at a time as ONE undo step', () => {
    const doc = makeDoc();
    const n = AT_ONCE + 5000;
    const work = ProgressiveImport.start(doc, imported(n), plan([['PARSEL', fresh('PARSEL')], ['YOL', fresh('YOL')]]));
    if ('error' in work) throw new Error(work.error);
    expect(work.total).toBe(n + 1);
    expect(work.created).toEqual(['PARSEL', 'YOL']);
    // The layers are there at once; the objects come slice by slice, and nothing is undoable meanwhile.
    expect(doc.layers.leaves().map((l) => l.name)).toEqual(['Mevcut', 'PARSEL', 'YOL']);
    expect(doc.busy).toBe(true);
    let slices = 0;
    let r: ReturnType<ProgressiveImport['step']>;
    do {
      r = work.step(0);
      slices++;
      expect(doc.size).toBe(work.done);
    } while (r === 'more');
    expect(r).toBe('done');
    expect(slices).toBeGreaterThan(10);
    expect(doc.size).toBe(n + 1);
    expect(doc.busy).toBe(false);
    // One step takes it all back: the objects and the layers made for them.
    expect(doc.undo()).toBe('NCZ: plan.ncz');
    expect(doc.size).toBe(0);
    expect(doc.layers.leaves().map((l) => l.name)).toEqual(['Mevcut']);
    expect(doc.undo()).toBeNull();
  });

  it('Durdur takes back everything it made and records nothing', () => {
    const doc = makeDoc();
    const work = ProgressiveImport.start(doc, imported(10_000), plan([['PARSEL', fresh('PARSEL')]]));
    if ('error' in work) throw new Error(work.error);
    expect(work.step(0)).toBe('more');
    expect(doc.size).toBeGreaterThan(0);
    work.stop();
    expect(doc.size).toBe(0);
    expect(doc.layers.leaves().map((l) => l.name)).toEqual(['Mevcut']);
    expect(doc.busy).toBe(false);
    expect(doc.undo()).toBeNull();
    expect(doc.dirty.value).toBe(false);
  });

  it('an object the drawing refuses takes the whole import back, and says which', () => {
    const doc = makeDoc();
    const circle: PageEntity = { kind: 'circle', id: 1, uid: ZERO, layerId: 'PARSEL', attrs: {}, c: { x: 0, y: 0 }, r: 1 };
    const d = imported(3000, [circle]);
    // A radius that is not a number (the page's packer refuses to write one; a module could send one):
    // after 3000 points' two numbers and the line's four, the circle's centre and then its radius.
    d.columns.floats[2 * 3000 + 4 + 2] = Number.NaN;
    const work = ProgressiveImport.start(doc, d, plan([['PARSEL', { kind: 'existing', id: 'a' }]]));
    if ('error' in work) throw new Error(work.error);
    let r: ReturnType<ProgressiveImport['step']>;
    do r = work.step(0);
    while (r === 'more');
    expect(typeof r === 'object' && r.error).toMatch(/^Dosyadan okunan nesneler çizime uymuyor \(İçe aktarılan nesne 3001 \(circle\)/);
    expect(doc.size).toBe(0);
    expect(doc.undo()).toBeNull();
  });

  it('refuses to start while another edit is open, and onto a locked layer', () => {
    const doc = makeDoc();
    const group = doc.beginGroup('Model');
    expect(ProgressiveImport.start(doc, imported(1), plan([['PARSEL', fresh('PARSEL')]]))).toEqual({ error: expect.stringMatching(/hâlâ çalışıyor/) });
    group.end();
    const locked = new CadDocument({ name: 't', layers: new LayerStore([{ id: 'a', name: 'Mevcut', locked: true }], 'a'), origin: { x: 0, y: 0 } });
    expect(ProgressiveImport.start(locked, imported(1), plan([['PARSEL', { kind: 'existing', id: 'a' }]]))).toEqual({ error: expect.stringMatching(/kilitli/) });
  });
});

describe('viewOf', () => {
  const box = (minX: number, minY: number, maxX: number, maxY: number) => ({ minX, minY, maxX, maxY });

  it('shows the chosen layers inside the import’s view, unless they lie only beyond it', () => {
    const view = box(100, 100, 200, 200);
    // A layer in the city with a stray at 0, 0: the city part.
    expect(viewOf(view, [box(0, 0, 150, 150)])).toEqual(box(100, 100, 150, 150));
    // A layer that is only the stray: where it is.
    expect(viewOf(view, [box(0, 0, 1, 1)])).toEqual(box(0, 0, 1, 1));
    expect(viewOf(view, [])).toBeNull();
    expect(viewOf(undefined, [box(1, 2, 3, 4), box(0, 5, 1, 6)])).toEqual(box(0, 2, 3, 6));
  });
});
