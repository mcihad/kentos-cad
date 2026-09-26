import type { V1Identities } from '../contracts/generated/V1Identities';
import { ColumnsReader, packDrawing, type PackedDrawing } from '../io/columns';
import { KcadError, sniffDrawing, type Dropped, type KcadCodec, type KcadProgress } from '../io/kcad';
import type { CadDocument, DocumentContent } from '../model/document';
import type { Entity } from '../model/entities';
import { DOCUMENT_FORMAT, DOCUMENT_VERSION, DrawingObjects, attachV1Identities, readDrawingHead, snapshotHead } from '../model/snapshot';
import { projectStylesProblem } from './cloud/incoming';

/**
 * A drawing file's bytes to document content, whatever version it is
 * (docs/specs/kcad-v2.md §8, docs/adr/0025): told apart by content, never by
 * name. A v2 file is decoded by the shared Rust codec in the formats worker
 * and brings every object's persistent id; a v1 file (JSON) is read here and
 * its ids, the project's id and the source record are derived from its
 * content in the worker meanwhile (docs/adr/0014). Either way the drawing is
 * checked like any file someone sent before anything is shown.
 *
 * Reading is staged (docs/adr/0030, TODOS.md FILE-20): the integrity check,
 * the project (name, layers, object count) before its objects, the objects
 * as the worker reads them, then the page's check of every object a chunk
 * per turn, so the page stays responsive and shows how far it is. A read
 * whose `stale` turns true (cancelled, overtaken by another) stops at the
 * next chunk and gives nothing; a read that fails gives only the reason.
 * Either way no half-read drawing leaves here.
 */

/** The `.kcad` v2 codec: the formats worker (tests run the module in process). */
export type DrawingCodec = KcadCodec;

export interface ReadDeps {
  codec: () => Promise<DrawingCodec>;
  identities: (text: string) => Promise<V1Identities>;
}

/** How far a read is: the codec's stages, then the page's check of the objects. */
export type ReadProgress = KcadProgress | { stage: 'objects'; done: number; total: number };

/** What a read hears and when it stops. */
export interface ReadWatch {
  progress?: (p: ReadProgress) => void;
  /** True once the read no longer matters (cancelled, another drawing opened): it stops at the next chunk. */
  stale?: () => boolean;
}

/** A drawing read from a file: its content, the version the file was in, and a warning worth saying. */
export type ReadDrawing =
  | { ok: true; content: DocumentContent; format: 'v1' | 'v2'; warning?: string }
  | { ok: false; error: string; cancelled?: true };

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const cancelled: ReadDrawing = { ok: false, error: 'Açma durduruldu; açık çizime dokunulmadı.', cancelled: true };

/** Reads a file's bytes into document content, or says why it cannot (the caller names the file). */
export async function readDrawing(bytes: Uint8Array, deps: ReadDeps, watch: ReadWatch = {}): Promise<ReadDrawing> {
  switch (sniffDrawing(bytes)) {
    case 'kcad':
    case 'kcad-damaged':
      return readV2(bytes, deps, watch);
    case 'json':
      return readV1(new TextDecoder().decode(bytes), deps, watch);
    case 'empty':
      return { ok: false, error: 'Dosya boş; içinde çizim yok. Başka bir dosya seçin ya da bir yedeği açın.' };
    case 'foreign':
      return { ok: false, error: 'KentOS çizim dosyası değil (ne KCAD v2 ne de v1). DXF ve koordinat listeleri Dosya → İçe aktar ile açılır.' };
  }
}

/** Lets the page paint and take input (a click on Vazgeç) between two chunks. */
export function yieldToPage(): Promise<void> {
  const s = (globalThis as { scheduler?: { yield?: () => Promise<void> } }).scheduler;
  if (s?.yield) return s.yield();
  return new Promise((resolve) => {
    const c = new MessageChannel();
    c.port1.onmessage = () => {
      c.port1.close();
      resolve();
    };
    c.port2.postMessage(null);
  });
}

/** Main-thread time between two yields while objects are checked (ms). */
const SLICE_MS = 12;

/**
 * Checks `count` objects (`next(i)` gives each), a slice of time per turn,
 * reporting between slices; null when the read went stale meanwhile.
 */
async function checkObjects(count: number, next: (i: number) => unknown, check: DrawingObjects, watch: ReadWatch): Promise<Entity[] | null> {
  const out = new Array<Entity>(count);
  let since = performance.now();
  for (let i = 0; i < count; i++) {
    out[i] = check.check(next(i), i);
    if ((i & 255) === 255 && performance.now() - since > SLICE_MS) {
      watch.progress?.({ stage: 'objects', done: i + 1, total: count });
      await yieldToPage();
      if (watch.stale?.()) return null;
      since = performance.now();
    }
  }
  watch.progress?.({ stage: 'objects', done: count, total: count });
  return out;
}

async function readV2(bytes: Uint8Array, deps: ReadDeps, watch: ReadWatch): Promise<ReadDrawing> {
  let packed: PackedDrawing;
  try {
    packed = await (await deps.codec()).decode(bytes, (p) => watch.progress?.(p));
  } catch (e) {
    if (e instanceof KcadError && e.code === 'cancelled') return cancelled;
    return { ok: false, error: message(e) };
  }
  if (watch.stale?.()) return cancelled;
  try {
    const head = readDrawingHead(JSON.parse(packed.head), 2);
    if (!head.ok) return head;
    const styles = projectStylesProblem(head.head.content.styles);
    if (styles) return { ok: false, error: `${styles}.` };
    const reader = new ColumnsReader(packed.columns);
    const entities = await checkObjects(reader.count, () => reader.next(), new DrawingObjects(head.head.leaves, 2), watch);
    if (!entities) return cancelled;
    if (!reader.done) throw new KcadError('bad_columns', 'Dosya biçim modülünden gelen çizimde nesnelerden sonra fazladan değer var; açık çizime dokunulmadı. Bu bir yazılım hatasıdır: durumu bildirin.');
    return { ok: true, content: { ...head.head.content, entities }, format: 'v2' };
  } catch (e) {
    return { ok: false, error: message(e) };
  }
}

async function readV1(text: string, deps: ReadDeps, watch: ReadWatch): Promise<ReadDrawing> {
  // Worked out in the formats worker while the page reads the drawing.
  const identities = deps.identities(text).then(
    (ids) => ({ ok: true as const, ids }),
    (e: unknown) => ({ ok: false as const, error: message(e) }),
  );
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { ok: false, error: 'Dosya JSON değil; bir KentOS çizimi (.kcad) seçin.' };
  }
  const head = readDrawingHead(data, 1);
  if (!head.ok) return head;
  // The project's own symbols are checked like any shared style file (untrusted data).
  const styles = projectStylesProblem(head.head.content.styles);
  if (styles) return { ok: false, error: `${styles}.` };
  const list = (data as { entities: unknown[] }).entities;
  watch.progress?.({ stage: 'project', name: head.head.content.name, layers: head.head.content.layers.length, objects: list.length });
  let entities: Entity[] | null;
  try {
    entities = await checkObjects(list.length, (i) => list[i], new DrawingObjects(head.head.leaves, 1), watch);
  } catch (e) {
    return { ok: false, error: message(e) };
  }
  if (!entities) return cancelled;
  const content: DocumentContent = { ...head.head.content, entities };
  const got = await identities;
  if (watch.stale?.()) return cancelled;
  const problem = got.ok ? attachV1Identities(content, got.ids) : got.error;
  if (problem)
    return {
      ok: true,
      content,
      format: 'v1',
      warning: `nesnelerin kalıcı kimlikleri dosyadan türetilemedi (${problem}); bu açılış için yeni kimlik verildi ve dosya yeniden açılınca kimlikler değişir. Dosyayı yeniden açmayı deneyin.`,
    };
  if (got.ok) {
    content.projectId = got.ids.project;
    content.migratedFrom = { format: DOCUMENT_FORMAT, version: DOCUMENT_VERSION, sourceSha256: got.ids.sourceSha256 };
  }
  return { ok: true, content, format: 'v1' };
}

/** A drawing's `.kcad` v2 bytes, the revision they hold and what KCAD v2 left out (in words). */
export interface EncodedDrawing {
  bytes: Uint8Array<ArrayBuffer>;
  revision: number;
  dropped: string | null;
}

/**
 * The drawing's `.kcad` v2 bytes and its revision, taken in one turn: the
 * drawing is packed into typed columns before anything else runs, so the
 * bytes hold the drawing of that moment (docs/adr/0030); the codec writes
 * and verifies them (the formats worker; tests run the module in process).
 * A local save and a cloud file project's revision are the same bytes.
 * Throws with the reason when the drawing cannot be written.
 */
export async function encodeDrawing(doc: CadDocument, codec: () => Promise<DrawingCodec>): Promise<EncodedDrawing> {
  const c = await codec();
  const revision = doc.revision;
  const { drawing, dropped } = packDrawing(snapshotHead(doc), doc.all());
  const bytes = await c.encode(drawing);
  return { bytes, revision, dropped: describeDropped(dropped) };
}

/** What a save left out, in words: fields KCAD v2 does not know, by where they were (`polyline.not` × 3). */
export function describeDropped(dropped: Dropped): string | null {
  const entries = Object.entries(dropped);
  if (!entries.length) return null;
  return entries.map(([where, n]) => (n > 1 ? `${where} (${n} kez)` : where)).join(', ');
}
