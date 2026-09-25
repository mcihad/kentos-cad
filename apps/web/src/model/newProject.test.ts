import { describe, expect, it } from 'vitest';
import { CadDocument } from './document';
import { LayerStore } from './layers';
import { NEW_PROJECT_NAME, newProjectContent, sheetAround } from './newProject';
import { PROJECT_SETTINGS_DEFAULTS } from './projectSettings';
import { readSnapshot, toSnapshot } from './snapshot';

const open = () => new CadDocument({ name: 'Eski', layers: new LayerStore([{ id: 'x', name: 'X' }], 'x'), origin: { x: 0, y: 0 } });

describe('new project', () => {
  it('starts empty with the standard layers, the chosen system and scale, and default units', () => {
    const c = newProjectContent({ name: '  Ada 200  ', srid: 5254, plotScale: 500 });
    expect(c.name).toBe('Ada 200');
    expect(c.settings).toEqual({ ...PROJECT_SETTINGS_DEFAULTS, srid: 5254, plotScale: 500 });
    expect(c.origin).toEqual({ x: 500_000, y: 4_320_000 });
    expect([c.entities.length, c.homeView, c.activeLayer]).toEqual([0, null, 'taslak']);
    const ids = new LayerStore([...c.layers], c.activeLayer).leaves().map((l) => l.id);
    // The map tools write to these by id (tools/catalog.ts, LAYERS).
    expect(ids).toEqual(expect.arrayContaining(['taslak', 'ada', 'parsel', 'yapi', 'kot', 'poligon', 'pafta']));
    const frame = c.layers.find((n) => n.id === 'g-pafta')!.children!.find((n) => n.id === 'pafta')!;
    expect(frame.style?.label?.template).toBe('Pafta {label}   1:500');
    expect(newProjectContent({ name: ' ', srid: 5256, plotScale: 1000 }).name).toBe(NEW_PROJECT_NAME);
    // The work mode chosen in the dialog; hybrid when none is given.
    expect(newProjectContent({ name: 'x', srid: 5256, plotScale: 1000 }).settings.workspace).toBe('hybrid');
    expect(newProjectContent({ name: 'x', srid: 5256, plotScale: 1000, workspace: 'gis' }).settings.workspace).toBe('gis');
    expect(() => newProjectContent({ name: 'x', srid: 1234, plotScale: 1000 })).toThrow(/EPSG:1234/);
  });

  it('replaces a drawing cleanly and saves as a valid file', () => {
    const doc = open();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    doc.replaceWith(newProjectContent({ name: 'Ada 200', srid: 4326, plotScale: 2000 }));
    expect([doc.size, doc.dirty.value, doc.canUndo.value, doc.name.value, doc.crs.value.srid]).toEqual([0, false, false, 'Ada 200', 4326]);
    expect(doc.origin).toEqual({ x: 35, y: 39 });
    const read = readSnapshot(JSON.stringify(toSnapshot(doc)));
    expect(read.ok && read.content.settings.plotScale).toBe(2000);
  });

  it('gives every project its own copy of the layer styles', () => {
    const a = newProjectContent({ name: 'a', srid: 5256, plotScale: 1000 });
    const b = newProjectContent({ name: 'b', srid: 5256, plotScale: 1000 });
    expect(a.layers).toEqual(b.layers);
    expect(a.layers[1].children![0].style).not.toBe(b.layers[1].children![0].style);
  });

  it('first shows one paper sheet at the plot scale', () => {
    expect(sheetAround({ x: 500_000, y: 4_320_000 }, 1000)).toEqual({ minX: 499_750, minY: 4_319_812.5, maxX: 500_250, maxY: 4_320_187.5 });
    const deg = sheetAround({ x: 35, y: 39 }, 1000, 'degree');
    expect(deg.maxX - deg.minX).toBeCloseTo(500 / 111_320, 12);
  });
});
