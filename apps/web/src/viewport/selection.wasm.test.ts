import { describe, expect, it } from 'vitest';
import hits from '../../../../fixtures/selection/v1/hits.json?raw';
import polygon from '../../../../fixtures/selection/v1/polygon.json?raw';
import { CoreStore } from '../wasm/core';
import { POLYGON_MODES, ringProblem } from './picking';

/**
 * Seçim ekleri (docs/adr/0187) through the WASM core, on the independent reference's cases (fixtures/selection/v1,
 * scripts/fixtures/selection_cases.py): every object a click could mean, the first of them the click's pick;
 * Çokgenle seç's three modes; the rings that cannot select, in the tools' words. The core runs the same files natively
 * (crates/shared/geometry-core/tests/all/selection.rs).
 */
const FILES: Record<string, string> = { 'hits.json': hits, 'polygon.json': polygon };
const read = (name: string) => JSON.parse(FILES[name]);

function store(objects: unknown): CoreStore {
  const s = new CoreStore();
  s.put(JSON.stringify(objects));
  return s;
}

describe('selection extras (fixtures/selection/v1)', () => {
  it('gives a click’s candidates most specific first, the first the pick', () => {
    const f = read('hits.json');
    const s = store(f.objects);
    for (const c of f.cases) {
      const [x, y] = c.at;
      expect(Array.from(s.hits(x, y, c.tol)), c.name).toEqual(c.expect);
      expect(s.hit(x, y, c.tol), `${c.name}: the pick`).toBe(c.expect[0]);
    }
  });

  it('selects inside, crossing and outside a concave polygon', () => {
    const f = read('polygon.json');
    const s = store(f.objects);
    for (const c of f.cases) {
      const ring = Float64Array.from((c.ring as number[][]).flat());
      for (const mode of POLYGON_MODES) expect(Array.from(s.inPolygon(ring, POLYGON_MODES.indexOf(mode))), `${c.name} ${mode}`).toEqual(c[mode]);
    }
  });

  it('says why a ring cannot select', () => {
    const f = read('polygon.json');
    for (const r of f.rings) expect(ringProblem((r.ring as number[][]).map(([x, y]) => ({ x, y }))), r.name).toBe(r.problem);
  });
});
