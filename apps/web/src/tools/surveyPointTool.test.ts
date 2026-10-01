import { beforeEach, describe, expect, it } from 'vitest';
import type { AppContext } from '../app/context';
import type { Entity, LineEntity, PointEntity } from '../model/entities';
import { LineTool } from './drawTools';
import { MoveTool } from './modifyTools';
import { namedPoint } from './namedPoint';
import { SurveyPointTool } from './surveyPointTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * Nokta (docs/adr/0152 §2–§4): Ad, Kod and Kot kept for the session, the name moving on by Artır, the question where a
 * point already is, Ctrl+Z giving the name back, and `#ad` giving a named point's place to the running tool. The cases
 * are worked out by hand from the ADR's rules; the desktop walks the same in crates/native/interaction/tests/survey_points.rs
 * and both play fixtures/interaction/v1/survey-points.json.
 */

beforeEach(() => {
  SurveyPointTool.next = '';
  SurveyPointTool.code = '';
  SurveyPointTool.z = null;
});

const points = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is PointEntity => e.kind === 'point');
const prompt = (name = '—', code = '—', z = 'yok') => `Nokta: nokta konumunu belirtin [Ad (A): ${name} / Kod (K): ${code} / Kot (Z): ${z}]`;

/** Answers the text field the key asked for, once it opens. */
async function answer(h: ReturnType<typeof toolHarness>, tool: SurveyPointTool, key: 'A' | 'K', value: string | null) {
  expect(tool.input(key)).toBe(true);
  expect(tool.prompt.value).toBe('Nokta: değeri yazın');
  await Promise.resolve();
  const field = h.state.textInputs.at(-1)!;
  if (value === null) field.cancel();
  else if (value === '') field.empty!();
  else field.commit(value);
}

describe('Nokta', () => {
  it('without a name, a code or an elevation writes plain points, as before', () => {
    const h = toolHarness();
    const tool = h.use(new SurveyPointTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe(prompt());
    tool.pointerDown(at(10, 5));
    const [p] = points(h);
    expect([p.p, p.label, p.attrs, p.z, p.layerId]).toEqual([pt(10, 5), undefined, {}, undefined, 'cizim']);
    expect(h.doc.undo()).toBe('Ekle');
    // A confirm leaves: it holds no point.
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('gives the points their name, code and elevation, the name moving on by Artır', async () => {
    const h = toolHarness();
    const tool = h.use(new SurveyPointTool(h.ctx));
    tool.activate();
    tool.pointerMove(at(4, 4));
    await answer(h, tool, 'A', ' 101 ');
    // The field opens by the cursor, the old value in it.
    expect([h.state.textInputs[0].at, h.state.textInputs[0].initial, h.state.textInputs[0].placeholder]).toEqual([pt(4, 4), '', 'Noktanın adı']);
    expect(tool.prompt.value).toBe(prompt('101'));
    await answer(h, tool, 'K', 'SN');
    expect(tool.input('z')).toBe(true);
    expect(tool.prompt.value).toBe('Nokta: noktaların kotunu yazın, metre (boş Enter: kotsuz)');
    // Clicks wait while the elevation is asked.
    tool.pointerDown(at(1, 1));
    expect(points(h)).toHaveLength(0);
    expect(tool.input('abc')).toBe(false);
    expect(tool.input('102.35')).toBe(true);
    expect(tool.prompt.value).toBe(prompt('101', 'SN', '102.350 m'));
    tool.pointerDown(at(0, 0));
    // Typed as Y,X: east 10, north 20.
    expect(tool.input('10,20')).toBe(true);
    expect(points(h).map((e) => [e.p, e.label, e.attrs, e.z])).toEqual([
      [pt(0, 0), '101', { Kod: 'SN' }, 102.35],
      [pt(10, 20), '102', { Kod: 'SN' }, 102.35],
    ]);
    expect(tool.prompt.value).toBe(prompt('103', 'SN', '102.350 m'));
    // The name the next point takes, beside the cursor.
    const log = canvasLog();
    tool.draw(log.g, log.view);
    expect(log.texts).toEqual(['103']);
    // An empty Enter clears the elevation; an empty field the code; Esc keeps the name.
    tool.input('Z');
    tool.confirm();
    await answer(h, tool, 'K', '');
    await answer(h, tool, 'A', null);
    expect(tool.prompt.value).toBe(prompt('103'));
    tool.pointerDown(at(30, 0));
    const last = points(h).at(-1)!;
    expect([last.label, last.attrs, last.z]).toEqual(['103', {}, undefined]);
    // Kept for the session.
    const again = h.use(new SurveyPointTool(h.ctx));
    again.activate();
    expect(again.prompt.value).toBe(prompt('104'));
  });

  it('moves a name on as Yazı’s Artır does; a name not ending in a number stays', async () => {
    const h = toolHarness();
    const tool = h.use(new SurveyPointTool(h.ctx));
    tool.activate();
    const names: string[] = [];
    for (const [first, x] of [['101/12', 0], ['P9', 10], ['A-009', 20], ['Köşe', 30]] as const) {
      await answer(h, tool, 'A', first);
      tool.pointerDown(at(x, 0));
      tool.pointerDown(at(x, 5));
      names.push(...points(h).slice(-2).map((e) => e.label!));
    }
    expect(names).toEqual(['101/12', '101/13', 'P9', 'P10', 'A-009', 'A-010', 'Köşe', 'Köşe']);
  });

  it('asks where a point already is: Düzelt, Ekle and Atla', async () => {
    const h = toolHarness();
    h.add({ kind: 'point', p: pt(5, 5), label: '7' });
    const tool = h.use(new SurveyPointTool(h.ctx));
    tool.activate();
    // Without a value Düzelt has nothing to give.
    tool.pointerDown(at(5, 5.0000005));
    expect(tool.prompt.value).toBe('Nokta: bu yerde “7” noktası var [Düzelt (D) / Ekle (E) / Atla (Esc)]');
    expect(tool.input('d')).toBe(true);
    expect(h.said().at(-1)).toBe('Nokta: düzeltilecek değer yok; Ad (A), Kod (K) ya da Kot (Z) verin.');
    expect(tool.cancel()).toBe(true);
    expect(h.said().at(-1)).toBe('Nokta: atlandı.');
    expect(points(h)).toHaveLength(1);
    await answer(h, tool, 'A', '201');
    await answer(h, tool, 'K', 'SN');
    tool.input('Z');
    tool.input('50');
    // Düzelt: the point there takes them, in one step; the name moves on.
    tool.pointerDown(at(5, 5));
    // Clicks and Enter wait for the answer.
    tool.pointerDown(at(9, 9));
    tool.confirm();
    expect(h.state.exited).toBe(0);
    expect(tool.input('D')).toBe(true);
    expect(points(h).map((e) => [e.p, e.label, e.attrs, e.z])).toEqual([[pt(5, 5), '201', { Kod: 'SN' }, 50]]);
    expect(h.said().at(-1)).toBe('Nokta: “201” noktası düzeltildi.');
    expect(tool.prompt.value).toBe(prompt('202', 'SN', '50.000 m'));
    // Ekle: the new one is written too.
    tool.pointerDown(at(5, 5));
    expect(tool.prompt.value).toBe('Nokta: bu yerde “201” noktası var [Düzelt (D) / Ekle (E) / Atla (Esc)]');
    tool.input('E');
    expect(points(h).map((e) => e.label)).toEqual(['201', '202']);
    // Atla: nothing, and the name stays.
    tool.pointerDown(at(5, 5));
    tool.cancel();
    expect(points(h)).toHaveLength(2);
    expect(tool.prompt.value).toBe(prompt('203', 'SN', '50.000 m'));
    // Undo takes Düzelt back whole.
    h.doc.undo();
    expect(h.doc.undo()).toBe('Nokta düzelt');
    expect(points(h).map((e) => [e.label, e.attrs, e.z])).toEqual([['7', {}, undefined]]);
  });

  it('says “adsız bir nokta” for an unnamed point there', () => {
    const h = toolHarness();
    h.add({ kind: 'point', p: pt(0, 0) });
    const tool = h.use(new SurveyPointTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toBe('Nokta: bu yerde adsız bir nokta var [Düzelt (D) / Ekle (E) / Atla (Esc)]');
  });

  it('takes the newest point or Düzelt back with Ctrl+Z, its name given back', async () => {
    const h = toolHarness();
    const tool = h.use(new SurveyPointTool(h.ctx));
    tool.activate();
    await answer(h, tool, 'A', '105');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    expect(tool.prompt.value).toBe(prompt('107'));
    expect(tool.undoStep()).toBe(true);
    expect(points(h).map((e) => e.label)).toEqual(['105']);
    expect(tool.prompt.value).toBe(prompt('106'));
    // Once: the older point is the drawing's to undo.
    expect(tool.undoStep()).toBe(false);
    tool.pointerDown(at(0, 0));
    tool.input('D');
    expect(points(h).map((e) => e.label)).toEqual(['106']);
    expect(tool.undoStep()).toBe(true);
    expect(points(h).map((e) => e.label)).toEqual(['105']);
    expect(tool.prompt.value).toBe(prompt('106'));
    // Out of the question first, as Esc.
    tool.pointerDown(at(0, 0));
    expect(tool.undoStep()).toBe(true);
    expect(tool.prompt.value).toBe(prompt('106'));
    // A name given anew stays.
    tool.pointerDown(at(20, 0));
    await answer(h, tool, 'A', '500');
    tool.undoStep();
    expect(tool.prompt.value).toBe(prompt('500'));
  });
});

describe('#ad', () => {
  /** The harness with `tool` as the running one. */
  const running = (h: ReturnType<typeof toolHarness>, tool: object) => ((h.ctx as unknown as { tools: { active: object } }).tools.active = tool);

  it('gives the running tool the named point’s place, as if clicked', () => {
    const h = toolHarness();
    h.add({ kind: 'point', p: pt(3, 4), label: ' 101 ' });
    h.add({ kind: 'point', p: pt(9, 9), label: '102' });
    h.add({ kind: 'point', p: pt(8, 8), label: '102', layerId: 'kilitli' });
    h.add({ kind: 'text', p: pt(1, 1), text: '103', height: 1, rotation: 0 });
    const tool = h.use(new LineTool(h.ctx));
    tool.activate();
    running(h, tool);
    const ctx = h.ctx as AppContext;
    expect(namedPoint(ctx, '#101')).toBe(true);
    expect(namedPoint(ctx, '  # 102 ')).toBe(true);
    expect(h.said().at(-1)).toBe('#102: bu adda 2 nokta var; koordinatı yazın.');
    // A text is no point.
    expect(namedPoint(ctx, '#103')).toBe(true);
    expect(h.said().at(-1)).toBe('#103: bu adda nokta yok.');
    // Not a name: the tool's.
    expect(namedPoint(ctx, '7,7')).toBe(false);
    expect(namedPoint(ctx, '#')).toBe(false);
    tool.input('7,7');
    const line = [...h.doc.all()].find((e): e is LineEntity => e.kind === 'line')!;
    expect([line.a, line.b]).toEqual([pt(3, 4), pt(7, 7)]);
  });

  it('says so where no point is asked', () => {
    const h = toolHarness();
    const target: Entity = h.add({ kind: 'point', p: pt(3, 4), label: '101' });
    const tool = h.use(new MoveTool(h.ctx, { id: 'move', label: 'Taşı', copy: false }));
    tool.activate();
    running(h, tool);
    expect(namedPoint(h.ctx, '#101')).toBe(true);
    expect(h.said().at(-1)).toBe('#101: bu adımda nokta istenmiyor.');
    expect(target.kind).toBe('point');
  });
});
