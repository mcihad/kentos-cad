import type { LayerField } from '../contracts/generated/LayerField';
import { layerFieldsProblem } from './layerFields';
import { tableProblem, type TableShape } from './tables';
import { imageProblem, type ImageShape } from './imageRules';
import { rasterProblem, type RasterShape } from './rasterRules';
import type { CrsDefinition } from '../contracts/generated/CrsDefinition';
import type { DatumTransform } from '../contracts/generated/DatumTransform';
import type { DimensionStyleDef } from '../contracts/generated/DimensionStyleDef';
import type { LayerState } from '../contracts/generated/LayerState';
import type { SurveySettings } from '../contracts/generated/SurveySettings';
import type { TopologySettings } from '../contracts/generated/TopologySettings';
import type { TextStyleDef } from '../contracts/generated/TextStyleDef';
import type { DocumentSnapshotV1 } from '../contracts/generated/DocumentSnapshotV1';
import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import type { LayerNode as ContractLayerNode } from '../contracts/generated/LayerNode';
import type { V1Identities } from '../contracts/generated/V1Identities';
import { isUuid } from '../core/uuid';
import { crsBySrid } from '../geo/crs';
import { DIMENSION_ARROWS, faceProblem, lookProblem, type DimensionLook, type TextFace } from './annotationStyles';
import { blockFaultMessage, definitionsFault, type AttributeDefinition, type BlockDefinition } from './blocks';
import type { CadDocument, DocumentContent } from './document';
import { MAX_LINE_WEIGHT, MAX_WIDTH_FACTOR, TEXT_ALIGNS, widthFactorOk, type Entity, type HatchAssoc, type HatchPattern, type TextAlign } from './entities';
import { assocProblem, patternProblem } from './hatchRules';
import type { LayerInit, LayerSnap } from './layers';
import { DRAWING_FONT_IDS, DRAWING_UNIT_IDS, LEGACY_HYBRID, WORKSPACE_IDS } from './projectSettings';

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
  if (doc.blocks.value.length) snap.blocks = structuredClone([...doc.blocks.value]) as DocumentSnapshotV1['blocks'];
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
  // The definitions travel with the head, not in the columns (docs/adr/0144).
  if (doc.blocks.value.length) head.blocks = structuredClone([...doc.blocks.value]) as DocumentSnapshotV2['blocks'];
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
 * counts, spec §6.11), its objects given the persistent ids the drawing
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
  private readonly blocks: ReadonlySet<string>;
  private readonly slots = new Set<number>();
  private readonly uids = new Set<string>();
  /** `blocks`: the definitions of the drawing's head, which its inserts must name. */
  constructor(leaves: ReadonlySet<string>, version: 1 | 2, blocks: readonly BlockDefinition[] = []) {
    this.leaves = leaves;
    this.v2 = version === DOCUMENT_VERSION_2;
    this.blocks = new Set(blocks.map((b) => b.id));
  }
  check(v: unknown, index: number): Entity {
    try {
      const uid = isObj(v) ? v.uid : undefined;
      const e = entity(v, `Nesne ${index + 1}`, this.leaves, this.slots, this.v2);
      if (e.kind === 'insert' && !this.blocks.has(e.block)) fail(`Nesne ${index + 1} (insert) › blok`, `${e.block} çizimde tanımlı değil`);
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
 * yarıçap: …"), counting from `before` + 1 (a list checked in chunks).
 */
export function readEntityList(list: readonly unknown[], layers: ReadonlySet<string>, where = 'Nesne', before = 0): { ok: true; entities: Entity[] } | { ok: false; error: string } {
  const ids = new Set<number>();
  try {
    return { ok: true, entities: list.map((e, i) => entity(e, `${where} ${before + i + 1}`, layers, ids)) };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

/**
 * Block definitions that come from outside a drawing file (a DXF import,
 * docs/adr/0144 §5) checked exactly like a file's: every field of each and
 * of its objects, then the block rules over the list.
 */
export function readBlockDefinitions(list: unknown): { ok: true; blocks: BlockDefinition[] } | { ok: false; error: string } {
  try {
    return { ok: true, blocks: definitions(list) };
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
/**
 * A second coordinate system's SRID: a whole number, another system than the project's own, never the second of a
 * project without a system (docs/adr/0167 §1; a project's own definition is a system, 0168 §1).
 */
const second = (v: unknown, srid: number, custom: boolean): number => {
  const where = 'Proje ayarları › ikinci koordinat sistemi';
  const s = num(v, where);
  if (!Number.isInteger(s) || s <= 0 || s > 0xffffffff) fail(where, 'bir EPSG kodu olmalı');
  return s === srid || (srid === 0 && !custom) ? fail(where, 'projeninkinden başka bir sistem olmalı; yerel projenin ikinci sistemi olmaz') : s;
};
/** The project's survey settings (docs/adr/0169 §3): their shape here, their rules where they are written and read (the KCAD codec). */
const surveyOf = (v: unknown): SurveySettings =>
  isObj(v) && Object.values(v).every(finite) ? (v as SurveySettings) : fail('Proje ayarları › ölçme', 'sayılardan oluşan bir harita olmalı');
/** A coordinate system definition of the project's (docs/adr/0168 §1): its shape here, its rules where it is written and read (the KCAD codec). */
const definition = (v: unknown, where: string): CrsDefinition =>
  isObj(v) && typeof v.name === 'string' && isObj(v.system) ? (v as unknown as CrsDefinition) : fail(where, 'bir koordinat sistemi tanımı ({name, system}) olmalı');
/**
 * A project's style table (docs/adr/0183): objects whose fields have the contract's types, as serde reads them; the
 * values' rules are the settings' own (`sanitizedTextStyles`, `sanitizedDimensionStyles` drop a style breaking them).
 */
function stylesOf<T>(v: unknown, kind: 'text' | 'dimension'): T[] {
  const where = `Proje ayarları › ${kind === 'text' ? 'yazı' : 'ölçü'} stilleri`;
  if (!Array.isArray(v)) return fail(where, 'liste olmalı');
  return v.map((s, i) => {
    const w = `${where} › ${i + 1}`;
    if (!isObj(s)) return fail(w, 'nesne olmalı');
    str(s.id, at(w, 'kimlik'));
    str(s.name, at(w, 'ad'));
    if (kind === 'text') {
      oneOf(s.font, DRAWING_FONT_IDS, at(w, 'yazı tipi'));
      for (const f of ['bold', 'italic'] as const) if (s[f] !== undefined) bool(s[f], at(w, f));
      for (const f of ['oblique', 'height', 'widthFactor'] as const) if (s[f] !== undefined) num(s[f], at(w, f));
      if (s.fontFile !== undefined) str(s.fontFile, at(w, 'yazı tipi dosyası'));
    } else {
      num(s.height, at(w, 'yükseklik'));
      if (s.arrow !== undefined) oneOf(s.arrow, DIMENSION_ARROWS, at(w, 'ok'));
      for (const f of ['arrowSize', 'extOffset', 'extBeyond', 'textGap'] as const) if (s[f] !== undefined) num(s[f], at(w, f));
      if (s.textPlace !== undefined) oneOf(s.textPlace, ['centre'] as const, at(w, 'değerin yeri'));
      if (s.decimals !== undefined && !(Number.isInteger(s.decimals) && (s.decimals as number) >= 0)) fail(at(w, 'basamak'), 'negatif olmayan tam sayı olmalı');
      if (s.unit !== undefined) oneOf(s.unit, DRAWING_UNIT_IDS, at(w, 'birim'));
      for (const f of ['prefix', 'suffix'] as const) if (s[f] !== undefined) str(s[f], at(w, f));
      if (s.font !== undefined) oneOf(s.font, DRAWING_FONT_IDS, at(w, 'yazı tipi'));
    }
    return s as T;
  });
}
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

const KINDS = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image', 'raster'] as const;
/** The dimension's kinds; KCAD schema 9 added the last five (docs/adr/0147). */
const DIMENSION_STYLES = ['aligned', 'linear', 'angular', 'radius', 'diameter', 'ordinate', 'arcLength', 'jogged', 'azimuth', 'slope'] as const;
const LINE_TYPES = ['continuous', 'dashed', 'dashdot', 'dotted'] as const;

function parse(data: unknown, version = DOCUMENT_VERSION): DocumentContent {
  if (!isObj(data)) throw new Bad('KentOS çizim dosyası değil (format ≠ kentos.document).');
  const { content, leaves } = head(data, version);
  const ids = new Set<number>();
  // A drawing's inserts name its definitions (docs/adr/0144), as `DrawingObjects` checks them one by one.
  const blocks = new Set((content.blocks ?? []).map((b) => b.id));
  const entities = (data.entities as unknown[]).map((e, i) => {
    const read = entity(e, `Nesne ${i + 1}`, leaves, ids);
    if (read.kind === 'insert' && !blocks.has(read.block)) fail(`Nesne ${i + 1} (insert) › blok`, `${read.block} çizimde tanımlı değil`);
    return read;
  });
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
  const blocks = data.blocks === undefined ? [] : definitions(data.blocks);
  return {
    leaves,
    content: {
      blocks,
      name: str(data.name, 'Ad'),
      settings: {
        srid,
        lengthDecimals: num(settings.lengthDecimals, 'Proje ayarları › uzunluk hassasiyeti'),
        areaDecimals: num(settings.areaDecimals, 'Proje ayarları › alan hassasiyeti'),
        areaUnit: oneOf(settings.areaUnit, ['m2', 'donum', 'ha'] as const, 'Proje ayarları › alan birimi'),
        angleUnit: oneOf(settings.angleUnit, ['grad', 'deg'] as const, 'Proje ayarları › açı birimi'),
        plotScale: num(settings.plotScale, 'Proje ayarları › çizim ölçeği'),
        // Files written before project types, and those that named the former Hibrit mode, have their type not
        // asked yet: none is kept (docs/adr/0165 §1).
        ...(settings.workspace === undefined || settings.workspace === null || settings.workspace === LEGACY_HYBRID
          ? {}
          : { workspace: oneOf(settings.workspace, WORKSPACE_IDS, 'Proje ayarları › proje türü') }),
        // And before drawing typefaces: Barlow, as they were drawn.
        drawingFont: settings.drawingFont === undefined ? 'barlow' : oneOf(settings.drawingFont, DRAWING_FONT_IDS, 'Proje ayarları › çizim yazı tipi'),
        // A local project's unit (docs/adr/0165 §2); none: metres.
        ...(settings.drawingUnit === undefined ? {} : { drawingUnit: oneOf(settings.drawingUnit, DRAWING_UNIT_IDS, 'Proje ayarları › çizim birimi') }),
        // The second coordinate system (docs/adr/0167 §1): another system than the project's own, never a local project's.
        // One the registry does not know is kept, not shown (it is a display aid, nothing is guessed from it).
        ...(settings.secondSrid === undefined ? {} : { secondSrid: second(settings.secondSrid, srid, settings.customCrs !== undefined) }),
        // The project's own systems and datum choices (docs/adr/0168): a definition only without an EPSG code, a second one
        // only instead of a second EPSG code.
        ...(settings.customCrs === undefined
          ? {}
          : srid !== 0
            ? fail('Proje ayarları › kendi sistemi', 'yalnız EPSG kodu olmayan (SRID 0) projede olur')
            : { customCrs: definition(settings.customCrs, 'Proje ayarları › kendi sistemi') }),
        ...(settings.secondCustomCrs === undefined
          ? {}
          : settings.secondSrid !== undefined || (srid === 0 && settings.customCrs === undefined)
            ? fail('Proje ayarları › ikinci sistemin tanımı', 'ikinci sistem ya EPSG kodu ya tanımdır; koordinat sistemi olmayan projenin ikinci sistemi olmaz')
            : { secondCustomCrs: definition(settings.secondCustomCrs, 'Proje ayarları › ikinci sistemin tanımı') }),
        ...(settings.datumTransforms === undefined
          ? {}
          : Array.isArray(settings.datumTransforms) && settings.datumTransforms.every(isObj)
            ? { datumTransforms: settings.datumTransforms as unknown as DatumTransform[] }
            : fail('Proje ayarları › datum dönüşümleri', 'liste olmalı')),
        ...(settings.survey === undefined ? {} : { survey: surveyOf(settings.survey) }),
        // The project's named layer states (docs/adr/0177 §4): a list of objects, kept as a project keeps them.
        ...(settings.layerStates === undefined
          ? {}
          : Array.isArray(settings.layerStates) && settings.layerStates.every(isObj)
            ? { layerStates: settings.layerStates as unknown as LayerState[] }
            : fail('Proje ayarları › katman durumları', 'liste olmalı')),
        // The project's text and dimension styles (docs/adr/0183): kept as a project keeps them.
        ...(settings.textStyles === undefined ? {} : { textStyles: stylesOf<TextStyleDef>(settings.textStyles, 'text') }),
        ...(settings.dimensionStyles === undefined ? {} : { dimensionStyles: stylesOf<DimensionStyleDef>(settings.dimensionStyles, 'dimension') }),
        // The project's topology rules (docs/adr/0202 §1): an object, kept as a project keeps it.
        ...(settings.topology === undefined
          ? {}
          : isObj(settings.topology)
            ? { topology: settings.topology as unknown as TopologySettings }
            : fail('Proje ayarları › topoloji kuralları', 'nesne olmalı')),
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

/**
 * The block definitions (docs/adr/0144), each checked field by field, its
 * objects as a drawing's (their layers need not be in the tree: an insert
 * draws them on its own), then the block rules over the list, said in the
 * documents' words.
 */
function definitions(v: unknown): BlockDefinition[] {
  if (!Array.isArray(v)) return fail('Bloklar', 'liste olmalı');
  const list = v.map((d, i): BlockDefinition => {
    const where = `Blok ${i + 1}`;
    if (!isObj(d)) return fail(where, 'nesne olmalı');
    const name = str(d.name, `${where} › ad`);
    const w = `${where} (“${name}”)`;
    if (!isUuid(d.id)) fail(`${w} › kimlik`, 'küçük harfli, tireli bir UUID olmalı');
    const base = vec(d.base, `${w} › taban noktası`);
    if (!Array.isArray(d.entities)) fail(`${w} › nesneler`, 'liste olmalı');
    const ids = new Set<number>();
    const entities = (d.entities as unknown[]).map((e, k) => entity(e, `${w} › nesne ${k + 1}`, null, ids));
    // A raster stays on its own layer, never in a block (docs/adr/0204 §9).
    const raster = entities.findIndex((e) => e.kind === 'raster');
    if (raster >= 0) fail(`${w} › nesne ${raster + 1}`, 'raster bloğa konamaz; raster kendi katmanında durur');
    const block: BlockDefinition = { id: d.id as string, name, base, entities };
    if (d.attributes !== undefined) {
      if (!Array.isArray(d.attributes)) fail(`${w} › öznitelikler`, 'liste olmalı');
      block.attributes = (d.attributes as unknown[]).map((a, k): AttributeDefinition => {
        const aw = `${w} › öznitelik ${k + 1}`;
        if (!isObj(a)) return fail(aw, 'nesne olmalı');
        const out: AttributeDefinition = { tag: str(a.tag, `${aw} › etiket`), p: vec(a.p, `${aw} › konum`), height: num(a.height, `${aw} › yükseklik`), rotation: num(a.rotation, `${aw} › açı`) };
        if (a.prompt !== undefined) out.prompt = str(a.prompt, `${aw} › soru`);
        if (a.value !== undefined) out.value = str(a.value, `${aw} › varsayılan`);
        textExtrasAt(a, aw);
        if (a.align !== undefined) out.align = a.align as TextAlign;
        if (a.widthFactor !== undefined) out.widthFactor = a.widthFactor as number;
        return out;
      });
    }
    if (d.description !== undefined) block.description = str(d.description, `${w} › açıklama`);
    return block;
  });
  const fault = definitionsFault(list);
  if (fault) fail('Bloklar', blockFaultMessage(fault, (i) => list[i]?.name ?? ''));
  return list;
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
    ...(v.snap !== undefined && { snap: layerSnap(v.snap, type, `${w} › kenet`) }),
    ...(v.fields !== undefined && { fields: layerFieldsAt(v.fields, type, `${w} › alanlar`) }),
  };
}

/**
 * A layer's fields (docs/adr/0199 §1), checked as the KCAD readers check them: on a layer only, a list that is not
 * empty of fields with a name and a known kind and nothing unknown, `required` only true, the list whole by its rules.
 */
function layerFieldsAt(v: unknown, type: 'group' | 'layer', where: string): LayerField[] {
  if (type === 'group') return fail(where, 'grubun alanları olmaz; alanlar yalnız katmanındır');
  if (!Array.isArray(v)) return fail(where, 'liste olmalı');
  const keys = ['name', 'alias', 'kind', 'length', 'scale', 'min', 'max', 'values', 'required', 'default'];
  const fields = v.map((f, i): LayerField => {
    const fw = `${where} › ${i + 1}`;
    if (!isObj(f)) return fail(fw, 'nesne olmalı');
    const unknown = Object.keys(f).find((k) => !keys.includes(k));
    if (unknown) return fail(`${fw} › ${unknown}`, 'bilinmeyen alan');
    const out: LayerField = { name: str(f.name, `${fw} › ad`), kind: oneOf(f.kind, ['text', 'integer', 'decimal', 'date', 'boolean'] as const, `${fw} › tür`) };
    if (f.alias !== undefined) out.alias = str(f.alias, `${fw} › takma ad`);
    for (const k of ['length', 'scale'] as const)
      if (f[k] !== undefined) {
        const n = num(f[k], `${fw} › ${k}`);
        if (!Number.isInteger(n) || n < 0 || n > 4294967295) fail(`${fw} › ${k}`, 'eksi olmayan tam sayı olmalı');
        out[k] = n;
      }
    for (const k of ['min', 'max', 'default'] as const) if (f[k] !== undefined) out[k] = str(f[k], `${fw} › ${k}`);
    if (f.values !== undefined) {
      if (!Array.isArray(f.values)) fail(`${fw} › değer listesi`, 'liste olmalı');
      out.values = (f.values as unknown[]).map((c, j) => {
        const cw = `${fw} › değer ${j + 1}`;
        if (!isObj(c)) return fail(cw, 'nesne olmalı');
        if (Object.keys(c).some((k) => k !== 'code' && k !== 'label')) fail(cw, 'yalnız code ve label olabilir');
        return { code: str(c.code, `${cw} › kod`), label: str(c.label, `${cw} › etiket`) };
      });
    }
    if (f.required !== undefined) {
      if (f.required !== true) fail(`${fw} › zorunlu`, 'yalnız true yazılır');
      out.required = true;
    }
    return out;
  });
  const problem = layerFieldsProblem(fields);
  if (problem) fail(where, problem);
  return fields;
}

/** The snap kinds a layer can keep to (contracts' `LAYER_SNAP_KINDS`; Uç nokta brings Çeyrek with it). */
const LAYER_SNAP_KINDS = ['endpoint', 'midpoint', 'center', 'node', 'intersection', 'perpendicular', 'tangent', 'nearest', 'centroid', 'extension', 'parallel', 'grid'] as const;

/**
 * A layer's own snapping (docs/adr/0163 §4), checked as the KCAD readers check it: on a layer only, exactly one of
 * `off` (true only) and a non-empty list of known, distinct kinds.
 */
function layerSnap(v: unknown, type: 'group' | 'layer', where: string): LayerSnap {
  if (type === 'group') return fail(where, 'grubun keneti olmaz; kenet yalnız katmanındır');
  if (!isObj(v)) return fail(where, 'nesne olmalı');
  const keys = Object.keys(v);
  if (keys.some((k) => k !== 'off' && k !== 'kinds')) return fail(where, 'yalnız off ya da kinds olabilir');
  if (v.off !== undefined && v.off !== true) return fail(where, 'off yalnız true olabilir');
  if ((v.off === true) === (v.kinds !== undefined)) return fail(where, 'ya off ya kinds, tam biri olmalı');
  if (v.off === true) return { off: true };
  const kinds = Array.isArray(v.kinds) ? v.kinds : fail(`${where} › türler`, 'liste olmalı');
  if (!kinds.length) return fail(`${where} › türler`, 'boş olamaz');
  kinds.forEach((k, i) => {
    oneOf(k, LAYER_SNAP_KINDS, `${where} › türler › ${i + 1}`);
    if (kinds.indexOf(k) !== i) fail(`${where} › türler`, `${String(k)} iki kez yazılmış`);
  });
  return { kinds: kinds as string[] };
}

/**
 * One object checked field by field: kind, a unique positive slot, a layer
 * of the file, text attributes, and its geometry by kind (every coordinate a
 * finite float64, lists long enough to draw). The object is kept as read.
 * `keepUid`: a v2 object's persistent id stays (its reader checks it); a v1
 * file has none, so one found there is not taken (ADR 0014).
 */
/**
 * A path's vertex elevations (docs/adr/0142): a list as long as its vertices, each a finite number or
 * null (a vertex without one, not 0).
 */
function elevationsAt(v: unknown, n: number, w: string, what: string): void {
  if (!Array.isArray(v)) fail(at(w, what), 'liste olmalı');
  const list = v as unknown[];
  if (list.length !== n) fail(at(w, what), `köşe sayısı kadar (${n}) olmalı; ${list.length} verildi`);
  list.forEach((z, i) => {
    if (z !== null) numAt(z, w, `${what} ${i + 1}`);
  });
}

/** A text's or an attribute definition's alignment and width factor (docs/adr/0145), when it has them. */
function textExtrasAt(v: Record<string, unknown>, w: string): void {
  if (v.align !== undefined) oneOf(v.align, TEXT_ALIGNS, at(w, 'hiza'));
  if (v.widthFactor !== undefined && !widthFactorOk(numAt(v.widthFactor, w, 'genişlik çarpanı'))) fail(at(w, 'genişlik çarpanı'), `0'dan büyük, en çok ${MAX_WIDTH_FACTOR} olmalı`);
}

/** A text's style and face (docs/adr/0183 §2): typed fields, bold and italic only when true, then the face's rules. */
function faceAt(v: Record<string, unknown>, w: string): void {
  if (v.textStyle !== undefined) strAt(v.textStyle, w, 'stil');
  if (v.font !== undefined) oneOf(v.font, DRAWING_FONT_IDS, at(w, 'yazı tipi'));
  for (const f of ['bold', 'italic'] as const) if (v[f] !== undefined && v[f] !== true) fail(at(w, f === 'bold' ? 'kalın' : 'italik'), 'yalnız true yazılır; alan yoksa yazı öyle değildir');
  if (v.oblique !== undefined) numAt(v.oblique, w, 'eğiklik');
  const problem = faceProblem(v as TextFace);
  if (problem) fail(at(w, problem[0]), problem[1]);
}

/** A dimension's style and look (docs/adr/0183 §3): typed fields, then the look's rules. */
function lookAt(v: Record<string, unknown>, w: string): void {
  if (v.dimStyle !== undefined) strAt(v.dimStyle, w, 'stil');
  if (v.arrow !== undefined) oneOf(v.arrow, DIMENSION_ARROWS, at(w, 'ok'));
  for (const f of ['arrowSize', 'extOffset', 'extBeyond', 'textGap'] as const) if (v[f] !== undefined) numAt(v[f], w, f);
  if (v.textPlace !== undefined) oneOf(v.textPlace, ['centre'] as const, at(w, 'değerin yeri'));
  if (v.decimals !== undefined && !(Number.isInteger(v.decimals) && (v.decimals as number) >= 0)) fail(at(w, 'basamak'), 'negatif olmayan tam sayı olmalı');
  if (v.unit !== undefined) oneOf(v.unit, DRAWING_UNIT_IDS, at(w, 'birim'));
  for (const f of ['prefix', 'suffix'] as const) if (v[f] !== undefined) strAt(v[f], w, f === 'prefix' ? 'önek' : 'sonek');
  if (v.font !== undefined) oneOf(v.font, DRAWING_FONT_IDS, at(w, 'yazı tipi'));
  const problem = lookProblem(v as DimensionLook);
  if (problem) fail(at(w, problem[0]), problem[1]);
}

/** A polygon's or a part's holes (`prefix` names the part): rings of 3 or more vertices, with bulges and elevations. */
function holesAt(holes: unknown[], w: string, prefix: string): void {
  holes.forEach((h, i) => {
    const name = `${prefix}ada ${i + 1}`;
    if (!isObj(h)) fail(at(w, name), 'nesne olmalı');
    const ring = h as Record<string, unknown>;
    const k = pointsAt(ring.pts, w, name, 3);
    if (ring.bulges !== undefined && numbersAt(ring.bulges, w, `${name} › bulge`) > k) fail(at(w, `${name} › bulge`), 'köşe sayısından uzun olamaz');
    if (ring.zs !== undefined) elevationsAt(ring.zs, k, w, `${name} › kotlar`);
  });
}

/** `layers`: the layers an object may be on; null for a block definition's object, whose layer need not be in the tree. */
function entity(v: unknown, where: string, layers: ReadonlySet<string> | null, ids: Set<number>, keepUid = false): Entity {
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
  if (layers && !layers.has(layerId)) fail(at(w, 'katman'), `“${layerId}” katmanı dosyada yok`);
  const attrs = isObj(v.attrs) ? v.attrs : fail(at(w, 'öznitelikler'), 'nesne olmalı');
  for (const k in attrs) if (typeof attrs[k] !== 'string') str(attrs[k], at(w, `öznitelik “${k}”`));
  if (v.lineWeight !== undefined) {
    const weight = numAt(v.lineWeight, w, 'kalınlık');
    if (weight < 0 || weight > MAX_LINE_WEIGHT) fail(at(w, 'kalınlık'), `0 ile ${MAX_LINE_WEIGHT} mm arasında olmalı`);
  }
  // Geometry by kind: every coordinate a finite float64, lists long enough to draw.
  switch (kind) {
    case 'point':
      pointAt(v.p, w, 'p');
      if (v.z !== undefined) numAt(v.z, w, 'z');
      // A multi-point object's other points (docs/adr/0174): each its place and elevation.
      if (v.parts !== undefined) {
        if (!Array.isArray(v.parts)) fail(at(w, 'noktalar'), 'liste olmalı');
        (v.parts as unknown[]).forEach((p, i) => {
          const name = `nokta ${i + 2}`;
          if (!isObj(p)) fail(at(w, name), 'nesne olmalı');
          const part = p as Record<string, unknown>;
          pointAt(part.p, w, name);
          if (part.z !== undefined) numAt(part.z, w, `${name} › kot`);
        });
      }
      break;
    case 'line':
      pointAt(v.a, w, 'a');
      pointAt(v.b, w, 'b');
      if (v.za !== undefined) numAt(v.za, w, 'başlangıcın kotu');
      if (v.zb !== undefined) numAt(v.zb, w, 'bitişin kotu');
      break;
    case 'polyline':
    case 'polygon': {
      const n = pointsAt(v.pts, w, 'köşeler', 2);
      if (v.bulges !== undefined && numbersAt(v.bulges, w, 'bulge') > n) fail(at(w, 'bulge'), 'köşe sayısından uzun olamaz');
      if (v.zs !== undefined) elevationsAt(v.zs, n, w, 'kotlar');
      if (v.holes !== undefined) {
        if (kind !== 'polygon' || !Array.isArray(v.holes)) fail(at(w, 'adalar'), 'yalnızca kapalı alanda, liste olarak');
        holesAt(v.holes as unknown[], w, '');
      }
      // A multi-part area's or polyline's other parts (docs/adr/0143, 0174): each as the object's own path, an area's
      // with its holes.
      if (v.parts !== undefined) {
        if (!Array.isArray(v.parts)) fail(at(w, 'parçalar'), 'liste olmalı');
        (v.parts as unknown[]).forEach((p, i) => {
          const name = `parça ${i + 2}`;
          if (!isObj(p)) fail(at(w, name), 'nesne olmalı');
          const part = p as Record<string, unknown>;
          const k = pointsAt(part.pts, w, name, 2);
          if (part.bulges !== undefined && numbersAt(part.bulges, w, `${name} › bulge`) > k) fail(at(w, `${name} › bulge`), 'köşe sayısından uzun olamaz');
          if (part.zs !== undefined) elevationsAt(part.zs, k, w, `${name} › kotlar`);
          if (part.holes !== undefined) {
            if (kind !== 'polygon' || !Array.isArray(part.holes)) fail(at(w, `${name} › adalar`), 'yalnızca kapalı alanda, liste olarak');
            holesAt(part.holes as unknown[], w, `${name} › `);
          }
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
      textExtrasAt(v, w);
      // The mask only when true (docs/adr/0145), as an insert's mirror.
      if (v.mask !== undefined && v.mask !== true) fail(at(w, 'zemin'), 'yalnız true yazılır; zeminsiz yazıda alan yoktur');
      // The object whose label it writes and the scale, both or neither (docs/adr/0175 §4).
      if ((v.labelOf === undefined) !== (v.labelScale === undefined)) fail(at(w, 'bağlı nesne'), 'nesnesi ve ölçeği birlikte verilir');
      if (v.labelOf !== undefined && !isUuid(v.labelOf)) fail(at(w, 'bağlı nesne'), 'küçük harfli, tireli bir UUID olmalı');
      if (v.labelScale !== undefined && !(numAt(v.labelScale, w, 'bağlı ölçek') > 0)) fail(at(w, 'bağlı ölçek'), "sıfırdan büyük olmalı");
      faceAt(v, w);
      break;
    case 'dimension': {
      pointAt(v.a, w, 'a');
      pointAt(v.b, w, 'b');
      numAt(v.offset, w, 'ötelenme');
      numAt(v.height, w, 'yazı yüksekliği');
      if (v.text !== undefined) strAt(v.text, w, 'metin');
      const style = v.style === undefined ? undefined : oneOf(v.style, DIMENSION_STYLES, at(w, 'ölçü türü'));
      if (v.angle !== undefined) numAt(v.angle, w, 'açı');
      if (v.c !== undefined) pointAt(v.c, w, 'köşe');
      // docs/adr/0147: what a style needs, and what only a slope has.
      if ((style === 'arcLength' || style === 'jogged') && v.c === undefined) fail(at(w, 'merkez'), 'eksik');
      if (style === 'ordinate' && v.angle !== undefined && v.angle !== 0 && v.angle !== 90) fail(at(w, 'eksen'), '0 (Y) ya da 90 (X) olmalı');
      if (style === 'slope') {
        numAt(v.za, w, 'birinci kot');
        numAt(v.zb, w, 'ikinci kot');
      } else if (v.za !== undefined || v.zb !== undefined) fail(at(w, 'kot'), 'yalnız eğim ölçüsünde yazılır');
      if (v.mask !== undefined && v.mask !== true) fail(at(w, 'zemin'), 'yalnız true yazılır; zeminsiz ölçüde alan yoktur');
      lookAt(v, w);
      break;
    }
    case 'hatch': {
      pointsAt(v.ring, w, 'sınır', 3);
      if (v.holes !== undefined) {
        if (!Array.isArray(v.holes)) fail(at(w, 'adalar'), 'liste olmalı');
        (v.holes as unknown[]).forEach((r, i) => pointsAt(r, w, `ada ${i + 1}`, 3));
      }
      const p = isObj(v.pattern) ? v.pattern : fail(at(w, 'desen'), 'eksik');
      oneOf(p.type, ['solid', 'lines', 'cross', 'pattern', 'gradient'] as const, at(w, 'desen türü'));
      numAt(p.angle, w, 'desen açısı');
      numAt(p.spacing, w, 'desen aralığı');
      // docs/adr/0186 §1: a pattern's name, scale and families, a gradient; each by its type, then the contract's rule.
      if (p.name !== undefined) strAt(p.name, w, 'desenin adı');
      if (p.scale !== undefined) numAt(p.scale, w, 'desenin ölçeği');
      if (p.lines !== undefined) {
        if (!Array.isArray(p.lines)) fail(at(w, 'desenin aileleri'), 'liste olmalı');
        (p.lines as unknown[]).forEach((l, i) => {
          const f = `${i + 1}. aile`;
          if (!isObj(l)) return fail(at(w, f), 'nesne olmalı');
          numAt(l.angle, w, `${f} açısı`);
          for (const key of ['origin', 'offset'] as const)
            if (!Array.isArray(l[key]) || (l[key] as unknown[]).length !== 2 || !(l[key] as unknown[]).every(finite)) fail(at(w, `${f} ${key}`), 'iki sayı olmalı');
          if (l.dashes !== undefined) numbersAt(l.dashes, w, `${f} kesikleri`);
        });
      }
      if (p.gradient !== undefined) {
        const g = isObj(p.gradient) ? p.gradient : fail(at(w, 'degrade'), 'nesne olmalı');
        oneOf(g.shape, ['linear', 'cylinder', 'spherical'] as const, at(w, 'degradenin biçimi'));
        strAt(g.color2, w, 'degradenin ikinci rengi');
        if (g.inverted !== undefined && g.inverted !== true) fail(at(w, 'degrade ters'), 'yalnız true yazılır');
      }
      const problem = patternProblem(p as unknown as HatchPattern);
      if (problem) fail(at(w, 'desen'), problem[1]);
      // What it follows: its objects by persistent id and its seed (docs/adr/0186 §6).
      if (v.assoc !== undefined) {
        const a = isObj(v.assoc) ? v.assoc : fail(at(w, 'ilişki'), 'nesne olmalı');
        if (!isUuid(a.outer)) fail(at(w, 'ilişkinin nesnesi'), 'küçük harfli, tireli bir UUID olmalı');
        for (const key of ['islands', 'cutouts'] as const)
          if (a[key] !== undefined && (!Array.isArray(a[key]) || (a[key] as unknown[]).length === 0 || !(a[key] as unknown[]).every(isUuid)))
            fail(at(w, `ilişkinin ${key === 'islands' ? 'adaları' : 'boş bıraktıkları'}`), 'boş olmayan bir UUID listesi olmalı');
        pointAt(a.seed, w, 'tohum');
        const tie = assocProblem(a as unknown as HatchAssoc);
        if (tie) fail(at(w, 'ilişki'), tie[1]);
      }
      break;
    }
    // A block placed (docs/adr/0144): its definition's id, a positive scale, `mirror` only when true.
    case 'insert':
      if (!isUuid(v.block)) fail(at(w, 'blok'), 'küçük harfli, tireli bir UUID olmalı');
      pointAt(v.p, w, 'konum');
      if (numAt(v.scale, w, 'ölçek') <= 0) fail(at(w, 'ölçek'), 'pozitif olmalı');
      numAt(v.rotation, w, 'dönüş');
      if (v.mirror !== undefined && v.mirror !== true) fail(at(w, 'aynalı'), 'yalnız true yazılır; aynalı olmayanda alan yoktur');
      break;
    // A leader (docs/adr/0146): two vertices or more, a positive height, a note that is not empty, a known arrowhead.
    case 'leader':
      pointsAt(v.pts, w, 'köşeler', 2);
      if (numAt(v.height, w, 'yükseklik') <= 0) fail(at(w, 'yükseklik'), 'pozitif olmalı');
      numAt(v.rotation, w, 'açı');
      if (v.text !== undefined && strAt(v.text, w, 'not') === '') fail(at(w, 'not'), 'boş olamaz; notsuz kılavuzda alan yoktur');
      if (v.arrow !== undefined) oneOf(v.arrow, ['open', 'dot', 'none'] as const, at(w, 'ok'));
      if (v.mask !== undefined && v.mask !== true) fail(at(w, 'zemin'), 'yalnız true yazılır; zeminsiz kılavuzda alan yoktur');
      break;
    // A table (docs/adr/0184 §1): its fields' types here, its rows, columns, cells and ranges by the contract's rule.
    case 'table': {
      pointAt(v.p, w, 'konum');
      numAt(v.rotation, w, 'açı');
      numAt(v.height, w, 'yazı yüksekliği');
      numbersAt(v.rows, w, 'satırlar');
      numbersAt(v.columns, w, 'sütunlar');
      if (!Array.isArray(v.cells) || !v.cells.every((r) => Array.isArray(r) && r.every((c) => typeof c === 'string'))) fail(at(w, 'hücreler'), 'metin listelerinin listesi olmalı');
      if (v.merges !== undefined && (!Array.isArray(v.merges) || !v.merges.every((r) => isObj(r) && ['row', 'col', 'rows', 'cols'].every((k) => Number.isInteger(r[k]) && (r[k] as number) >= 0))))
        fail(at(w, 'birleşik alanlar'), 'satır, sütun ve boyları sayı olan alanlar olmalı');
      if (v.aligns !== undefined) {
        if (!Array.isArray(v.aligns)) fail(at(w, 'hizalar'), 'liste olmalı');
        (v.aligns as unknown[]).forEach((a, i) => oneOf(a, ['left', 'center', 'right'] as const, at(w, `${i + 1}. hiza`)));
      }
      if (v.header !== undefined && v.header !== true) fail(at(w, 'başlık'), 'yalnız true yazılır; başlıksız tabloda alan yoktur');
      if (v.grid !== undefined) oneOf(v.grid, ['outer', 'rows', 'none'] as const, at(w, 'çizgiler'));
      if (v.frame !== undefined) numAt(v.frame, w, 'çerçeve kalınlığı');
      if (v.source !== undefined) {
        const s = isObj(v.source) ? v.source : fail(at(w, 'kaynak'), 'nesne olmalı');
        const kind = oneOf(s.kind, ['coordinates', 'areas', 'attributes', 'file'] as const, at(w, 'kaynak türü'));
        if (kind === 'file') {
          strAt(s.name, w, 'kaynak dosya');
          if (s.sheet !== undefined) strAt(s.sheet, w, 'kaynak sayfa');
        } else if (!Array.isArray(s.objects) || !s.objects.every(isUuid)) fail(at(w, 'kaynak nesneler'), 'küçük harfli, tireli UUID listesi olmalı');
      }
      faceAt(v, w);
      const problem = tableProblem(v as unknown as TableShape);
      if (problem) fail(at(w, problem[0]), problem[1]);
      break;
    }
    // A picture (docs/adr/0192 §1): its fields' types here, its size, one source, clip and opacity by the contract's rule.
    case 'image': {
      pointAt(v.p, w, 'konum');
      numAt(v.width, w, 'genişlik');
      numAt(v.height, w, 'yükseklik');
      numAt(v.rotation, w, 'dönüş');
      if (v.mirror !== undefined && v.mirror !== true) fail(at(w, 'ayna'), 'yalnız true yazılır; aynasız resimde alan yoktur');
      if (v.asset !== undefined) strAt(v.asset, w, 'varlık');
      if (v.file !== undefined) strAt(v.file, w, 'dosya');
      if (v.clip !== undefined) pointsAt(v.clip, w, 'kırpma', 3);
      if (v.opacity !== undefined) numAt(v.opacity, w, 'donukluk');
      const problem = imageProblem(v as unknown as ImageShape);
      if (problem) fail(at(w, 'resim'), problem);
      break;
    }
    // A raster (docs/adr/0204 §2): its fields' types here, its affine, size, bands, one source, look and opacity by the contract's rule.
    case 'raster': {
      numbersAt(v.affine, w, 'dönüşüm');
      numAt(v.width, w, 'genişlik');
      numAt(v.height, w, 'yükseklik');
      numAt(v.bands, w, 'bant sayısı');
      oneOf(v.sample, ['u8', 'i8', 'u16', 'i16', 'u32', 'i32', 'f32', 'f64'] as const, at(w, 'örnek türü'));
      if (v.asset !== undefined) strAt(v.asset, w, 'varlık');
      if (v.file !== undefined) strAt(v.file, w, 'dosya');
      numAt(v.srid, w, 'sistem');
      const st = isObj(v.style) ? v.style : fail(at(w, 'görünüş'), 'nesne olmalı');
      oneOf(st.render, ['rgb', 'gray', 'palette', 'ramp', 'hillshade', 'rampShade'] as const, at(w, 'görünüş türü'));
      numbersAt(st.bands, w, 'görünüşün bantları');
      if (st.stretch !== undefined) oneOf(st.stretch, ['none', 'minMax', 'percent', 'manual'] as const, at(w, 'gerdirme'));
      for (const k of ['min', 'max', 'azimuth', 'altitude', 'zFactor', 'nodata'] as const) if (st[k] !== undefined) numAt(st[k], w, k);
      if (st.ramp !== undefined) strAt(st.ramp, w, 'rampa');
      if (st.invert !== undefined && typeof st.invert !== 'boolean') fail(at(w, 'ters'), 'true ya da false olmalı');
      if (st.resampling !== undefined) oneOf(st.resampling, ['bilinear', 'nearest'] as const, at(w, 'örnekleme'));
      if (v.opacity !== undefined) numAt(v.opacity, w, 'donukluk');
      const problem = rasterProblem(v as unknown as RasterShape);
      if (problem) fail(at(w, 'raster'), problem);
      break;
    }
  }
  // Checked field by field above; the object is kept as read (optional fields included).
  return v as unknown as Entity;
}
