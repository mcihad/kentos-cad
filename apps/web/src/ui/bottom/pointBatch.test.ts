import { describe, expect, it } from 'vitest';
import { CadDocument } from '../../model/document';
import type { Entity, NewEntity } from '../../model/entities';
import { LayerStore, type LayerInit } from '../../model/layers';
import { batchTargets, runBatch, type BatchOp } from './pointBatch';

/**
 * Nokta editörü's batch operations against fixtures/point-editor/v1/batch.json (scripts/fixtures/point_batch_cases.py,
 * worked out from the rules with no KentOS code): every case's drawing after the operation, the messages said and the
 * undo step, as the desktop's `points/batch.rs` gives them too; and the rows the operations take.
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
