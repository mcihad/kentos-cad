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
 * The drawing as a v2 snapshot to write (docs/specs/kcad-v2.md): objects in
 * document order with their persistent ids, the project's id and source
 * record. The objects are shallow copies: the codec copies the snapshot
 * when it is handed over, in the same turn (the worker's message; the
 * in-process codec in tests).
 */
export function toSnapshotV2(doc: CadDocument): DocumentSnapshotV2 {
  const entities: ContractEntity[] = [];
  const uids: string[] = [];
  for (const { uid, ...e } of doc.all()) {
    if (!uid) throw new Error(`Nesne ${e.id} (${e.kind}): kalıcı kimliği yok; çizim KCAD v2 olarak yazılamaz.`);
    entities.push(e as ContractEntity);
    uids.push(uid);
  }
  const snap: DocumentSnapshotV2 = {
    format: DOCUMENT_FORMAT,
    version: DOCUMENT_VERSION_2,
    name: doc.name.value,
    settings: doc.settings.toJSON(),
    origin: { ...doc.origin },
    layers: structuredClone([...doc.layers.tree]) as ContractLayerNode[],
    activeLayer: doc.layers.active.value,
    entities,
    uids,
    styles: { items: structuredClone([...doc.styles.value.items]), categories: structuredClone([...doc.styles.value.categories]) },
  };
  if (doc.homeView) snap.homeView = { ...doc.homeView };
  if (doc.projectId) snap.projectId = doc.projectId;
  if (doc.migratedFrom) snap.migratedFrom = { ...doc.migratedFrom };
  return snap;
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
 * Reads a v2 drawing the codec decoded (the contract's JSON form) into
 * document content, checked like a v1 file (the drawing's own rules: known
 * CRS, layers, vertex counts, spec §6.10), its objects given the persistent
 * ids the file holds, one each.
 */
export function readSnapshotV2(data: unknown): ReadResult {
  try {
    return { ok: true, content: parse(data, DOCUMENT_VERSION_2) };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
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

class Bad extends Error {}
const fail = (where: string, what: string): never => {
  throw new Bad(`${where}: ${what}`);
};
const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const num = (v: unknown, where: string): number => (typeof v === 'number' && Number.isFinite(v) ? v : fail(where, 'sonlu bir sayı olmalı'));
const str = (v: unknown, where: string): string => (typeof v === 'string' ? v : fail(where, 'metin olmalı'));
const bool = (v: unknown, where: string): boolean => (typeof v === 'boolean' ? v : fail(where, 'doğru/yanlış olmalı'));
const oneOf = <T extends string>(v: unknown, values: readonly T[], where: string): T => (values.includes(v as T) ? (v as T) : fail(where, `şunlardan biri olmalı: ${values.join(', ')}`));
const vec = (v: unknown, where: string) => (isObj(v) ? { x: num(v.x, `${where}.x`), y: num(v.y, `${where}.y`) } : fail(where, 'nokta ({x, y}) olmalı'));
const vecs = (v: unknown, where: string, min: number) => {
  if (!Array.isArray(v) || v.length < min) fail(where, `en az ${min} noktalık liste olmalı`);
  return (v as unknown[]).map((p, i) => vec(p, `${where}[${i}]`));
};
const nums = (v: unknown, where: string) => (Array.isArray(v) ? v.map((n, i) => num(n, `${where}[${i}]`)) : fail(where, 'sayı listesi olmalı'));
const opt = <T>(v: unknown, read: (v: unknown) => T): T | undefined => (v === undefined ? undefined : read(v));

const KINDS = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'xline', 'ray', 'text', 'dimension', 'hatch'] as const;
const LINE_TYPES = ['continuous', 'dashed', 'dashdot', 'dotted'] as const;

function parse(data: unknown, version = DOCUMENT_VERSION): DocumentContent {
  if (!isObj(data) || data.format !== DOCUMENT_FORMAT) throw new Bad('KentOS çizim dosyası değil (format ≠ kentos.document).');
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
  const ids = new Set<number>();
  const entities = Array.isArray(data.entities) ? data.entities.map((e, i) => entity(e, `Nesne ${i + 1}`, leaves, ids)) : fail('Nesneler', 'liste olmalı');
  const styles = isObj(data.styles) && Array.isArray(data.styles.items) && Array.isArray(data.styles.categories) ? data.styles : fail('Proje stilleri', '{items, categories} olmalı');
  const hv = data.homeView;
  return {
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
    entities,
    styles: { items: styles.items as DocumentContent['styles']['items'], categories: styles.categories as DocumentContent['styles']['categories'] },
    ...(version === DOCUMENT_VERSION_2 ? identities(data, entities) : {}),
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
  const projectId = opt(data.projectId, (p) => (isUuid(p) ? p : fail('Proje kimliği', 'küçük harfli, tireli bir UUID olmalı')));
  const source = data.migratedFrom;
  const migratedFrom = opt(source, (s) =>
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

function entity(v: unknown, where: string, layers: ReadonlySet<string>, ids: Set<number>): Entity {
  if (!isObj(v)) return fail(where, 'nesne olmalı');
  // v1 has no persistent ids: one found here is not taken for the object's (they are derived; ADR 0014).
  if ('uid' in v) delete v.uid;
  const kind = oneOf(v.kind, KINDS, `${where} › tür`);
  const w = `${where} (${kind})`;
  const id = num(v.id, `${w} › kimlik`);
  if (!Number.isInteger(id) || id < 1 || ids.has(id)) fail(`${w} › kimlik`, 'benzersiz, pozitif tam sayı olmalı');
  ids.add(id);
  const layerId = str(v.layerId, `${w} › katman`);
  if (!layers.has(layerId)) fail(`${w} › katman`, `“${layerId}” katmanı dosyada yok`);
  const attrs = isObj(v.attrs) ? v.attrs : fail(`${w} › öznitelikler`, 'nesne olmalı');
  for (const [k, a] of Object.entries(attrs)) str(a, `${w} › öznitelik “${k}”`);
  // Geometry by kind: every coordinate a finite float64, lists long enough to draw.
  switch (kind) {
    case 'point':
      vec(v.p, `${w} › p`);
      opt(v.z, (z) => num(z, `${w} › z`));
      break;
    case 'line':
      vec(v.a, `${w} › a`);
      vec(v.b, `${w} › b`);
      break;
    case 'polyline':
    case 'polygon': {
      const pts = vecs(v.pts, `${w} › köşeler`, 2);
      const b = opt(v.bulges, (x) => nums(x, `${w} › bulge`));
      if (b && b.length > pts.length) fail(`${w} › bulge`, 'köşe sayısından uzun olamaz');
      if (v.holes !== undefined) {
        if (kind !== 'polygon' || !Array.isArray(v.holes)) fail(`${w} › adalar`, 'yalnızca kapalı alanda, liste olarak');
        (v.holes as unknown[]).forEach((h, i) => {
          if (!isObj(h)) fail(`${w} › ada ${i + 1}`, 'nesne olmalı');
          const hp = vecs((h as Record<string, unknown>).pts, `${w} › ada ${i + 1}`, 3);
          const hb = opt((h as Record<string, unknown>).bulges, (x) => nums(x, `${w} › ada ${i + 1} › bulge`));
          if (hb && hb.length > hp.length) fail(`${w} › ada ${i + 1} › bulge`, 'köşe sayısından uzun olamaz');
        });
      }
      break;
    }
    case 'circle':
      vec(v.c, `${w} › merkez`);
      if (num(v.r, `${w} › yarıçap`) <= 0) fail(`${w} › yarıçap`, 'pozitif olmalı');
      break;
    case 'arc':
      vec(v.c, `${w} › merkez`);
      num(v.r, `${w} › yarıçap`);
      num(v.a0, `${w} › başlangıç açısı`);
      num(v.a1, `${w} › bitiş açısı`);
      break;
    case 'ellipse':
      vec(v.c, `${w} › merkez`);
      vec(v.major, `${w} › büyük eksen`);
      num(v.ratio, `${w} › oran`);
      num(v.t0, `${w} › t0`);
      num(v.t1, `${w} › t1`);
      break;
    case 'spline':
      vecs(v.pts, `${w} › noktalar`, 2);
      bool(v.closed, `${w} › kapalı`);
      break;
    case 'xline':
    case 'ray':
      vec(v.p, `${w} › taban noktası`);
      vec(v.dir, `${w} › doğrultu`);
      break;
    case 'text':
      vec(v.p, `${w} › konum`);
      str(v.text, `${w} › metin`);
      num(v.height, `${w} › yükseklik`);
      num(v.rotation, `${w} › açı`);
      break;
    case 'dimension':
      vec(v.a, `${w} › a`);
      vec(v.b, `${w} › b`);
      num(v.offset, `${w} › ötelenme`);
      num(v.height, `${w} › yazı yüksekliği`);
      opt(v.text, (t) => str(t, `${w} › metin`));
      opt(v.style, (s) => oneOf(s, ['aligned', 'linear', 'angular', 'radius', 'diameter'] as const, `${w} › ölçü türü`));
      opt(v.angle, (a) => num(a, `${w} › açı`));
      opt(v.c, (c) => vec(c, `${w} › köşe`));
      break;
    case 'hatch': {
      vecs(v.ring, `${w} › sınır`, 3);
      opt(v.holes, (h) => (Array.isArray(h) ? h.forEach((r, i) => vecs(r, `${w} › ada ${i + 1}`, 3)) : fail(`${w} › adalar`, 'liste olmalı')));
      const p = isObj(v.pattern) ? v.pattern : fail(`${w} › desen`, 'eksik');
      oneOf(p.type, ['solid', 'lines', 'cross'] as const, `${w} › desen türü`);
      num(p.angle, `${w} › desen açısı`);
      num(p.spacing, `${w} › desen aralığı`);
      break;
    }
  }
  // Checked field by field above; the object is kept as read (optional fields included).
  return v as unknown as Entity;
}
