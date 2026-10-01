import { describe, expect, it } from 'vitest';
import type { DimensionEntity } from '../model/entities';
import { DimensionTool } from './dimensionTool';
import { at, pt, toolHarness } from './toolHarness';

/**
 * Ölçülendirme's ADR 0147 §7 ways (the new kinds, Açı from an arc or a circle, Zemin, Doğrusal's typed direction), as the desktop's
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
    expect(tool.prompt.value).toBe('Ölçü: koordinat ölçüsünün noktasını belirtin [Zemin (Z): kapalı / Hizalı (H) / Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç) / Yay uzunluğu (U) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]');
    tool.pointerDown(at(10, 20));
    expect(tool.prompt.value).toBe('Ölçü: çizginin ucunu gösterin ya da uzunluğunu yazın [Y koordinatı (Y) / X koordinatı (X) / Eksen (O): imleçten / Zemin (Z): kapalı]');
    // A typed length goes toward the cursor, as every point tool's: straight up 12 m.
    tool.pointerMove(at(10, 40));
    expect(tool.input('12')).toBe(true);
    expect(dims(h).at(-1)).toMatchObject({ a: pt(10, 20), b: pt(10, 32), style: 'ordinate', angle: 0 });
    // Up and a little across: its Y.
    tool.pointerDown(at(10, 20));
    tool.pointerDown(at(16, 40));
    expect(dims(h).at(-1)).toMatchObject({ a: pt(10, 20), b: pt(16, 40), offset: 0, style: 'ordinate', angle: 0 });
    expect(h.said().at(-1)).toBe('Koordinat ölçüsü eklendi: Y=10.000');
    // Across: its X; Y locks it; Eksen gives it back to the cursor.
    tool.pointerDown(at(10, 20));
    tool.pointerDown(at(-10, 22));
    expect(dims(h).at(-1)?.angle).toBe(90);
    tool.pointerDown(at(10, 20));
    expect(tool.input('Y')).toBe(true);
    expect(tool.prompt.value.endsWith('Eksen (O): Y / Zemin (Z): kapalı]')).toBe(true);
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
    expect(tool.prompt.value).toBe('Ölçü: yay uzunluğu ölçülecek yaya tıklayın [Kısmi (K) / Zemin (Z): kapalı / Hizalı (H) / Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç) / Koordinat (O) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]');
    h.state.hit = circle;
    tool.pointerDown(at(45, 0));
    expect(h.said().at(-1)).toBe("Bir yaya ya da çoklu çizginin ya da alanın yaylı kenarına tıklayın; tam daire için Yarıçap ya da Çap'ı kullanın.");
    const s = 10 * Math.SQRT1_2;
    h.state.hit = arc;
    tool.pointerDown(at(s, s));
    expect(tool.prompt.value).toBe('Ölçü: ölçü yayının yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]');
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

describe('Ölçülendirme: Kırıklı yarıçap, Semt ve Eğim', () => {
  it('Kırıklı yarıçap: a circle, the centre shown, the point on the arc, then its jog; a point leaving no room is refused', () => {
    const h = toolHarness();
    const circle = h.add({ kind: 'circle', c: pt(-290, 0), r: 300 });
    const tool = h.use(new DimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('I')).toBe(true);
    expect(tool.prompt.value.startsWith('Ölçü: kırıklı yarıçapı ölçülecek daireye ya da yaya tıklayın [')).toBe(true);
    h.state.hit = circle;
    tool.pointerDown(at(10, 0));
    h.state.hit = null;
    expect(tool.prompt.value).toBe('Ölçü: çizginin başlayacağı merkezi gösterin');
    tool.pointerDown(at(-10, 4));
    expect(tool.prompt.value).toBe('Ölçü: yaydaki noktayı gösterin');
    tool.pointerDown(at(-12, 30));
    expect(h.said().at(-1)).toBe('Gösterilen merkez, yarıçap boyunca yaydaki noktadan geride ve yarıçapa yakın olmalı; başka bir yer gösterin.');
    tool.pointerDown(at(10.5, 0));
    expect(tool.prompt.value).toBe('Ölçü: kırığın yerini gösterin ya da uzaklığını yazın [Zemin (Z): kapalı]');
    expect(tool.input('6')).toBe(true);
    const d = dims(h).at(-1)!;
    expect(d).toMatchObject({ style: 'jogged', a: pt(-290, 0), c: pt(-10, 4), offset: 6 });
    expect(near(d.b.x, 10) && near(d.b.y, 0)).toBe(true);
    // Ctrl+Z: the points first, then the circle.
    h.state.hit = circle;
    tool.pointerDown(at(10, 0));
    h.state.hit = null;
    tool.pointerDown(at(-10, 4));
    expect(tool.undoStep()).toBe(true);
    expect(tool.prompt.value).toBe('Ölçü: çizginin başlayacağı merkezi gösterin');
    expect(tool.undoStep()).toBe(true);
    expect(tool.prompt.value.startsWith('Ölçü: kırıklı yarıçapı ölçülecek')).toBe(true);
    expect(tool.input('H')).toBe(true);
  });

  it('Semt: two points, or an edge with Kenardan, then its arrow (left positive)', () => {
    const h = toolHarness();
    const line = h.add({ kind: 'line', a: pt(0, 0), b: pt(30, 40) });
    const tool = h.use(new DimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('T')).toBe(true);
    expect(tool.prompt.value.startsWith('Ölçü: semt ölçüsünün başlangıcını gösterin [Kenardan (K) / Zemin (Z): kapalı / Hizalı (H)')).toBe(true);
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toBe('Ölçü: kenarın sonunu gösterin');
    tool.pointerDown(at(30, 40));
    expect(tool.prompt.value).toBe('Ölçü: okun yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]');
    tool.pointerDown(at(11, 23));
    expect(dims(h).at(-1)).toMatchObject({ style: 'azimuth', a: pt(0, 0), b: pt(30, 40) });
    expect(near(dims(h).at(-1)!.offset, 5)).toBe(true);
    expect(tool.input('K')).toBe(true);
    expect(tool.prompt.value.startsWith('Ölçü: semti ölçülecek kenara tıklayın [Noktalardan (K)')).toBe(true);
    h.state.hit = line;
    tool.pointerDown(at(15, 20));
    h.state.hit = null;
    expect(tool.prompt.value).toBe('Ölçü: okun yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]');
    expect(tool.input('-3')).toBe(true);
    expect(dims(h).at(-1)).toMatchObject({ a: pt(0, 0), b: pt(30, 40), offset: -3 });
    expect(tool.input('K')).toBe(true);
    expect(tool.input('H')).toBe(true);
  });

  it('Eğim: each point’s elevation from the vertex it snapped to, else asked; then its arrow', () => {
    const h = toolHarness();
    const point = h.add({ kind: 'point', p: pt(0, 0), z: 105.25 });
    const tool = h.use(new DimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('E')).toBe(true);
    expect(tool.prompt.value.startsWith('Ölçü: eğim ölçüsünün birinci noktasını gösterin [Kenardan (K)')).toBe(true);
    tool.pointerDown(at(0, 0, { snap: { kind: 'node', point: pt(0, 0), entityId: point.id } }));
    expect(tool.prompt.value).toBe('Ölçü: ikinci noktayı gösterin');
    tool.pointerDown(at(40, 0));
    expect(tool.prompt.value).toBe('Ölçü: ikinci noktanın kotunu yazın (m)');
    tool.pointerDown(at(20, 5));
    expect(h.said().at(-1)).toBe('Önce noktanın kotunu metre olarak yazın; geri almak için Ctrl+Z.');
    expect(tool.input('abc')).toBe(false);
    expect(tool.input('104.75')).toBe(true);
    expect(tool.prompt.value).toBe('Ölçü: okun yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]');
    expect(tool.input('1.5')).toBe(true);
    expect(dims(h).at(-1)).toMatchObject({ style: 'slope', a: pt(0, 0), b: pt(40, 0), za: 105.25, zb: 104.75, offset: 1.5 });
    expect(tool.input('H')).toBe(true);
  });
});

describe('Ölçülendirme: Açı, Zemin ve Doğrusal’ın doğrultusu', () => {
  it('Açı from an arc or a circle: the arc’s own angle; from the point clicked on the circle to a second point; Ctrl+Z takes them back', () => {
    const h = toolHarness();
    const arc = h.add({ kind: 'arc', c: pt(0, 0), r: 10, a0: 0, a1: Math.PI / 2 });
    const circle = h.add({ kind: 'circle', c: pt(40, 0), r: 5 });
    const tool = h.use(new DimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('A')).toBe(true);
    // Nothing there: what to pick is said.
    tool.pointerDown(at(0, -30));
    expect(h.said().at(-1)).toBe('Açı için düz bir kenara, yaya ya da daireye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin.');
    const s = 10 * Math.SQRT1_2;
    h.state.hit = arc;
    tool.pointerDown(at(s, s));
    expect(tool.prompt.value).toBe('Ölçü: yayın yerini gösterin ya da yarıçap yazın [Zemin (Z): kapalı]');
    // The arc's own angle wherever the cursor is; the radius typed, its sign dropped.
    h.state.hit = null;
    tool.pointerMove(at(-3, -3));
    expect(tool.input('-6')).toBe(true);
    const d = dims(h).at(-1)!;
    expect(d).toMatchObject({ style: 'angular', c: pt(0, 0) });
    expect(near(d.offset, 6) && near(d.a.x, 10) && near(d.a.y, 0) && near(d.b.x, 0) && near(d.b.y, 10)).toBe(true);
    expect(h.said().at(-1)).toBe('Açı ölçüsü eklendi: 100.0000 g');
    // Daireden: the click is put on the circle (east of its centre), then a second point north of it.
    h.state.hit = circle;
    tool.pointerDown(at(45.2, 0));
    expect(tool.prompt.value).toBe('Ölçü: açının ikinci noktasını gösterin');
    h.state.hit = null;
    tool.pointerDown(at(40, 10));
    expect(tool.prompt.value).toBe('Ölçü: yayın yerini gösterin ya da yarıçap yazın [Zemin (Z): kapalı]');
    // Ctrl+Z: the second point, then the circle.
    expect(tool.undoStep()).toBe(true);
    expect(tool.prompt.value).toBe('Ölçü: açının ikinci noktasını gösterin');
    expect(tool.undoStep()).toBe(true);
    expect(tool.prompt.value.startsWith('Ölçü: açı ölçüsü için bir kenara')).toBe(true);
    h.state.hit = circle;
    tool.pointerDown(at(45.2, 0));
    h.state.hit = null;
    tool.pointerDown(at(40, 10));
    // Between the two, counter-clockwise from the first.
    tool.pointerDown(at(43, 3));
    const c = dims(h).at(-1)!;
    expect(c).toMatchObject({ style: 'angular', c: pt(40, 0), a: pt(45, 0), b: pt(40, 10) });
    expect(near(c.offset, Math.sqrt(18))).toBe(true);
    expect(h.said().at(-1)).toBe('Açı ölçüsü eklendi: 100.0000 g');
    expect(tool.input('H')).toBe(true);
  });

  it('Zemin: offered while nothing is picked and while placing; written with the dimension, kept for the next', () => {
    const h = toolHarness();
    const tool = h.use(new DimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('Z')).toBe(true);
    expect(tool.prompt.value).toContain('[Zemin (Z): açık / Doğrusal (D)');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    expect(tool.prompt.value).toBe('Ölçü: ölçü çizgisinin yerini gösterin ya da mesafe yazın [Zemin (Z): açık]');
    tool.pointerDown(at(5, 3));
    expect(dims(h).at(-1)?.mask).toBe(true);
    // Off while the next is placed: written without.
    tool.pointerDown(at(0, 10));
    tool.pointerDown(at(10, 10));
    expect(tool.input('Z')).toBe(true);
    expect(tool.prompt.value).toBe('Ölçü: ölçü çizgisinin yerini gösterin ya da mesafe yazın [Zemin (Z): kapalı]');
    tool.pointerDown(at(5, 13));
    expect(dims(h).at(-1)?.mask).toBeUndefined();
  });

  it('Doğrusal’s Açı (A): the measuring direction typed in the project’s angle unit, kept like the Y and X locks', () => {
    const h = toolHarness();
    const tool = h.use(new DimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('D')).toBe(true);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 10));
    expect(tool.input('A')).toBe(true);
    expect(tool.prompt.value).toBe('Ölçü: ölçme doğrultusunu yazın (grad, doğudan saatin tersine)');
    // Not a number: refused, still asked.
    expect(tool.input('Y')).toBe(false);
    expect(tool.prompt.value.startsWith('Ölçü: ölçme doğrultusunu')).toBe(true);
    // 50 grads: 45°.
    expect(tool.input('50')).toBe(true);
    expect(tool.prompt.value.endsWith('[Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): 50.0000 g / Açı (A) / Zemin (Z): kapalı]')).toBe(true);
    tool.pointerDown(at(0, 10));
    const d = dims(h).at(-1)!;
    expect(d.style).toBe('linear');
    expect(near(d.angle!, 45)).toBe(true);
    // The whole diagonal lies along 45°: 10√2.
    expect(h.said().at(-1)).toBe('Doğrusal ölçü eklendi: 14.142');
    // Kept: the next linear dimension is measured along it too.
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    expect(tool.prompt.value).toContain('Yön (O): 50.0000 g');
    // Back to the cursor and to Hizalı: the tool's memory as found.
    expect(tool.input('O')).toBe(true);
    tool.confirm();
    expect(tool.input('H')).toBe(true);
  });
});
