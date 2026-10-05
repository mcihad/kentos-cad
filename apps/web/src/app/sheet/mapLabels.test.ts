import { afterEach, describe, expect, it } from 'vitest';
import { CadDocument } from '../../model/document';
import type { Entity } from '../../model/entities';
import { LayerStore, type LabelStyle } from '../../model/layers';
import type { LabelText } from '../../model/ops/labelText';
import type { CanvasPalette } from '../../render/color';
import { PickIndex } from '../../viewport/picking';
import { DEFAULT_LABELS } from '../../viewport/storeRecords';
import { mapTexts, PX_MM, type VecText } from './mapLabels';

/**
 * Etiketleri yazıya çevir writes a label where the sheet writes it (docs/adr/0175 §1): for the same drawing at the
 * same scale, each layer label the sheet's map frame writes (app/sheet/mapLabels.ts) and the text the core makes of it
 * (`ops::label_text` through the geometry store) have the same text, place, height and rotation, and alignments that
 * match (the sheet's centre or left on the middle; the text's middle centre or middle left). The labels stand far
 * apart, so that neither thins any of them.
 */
const disposables: PickIndex[] = [];
afterEach(() => disposables.splice(0).forEach((p) => p.dispose()));

const PARCEL: LabelStyle = { placement: 'center', size: 10, grow: 1, maxSize: 14 };
const CORNER: LabelStyle = { placement: 'corner', size: 11, template: 'Ada {label}' };
const POINT: LabelStyle = { placement: 'beside', size: 10.5 };
const ROAD: LabelStyle = { placement: 'along', size: 10 };

function drawing() {
  const doc = new CadDocument({
    name: 'Etiketler',
    layers: new LayerStore(
      [
        { id: 'parsel', name: 'Parsel', style: { label: PARCEL } },
        { id: 'ada', name: 'Ada', style: { label: CORNER } },
        { id: 'nokta', name: 'Nokta', style: { label: POINT } },
        { id: 'yol', name: 'Yol', style: { label: ROAD } },
        { id: 'dere', name: 'Dere' },
      ],
      'parsel',
    ),
    origin: { x: 0, y: 0 },
  });
  const x0 = 486_500;
  const y0 = 4_420_100;
  const P = (x: number, y: number) => ({ x: x0 + x, y: y0 + y });
  doc.add({ kind: 'polygon', layerId: 'parsel', attrs: {}, label: '101', pts: [P(0, 0), P(40, 0), P(40, 30), P(0, 30)] });
  doc.add({ kind: 'polygon', layerId: 'ada', attrs: {}, label: '12', pts: [P(200, 0), P(260, 0), P(250, 50), P(200, 45)] });
  doc.add({ kind: 'point', layerId: 'nokta', attrs: {}, label: 'P7', p: P(400, 20) });
  doc.add({ kind: 'polyline', layerId: 'yol', attrs: {}, label: 'Cumhuriyet Cd.', pts: [P(0, 200), P(60, 220), P(120, 230), P(180, 260)] });
  // Westward: the sheet turns it half a turn to read upright, as the text does.
  doc.add({ kind: 'line', layerId: 'yol', attrs: {}, label: '1203. Sk.', a: P(400, 300), b: P(300, 250) });
  // A multi-part polyline with its kind's default style: on its longest part.
  doc.add({ kind: 'polyline', layerId: 'dere', attrs: {}, label: 'Dere', pts: [P(0, 400), P(10, 400)], parts: [{ pts: [P(100, 420), P(160, 470), P(220, 480), P(300, 520)] }] });
  const picker = new PickIndex(doc);
  disposables.push(picker);
  return { doc, picker, box: { minX: x0 - 100, minY: y0 - 100, maxX: x0 + 500, maxY: y0 + 600 } };
}

const palette = { fg: '#E4EAF0', fgDim: '#A9B4C0', label: '#E4EAF0', labelHalo: '#151B22', paper: '#FFFFFF', drawingFont: 'Barlow' } as CanvasPalette;

/** A degree count in (−180°, 180°]. */
const turn = (d: number) => {
  let r = d % 360;
  if (r > 180) r -= 360;
  if (r <= -180) r += 360;
  return r;
};

describe('Etiketleri yazıya çevir and the sheet', () => {
  for (const scale of [500, 1000, 2500]) {
    it(`writes each label where the sheet writes it, 1:${scale}`, () => {
      const { doc, picker, box } = drawing();
      const pxPerM = 96_000 / (25.4 * scale);
      const sheet = mapTexts({
        doc,
        palette,
        font: 'barlow',
        spots: picker.labels(box, pxPerM, null),
        pxPerM,
        box: { minX: box.minX, maxY: box.maxY, width: (box.maxX - box.minX) * pxPerM, height: (box.maxY - box.minY) * pxPerM },
        dimensionText: () => '',
        pieces: () => null,
        measure: (_font, text) => text.length * 6,
      }).texts;
      const all: Entity[] = [...doc.all()];
      const wanted = all.map((e) => ({ id: e.id, label: e.label ?? '', style: (doc.layers.get(e.layerId)?.style.label ?? DEFAULT_LABELS[e.kind]) as LabelStyle }));
      const core = picker.labelTexts(wanted, scale, true);
      // At 1:2500 (1.5 px/m) the brook's default style (from 1.6 px/m) writes it on neither.
      const shown = scale > 2000 ? 5 : 6;
      expect([sheet.length, core.texts.length, core.outOfScale, core.small, core.overlapping]).toEqual([shown, shown, 6 - shown, 0, 0]);
      expect(core.texts.map((t) => t.text).sort()).toEqual(sheet.map((t) => t.text).sort());
      const align = (t: VecText) => (t.align === 'center' ? 'middleCenter' : 'middleLeft');
      for (const t of core.texts as LabelText[]) {
        const s = sheet.find((v) => v.text === t.text);
        expect(s, t.text).toBeDefined();
        if (!s) continue;
        expect(s.baseline).toBe('middle');
        expect([t.text, align(s)]).toEqual([t.text, t.align]);
        expect(Math.abs(t.p.x - s.x), `${t.text} x`).toBeLessThan(1e-9);
        expect(Math.abs(t.p.y - s.y), `${t.text} y`).toBeLessThan(1e-9);
        expect(Math.abs(t.height - s.size / PX_MM / pxPerM), `${t.text} height`).toBeLessThan(1e-12);
        expect(Math.abs(t.rotation - turn(s.rotation)), `${t.text} rotation`).toBeLessThan(1e-9);
      }
    });
  }
});
