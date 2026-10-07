import { describe, expect, it } from 'vitest';
import type { TopologyException } from '../../contracts/generated/TopologyException';
import type { TopologyRule } from '../../contracts/generated/TopologyRule';
import type { EntityGeometry } from '../entities';
import { geoMeasure } from './geoprocess';
import { topologyCatalog, topologyCheck, topologyFix, type TopologyChange } from './topologyRules';

/**
 * Topoloji kuralları (docs/adr/0202) through the WASM core, against the independent reference in
 * fixtures/topology-rules/v1/cases.json (scripts/fixtures/topology_rules_cases.py, no KentOS code), the cases the core
 * runs natively in crates/shared/geometry-core/tests/all/topology_rules.rs: each rule's findings and what the fixes write.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Pt {
  x: number;
  y: number;
}

interface Want {
  rule: string;
  problem: string;
  objects: number[];
  at: Pt;
  measure: number | null;
  measureKind: string | null;
  exception: boolean;
  fixes: string[];
}

interface WantChange {
  object: number;
  remove?: boolean;
  kind?: string;
  parts?: number;
  holes?: number | null;
  area?: number;
  length?: number;
  pts?: Pt[];
  p?: Pt;
}

interface Case {
  id: string;
  tolerance: number;
  objects: { layer: string; uid: string; shape: EntityGeometry }[];
  rules: TopologyRule[];
  exceptions?: TopologyException[];
  findings: Want[];
  fixed?: { finding: number; fix: string; changes: WantChange[] }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/topology-rules/v1/cases.json', import.meta.url), 'utf8')) as { format: string; cases: Case[] };

const close = (got: number, want: number, rel: number) => Math.abs(got - want) <= rel * Math.max(Math.abs(want), 1);

function sameChange(name: string, got: TopologyChange, want: WantChange): string[] {
  const off: string[] = [];
  if (got.object !== want.object) off.push(`${name}: nesne ${got.object} ≠ ${want.object}`);
  if (want.remove) return got.shape ? [...off, `${name}: silinmeli`] : off;
  const s = got.shape;
  if (!s) return [...off, `${name}: şekil yok`];
  if (want.pts) {
    const pts = s.kind === 'line' ? [s.a, s.b] : s.kind === 'polygon' || s.kind === 'polyline' ? s.pts : [];
    if (pts.length !== want.pts.length || pts.some((p, i) => !close(p.x, want.pts![i].x, 1e-12) || !close(p.y, want.pts![i].y, 1e-12))) off.push(`${name}: köşeler ${JSON.stringify(pts)}`);
    return off;
  }
  if (want.p) {
    if (s.kind !== 'point' || !close(s.p.x, want.p.x, 1e-12) || !close(s.p.y, want.p.y, 1e-12)) off.push(`${name}: nokta ${JSON.stringify(s)}`);
    return off;
  }
  const m = geoMeasure([s])[0];
  if (s.kind !== want.kind) off.push(`${name}: tür ${s.kind}`);
  if (m.parts !== want.parts) off.push(`${name}: parça ${m.parts}`);
  if (want.holes !== undefined && want.holes !== null && m.holes !== want.holes) off.push(`${name}: delik ${m.holes}`);
  if (want.area !== undefined && !close(m.area ?? NaN, want.area, 1e-9)) off.push(`${name}: alan ${m.area}`);
  if (want.length !== undefined && !close(m.length ?? NaN, want.length, 1e-9)) off.push(`${name}: uzunluk ${m.length}`);
  return off;
}

describe('Topoloji kuralları', () => {
  it('is the reference’s file', () => expect(file.format).toBe('kentos.topology-rule-cases'));

  it('finds what the reference finds and the fixes write it', () => {
    expect(file.cases.length).toBeGreaterThanOrEqual(17);
    const off = file.cases.flatMap((c) => {
      const entities = c.objects.map((o) => o.shape);
      const layers = c.objects.map((o) => o.layer);
      const uids = c.objects.map((o) => o.uid);
      const got = topologyCheck(entities, layers, uids, c.rules, c.tolerance, c.exceptions ?? []);
      if (got.findings.length !== c.findings.length) return [`${c.id}: ${got.findings.length} bulgu, beklenen ${c.findings.length}`];
      const out: string[] = [];
      got.findings.forEach((g, k) => {
        const w = c.findings[k];
        const name = `${c.id} #${k}`;
        if (c.rules[g.rule].id !== w.rule) out.push(`${name}: kural`);
        if (g.problem !== w.problem) out.push(`${name}: sorun ${g.problem}`);
        if (JSON.stringify(g.objects) !== JSON.stringify(w.objects)) out.push(`${name}: nesneler ${JSON.stringify(g.objects)}`);
        if (Math.abs(g.at.x - w.at.x) > 1e-6 || Math.abs(g.at.y - w.at.y) > 1e-6) out.push(`${name}: yer ${JSON.stringify(g.at)}`);
        if (w.measure === null ? g.measure !== undefined : !(close(g.measure ?? NaN, w.measure, 1e-6) || Math.abs((g.measure ?? NaN) - w.measure) <= 1e-9)) out.push(`${name}: ölçü ${g.measure}`);
        if (g.exception !== w.exception) out.push(`${name}: istisna`);
        if (JSON.stringify(g.fixes.map((f) => f.key)) !== JSON.stringify(w.fixes)) out.push(`${name}: düzeltmeler ${JSON.stringify(g.fixes)}`);
      });
      for (const x of c.fixed ?? []) {
        const name = `${c.id} #${x.finding} ${x.fix}`;
        const changes = topologyFix(entities, layers, c.rules, got.findings[x.finding], x.fix);
        if (changes.length !== x.changes.length) out.push(`${name}: ${changes.length} değişiklik`);
        else changes.forEach((g, i) => out.push(...sameChange(name, g, x.changes[i])));
      }
      return out;
    });
    expect(off).toEqual([]);
  });

  it('names the kinds, the problems and the fixes', () => {
    const c = topologyCatalog();
    expect(c.kinds.map((k) => k.key)).toEqual([
      'mustNotOverlap',
      'mustNotHaveGaps',
      'mustNotHaveSlivers',
      'mustNotHaveDuplicates',
      'mustNotHaveDangles',
      'mustNotHaveShortEdges',
      'mustNotHaveSmallAngles',
      'mustBeValid',
      'mustNotHaveMissingVertices',
      'mustNotOverlapWith',
      'mustBeCoveredBy',
      'boundaryMustBeCoveredBy',
      'mustBeOnEndOf',
    ]);
    expect(c.kinds[2]).toMatchObject({ value: 'length', defaultValue: 0.1, valueLabel: 'En az genişlik' });
    expect(c.kinds[10]).toMatchObject({ between: true, label: '… içinde kalmalı' });
    expect(c.fixes.find((f) => f.key === 'mergeNeighbour')?.label).toBe('Komşuya kat');
  });
});
