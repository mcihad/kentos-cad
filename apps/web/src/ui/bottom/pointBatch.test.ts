import { describe, expect, it } from 'vitest';
import { CadDocument } from '../../model/document';
import type { Entity, NewEntity } from '../../model/entities';
import { LayerStore, type LayerInit } from '../../model/layers';
import { elevatedPaths } from '../../product/elevation';
import { batchTargets, planDedupe, runBatch, type BatchOp } from './pointBatch';

/**
 * Nokta editörü's batch operations against fixtures/point-editor/v1/batch.json and dedupe.json
 * (scripts/fixtures/point_batch_cases.py, point_dedupe_cases.py, worked out from the rules with no KentOS code): every
 * case's drawing after the operation, the messages said and the undo step, as the desktop's `points/batch.rs` gives
 * them too; the rows the operations take; Çift noktaları ayıkla's groups and summary.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  version: number;
  layers: LayerInit[];
  objects: (NewEntity & { id: number })[];
  cases: { name: string; targets: number[]; op: BatchOp; expected: { said: string[]; step: string | null; objects: unknown[] } }[];
  targets: { name: string; shown: number[]; selected: number[]; expected: { targets: number[]; header: string } }[];
}

/** An object as the cases compare it: its kind, layer, label and attributes, and a point's place and elevation. */
function view(e: Entity): unknown {
  const head = { kind: e.kind, layerId: e.layerId, label: e.label ?? null, attrs: e.attrs };
  return e.kind === 'point' ? { ...head, at: [e.p.x, e.p.y], z: e.z ?? null } : head;
}

describe('Nokta editörü: toplu işlemler', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/point-editor/v1/batch.json', import.meta.url), 'utf8')) as File;

  it('is the reference’s file', () => {
    expect([file.format, file.version]).toEqual(['kentos.point-editor-batch', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(20);
  });

  it('writes every case as the reference does', () => {
    const off = file.cases.flatMap((c) => {
      const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore(file.layers, 'cizim'), origin: { x: 0, y: 0 } });
      // The drawing's ids are its order from 1: the cases name the points by them.
      for (const { id: _id, ...o } of file.objects) doc.add(o as NewEntity);
      const before = doc.revision;
      const got = runBatch(doc, c.targets, c.op);
      const objects = [...doc.all()].map(view);
      const step = got.step === null ? (doc.revision === before ? null : 'yazıldı') : doc.undo();
      const seen = { said: got.said, step, objects };
      return JSON.stringify(seen) === JSON.stringify(c.expected) ? [] : [`${c.name}: ${JSON.stringify(seen)}`];
    });
    expect(off).toEqual([]);
  });

  it('takes the selected rows in the table’s order, or every row', () => {
    for (const c of file.targets) {
      const selected = new Set(c.selected);
      expect(batchTargets(c.shown, (id) => selected.has(id)), c.name).toEqual({ ids: c.expected.targets, header: c.expected.header });
    }
  });
});

/** Numbers within 1e-9 (the mean is worked out exactly there), everything else exactly; a missing field null. */
function differ(a: unknown, e: unknown, path: string): string | null {
  if (typeof a === 'number' && typeof e === 'number') return Math.abs(a - e) <= 1e-9 ? null : `${path}: ${a} ≠ ${e}`;
  if (Array.isArray(a) || Array.isArray(e)) {
    if (!Array.isArray(a) || !Array.isArray(e) || a.length !== e.length) return `${path}: ${JSON.stringify(a)?.slice(0, 120)} ≠ ${JSON.stringify(e)?.slice(0, 120)}`;
    for (let i = 0; i < a.length; i++) {
      const d = differ(a[i], e[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (a && e && typeof a === 'object' && typeof e === 'object') {
    for (const k of new Set([...Object.keys(a), ...Object.keys(e)])) {
      const d = differ((a as Record<string, unknown>)[k] ?? null, (e as Record<string, unknown>)[k] ?? null, `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return (a ?? null) === (e ?? null) ? null : `${path}: ${String(a)} ≠ ${String(e)}`;
}

interface DedupeFile {
  format: string;
  version: number;
  layers: LayerInit[];
  objects: (NewEntity & { id: number })[];
  cases: {
    name: string;
    targets: number[];
    by: 'name' | 'place';
    tolerance: string;
    keep: 'first' | 'last' | 'average';
    follow: boolean;
    expected: { groups: number[][]; summary: string | null; said: string[]; step: string | null; objects: unknown[] };
  }[];
}

describe('Nokta editörü: çift noktaları ayıkla', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/point-editor/v1/dedupe.json', import.meta.url), 'utf8')) as DedupeFile;
  /** An object with line work's paths too. */
  const seen = (e: Entity): unknown =>
    e.kind === 'point' ? view(e) : { ...(view(e) as object), paths: elevatedPaths(e).map((p) => ({ pts: p.pts.map((q) => [q.x, q.y]), zs: p.zs ?? p.pts.map(() => null) })) };

  it('is the reference’s file', () => {
    expect([file.format, file.version]).toEqual(['kentos.point-editor-dedupe', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(15);
  });

  it('finds the groups and writes every case as the reference does', () => {
    const off = file.cases.flatMap((c) => {
      const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore(file.layers, 'cizim'), origin: { x: 0, y: 0 } });
      // The drawing's ids are its order from 1: the cases name the points by them.
      for (const { id: _id, ...o } of file.objects) doc.add(o as NewEntity);
      const op: BatchOp = { kind: 'dedupe', by: c.by, tolerance: c.tolerance, keep: c.keep };
      const plan = planDedupe(doc, c.targets, op);
      const before = doc.revision;
      const got = runBatch(doc, c.targets, op, c.follow);
      const objects = [...doc.all()].map(seen);
      const step = got.step === null ? (doc.revision === before ? null : 'yazıldı') : doc.undo();
      const d = differ({ groups: plan.groups, summary: plan.summary, said: got.said, step, objects }, c.expected, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });
});
