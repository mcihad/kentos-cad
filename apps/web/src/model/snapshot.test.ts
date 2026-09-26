import { describe, expect, it } from 'vitest';
import sampleText from '../../../../fixtures/document/v1/sample.json?raw';
import { CadDocument } from './document';
import { LayerStore } from './layers';
import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import { readSnapshot, readSnapshotV2, toSnapshot, toSnapshotV2 } from './snapshot';
import { snapshotSampleDocument } from './snapshotSample';

const blank = () => new CadDocument({ name: 'boş', layers: new LayerStore([{ id: 'x', name: 'X' }], 'x'), origin: { x: 0, y: 0 } });
/** The objects without their persistent ids, which v1 does not write (docs/adr/0014). */
const bare = (doc: CadDocument) => [...doc.all()].map(({ uid: _uid, ...e }) => e);

describe('.kcad snapshots', () => {
  it('writes and reads every object kind losslessly, down to the last bit', () => {
    const src = snapshotSampleDocument();
    const text = JSON.stringify(toSnapshot(src));
    const read = readSnapshot(text);
    expect(read.ok).toBe(true);
    if (!read.ok) return;
    const doc = blank();
    doc.replaceWith(read.content);
    // Objects, layer tree, settings, anchor, start view and styles are the same values.
    expect(bare(doc)).toEqual(bare(src));
    // v1 keeps no persistent ids: the file has none, and the drawing gives every object one.
    expect(JSON.parse(text).entities.some((e: object) => 'uid' in e)).toBe(false);
    expect([...doc.all()].every((e) => !!e.uid && doc.byUid(e.uid)?.id === e.id)).toBe(true);
    expect(doc.layers.tree).toEqual(src.layers.tree);
    expect(doc.layers.active.value).toBe('parsel');
    expect(doc.settings.toJSON()).toEqual(src.settings.toJSON());
    expect(doc.origin).toEqual(src.origin);
    expect(doc.homeView).toEqual(src.homeView);
    expect(doc.styles.value).toEqual(src.styles.value);
    expect(doc.name.value).toBe('Örnek pafta.kcad');
    // TM coordinates keep every bit through JSON (shortest round-trip form of float64).
    const p = [...doc.all()].find((e) => e.kind === 'polygon')!;
    const q = [...src.all()].find((e) => e.kind === 'polygon')!;
    expect(p.kind === 'polygon' && q.kind === 'polygon' && p.pts.every((v, i) => Object.is(v.x, q.pts[i].x) && Object.is(v.y, q.pts[i].y))).toBe(true);
    // A loaded drawing starts clean, with no history, and new objects get fresh ids.
    expect(doc.dirty.value).toBe(false);
    expect(doc.canUndo.value).toBe(false);
    const added = doc.add({ kind: 'point', layerId: 'cizim', p: { x: 0, y: 0 }, attrs: {} });
    expect(added.id).toBeGreaterThan(Math.max(...[...src.all()].map((e) => e.id)));
    // Writing it again gives the same file.
    doc.remove([added.id]);
    expect(JSON.parse(JSON.stringify(toSnapshot(doc)))).toEqual(JSON.parse(text));
  });
  it('matches the recorded fixture the Rust contracts read', () => {
    expect(JSON.parse(sampleText)).toEqual(JSON.parse(JSON.stringify(toSnapshot(snapshotSampleDocument()))));
    expect(readSnapshot(sampleText).ok).toBe(true);
  });
  it('reads a file from before work modes and drawing typefaces as hybrid and Barlow, and keeps an announced mode', () => {
    const good = JSON.parse(JSON.stringify(toSnapshot(snapshotSampleDocument())));
    delete good.settings.workspace;
    delete good.settings.drawingFont;
    const old = readSnapshot(JSON.stringify(good));
    expect(old.ok && old.content.settings.workspace).toBe('hybrid');
    expect(old.ok && old.content.settings.drawingFont).toBe('barlow');
    good.settings.workspace = 'plan3d';
    const soon = readSnapshot(JSON.stringify(good));
    expect(soon.ok && soon.content.settings.workspace).toBe('plan3d');
  });
  it('refuses what it does not know, saying what and where', () => {
    const good = JSON.parse(JSON.stringify(toSnapshot(snapshotSampleDocument())));
    const err = (mutate: (d: any) => void) => {
      const d = structuredClone(good);
      mutate(d);
      const r = readSnapshot(JSON.stringify(d));
      return r.ok ? null : r.error;
    };
    expect(readSnapshot('{').ok).toBe(false);
    expect(err((d) => (d.format = 'kentos-style'))).toContain('KentOS çizim dosyası değil');
    expect(err((d) => (d.version = 2))).toContain('sürümü 2');
    expect(err((d) => (d.settings.srid = 99999))).toContain('EPSG:99999 tanınmıyor');
    expect(err((d) => (d.settings.workspace = 'bim'))).toContain('Proje ayarları › çalışma modu');
    expect(err((d) => (d.settings.drawingFont = 'comic-sans'))).toContain('Proje ayarları › çizim yazı tipi');
    expect(err((d) => (d.entities[0].kind = 'blok'))).toContain('Nesne 1 › tür');
    expect(err((d) => (d.entities[1].layerId = 'yok'))).toContain('“yok” katmanı dosyada yok');
    expect(err((d) => (d.entities[1].a.x = '486512.34'))).toContain('a.x: sonlu bir sayı olmalı');
    expect(err((d) => (d.entities[2].bulges = [0, 0, 0, 0, 1]))).toContain('bulge: köşe sayısından uzun olamaz');
    expect(err((d) => (d.entities[4].id = d.entities[3].id))).toContain('benzersiz');
    expect(err((d) => (d.layers[0].children[0].style.lineType = 'wavy'))).toContain('çizgi tipi');
  });
});

describe('dirty follows what a file holds', () => {
  it('is set by edits, undo, redo and layer changes, and a save clears it only if nothing changed since', () => {
    const doc = snapshotSampleDocument();
    expect(doc.dirty.value).toBe(false);
    const p = doc.add({ kind: 'point', layerId: 'cizim', p: { x: 1, y: 1 }, attrs: {} });
    expect(doc.dirty.value).toBe(true);
    // A save of this revision makes it clean.
    doc.markSaved(doc.revision);
    expect(doc.dirty.value).toBe(false);
    // Undo and redo change the drawing, so they make it dirty again.
    doc.undo();
    expect(doc.dirty.value).toBe(true);
    doc.markSaved(doc.revision);
    doc.redo();
    expect(doc.dirty.value).toBe(true);
    doc.markSaved(doc.revision);
    // Visibility, locks and names are saved in the file too.
    doc.layers.toggleVisible('bina');
    expect(doc.dirty.value).toBe(true);
    // A slow save that started before an edit does not clear the newer edit.
    const writing = doc.revision;
    doc.update(p.id, { attrs: { Ad: 'P2' } });
    doc.markSaved(writing);
    expect(doc.dirty.value).toBe(true);
    // A failed transaction leaves it as it was.
    doc.markSaved(doc.revision);
    expect(() => doc.transact('x', () => {
      doc.add({ kind: 'point', layerId: 'cizim', p: { x: 2, y: 2 }, attrs: {} });
      throw new Error('hata');
    })).toThrow();
    expect(doc.dirty.value).toBe(false);
  });
});

describe('.kcad v2 content (docs/specs/kcad-v2.md)', () => {
  it('carries every object with its persistent id, the project id and the source record, and reads back into the same drawing', () => {
    const src = snapshotSampleDocument();
    src.projectId = '0192f5a0-0000-7000-8000-00000000abcd';
    src.migratedFrom = { format: 'kentos.document', version: 1, sourceSha256: 'ab'.repeat(32) };
    const snap = toSnapshotV2(src);
    expect([snap.format, snap.version, snap.entities.length, snap.uids.length]).toEqual(['kentos.document', 2, 13, 13]);
    // Slots stay in the drawing; objects carry no `uid` of their own, the list does.
    expect(snap.entities.some((e) => 'uid' in e)).toBe(false);
    expect(snap.uids).toEqual([...src.all()].map((e) => e.uid));
    const read = readSnapshotV2(structuredClone(snap));
    if (!read.ok) throw new Error(read.error);
    const doc = blank();
    doc.replaceWith(read.content);
    expect([...doc.all()].map((e) => e.uid)).toEqual(snap.uids);
    expect(bare(doc)).toEqual(bare(src));
    expect([doc.projectId, doc.migratedFrom]).toEqual([src.projectId, src.migratedFrom]);
  });

  it('refuses ids that are not one unique UUID per object, and a bad source record', () => {
    const good = toSnapshotV2(snapshotSampleDocument());
    const err = (mutate: (d: DocumentSnapshotV2) => void) => {
      const d = structuredClone(good);
      mutate(d);
      const r = readSnapshotV2(d);
      return r.ok ? null : r.error;
    };
    expect(err((d) => d.uids.pop())).toContain('13 nesne ama 12 kimlik var');
    expect(err((d) => (d.uids[4] = d.uids[3]))).toContain('Nesne 5 (circle) › kalıcı kimlik');
    expect(err((d) => (d.uids[0] = 'P1'))).toContain('Nesne 1 (point) › kalıcı kimlik');
    expect(err((d) => (d.projectId = 'proje'))).toContain('Proje kimliği');
    expect(err((d) => (d.migratedFrom = { format: 'kentos.document', version: 2, sourceSha256: 'ab'.repeat(32) }))).toContain('Göç kaynağı');
    expect(err((d) => ((d as { version: number }).version = 1))).toContain('sürümü 1');
  });
});
