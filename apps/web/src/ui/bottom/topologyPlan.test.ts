import { describe, expect, it } from 'vitest';
import type { TopologyFinding } from '../../model/ops/topologyRules';
import { countText, findingView, fixLabel, measureText, ruleText, shownFindings } from './topologyPlan';

/** The Topoloji tab's rules (docs/adr/0202 §5); the desktop's topology/plan.rs has the same answers. */
const finding = (over: Partial<TopologyFinding>): TopologyFinding => ({
  rule: 0,
  problem: 'overlap',
  label: 'Çakışma',
  objects: [0, 1],
  at: { x: 0, y: 0 },
  bounds: { minX: 0, minY: 0, maxX: 0, maxY: 0 },
  regions: [],
  edges: [],
  fixes: [],
  exception: false,
  ...over,
});

describe('Topoloji tab', () => {
  it('shows the open findings, the exceptions or all, of one rule or every rule', () => {
    const fs = [finding({}), finding({ exception: true }), finding({ rule: 1 }), finding({ rule: 1, exception: true })];
    expect(shownFindings(fs, 'open', null)).toEqual([0, 2]);
    expect(shownFindings(fs, 'exception', null)).toEqual([1, 3]);
    expect(shownFindings(fs, 'all', 1)).toEqual([2, 3]);
    expect(countText(2, 2, 2)).toBe('2 bulgu (açık 2, istisna 2)');
  });

  it('zooms to a finding with room round it, a point to 10 m', () => {
    expect(findingView({ minX: 5, minY: 5, maxX: 5, maxY: 5 })).toEqual({ minX: -2, minY: -2, maxX: 12, maxY: 12 });
    expect(findingView({ minX: 0, minY: 0, maxX: 40, maxY: 2 })).toEqual({ minX: -8, minY: -6, maxX: 48, maxY: 8 });
  });

  it('names a rule with its other layer, a measure in the project’s units, a fix with what it acts on', () => {
    const kinds = [{ key: 'mustBeCoveredBy' as const, label: '… içinde kalmalı', takes: '', between: true }];
    expect(ruleText({ id: 'r', kind: 'mustBeCoveredBy', layer: 'b', other: 'p' }, kinds, (id) => (id === 'p' ? 'Parsel' : id))).toBe('Parsel içinde kalmalı');
    const fmt = { area: (m: number) => `${m} m²`, length: (m: number) => `${m} m`, angle: (a: number) => `${a} rad` };
    expect(measureText(finding({ measure: 12, measureKind: 'area' }), fmt)).toBe('12 m²');
    expect(measureText(finding({ measure: 0, measureKind: 'distance' }), fmt)).toBe('');
    expect(measureText(finding({}), fmt)).toBe('');
    const ids = [12, 34];
    const f = finding({ subject: 1, measure: 0.6 });
    expect(fixLabel(f, { key: 'subtractFirst', label: 'Birinci nesneden çıkar' }, ids, (m) => `${m} m`)).toBe('Birinci nesneden çıkar (#12)');
    expect(fixLabel(f, { key: 'subtractSecond', label: 'İkinci nesneden çıkar' }, ids, (m) => `${m} m`)).toBe('İkinci nesneden çıkar (#34)');
    expect(fixLabel(f, { key: 'mergeNeighbour', label: 'Komşuya kat' }, ids, (m) => `${m} m`)).toBe('Komşuya kat (#34)');
    expect(fixLabel(f, { key: 'snapEnd', label: 'Ucu en yakın çizgiye taşı' }, ids, (m) => `${m} m`)).toBe('Ucu en yakın çizgiye taşı (0.6 m)');
    expect(fixLabel(f, { key: 'repair', label: 'Onar' }, ids, (m) => `${m} m`)).toBe('Onar');
  });
});
