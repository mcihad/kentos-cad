import { describe, expect, it } from 'vitest';
import type { DimensionEntity } from '../model/entities';
import { DimensionTool } from './dimensionTool';
import { at, pt, toolHarness } from './toolHarness';

/**
 * Ölçülendirme's Koordinat and Yay uzunluğu (docs/adr/0147 §7), as the desktop's
 * crates/native/interaction/tests/dimension.rs walks them; values worked out by hand.
 */
const dims = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is DimensionEntity => e.kind === 'dimension');
const near = (a: number, b: number) => Math.abs(a - b) < 1e-9;

describe('Ölçülendirme: Koordinat', () => {
  it('takes its point, then its line’s end (or its length toward the cursor, typed); the axis follows the cursor unless locked', () => {
    const h = toolHarness();
    const tool = h.use(new DimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('O')).toBe(true);
    expect(tool.prompt.value).toBe('Ölçü: koordinat ölçüsünün noktasını belirtin [Hizalı (H) / Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç) / Yay uzunluğu (U)]');
    tool.pointerDown(at(10, 20));
    expect(tool.prompt.value).toBe('Ölçü: çizginin ucunu gösterin ya da uzunluğunu yazın [Y koordinatı (Y) / X koordinatı (X) / Eksen (O): imleçten]');
    // A typed length goes toward the cursor, as every point tool's: straight up 12 m.
    tool.pointerMove(at(10, 40));
    expect(tool.input('12')).toBe(true);
    expect(dims(h).at(-1)).toMatchObject({ a: pt(10, 20), b: pt(10, 32), style: 'ordinate', angle: 0 });
    // Up and a little across: its Y.
    tool.pointerDown(at(10, 20));
    tool.pointerDown(at(16, 40));
    expect(dims(h).at(-1)).toMatchObject({ a: pt(10, 20), b: pt(16, 40), offset: 0, style: 'ordinate', angle: 0 });
    expect(h.said().at(-1)).toBe('Koordinat ölçüsü eklendi: 10.000');
    // Across: its X; Y locks it; Eksen gives it back to the cursor.
    tool.pointerDown(at(10, 20));
    tool.pointerDown(at(-10, 22));
    expect(dims(h).at(-1)?.angle).toBe(90);
    tool.pointerDown(at(10, 20));
    expect(tool.input('Y')).toBe(true);
    expect(tool.prompt.value.endsWith('Eksen (O): Y]')).toBe(true);
    tool.pointerDown(at(-10, 22));
    expect(dims(h).at(-1)?.angle).toBe(0);
    tool.pointerDown(at(10, 20));
    expect(tool.input('O')).toBe(true);
    tool.pointerDown(at(-10, 22));
    expect(dims(h).at(-1)?.angle).toBe(90);
    // Its end too close to the point: said, nothing written.
    const before = dims(h).length;
    tool.pointerDown(at(10, 20));
    tool.pointerDown(at(10.5, 20.3));
    expect(dims(h)).toHaveLength(before);
    expect(h.said().at(-1)).toBe('Çizginin ucu noktaya çok yakın; imleci noktadan eksene dik yönde uzaklaştırın.');
  });
});

describe('Ölçülendirme: Yay uzunluğu', () => {
  it('picks an arc, not a circle, then where its dimension arc goes or its typed distance; Kısmi takes two points on it', () => {
    const h = toolHarness();
    const arc = h.add({ kind: 'arc', c: pt(0, 0), r: 10, a0: 0, a1: Math.PI / 2 });
    const circle = h.add({ kind: 'circle', c: pt(40, 0), r: 5 });
    const tool = h.use(new DimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('U')).toBe(true);
    expect(tool.prompt.value).toBe('Ölçü: yay uzunluğu ölçülecek yaya tıklayın [Kısmi (K) / Hizalı (H) / Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç) / Koordinat (O)]');
    h.state.hit = circle;
    tool.pointerDown(at(45, 0));
    expect(h.said().at(-1)).toBe("Bir yaya ya da çoklu çizginin ya da alanın yaylı kenarına tıklayın; tam daire için Yarıçap ya da Çap'ı kullanın.");
    const s = 10 * Math.SQRT1_2;
    h.state.hit = arc;
    tool.pointerDown(at(s, s));
    expect(tool.prompt.value).toBe('Ölçü: ölçü yayının yerini gösterin ya da uzaklık yazın');
    tool.pointerMove(at(0, 14));
    expect(tool.input('3')).toBe(true);
    const d = dims(h).at(-1)!;
    expect(d).toMatchObject({ style: 'arcLength', c: pt(0, 0), offset: 3 });
    expect(near(d.a.x, 10) && near(d.a.y, 0) && near(d.b.x, 0) && near(d.b.y, 10)).toBe(true);
    // π/2 × 10.
    expect(h.said().at(-1)).toBe('Yay uzunluğu ölçüsü eklendi: 15.708');
    // Kısmi: the arc, its two points (Ctrl+Z takes the last back), then the place by the cursor, 2 m out.
    expect(tool.input('K')).toBe(true);
    expect(tool.prompt.value).toContain('Bütün yay (K)');
    tool.pointerDown(at(s, s));
    expect(tool.prompt.value).toBe('Ölçü: yayın üstünde ölçünün başlangıcını gösterin');
    h.state.hit = null;
    tool.pointerDown(at(0, 10));
    expect(tool.prompt.value).toBe('Ölçü: yayın üstünde ölçünün sonunu gösterin');
    expect(tool.undoStep()).toBe(true);
    expect(tool.prompt.value).toBe('Ölçü: yayın üstünde ölçünün başlangıcını gösterin');
    tool.pointerDown(at(0, 10));
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(12 * Math.SQRT1_2, 12 * Math.SQRT1_2));
    const part = dims(h).at(-1)!;
    expect(near(part.offset, 2)).toBe(true);
    // Its ends counter-clockwise whatever the clicks' order.
    expect(near(part.a.x, 10) && near(part.b.y, 10)).toBe(true);
  });
});
