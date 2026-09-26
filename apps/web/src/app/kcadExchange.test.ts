import { describe, expect, it } from 'vitest';
import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import type { Entity } from '../contracts/generated/Entity';
import { difference } from '../io/kcad';
import { unpackSnapshot } from '../io/columns';
import { formatsBuilt, kcadInProcess } from '../io/testFormats';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { toSnapshotV2 } from '../model/snapshot';
import type { AppContext } from './context';
import { DocumentFiles, type DrawingFileHandle } from './fileIO';

/**
 * The cross-platform acceptance of TODOS.md §9 (docs/adr/0025): a drawing the
 * desktop saved opens on the web, is edited and saved; the desktop reads it
 * back without loss, edits it and saves; the web reads that back without
 * loss. The files between the two are committed (fixtures/kcad/v2/exchange),
 * and each platform's own save and open path writes and reads them, so the
 * chain holds without running both apps at once:
 *
 * 1. `migrated.kcad`: the web's v1 sample saved as v2, by the desktop and by
 *    the web alike, byte for byte (their tests; the Python writer made it first).
 * 2. Web (here): opens it, edits (P1's attribute, the line's end, the ellipse
 *    removed, a point added, the Bina layer unlocked), saves → `web-edited.kcad`.
 * 3. Desktop (apps/desktop/src/document.rs): reads that, finds every web edit
 *    and nothing else changed, edits (the parcel's attribute, the text
 *    removed, the Çizim layer shown), saves → `desktop-edited.kcad`.
 * 4. Web (here): reads that, finds every edit of both and nothing else changed.
 *
 * “Nothing else changed” is bit for bit: the file is decoded, the expected
 * edits are applied by hand to the drawing before them, and the two must
 * not differ in a single number or persistent id.
 *
 * KENTOS_WRITE_EXCHANGE=1 writes `web-edited.kcad` instead of comparing it
 * (the desktop's test writes its own file the same way).
 */

const env = (globalThis as unknown as { process: { env: Record<string, string | undefined> } }).process.env;
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer>; writeFileSync(u: URL, b: Uint8Array): void } } }).process.getBuiltinModule('node:fs');
const at = (name: string) => new URL(`../../../../fixtures/kcad/v2/${name}`, import.meta.url);
const read = (name: string) => new Uint8Array(fs.readFileSync(at(name)));

/** The persistent id the web's new point gets (a fixed maker, so the file is the same every run). */
const NEW_POINT = '0192f5a0-7c3e-7000-8000-00000000e001';

function memoryFile(name: string, bytes = new Uint8Array()) {
  const file = {
    name,
    bytes,
    getFile: async () => new Blob([file.bytes]),
    createWritable: async () => {
      const parts: Uint8Array[] = [];
      return {
        write: async (d: string | Uint8Array) => void parts.push(typeof d === 'string' ? new TextEncoder().encode(d) : d),
        close: async () => {
          const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
          let i = 0;
          for (const p of parts) (out.set(p, i), (i += p.length));
          file.bytes = out;
        },
      };
    },
  };
  return file satisfies DrawingFileHandle;
}

function web() {
  const doc = new CadDocument({ name: 'boş', layers: new LayerStore([{ id: 'x', name: 'X' }], 'x'), origin: { x: 0, y: 0 }, newUid: () => NEW_POINT });
  const noop = () => {};
  const ctx = {
    doc,
    log: { success: noop, warn: noop, error: (m: string) => { throw new Error(m); }, info: noop },
    tools: { activate: noop },
    selection: { clear: noop },
    view: { camera: { fit: noop }, zoomExtents: noop },
    cloud: { project: { value: null }, sync: { value: null }, autosaves: () => false, leave: async () => 0, detach: noop },
  } as unknown as AppContext;
  const files = new DocumentFiles(ctx);
  files.kcad = kcadInProcess;
  return { doc, files };
}

const decode = async (bytes: Uint8Array) => unpackSnapshot(await (await kcadInProcess()).decode(bytes));

/** The object with this persistent id, and its index. */
function byUid(s: DocumentSnapshotV2, uid: string): [Entity, number] {
  const i = s.uids.indexOf(uid);
  if (i < 0) throw new Error(`${uid} yok`);
  return [s.entities[i], i];
}

function removeUid(s: DocumentSnapshotV2, uid: string) {
  const [, i] = byUid(s, uid);
  s.entities.splice(i, 1);
  s.uids.splice(i, 1);
}

function layer(s: DocumentSnapshotV2, id: string) {
  const walk = (list: DocumentSnapshotV2['layers']): DocumentSnapshotV2['layers'][number] | undefined => {
    for (const n of list) {
      if (n.id === id) return n;
      const found = walk(n.children);
      if (found) return found;
    }
  };
  const found = walk(s.layers);
  if (!found) throw new Error(`${id} katmanı yok`);
  return found;
}

const uidOfKind = (s: DocumentSnapshotV2, kind: string) => s.uids[s.entities.findIndex((e) => e.kind === kind)];

/** The web's edits, applied by hand to the drawing before them: what the web must have saved. */
function webEdits(before: DocumentSnapshotV2): DocumentSnapshotV2 {
  const s = structuredClone(before);
  const [point] = byUid(s, uidOfKind(before, 'point'));
  point.attrs = { Ad: 'P1-web' };
  const [line] = byUid(s, uidOfKind(before, 'line'));
  if (line.kind === 'line') line.b = { x: line.b.x + 1.5, y: line.b.y };
  removeUid(s, uidOfKind(before, 'ellipse'));
  s.entities.push({ kind: 'point', id: 0, layerId: 'cizim', attrs: { Ad: 'P2-web' }, p: { x: 486520.125, y: 4420195.75 } });
  s.uids.push(NEW_POINT);
  layer(s, 'bina').locked = false;
  return s;
}

/** The desktop's edits (apps/desktop/src/document.rs), applied by hand: what the desktop must have saved. */
function desktopEdits(before: DocumentSnapshotV2): DocumentSnapshotV2 {
  const s = structuredClone(before);
  const [polygon] = byUid(s, uidOfKind(before, 'polygon'));
  polygon.attrs = { ...polygon.attrs, Nitelik: 'Arsa' };
  removeUid(s, uidOfKind(before, 'text'));
  layer(s, 'cizim').visible = true;
  return s;
}

describe.skipIf(!formatsBuilt)('KCAD v2 between the desktop and the web (TODOS.md §9 acceptance)', () => {
  it('the web opens what the desktop saved, edits it and saves it: web-edited.kcad', async () => {
    const { doc, files } = web();
    const migrated = read('migrated.kcad');
    expect(await files.load(migrated, memoryFile('Örnek pafta.kcad', migrated))).toBe(true);
    const find = (kind: string) => [...doc.all()].find((e) => e.kind === kind)!;
    doc.update(find('point').id, { attrs: { Ad: 'P1-web' } });
    const line = find('line');
    if (line.kind === 'line') doc.update(line.id, { b: { x: line.b.x + 1.5, y: line.b.y } });
    doc.remove([find('ellipse').id]);
    doc.add({ kind: 'point', layerId: 'cizim', p: { x: 486520.125, y: 4420195.75 }, attrs: { Ad: 'P2-web' } });
    doc.layers.toggleLocked('bina');
    // Saved where it was opened from, as Ctrl+S does for a v2 file.
    const out = memoryFile('Örnek pafta.kcad');
    files.handle = out;
    expect(await files.save()).toBe(true);
    expect(difference(await decode(out.bytes), webEdits(await decode(migrated)))).toBeNull();
    if (env.KENTOS_WRITE_EXCHANGE) fs.writeFileSync(at('exchange/web-edited.kcad'), out.bytes);
    expect(out.bytes).toEqual(read('exchange/web-edited.kcad'));
  });

  it('the web reads back what the desktop saved of it, without loss: desktop-edited.kcad', async () => {
    const { doc, files } = web();
    const bytes = read('exchange/desktop-edited.kcad');
    expect(await files.load(bytes, memoryFile('Örnek pafta.kcad', bytes))).toBe(true);
    const want = desktopEdits(await decode(read('exchange/web-edited.kcad')));
    const got = toSnapshotV2(doc);
    expect(difference(got, want)).toBeNull();
    expect(got.uids).toEqual(want.uids);
    // The web's new point and edits made the round trip too.
    expect(doc.byUid(NEW_POINT)?.attrs).toEqual({ Ad: 'P2-web' });
  });
});
