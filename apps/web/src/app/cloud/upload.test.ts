import { describe, expect, it } from 'vitest';
import type { DrawingEntity } from '../../model/entities';
import { ApiFailure } from './api';
import { newDoc, pt, serverFor } from './syncTesting';
import { UPLOAD_BATCH, again, uploadObjects } from './upload';

/**
 * Uploading a drawing's objects into a new cloud project (ADR 0014 slice 3,
 * docs/adr/0026): each under its persistent id, so the same drawing
 * uploaded twice gives two projects with the same ids, and a batch whose
 * answer was lost goes again unchanged and is answered from the log.
 */

const target = { tenantId: 't', projectId: 'p', cursor: '0' };

describe('uploadObjects', () => {
  it('sends every object under its persistent id, and the same drawing twice gives the same ids', async () => {
    const doc = newDoc();
    doc.addMany(Array.from({ length: UPLOAD_BATCH + 5 }, (_, i) => pt(i)));
    const entities = [...doc.all()] as DrawingEntity[];
    const ids = entities.map((e) => e.uid).sort();
    const done: number[] = [];
    const first = serverFor(doc);
    const up = await uploadObjects(first, target, entities, (n) => done.push(n), []);
    expect([first.commits, done]).toEqual([2, [0, UPLOAD_BATCH, UPLOAD_BATCH + 5]]);
    expect([...first.store.keys()].sort()).toEqual(ids);
    expect(up.records.map((r) => r.id).sort()).toEqual(ids);
    // Versions as the server gave them (the batch's data revision), not assumed.
    expect(new Set(up.records.map((r) => r.version))).toEqual(new Set(['1', '2']));
    expect(up.cursor).toBe(first.history.at(-1)!.seq);
    const second = serverFor(doc);
    await uploadObjects(second, target, entities, undefined, []);
    expect([...second.store.keys()].sort()).toEqual(ids);
    // Each project has its own copy: changing one leaves the other.
    second.commitAs('baska', [{ op: 'delete', id: entities[0].uid }]);
    expect([first.store.size, second.store.size]).toEqual([UPLOAD_BATCH + 5, UPLOAD_BATCH + 4]);
  });

  it('a batch whose answer was lost goes again unchanged and is committed once', async () => {
    const doc = newDoc();
    doc.addMany([pt(1), pt(2), pt(3)]);
    const entities = [...doc.all()] as DrawingEntity[];
    const server = serverFor(doc);
    server.loseNextAnswer = true;
    const up = await uploadObjects(server, target, entities, undefined, [0]);
    expect([server.commits, server.replays, server.store.size]).toEqual([1, 1, 3]);
    expect(up.records.map((r) => r.id).sort()).toEqual(entities.map((e) => e.uid).sort());
  });

  it('tries again only while the server does not answer, and not for ever', async () => {
    let calls = 0;
    const flaky = () => (++calls < 3 ? Promise.reject(new ApiFailure(503, {}, 'meşgul')) : Promise.resolve('tamam'));
    expect(await again(flaky, [0, 0, 0])).toBe('tamam');
    expect(calls).toBe(3);
    calls = 0;
    await expect(again(() => (calls++, Promise.reject(new ApiFailure(409, { error: 'conflict' }, 'Çakışma'))), [0, 0])).rejects.toThrow('Çakışma');
    expect(calls).toBe(1);
    calls = 0;
    await expect(again(() => (calls++, Promise.reject(new ApiFailure(0, {}, 'Sunucuya ulaşılamadı.'))), [0, 0])).rejects.toThrow('ulaşılamadı');
    expect(calls).toBe(3);
  });
});
