import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/store-records/v1/cases.json?raw';
import { readSnapshot } from '../model/snapshot';
import { packEntities } from '../wasm/pack';

/**
 * What the web puts in the geometry store for an object is what the desktop
 * puts (docs/adr/0029): the objects of the shared drawing in
 * fixtures/store-records/v1/cases.json, read as the app reads a file and
 * packed as PickIndex packs them (../wasm/pack.ts), give the hand-written
 * records; the desktop's `kentos_interaction::spatial::record`, packed by the
 * core's Packer, gives the same (crates/native/interaction/tests/records.rs).
 */

interface Record {
  id: number;
  nums: (number | 'NaN')[];
  strings: string[];
  note?: string;
}

interface Fixture {
  format: string;
  version: number;
  document: unknown;
  records: Record[];
}

const fixture = JSON.parse(text) as Fixture;

describe('store records (fixtures/store-records/v1)', () => {
  it('is a store-records v1 file with a record for every object', () => {
    expect(fixture.format).toBe('kentos.store-records');
    expect(fixture.version).toBe(1);
    expect(fixture.records.length).toBeGreaterThanOrEqual(20);
  });

  const read = readSnapshot(JSON.stringify(fixture.document));
  if (!read.ok) throw new Error(`Kayıt çizimi okunamadı: ${read.error}`);
  const entities = read.content.entities;

  it('reads every object of the drawing', () => {
    expect(entities.map((e) => e.id)).toEqual(fixture.records.map((r) => r.id));
  });

  for (const [i, want] of fixture.records.entries())
    it(`${entities[i]?.kind ?? '?'} ${want.id}${want.note ? `: ${want.note}` : ''}`, () => {
      const packed = packEntities([entities[i]]);
      // NaN equals NaN here; every other number is compared exactly.
      expect(Array.from(packed.nums)).toEqual(want.nums.map((n) => (n === 'NaN' ? Number.NaN : n)));
      expect(JSON.parse(packed.strings)).toEqual(want.strings);
    });
});
