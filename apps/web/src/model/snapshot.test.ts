import { describe, expect, it } from 'vitest';
import sampleText from '../../../../fixtures/document/v1/sample.json?raw';
import { CadDocument } from './document';
import { LayerStore } from './layers';
import { readSnapshot, toSnapshot } from './snapshot';
import { snapshotSampleDocument } from './snapshotSample';

const blank = () => new CadDocument({ name: 'boş', layers: new LayerStore([{ id: 'x', name: 'X' }], 'x'), origin: { x: 0, y: 0 } });

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
    expect([...doc.all()]).toEqual([...src.all()]);
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
