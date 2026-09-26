import type { DocumentSnapshotV1 } from '../contracts/generated/DocumentSnapshotV1';
import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { V1Identities } from '../contracts/generated/V1Identities';
import { isUuid } from '../core/uuid';
import { crsBySrid } from '../geo/crs';
import type { CadDocument, DocumentContent } from './document';
import type { Entity } from './entities';
import type { LayerInit } from './layers';
import { DRAWING_FONT_IDS, WORKSPACE_IDS } from './projectSettings';

/**
 * The drawing as a versioned file (`.kcad`): the v1 JSON (contract
 * `DocumentSnapshotV1`, docs/adr/0002-contracts-fixtures.md) and what the
 * binary v2 holds (`DocumentSnapshotV2`, docs/specs/kcad-v2.md), which the
 * shared Rust codec writes and reads in the formats worker (io/kcad.ts).
 * Writing is lossless (float64 coordinates, bulges, holes, ellipses,
 * dimensions, the project's styles), reading checks every field and refuses
 * anything it does not know with a message that says what and where; it
 * never guesses (CLAUDE.md §9.7). The project's style items are opaque here
 * (the style layer checks them).
 *
 * v1 keeps no persistent object ids (docs/adr/0014): they are not written,
 * and when a file is opened they are derived from its content by the Rust
 * contracts (`attachV1Identities`), the same every time. v2 keeps them, with
 * the project's id and, for a drawing migrated from v1, the source record.
 *
 * A file is read in stages (docs/adr/0030): its head first
 * (`readDrawingHead`), then its objects one at a time (`DrawingObjects`), so
 * a large drawing is checked in chunks the page can show and stop.
 */

export const DOCUMENT_FORMAT = 'kentos.document';
export const DOCUMENT_VERSION = 1;
export const DOCUMENT_VERSION_2 = 2;
export const DOCUMENT_EXTENSION = '.kcad';
/** No registered KCAD media type exists, so none is claimed (TODOS.md FILE-13). */
export const DOCUMENT_MIME = 'application/octet-stream';

/** The drawing as a snapshot; deep copies, so later edits do not change it. Persistent ids are not written in v1. */
export function toSnapshot(doc: CadDocument): DocumentSnapshotV1 {
  const entities: ContractEntity[] = [...doc.all()].map(({ uid: _uid, ...e }) => structuredClone(e));
  const layers: ContractLayerNode[] = structuredClone([...doc.layers.tree]);
  const snap: DocumentSnapshotV1 = {
    format: DOCUMENT_FORMAT,
    version: DOCUMENT_VERSION,
    name: doc.name.value,
    settings: doc.settings.toJSON(),
    origin: { ...doc.origin },
    layers,
    activeLayer: doc.layers.active.value,
    entities,
    styles: { items: structuredClone([...doc.styles.value.items]), categories: structuredClone([...doc.styles.value.categories]) },
  };
  if (doc.homeView) snap.homeView = { ...doc.homeView };
  return snap;
}

/**
 * Everything a v2 file holds but the objects (docs/specs/kcad-v2.md): the
 * name, settings, anchor, start view, layer tree, active layer, the
 * project's styles, id and source record. Copies; the objects are packed
 * from the drawing itself in the same turn (io/columns.ts).
 */
export function snapshotHead(doc: CadDocument): Omit<DocumentSnapshotV2, 'entities' | 'uids'> {
  const head: Omit<DocumentSnapshotV2, 'entities' | 'uids'> = {
    format: DOCUMENT_FORMAT,
    version: DOCUMENT_VERSION_2,
    name: doc.name.value,
    settings: doc.settings.toJSON(),
    origin: { ...doc.origin },
    layers: structuredClone([...doc.layers.tree]) as ContractLayerNode[],
    activeLayer: doc.layers.active.value,
    styles: { items: structuredClone([...doc.styles.value.items]), categories: structuredClone([...doc.styles.value.categories]) },
  };
  if (doc.homeView) head.homeView = { ...doc.homeView };
  if (doc.projectId) head.projectId = doc.projectId;
  if (doc.migratedFrom) head.migratedFrom = { ...doc.migratedFrom };
  return head;
}

/**
 * The drawing as a v2 snapshot, the contract's JSON form: objects in
 * document order with their persistent ids, the project's id and source
 * record. The objects are shallow copies. Saving packs the drawing instead
 * (`snapshotHead` and io/columns.ts); tests compare with this form.
 */
export function toSnapshotV2(doc: CadDocument): DocumentSnapshotV2 {
  const entities: ContractEntity[] = [];
  const uids: string[] = [];
  for (const { uid, ...e } of doc.all()) {
    if (!uid) throw new Error(`Nesne ${e.id} (${e.kind}): kalıcı kimliği yok; çizim KCAD v2 olarak yazılamaz.`);
    entities.push(e as ContractEntity);
    uids.push(uid);
  }
  return { ...snapshotHead(doc), entities, uids };
}

export type ReadResult = { ok: true; content: DocumentContent } | { ok: false; error: string };

/** Reads a snapshot's text into document content, or says why it cannot. */
export function readSnapshot(text: string): ReadResult {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { ok: false, error: 'Dosya JSON değil; bir KentOS çizimi (.kcad) seçin.' };
  }
  try {
    return { ok: true, content: parse(data) };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

/**
 * Reads a v2 drawing in the contract's JSON form into document content,
 * checked like a v1 file (the drawing's own rules: known CRS, layers, vertex
 * counts, spec §6.10), its objects given the persistent ids the drawing
 * holds, one each. The app reads a file in stages instead
 * (`readDrawingHead`, `DrawingObjects`): the same rules.
 */
export function readSnapshotV2(data: unknown): ReadResult {
  try {
    return { ok: true, content: parse(data, DOCUMENT_VERSION_2) };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

/** A drawing's head, checked: everything but its objects, and the layers objects may be on. */
export interface DrawingHeadRead {
  content: Omit<DocumentContent, 'entities'>;
  /** The ids of the layers (not groups) an object may be on. */
  leaves: ReadonlySet<string>;
}

/**
 * A drawing's head checked like a file's (docs/adr/0030): format and
 * version, the project settings (a known CRS), the layer tree (at least one
 * layer), the project's styles and start view; for v2 also the project's id
 * and source record. A v2 head is the contract's JSON of everything but the
 * objects; a v1 file's is the file itself (its objects are checked after).
 */
export function readDrawingHead(data: unknown, version: 1 | 2): { ok: true; head: DrawingHeadRead } | { ok: false; error: string } {
  try {
    if (!isObj(data)) throw new Bad('KentOS çizim dosyası değil (format ≠ kentos.document).');
    const read = head(data, version);
    return { ok: true, head: { content: { ...read.content, ...(version === DOCUMENT_VERSION_2 ? source(data) : {}) }, leaves: read.leaves } };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

/**
 * A drawing's objects checked one at a time as they are read, so a large
 * file is checked in chunks the page can show and stop: each like a file's
 * object, on a layer of the file. A v2 object keeps its persistent id, a
 * lowercase UUID given once; a v1 file has none (derived after, ADR 0014).
 * `check` returns the object or throws the reason with its place.
 */
export class DrawingObjects {
  private readonly leaves: ReadonlySet<string>;
  private readonly v2: boolean;
  private readonly slots = new Set<number>();
  private readonly uids = new Set<string>();
  constructor(leaves: ReadonlySet<string>, version: 1 | 2) {
    this.leaves = leaves;
    this.v2 = version === DOCUMENT_VERSION_2;
  }
  check(v: unknown, index: number): Entity {
    try {
      const uid = isObj(v) ? v.uid : undefined;
      const e = entity(v, `Nesne ${index + 1}`, this.leaves, this.slots, this.v2);
      if (this.v2) {
        if (!isUuid(uid) || this.uids.has(uid)) fail(`Nesne ${index + 1} (${e.kind}) › kalıcı kimlik`, 'küçük harfli, tireli, benzersiz bir UUID olmalı');
        this.uids.add(uid as string);
      }
      return e;
    } catch (e) {
      throw e instanceof Bad ? new Error(e.message) : e;
    }
  }
}

/**
 * Gives the objects of a v1 drawing just read (`readSnapshot`) the
 * persistent ids the Rust contracts derived from the same text
 * (`v1Identities`, docs/adr/0014): object by object, in file order, each
 * matched by its local id. Returns what does not match instead, and then
 * changes nothing.
 */
export function attachV1Identities(content: DocumentContent, ids: V1Identities): string | null {
  const list = ids.entities;
  if (list.length !== content.entities.length) return `kimlik listesinde ${list.length} nesne var, çizimde ${content.entities.length}`;
  for (let i = 0; i < list.length; i++) {
    const e = content.entities[i];
    if (list[i].id !== e.id || !isUuid(list[i].uid)) return `${i + 1}. nesnenin kimliği uyuşmuyor (yerel ${e.id}, listede ${list[i].id})`;
  }
  for (let i = 0; i < list.length; i++) content.entities[i].uid = list[i].uid;
  return null;
}

/**
 * Objects that come from outside a drawing file (a format import) checked
 * exactly like a file's: every field, finite coordinates, lists long enough
 * to draw, a layer among `layers`, unique positive ids. The objects are
 * returned as given; `where` names them in the message ("Nesne 12 (arc) ›
 * yarıçap: …").
 */
export function readEntityList(list: readonly unknown[], layers: ReadonlySet<string>, where = 'Nesne'): { ok: true; entities: Entity[] } | { ok: false; error: string } {
  const ids = new Set<number>();
  try {
    return { ok: true, entities: list.map((e, i) => entity(e, `${where} ${i + 1}`, layers, ids)) };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

// ── Validation ─────────────────────────────────────────────────────────
// A large drawing has millions of vertices: the checks make no string and
// no copy unless one fails. `w` is an object's own place ("Nesne 12 (arc)"),
// `f` a field of it; `at(w, f)` is made only for a message.

class Bad extends Error {}
const fail = (where: string, what: string): never => {
  throw new Bad(`${where}: ${what}`);
};
const at = (w: string, f: string) => `${w} › ${f}`;
const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const num = (v: unknown, where: string): number => (finite(v) ? v : fail(where, 'sonlu bir sayı olmalı'));
const str = (v: unknown, where: string): string => (typeof v === 'string' ? v : fail(where, 'metin olmalı'));
const bool = (v: unknown, where: string): boolean => (typeof v === 'boolean' ? v : fail(where, 'doğru/yanlış olmalı'));
const oneOf = <T extends string>(v: unknown, values: readonly T[], where: string): T => (values.includes(v as T) ? (v as T) : fail(where, `şunlardan biri olmalı: ${values.join(', ')}`));
const vec = (v: unknown, where: string) => (isObj(v) ? { x: num(v.x, `${where}.x`), y: num(v.y, `${where}.y`) } : fail(where, 'nokta ({x, y}) olmalı'));
const opt = <T>(v: unknown, read: (v: unknown) => T): T | undefined => (v === undefined ? undefined : read(v));
// An object's fields, checked in place: the message is made only when a check fails.
const numAt = (v: unknown, w: string, f: string): number => (finite(v) ? v : num(v, at(w, f)));
const strAt = (v: unknown, w: string, f: string): string => (typeof v === 'string' ? v : str(v, at(w, f)));
const pointAt = (v: unknown, w: string, f: string): void => {
  if (!isObj(v) || !finite(v.x) || !finite(v.y)) vec(v, at(w, f));
};
/** A list of at least `min` points; its length. */
const pointsAt = (v: unknown, w: string, f: string, min: number): number => {
  if (!Array.isArray(v) || v.length < min) return fail(at(w, f), `en az ${min} noktalık liste olmalı`);
  for (let i = 0; i < v.length; i++) {
    const p: unknown = v[i];
    if (!isObj(p) || !finite(p.x) || !finite(p.y)) vec(p, `${at(w, f)}[${i}]`);
  }
  return v.length;
};
/** A list of numbers; its length. */
const numbersAt = (v: unknown, w: string, f: string): number => {
  if (!Array.isArray(v)) return fail(at(w, f), 'sayı listesi olmalı');
  for (let i = 0; i < v.length; i++) if (!finite(v[i])) num(v[i], `${at(w, f)}[${i}]`);
  return v.length;
};

const KINDS = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'xline', 'ray', 'text', 'dimension', 'hatch'] as const;
const LINE_TYPES = ['continuous', 'dashed', 'dashdot', 'dotted'] as const;

function parse(data: unknown, version = DOCUMENT_VERSION): DocumentContent {
  if (!isObj(data)) throw new Bad('KentOS çizim dosyası değil (format ≠ kentos.document).');
  const { content, leaves } = head(data, version);
  const ids = new Set<number>();
  const entities = (data.entities as unknown[]).map((e, i) => entity(e, `Nesne ${i + 1}`, leaves, ids));
  return { ...content, entities, ...(version === DOCUMENT_VERSION_2 ? identities(data, entities) : {}) };
}

/** Everything a drawing file holds but its objects, checked; and the layers objects may be on. */
function head(data: Record<string, unknown>, version: number): { content: Omit<DocumentContent, 'entities'>; leaves: Set<string> } {
  if (data.format !== DOCUMENT_FORMAT) throw new Bad('KentOS çizim dosyası değil (format ≠ kentos.document).');
  if (data.version !== version)
    throw new Bad(typeof data.version === 'number' ? `Çizim dosyası sürümü ${data.version} bu uygulamada okunamıyor (desteklenen: ${version}).` : 'Çizim dosyasında sürüm yok.');
  const settings = isObj(data.settings) ? data.settings : fail('Proje ayarları', 'eksik');
  const srid = num(settings.srid, 'Proje ayarları › SRID');
  // A drawing's CRS is never guessed: an unknown SRID stops the open (CLAUDE.md §9.7).
  if (!crsBySrid(srid)) fail('Proje ayarları › SRID', `EPSG:${srid} tanınmıyor; koordinat sistemi tahmin edilmez`);
  const layers = Array.isArray(data.layers) ? data.layers.map((n, i) => layer(n, `Katman ${i + 1}`)) : fail('Katmanlar', 'liste olmalı');
  const leaves = new Set<string>();
  const walk = (list: LayerInit[]) => list.forEach((l) => (l.type === 'group' ? walk(l.children ?? []) : leaves.add(l.id!)));
  walk(layers);
  if (!leaves.size) fail('Katmanlar', 'en az bir katman olmalı');
  if (!Array.isArray(data.entities)) fail('Nesneler', 'liste olmalı');
  const styles = isObj(data.styles) && Array.isArray(data.styles.items) && Array.isArray(data.styles.categories) ? data.styles : fail('Proje stilleri', '{items, categories} olmalı');
  const hv = data.homeView;
  return {
    leaves,
    content: {
      name: str(data.name, 'Ad'),
      settings: {
        srid,
        lengthDecimals: num(settings.lengthDecimals, 'Proje ayarları › uzunluk hassasiyeti'),
        areaDecimals: num(settings.areaDecimals, 'Proje ayarları › alan hassasiyeti'),
        areaUnit: oneOf(settings.areaUnit, ['m2', 'donum', 'ha'] as const, 'Proje ayarları › alan birimi'),
        angleUnit: oneOf(settings.angleUnit, ['grad', 'deg'] as const, 'Proje ayarları › açı birimi'),
        plotScale: num(settings.plotScale, 'Proje ayarları › çizim ölçeği'),
        // Files written before work modes have none: they open as they always did (hybrid).
        workspace: settings.workspace === undefined ? 'hybrid' : oneOf(settings.workspace, WORKSPACE_IDS, 'Proje ayarları › çalışma modu'),
        // And before drawing typefaces: Barlow, as they were drawn.
        drawingFont: settings.drawingFont === undefined ? 'barlow' : oneOf(settings.drawingFont, DRAWING_FONT_IDS, 'Proje ayarları › çizim yazı tipi'),
      },
      origin: vec(data.origin, 'Yerel orijin'),
      homeView: isObj(hv) ? { minX: num(hv.minX, 'Başlangıç görünümü'), minY: num(hv.minY, 'Başlangıç görünümü'), maxX: num(hv.maxX, 'Başlangıç görünümü'), maxY: num(hv.maxY, 'Başlangıç görünümü') } : null,
      layers,
      activeLayer: str(data.activeLayer, 'Etkin katman'),
      styles: { items: styles.items as DocumentContent['styles']['items'], categories: styles.categories as DocumentContent['styles']['categories'] },
    },
  };
}

const SHA256 = /^[0-9a-f]{64}$/;

/** A v2 drawing's ids: one persistent id per object, unique, given to the objects; the project's id and source record. */
function identities(data: Record<string, unknown>, entities: Entity[]): Pick<DocumentContent, 'projectId' | 'migratedFrom'> {
  const uids = Array.isArray(data.uids) ? data.uids : fail('Kalıcı kimlikler', 'liste olmalı');
  if (uids.length !== entities.length) fail('Kalıcı kimlikler', `${entities.length} nesne ama ${uids.length} kimlik var`);
  const seen = new Set<string>();
  uids.forEach((uid, i) => {
    if (!isUuid(uid) || seen.has(uid)) fail(`Nesne ${i + 1} (${entities[i].kind}) › kalıcı kimlik`, 'küçük harfli, tireli, benzersiz bir UUID olmalı');
    seen.add(uid);
  });
  entities.forEach((e, i) => (e.uid = uids[i] as string));
  return source(data);
}

/** A v2 drawing's project id and source record, checked. */
function source(data: Record<string, unknown>): Pick<DocumentContent, 'projectId' | 'migratedFrom'> {
  const projectId = opt(data.projectId, (p) => (isUuid(p) ? p : fail('Proje kimliği', 'küçük harfli, tireli bir UUID olmalı')));
  const migratedFrom = opt(data.migratedFrom, (s) =>
    isObj(s) && s.format === DOCUMENT_FORMAT && s.version === DOCUMENT_VERSION && typeof s.sourceSha256 === 'string' && SHA256.test(s.sourceSha256)
      ? { format: DOCUMENT_FORMAT, version: DOCUMENT_VERSION, sourceSha256: s.sourceSha256 }
      : fail('Göç kaynağı', 'kentos.document sürüm 1 ve 64 onaltılık haneli SHA-256 olmalı'),
  );
  return { projectId: projectId ?? null, migratedFrom: migratedFrom ?? null };
}

function layer(v: unknown, where: string): LayerInit {
  if (!isObj(v)) return fail(where, 'nesne olmalı');
  const name = str(v.name, `${where} › ad`);
  const w = `${where} (“${name}”)`;
  const type = oneOf(v.type, ['group', 'layer'] as const, `${w} › tür`);
  const s = isObj(v.style) ? v.style : fail(`${w} › stil`, 'eksik');
  const children = Array.isArray(v.children) ? v.children.map((c, i) => layer(c, `${w} › ${i + 1}`)) : fail(`${w} › alt katmanlar`, 'liste olmalı');
  return {
    id: str(v.id, `${w} › kimlik`),
    name,
    type,
    visible: bool(v.visible, `${w} › görünür`),
    locked: bool(v.locked, `${w} › kilit`),
    expanded: bool(v.expanded, `${w} › açık`),
    // The simple look is checked; label, point and renderer are kept as they are (the style layer reads them).
    style: { ...(s as object), color: str(s.color, `${w} › renk`), lineType: oneOf(s.lineType, LINE_TYPES, `${w} › çizgi tipi`), lineWeight: num(s.lineWeight, `${w} › kalınlık`) },
    children,
  };
}

/**
 * One object checked field by field: kind, a unique positive slot, a layer
 * of the file, text attributes, and its geometry by kind (every coordinate a
 * finite float64, lists long enough to draw). The object is kept as read.
 * `keepUid`: a v2 object's persistent id stays (its reader checks it); a v1
 * file has none, so one found there is not taken (ADR 0014).
 */
function entity(v: unknown, where: string, layers: ReadonlySet<string>, ids: Set<number>, keepUid = false): Entity {
  if (!isObj(v)) return fail(where, 'nesne olmalı');
  if (!keepUid && 'uid' in v) delete v.uid;
  const kind = (KINDS as readonly unknown[]).includes(v.kind) ? (v.kind as (typeof KINDS)[number]) : oneOf(v.kind, KINDS, at(where, 'tür'));
  const w = `${where} (${kind})`;
  const id = v.id;
  if (!finite(id) || !Number.isInteger(id) || id < 1 || ids.has(id)) {
    num(id, at(w, 'kimlik'));
    fail(at(w, 'kimlik'), 'benzersiz, pozitif tam sayı olmalı');
  }
  ids.add(id as number);
  const layerId = strAt(v.layerId, w, 'katman');
  if (!layers.has(layerId)) fail(at(w, 'katman'), `“${layerId}” katmanı dosyada yok`);
  const attrs = isObj(v.attrs) ? v.attrs : fail(at(w, 'öznitelikler'), 'nesne olmalı');
  for (const k in attrs) if (typeof attrs[k] !== 'string') str(attrs[k], at(w, `öznitelik “${k}”`));
  // Geometry by kind: every coordinate a finite float64, lists long enough to draw.
  switch (kind) {
    case 'point':
      pointAt(v.p, w, 'p');
      if (v.z !== undefined) numAt(v.z, w, 'z');
      break;
    case 'line':
      pointAt(v.a, w, 'a');
      pointAt(v.b, w, 'b');
      break;
    case 'polyline':
    case 'polygon': {
      const n = pointsAt(v.pts, w, 'köşeler', 2);
      if (v.bulges !== undefined && numbersAt(v.bulges, w, 'bulge') > n) fail(at(w, 'bulge'), 'köşe sayısından uzun olamaz');
      if (v.holes !== undefined) {
        if (kind !== 'polygon' || !Array.isArray(v.holes)) fail(at(w, 'adalar'), 'yalnızca kapalı alanda, liste olarak');
        (v.holes as unknown[]).forEach((h, i) => {
          if (!isObj(h)) fail(at(w, `ada ${i + 1}`), 'nesne olmalı');
          const ring = h as Record<string, unknown>;
          const k = pointsAt(ring.pts, w, `ada ${i + 1}`, 3);
          if (ring.bulges !== undefined && numbersAt(ring.bulges, w, `ada ${i + 1} › bulge`) > k) fail(at(w, `ada ${i + 1} › bulge`), 'köşe sayısından uzun olamaz');
        });
      }
      break;
    }
    case 'circle':
      pointAt(v.c, w, 'merkez');
      if (numAt(v.r, w, 'yarıçap') <= 0) fail(at(w, 'yarıçap'), 'pozitif olmalı');
      break;
    case 'arc':
      pointAt(v.c, w, 'merkez');
      numAt(v.r, w, 'yarıçap');
      numAt(v.a0, w, 'başlangıç açısı');
      numAt(v.a1, w, 'bitiş açısı');
      break;
    case 'ellipse':
      pointAt(v.c, w, 'merkez');
      pointAt(v.major, w, 'büyük eksen');
      numAt(v.ratio, w, 'oran');
      numAt(v.t0, w, 't0');
      numAt(v.t1, w, 't1');
      break;
    case 'spline':
      pointsAt(v.pts, w, 'noktalar', 2);
      if (typeof v.closed !== 'boolean') bool(v.closed, at(w, 'kapalı'));
      break;
    case 'xline':
    case 'ray':
      pointAt(v.p, w, 'taban noktası');
      pointAt(v.dir, w, 'doğrultu');
      break;
    case 'text':
      pointAt(v.p, w, 'konum');
      strAt(v.text, w, 'metin');
      numAt(v.height, w, 'yükseklik');
      numAt(v.rotation, w, 'açı');
      break;
    case 'dimension':
      pointAt(v.a, w, 'a');
      pointAt(v.b, w, 'b');
      numAt(v.offset, w, 'ötelenme');
      numAt(v.height, w, 'yazı yüksekliği');
      if (v.text !== undefined) strAt(v.text, w, 'metin');
      if (v.style !== undefined) oneOf(v.style, ['aligned', 'linear', 'angular', 'radius', 'diameter'] as const, at(w, 'ölçü türü'));
      if (v.angle !== undefined) numAt(v.angle, w, 'açı');
      if (v.c !== undefined) pointAt(v.c, w, 'köşe');
      break;
    case 'hatch': {
      pointsAt(v.ring, w, 'sınır', 3);
      if (v.holes !== undefined) {
        if (!Array.isArray(v.holes)) fail(at(w, 'adalar'), 'liste olmalı');
        (v.holes as unknown[]).forEach((r, i) => pointsAt(r, w, `ada ${i + 1}`, 3));
      }
      const p = isObj(v.pattern) ? v.pattern : fail(at(w, 'desen'), 'eksik');
      oneOf(p.type, ['solid', 'lines', 'cross'] as const, at(w, 'desen türü'));
      numAt(p.angle, w, 'desen açısı');
      numAt(p.spacing, w, 'desen aralığı');
      break;
    }
  }
  // Checked field by field above; the object is kept as read (optional fields included).
  return v as unknown as Entity;
}
