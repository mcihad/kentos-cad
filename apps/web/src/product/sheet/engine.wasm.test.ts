import { describe, expect, it } from 'vitest';
import type { Op } from '../../contracts/generated/sheet/Op';
import { bookText, SheetEngineError, type BookText } from './engine';
import { fixture, fixtureFiles, fixtureText, testEngine } from './engineTesting';

/**
 * The sheet engine at its WASM boundary (docs/sheet/design.md §14): the
 * shared fixtures (fixtures/sheet/v1, written for the Rust tests) run
 * through the built package and the web's typed wrapper (engine.ts), and
 * the answers are the Rust tests' own: every operation's book, inverse and
 * label (and the inverse taking the book back), the display lists, the
 * sync plans, the relayouts, the snapping, the hits, the atlas pages, the
 * preflight findings, the templates (and every broken one's code), the
 * modes' tools and the gallery's order. A refusal comes up as a
 * `SheetEngineError` with the engine's code and its Turkish message.
 */

const e = testEngine();
const read = (b: unknown): BookText => e.readBook(typeof b === 'string' ? b : JSON.stringify(b));
const digest = (b: BookText) => e.bookDigest(b);

describe('sheet engine over WASM', () => {
  it('reports its version and schemas', () => {
    expect(e.info.bookSchema).toBe('kentos.sheet/1');
    expect(e.info.templateSchema).toBe('kentos.sheet.template/1');
    expect(e.info.nudge).toEqual([1000, 10000, 100]);
    expect(e.paperSizes().find((p) => p.id === 'a3')).toMatchObject({ name: 'A3', width: 297_000, height: 420_000 });
    expect(e.standardScales()).toContain(1000);
  });

  it('gives a map the scale that shows the drawing area, and says what a coordinate list reads (docs/adr/0206)', () => {
    // The Rust tests' cases (crates/shared/sheet/tests/all/fit.rs, coordinates.rs): a frame 200 × 150 mm.
    const frame = { left: 10_000, top: 20_000, width: 200_000, height: 150_000 };
    expect(e.fitViewScale(300, 150, 0, frame)).toBe(2000);
    expect(e.fitViewScale(300, 150, 45_000, frame)).toBe(2500);
    expect(e.fitViewScale(50_001, 10_000, 0, frame)).toBe(250_005);
    expect(e.fitViewScale(0, 10, 0, frame)).toBeNull();
    const points = (n: number) => Array.from({ length: n }, (_, i) => ({ x: i, y: 0 }));
    expect(e.coordinateSummary({ type: 'objects', uids: ['a', 'b', 'c'] }, { item: 'k', points: points(8), closed: false, objects: 2, missing: 1 }, null)).toBe(
      'Seçilen 3 nesne (çizimde olmayan: 1): 8 nokta.',
    );
    expect(e.coordinateSummary({ type: 'layer', layer: 'parsel' }, { item: 'k', points: points(48), closed: false, objects: 12 }, 'Parsel')).toBe('“Parsel” katmanı: 12 nesne, 48 nokta.');
    expect(e.coordinateSummary({ type: 'objects', uids: ['a'] }, { item: 'k', points: points(4), closed: true, area: 812.4, objects: 1 }, null)).toBe(
      'Seçilen 1 nesne: kapalı şeklin 4 köşesi, alanı 812.40 m².',
    );
    const list = (columns?: Record<string, string>) => ({
      type: 'coordinateList' as const,
      source: { type: 'selection' as const },
      naming: { type: 'given' as const },
      z: false,
      decimals: 2,
      closingRow: false,
      areaRow: false,
      title: '',
      titleStyle: { font: 'barlow', size: 3000, weight: 600, italic: false, color: '#000000' },
      headerStyle: { text: { font: 'barlow', size: 2000, weight: 600, italic: false, color: '#000000' }, padding: 500 },
      cellStyle: { text: { font: 'barlow', size: 2000, weight: 400, italic: false, color: '#000000' }, padding: 500 },
      lines: { outer: { color: '#000000', width: 350, dash: [], cap: 'butt', join: 'miter' }, rows: null, columns: null, header: null },
      overflow: { type: 'clip' as const },
      ...(columns ? { columns } : {}),
    });
    expect(e.coordinateHeadings(list() as never, true)).toEqual(['Nokta', 'Y (m)', 'X (m)', 'Z (m)', 'Alan']);
    expect(e.coordinateHeadings(list() as never, false).slice(1, 3)).toEqual(['X (m)', 'Y (m)']);
    expect(e.coordinateHeadings(list({ point: 'No', area: 'Yüzölçümü' }) as never, true)).toEqual(['No', 'Y (m)', 'X (m)', 'Z (m)', 'Yüzölçümü']);
  });

  it('applies every operation of ops/ as the Rust tests expect, and its inverse takes the book back', () => {
    const fx = fixture<{ book: string; cases: { name: string; op: Op; expect: { error?: string; bookSha256?: string; inverse?: Op[]; label?: string } }[] }>('ops/cases.json');
    const base = read(fixtureText(`ops/${fx.book}`));
    const baseDigest = digest(base);
    let applied = 0;
    let refused = 0;
    for (const c of fx.cases) {
      if (c.expect.error) {
        let err: unknown;
        try {
          e.applyOp(base, c.op);
        } catch (x) {
          err = x;
        }
        expect(err, c.name).toBeInstanceOf(SheetEngineError);
        expect((err as SheetEngineError).code, c.name).toBe(c.expect.error);
        expect((err as SheetEngineError).message.length, c.name).toBeGreaterThan(5);
        refused++;
        continue;
      }
      const a = e.applyOp(base, c.op);
      expect(digest(bookText(a.book)), `${c.name}: kitap`).toBe(c.expect.bookSha256);
      expect(a.inverse, `${c.name}: ters işlem`).toEqual(c.expect.inverse);
      expect(a.label, `${c.name}: etiket`).toBe(c.expect.label);
      const back = e.applyOps(bookText(a.book), a.inverse);
      expect(digest(bookText(back.book)), `${c.name}: geri`).toBe(baseDigest);
      applied++;
    }
    expect(applied).toBeGreaterThan(40);
    expect(refused).toBeGreaterThan(10);
  });

  it('draws the recorded display lists and writes them as SVG', () => {
    for (const f of fixtureFiles('display')) {
      const fx = fixture<{ book: unknown; sheet: string; inputs: never; expect: unknown }>(`display/${f}`);
      const book = read(typeof fx.book === 'string' ? fixtureText(`display/${fx.book}`) : fx.book);
      const list = e.displayList(book, fx.sheet, fx.inputs);
      expect(list, f).toEqual(fx.expect);
      expect(e.toSvg(list, { maps: [], assets: [], placeholders: true }).startsWith('<?xml'), f).toBe(true);
    }
  });

  it('plans the sync of §13’s table', () => {
    for (const f of fixtureFiles('sync'))
      for (const c of fixture<{ cases: { description: string; local: never[]; remote: never[]; expect: unknown }[] }>(`sync/${f}`).cases) expect(e.planSync(c.local, c.remote).actions, c.description).toEqual(c.expect);
  });

  it('lays items out again on another paper (relayout through SetPage)', () => {
    for (const f of fixtureFiles('relayout')) {
      const fx = fixture<{ book: unknown; sheet: string; cases: { description: string; page: never; expect: Record<string, unknown> }[] }>(`relayout/${f}`);
      const book = read(fx.book);
      for (const c of fx.cases) {
        const a = e.applyOp(book, { op: 'setPage', owner: { kind: 'sheet', id: fx.sheet }, page: c.page, relayout: true });
        const items = [...a.book.sheets.find((s) => s.id === fx.sheet)!.items, ...a.book.masters.flatMap((m) => m.items)];
        for (const [id, frame] of Object.entries(c.expect)) {
          const item = items.find((i) => i.id === id);
          if (item) expect(item.frame, `${f} ${c.description}: ${id}`).toEqual(frame);
        }
      }
    }
  });

  it('snaps a drag, a resize and a rotation as the fixtures say', () => {
    for (const f of fixtureFiles('snap')) {
      const fx = fixture<{ book: unknown; sheet: string; moving: string[]; options?: never; queries: { description: string; delta: [number, number]; tolerance: number; expect: unknown }[]; resize: { description: string; handle: never; to: [number, number]; tolerance: number; expect: unknown }[]; rotation: { angle: number; step: boolean; expect: number }[] }>(`snap/${f}`);
      const drag = e.snapDrag(read(fx.book), fx.sheet, fx.moving, fx.options);
      try {
        for (const q of fx.queries) expect(drag.query(q.delta[0], q.delta[1], q.tolerance), q.description).toEqual(q.expect);
        for (const q of fx.resize) expect(drag.resize(q.handle, q.to[0], q.to[1], q.tolerance, false), q.description).toEqual(q.expect);
      } finally {
        drag.free();
      }
      for (const q of fx.rotation) expect(e.snapRotation(q.angle, q.step), JSON.stringify(q)).toBe(q.expect);
    }
  });

  it('refuses a drag of an unknown item with its code', () => {
    const fx = fixture<{ book: unknown; sheet: string }>('snap/basic.json');
    expect(() => e.snapDrag(read(fx.book), fx.sheet, ['yok-boyle-bir-oge'])).toThrow(SheetEngineError);
  });

  it('finds the items under a point, in a box and a handle', () => {
    for (const f of fixtureFiles('hit')) {
      const fx = fixture<{ book: unknown; sheet: string; queries: { description: string; query: never; expect: unknown }[] }>(`hit/${f}`);
      const book = read(fx.book);
      for (const q of fx.queries) expect(e.hitTest(book, fx.sheet, q.query), q.description).toEqual(q.expect);
    }
  });

  it('finds the preflight’s findings, errors first', () => {
    for (const f of fixtureFiles('preflight')) {
      const fx = fixture<{ book: unknown; sheet: string; inputs: never; expect: { severity: string; code: string; item?: string }[] }>(`preflight/${f}`);
      const found = e.preflight(read(fx.book), fx.sheet, fx.inputs);
      const key = (x: { severity: string; code: string; item?: string }) => `${x.severity}|${x.code}|${x.item ?? ''}`;
      expect([...new Set(found.map(key))].sort(), f).toEqual([...new Set(fx.expect.map(key))].sort());
      const rank = { error: 0, warning: 1, info: 2 } as const;
      expect(found.map((x) => rank[x.severity]), f).toEqual([...found.map((x) => rank[x.severity])].sort());
      for (const x of found) expect(x.message.length, x.code).toBeGreaterThan(5);
    }
  });

  it('reads every valid .kpafta file and writes it back the same; refuses every broken one with its code (the one codec)', () => {
    const valid = fixtureFiles('kpafta/valid', '.kpafta');
    expect(valid.length).toBeGreaterThan(0);
    for (const f of valid) {
      const file = e.decodeKpafta(fixtureText(`kpafta/valid/${f}`));
      expect(file.format, f).toBe('kentos.sheet.file');
      expect(e.decodeKpafta(e.encodeKpafta(file.book, file.assets)), f).toEqual(file);
    }
    const invalid = fixtureFiles('kpafta/invalid');
    expect(invalid.length).toBe(10);
    for (const f of invalid) {
      const fx = fixture<{ file: unknown; expect: { code: string } }>(`kpafta/invalid/${f}`);
      let err: unknown;
      try {
        e.decodeKpafta(typeof fx.file === 'string' ? fx.file : JSON.stringify(fx.file));
      } catch (x) {
        err = x;
      }
      expect((err as SheetEngineError)?.code, f).toBe(fx.expect.code);
    }
  });

  it('reads a valid template and refuses every broken one with its code', () => {
    for (const f of fixtureFiles('templates/valid')) expect(e.validateTemplate(fixtureText(`templates/valid/${f}`)).schema).toBe('kentos.sheet.template/1');
    for (const f of fixtureFiles('templates/invalid')) {
      const fx = fixture<{ template: unknown; expect: { code: string } }>(`templates/invalid/${f}`);
      let err: unknown;
      try {
        e.validateTemplate(JSON.stringify(fx.template));
      } catch (x) {
        err = x;
      }
      expect((err as SheetEngineError)?.code, f).toBe(fx.expect.code);
    }
  });

  it('gives each mode its tools and the gallery its order (§11a)', () => {
    const fx = fixture<{
      profiles: { description: string; workspace: never; capabilities: never; expect: { profile: string; defaultTemplate: string; tools: Record<string, Record<string, unknown>>; absent: string[] } }[];
      ranking: { description: string; workspace: never; projectType: never; expect: unknown }[];
    }>('profiles/modes.json');
    for (const c of fx.profiles) {
      const p = e.profileFor(c.workspace, c.capabilities);
      expect(p.id, c.description).toBe(c.expect.profile);
      expect(p.defaultTemplate, c.description).toBe(c.expect.defaultTemplate);
      const tools = e.toolAvailability(c.workspace, c.capabilities);
      for (const [id, want] of Object.entries(c.expect.tools)) {
        const t = tools.find((x) => x.id === id);
        expect(t, `${c.description}: ${id}`).toBeTruthy();
        for (const [k, v] of Object.entries(want)) expect(k === 'presets' ? t!.presets.map((x) => x.id) : (t as unknown as Record<string, unknown>)[k], `${c.description}: ${id}.${k}`).toEqual(v);
      }
      for (const id of c.expect.absent) expect(tools.some((t) => t.id === id), `${c.description}: ${id}`).toBe(false);
    }
    const metas = e.systemTemplates().map((t) => t.meta);
    for (const c of fx.ranking) expect(e.rankTemplates(metas, c.workspace, c.projectType), c.description).toEqual(c.expect);
  });

  it('compiles expressions and says why one does not', () => {
    expect(e.checkExpression('@olcek')).toBeNull();
    expect(e.checkExpression('[Tapu alanı (m²)] * 2')).toBeNull();
    expect(e.checkExpression('to_int(Parsel)')).toBeNull();
    expect(e.checkExpression('bilinmeyen_islev(1)')?.code).toBe('expression_error');
  });
});
