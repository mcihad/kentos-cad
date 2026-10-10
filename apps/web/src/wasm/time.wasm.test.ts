import { describe, expect, it } from 'vitest';
import type { NewEntity } from '../model/entities';
import { autoStep, floorTime, layerTimes, readTime, showEnds, showTime, showWindow, shownIn, timePosition, timePositions, writeTime, type TimeUnit, type TimeWindow } from '../model/time';
import { layerDocument } from '../style/cases';
import { PickIndex } from '../viewport/picking';

/**
 * Zamansal katmanların zamanı (docs/adr/0210 §3–§5) through the web's way to the core (`model/time.ts`: the call table
 * and the typed `timeLayer` and `timeLayerMask`) against fixtures/temporal/v1/cases.json, which
 * scripts/fixtures/temporal_cases.py writes from the ADR without KentOS code; the core's own test reads the same file
 * (crates/shared/geometry-core/tests/all/time.rs). Everything exactly: moments are whole milliseconds.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type N = number | 'inf' | '-inf';
const num = (v: N): number => (v === 'inf' ? Infinity : v === '-inf' ? -Infinity : v);
interface Rule {
  ranged: boolean;
  cumulative: boolean;
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/temporal/v1/cases.json', import.meta.url), 'utf8')) as {
  format: string;
  reads: { text: string; expect: N | 'empty' | 'unreadable' }[];
  writes: { ms: N; dateOnly: boolean; expect: string }[];
  shows: { ms: N; unit: TimeUnit; expect: string }[];
  showWindows: { window: { instant: N } | { range: [N, N] }; unit: TimeUnit; expect: string }[];
  showEnds: { first: N; last: N; unit: TimeUnit; expect: [string, string] }[];
  floors: { ms: N; unit: TimeUnit; expect: N }[];
  steps: { anchor: N; n: number; unit: TimeUnit; k: number; expect: N }[];
  positions: { extent: [N, N]; n: number; unit: TimeUnit; expect: { anchor: N; k: number } | null }[];
  autoSteps: { extent: [N, N]; expect: { n: number; unit: TimeUnit } }[];
  times: {
    rule: Rule;
    start: string | null;
    end: string | null;
    expect: { s: N; e: N; mode: 'range' | 'instant' | 'cumulative' } | null;
    windows?: { window: { instant: N } | { range: [N, N] }; expect: boolean }[];
  }[];
  summaries: { rule: Rule; values: [string | null, string | null][]; expect: { timed: number; timeless: number; unreadable: number; extent: [N, N] | null } }[];
};

const MODES = ['range', 'instant', 'cumulative'] as const;
const value = (v: string | null) => (v === null ? undefined : v);

describe('temporal values through the core (docs/adr/0210 §3–§5)', () => {
  it('reads the shared cases', () => expect(file.format).toBe('kentos.temporal-cases'));

  it('reads, writes, shows and rounds moments as the reference does', () => {
    for (const c of file.reads) {
      const r = readTime(c.text);
      const got = r.kind === 'moment' ? r.t : r.kind;
      expect(got, c.text).toBe(c.expect === 'empty' || c.expect === 'unreadable' ? c.expect : num(c.expect));
    }
    for (const c of file.writes) expect(writeTime(num(c.ms), c.dateOnly), String(c.ms)).toBe(c.expect);
    for (const c of file.shows) expect(showTime(num(c.ms), c.unit), `${c.ms} ${c.unit}`).toBe(c.expect);
    for (const c of file.showWindows) {
      const w: TimeWindow = 'instant' in c.window ? { kind: 'instant', a: num(c.window.instant) } : { kind: 'range', a: num(c.window.range[0]), b: num(c.window.range[1]) };
      expect(showWindow(w, c.unit), JSON.stringify(c)).toBe(c.expect);
    }
    for (const c of file.showEnds) expect(showEnds(num(c.first), num(c.last), c.unit), JSON.stringify(c)).toEqual(c.expect);
    for (const c of file.floors) expect(floorTime(num(c.ms), c.unit), `${c.ms} ${c.unit}`).toBe(num(c.expect));
  });

  it('steps the slider and finds its positions as the reference does', () => {
    for (const c of file.steps) expect(timePosition(num(c.anchor), { n: c.n, unit: c.unit }, c.k), JSON.stringify(c)).toBe(num(c.expect));
    for (const c of file.positions) {
      const got = timePositions([num(c.extent[0]), num(c.extent[1])], { n: c.n, unit: c.unit });
      expect(got, JSON.stringify(c)).toEqual(c.expect === null ? null : { anchor: num(c.expect.anchor), last: c.expect.k });
    }
    for (const c of file.autoSteps) expect(autoStep([num(c.extent[0]), num(c.extent[1])]), JSON.stringify(c)).toEqual(c.expect);
  });

  it('gives an object its time and shows it in a window as the table says', () => {
    for (const c of file.times) {
      const { times } = layerTimes(c.rule.ranged, c.rule.cumulative, [[value(c.start), value(c.end)]]);
      const [s, e, mode] = times;
      if (c.expect === null) {
        expect(mode, JSON.stringify(c)).toBe(-1);
        continue;
      }
      expect([s, e, MODES[mode]], JSON.stringify(c)).toEqual([num(c.expect.s), num(c.expect.e), c.expect.mode]);
      for (const w of c.windows ?? []) {
        const window: TimeWindow = 'instant' in w.window ? { kind: 'instant', a: num(w.window.instant) } : { kind: 'range', a: num(w.window.range[0]), b: num(w.window.range[1]) };
        const shown = shownIn(c.rule.ranged, c.rule.cumulative, [[value(c.start), value(c.end)]], window);
        expect(shown[0] === 1, `${JSON.stringify(c.expect)} ${JSON.stringify(w)}`).toBe(w.expect);
      }
    }
  });

  it('sums a layer up as the reference does', () => {
    for (const c of file.summaries) {
      const { summary } = layerTimes(c.rule.ranged, c.rule.cumulative, c.values.map(([s, e]) => [value(s), value(e)]));
      const e = c.expect;
      expect(summary, JSON.stringify(c)).toEqual({ timed: e.timed, timeless: e.timeless, unreadable: e.unreadable, extent: e.extent && [num(e.extent[0]), num(e.extent[1])] });
    }
  });

  it('the viewport’s store takes a temporal layer’s times in one call and filters by the window', () => {
    const at = (text: string): number => {
      const r = readTime(text);
      if (r.kind !== 'moment') throw new Error(text);
      return r.t;
    };
    const point = (x: number, attrs: Record<string, string>): NewEntity => ({ kind: 'point', layerId: 'k', attrs, p: { x, y: 0 } });
    const doc = layerDocument('Parsel', { color: 'ink', lineType: 'continuous', lineWeight: 0.25 }, [
      point(0, { s: '2010-01-01', e: '2015-01-01' }),
      point(1, { s: '2014-06-01' }),
      // An unreadable start: open before; its end has passed.
      point(2, { s: 'yarın', e: '2012-01-01' }),
      // Timeless: shown always.
      point(3, {}),
      // A zone, and an end of a letter two bytes long in UTF-8 (the lengths cross in UTF-16 units).
      point(4, { s: '2012-03-01T10:00:00+03:00', e: 'ğ' }),
    ]);
    const ids = doc.byLayer('k').map((e) => e.id);
    const index = new PickIndex(doc);
    doc.setLayerTime('k', { start: 's', end: 'e' }, 'Zaman ayarları');
    expect(index.timeSummary()).toEqual({ count: 4, extent: [at('2010-01-01'), at('2015-01-01')] });
    index.setTimeWindow({ kind: 'instant', a: at('2014-07-01') });
    expect([...index.timeShown(ids)]).toEqual([1, 1, 0, 1, 1]);
    index.setTimeWindow({ kind: 'range', a: at('2011-01-01'), b: at('2012-03-01T07:00:00Z') });
    expect([...index.timeShown(ids)]).toEqual([1, 0, 1, 1, 0]);
    // The setting taken away: every object shows, none has a time.
    doc.setLayerTime('k', null, 'Zaman ayarları');
    expect([...index.timeShown(ids)]).toEqual([1, 1, 1, 1, 1]);
    expect(index.timeSummary()).toEqual({ count: 0, extent: null });
  });
});
