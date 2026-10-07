import { describe, expect, it } from 'vitest';
import { isUuid } from '../core/uuid';
import type { BlockDefinition } from './blocks';
import { CadDocument, Refusal } from './document';
import type { Entity, NewEntity } from './entities';
import type { LayerField } from './layerFields';
import { LayerStore, type LayerInit, type LayerSnap, type LayerStyle } from './layers';
import type { ProjectSettingsData } from './projectSettings';
import { readSnapshot } from './snapshot';

/**
 * The shared document fixtures (fixtures/document-ops/v1/*.json, TODOS.md
 * DOM-02, docs/adr/0020) run against the web's CadDocument, which they
 * describe: each scenario opens its setup file the way the app opens a
 * drawing (readSnapshot, replaceWith), applies the steps and compares the
 * state after each one. The desktop's native document runs the same files
 * (crates/native/domain/tests/all/fixtures.rs). The format is in
 * fixtures/document-ops/README.md.
 */

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

interface LayerExpect {
  visible?: boolean;
  locked?: boolean;
  expanded?: boolean;
  name?: string;
  style?: Json;
  isVisible?: boolean;
  isLocked?: boolean;
  /** A layer's own snapping; null when it has none (docs/adr/0163 §4). */
  snap?: Json;
  /** A layer's fields; empty when it has none (docs/adr/0199 §1). */
  fields?: Json;
}

interface Expect {
  ids?: number[];
  count?: number;
  entities?: Record<string, Json>;
  byLayer?: Record<string, number[]>;
  canUndo?: boolean;
  canRedo?: boolean;
  dirty?: boolean;
  revision?: 'same' | 'changed';
  layers?: Record<string, LayerExpect>;
  activeLayer?: string;
  /** Container ("" for the top of the tree) → the ids of its nodes, in order. */
  tree?: Record<string, string[]>;
  /** The drawing's name. */
  name?: string;
  /** Project settings fields to check (the others are not). */
  settings?: Record<string, Json>;
  /** Object → the name of a persistent id taken with `captureUid`, or "new": one taken by none. */
  uids?: Record<string, string>;
  /** The block definitions' names, in order (docs/adr/0144). */
  blocks?: string[];
  /** Block id → fields: name, base, description, and the counts of its entities and attributes. */
  blockFields?: Record<string, Record<string, Json>>;
}

interface Step {
  op: string;
  expect?: Expect;
  returns?: Json;
  catch?: string;
  [field: string]: unknown;
}

interface Scenario {
  name: string;
  setup?: Json;
  steps: Step[];
}

interface Fixture {
  format: string;
  version: number;
  title: string;
  setup: Json;
  scenarios: Scenario[];
}

const EXPECT_KEYS: readonly string[] = ['ids', 'count', 'entities', 'byLayer', 'canUndo', 'canRedo', 'dirty', 'revision', 'layers', 'activeLayer', 'tree', 'name', 'settings', 'uids', 'blocks', 'blockFields'];

/**
 * An object as the fixtures write it: persistent ids are random (UUIDv7) or
 * derived from the file (v5), so they are compared only by `uids`.
 */
const withoutUid = (e: Entity | undefined) => {
  if (!e) return e;
  const { uid: _uid, ...rest } = e;
  return rest;
};

const files = import.meta.glob<string>('../../../../fixtures/document-ops/v1/*.json', { query: '?raw', import: 'default', eager: true });

/** The error a fixture's `throw` raises; only it can be caught by `catch`. */
class Thrown extends Error {}

/** `v` with every `"$uid:name"` the persistent id `captureUid` kept as `name` (a linked text names its object by it, docs/adr/0175 §4). */
const withUids = (v: unknown, uids: ReadonlyMap<string, string>): unknown =>
  typeof v === 'string'
    ? v.startsWith('$uid:') && uids.has(v.slice(5))
      ? uids.get(v.slice(5))
      : v
    : Array.isArray(v)
      ? v.map((x) => withUids(x, uids))
      : v && typeof v === 'object'
        ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, withUids(x, uids)]))
        : v;

/** A patch as the web's API takes it: `null` in the file removes the field (undefined). */
const patchOf = (patch: unknown): Record<string, unknown> => Object.fromEntries(Object.entries(patch as Record<string, unknown>).map(([k, v]) => [k, v === null ? undefined : v]));

class Run {
  readonly doc: CadDocument;
  private readonly groups: { end(): void; cancel(): void }[] = [];
  /** The group ended last, for `endGroupAgain`. */
  private ended: { end(): void; cancel(): void } | null = null;
  private readonly revisions = new Map<string, number>();
  private readonly uids = new Map<string, string>();

  constructor(setup: Json) {
    const read = readSnapshot(JSON.stringify(setup));
    if (!read.ok) throw new Error(`Kurulum dosyası okunamadı: ${read.error}`);
    this.doc = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
    this.doc.replaceWith(read.content);
  }

  step(given: Step, where: string): void {
    const step = withUids(given, this.uids) as Step;
    const before = this.doc.revision;
    let result: unknown;
    try {
      result = this.apply(step, where);
    } catch (e) {
      // A fixture's own `throw`, or an edit the document refuses (removeLayer).
      if (step.catch === undefined || !(e instanceof Thrown || e instanceof Refusal)) throw e;
      expect(e.message, `${where}: yakalanan hata`).toBe(step.catch);
      this.check(step.expect, before, where);
      return;
    }
    if (step.catch !== undefined) throw new Error(`${where}: “${step.catch}” hatası bekleniyordu, adım hatasız bitti`);
    if ('returns' in step) expect(result ?? null, `${where}: dönen değer`).toEqual(step.returns);
    this.check(step.expect, before, where);
  }

  private steps(list: unknown, where: string): void {
    (list as Step[]).forEach((s, i) => this.step(s, `${where} › ${i + 1} ${s.op}`));
  }

  private apply(s: Step, where: string): unknown {
    const doc = this.doc;
    const layers = doc.layers;
    const entityId = s.id as number;
    const layerId = s.id as string;
    switch (s.op) {
      case 'check':
        return undefined;
      case 'add':
        return doc.add(s.entity as NewEntity).id;
      case 'addMany':
        return doc.addMany(s.entities as NewEntity[], s.label as string | undefined).map((e) => e.id);
      case 'update':
        return doc.update(entityId, patchOf(s.patch) as Partial<Entity>);
      case 'updateMany':
        return doc.updateMany(
          (s.patches as { id: number }[]).map((p) => ({ ...patchOf(p), id: p.id }) as Partial<Entity> & { id: number }),
          s.label as string | undefined,
        );
      case 'remove':
        return doc.remove(s.ids as number[]);
      case 'transact':
        return doc.transact(s.label as string, () => {
          this.steps(s.steps, where);
          if (s.throw !== undefined) throw new Thrown(s.throw as string);
        });
      case 'beginGroup':
        this.groups.push(doc.beginGroup(s.label as string));
        return undefined;
      case 'endGroup':
      case 'cancelGroup': {
        const group = this.groups.pop();
        if (!group) throw new Error(`${where}: açık grup yok`);
        this.ended = group;
        return s.op === 'endGroup' ? group.end() : group.cancel();
      }
      case 'endGroupAgain':
        if (!this.ended) throw new Error(`${where}: bitmiş grup yok`);
        return this.ended.end();
      case 'undo':
        return doc.undo();
      case 'redo':
        return doc.redo();
      case 'captureRevision':
        this.revisions.set(s.as as string, doc.revision);
        return undefined;
      case 'markSaved': {
        const revision = this.revisions.get(s.revision as string);
        if (revision === undefined) throw new Error(`${where}: “${s.revision as string}” sürümü alınmadı`);
        return doc.markSaved(revision);
      }
      case 'markUnsaved':
        return doc.markUnsaved();
      case 'captureUid': {
        const uid = doc.uidOf(entityId);
        if (uid === undefined) throw new Error(`${where}: ${entityId} nesnesi yok`);
        this.uids.set(s.as as string, uid);
        return undefined;
      }
      case 'repeat':
        for (let k = 0; k < (s.times as number); k++) this.steps(s.steps, `${where} (${k + 1}.)`);
        return undefined;
      case 'setVisible':
        return layers.setVisible(layerId, s.visible as boolean);
      case 'toggleVisible':
        return layers.toggleVisible(layerId);
      case 'toggleLocked':
        return layers.toggleLocked(layerId);
      case 'setSnap':
        return layers.setSnap(layerId, (s.snap ?? null) as LayerSnap | null);
      case 'isolate':
        return layers.isolate(layerId);
      case 'isolateLayers':
        return layers.isolateLayers(s.ids as string[]);
      case 'showAll':
        return layers.showAll();
      case 'setExpanded':
        return layers.setExpanded(layerId, s.expanded as boolean);
      case 'setActive':
        return layers.setActive(layerId);
      case 'rename':
        return layers.rename(layerId, s.name as string);
      case 'setLayerStyle':
        return doc.setLayerStyle(layerId, patchOf(s.patch) as Partial<LayerStyle>, s.label as string | undefined);
      case 'setLayerFields':
        return doc.setLayerFields(layerId, (s.fields ?? []) as LayerField[], (s.renames ?? []) as [string, string][]);
      case 'addLayer':
        return doc.addLayer(s.layer as LayerInit, (s.parent as string | null | undefined) ?? null, { activate: s.activate === true }).id;
      case 'removeLayer':
        return doc.removeLayer(layerId);
      case 'uniqueLayerName':
        return layers.uniqueName(s.base as string);
      case 'setName':
        return doc.name.set(s.name as string);
      case 'setSettings':
        return doc.settings.assign(patchOf(s.patch) as Partial<ProjectSettingsData>);
      // Block definitions (docs/adr/0144); a refusal is caught as a fixture's own `throw` is.
      case 'addBlock':
        return doc.addBlock(s.block as BlockDefinition);
      case 'updateBlock':
        return doc.updateBlock(s.block as BlockDefinition);
      case 'removeBlock':
        return doc.removeBlock(s.id as string);
      case 'blockRemovalRefused':
        return doc.blockRemovalRefused(s.id as string);
      default:
        throw new Error(`${where}: bilinmeyen işlem “${s.op}”`);
    }
  }

  private check(e: Expect | undefined, before: number, where: string): void {
    if (!e) return;
    // A misspelt expectation would otherwise pass unchecked.
    expect(Object.keys(e).filter((k) => !EXPECT_KEYS.includes(k)), `${where}: bilinmeyen beklenti`).toEqual([]);
    const doc = this.doc;
    if (e.ids) expect([...doc.all()].map((x) => x.id), `${where}: nesneler`).toEqual(e.ids);
    if (e.blocks) expect(doc.blocks.value.map((b) => b.name), `${where}: bloklar`).toEqual(e.blocks);
    for (const [id, fields] of Object.entries(e.blockFields ?? {})) {
      const block = doc.block(id);
      expect(block, `${where}: ${id} bloğu`).toBeDefined();
      const got: Record<string, Json> = {
        name: block!.name,
        base: { ...block!.base },
        description: block!.description ?? null,
        entities: block!.entities.length,
        attributes: block!.attributes?.length ?? 0,
      };
      for (const [field, want] of Object.entries(fields)) {
        expect(field in got, `${where}: ${id} bloğu › bilinmeyen alan ${field}`).toBe(true);
        expect(got[field], `${where}: ${id} bloğu › ${field}`).toEqual(want);
      }
    }
    if (e.count !== undefined) expect(doc.size, `${where}: nesne sayısı`).toBe(e.count);
    for (const [id, want] of Object.entries(e.entities ?? {})) expect(withoutUid(doc.get(Number(id))), `${where}: nesne ${id}`).toEqual(want);
    for (const [id, name] of Object.entries(e.uids ?? {})) {
      const uid = doc.uidOf(Number(id));
      expect(isUuid(uid), `${where}: ${id} nesnesinin kalıcı kimliği`).toBe(true);
      if (name === 'new') expect([...this.uids.values()], `${where}: ${id} nesnesinin kimliği yeni olmalı`).not.toContain(uid);
      else {
        expect(this.uids.has(name), `${where}: “${name}” kimliği alınmadı`).toBe(true);
        expect(uid, `${where}: ${id} nesnesinin kimliği “${name}” olmalı`).toBe(this.uids.get(name));
      }
    }
    for (const [layer, ids] of Object.entries(e.byLayer ?? {}))
      expect(
        doc.byLayer(layer).map((x) => x.id),
        `${where}: “${layer}” katmanının nesneleri`,
      ).toEqual(ids);
    if (e.canUndo !== undefined) expect(doc.canUndo.value, `${where}: canUndo`).toBe(e.canUndo);
    if (e.canRedo !== undefined) expect(doc.canRedo.value, `${where}: canRedo`).toBe(e.canRedo);
    if (e.dirty !== undefined) expect(doc.dirty.value, `${where}: dirty`).toBe(e.dirty);
    if (e.revision) expect(doc.revision === before ? 'same' : 'changed', `${where}: sürüm`).toBe(e.revision);
    for (const [id, want] of Object.entries(e.layers ?? {})) {
      const node = doc.layers.get(id);
      expect(node, `${where}: “${id}” katmanı`).toBeDefined();
      const got: LayerExpect = {
        visible: node?.visible,
        locked: node?.locked,
        expanded: node?.expanded,
        name: node?.name,
        style: node?.style as unknown as Json,
        isVisible: doc.layers.isVisible(id),
        isLocked: doc.layers.isLocked(id),
        snap: (node?.snap ?? null) as Json,
        fields: (node?.fields ?? []) as unknown as Json,
      };
      for (const key of Object.keys(want) as (keyof LayerExpect)[]) {
        expect(key in got, `${where}: “${id}” katmanı › bilinmeyen alan ${key}`).toBe(true);
        expect(got[key], `${where}: “${id}” katmanı › ${key}`).toEqual(want[key]);
      }
    }
    if (e.activeLayer !== undefined) expect(doc.layers.active.value, `${where}: etkin katman`).toBe(e.activeLayer);
    if (e.name !== undefined) expect(doc.name.value, `${where}: ad`).toBe(e.name);
    for (const [field, want] of Object.entries(e.settings ?? {}))
      expect((doc.settings.toJSON() as unknown as Record<string, Json>)[field], `${where}: ayar ${field}`).toEqual(want);
    for (const [container, ids] of Object.entries(e.tree ?? {})) {
      const nodes = container === '' ? doc.layers.tree : doc.layers.get(container)?.children;
      expect(nodes, `${where}: “${container}” grubu`).toBeDefined();
      expect(nodes?.map((n) => n.id), `${where}: “${container}” altındakiler`).toEqual(ids);
    }
  }
}

describe('document operation fixtures (fixtures/document-ops/v1)', () => {
  it('finds the fixture files', () => {
    expect(Object.keys(files).length).toBeGreaterThanOrEqual(6);
  });
  for (const [path, text] of Object.entries(files)) {
    const fixture = JSON.parse(text) as Fixture;
    const file = path.split('/').pop();
    describe(`${file}: ${fixture.title}`, () => {
      it('is a document-ops v1 file', () => {
        expect(fixture.format).toBe('kentos.document-ops');
        expect(fixture.version).toBe(1);
      });
      for (const scenario of fixture.scenarios)
        it(scenario.name, () => {
          const run = new Run(scenario.setup ?? fixture.setup);
          expect(run.doc.dirty.value, 'kurulumdan sonra').toBe(false);
          scenario.steps.forEach((s, i) => run.step(s, `${scenario.name} › ${i + 1} ${s.op}`));
        });
    });
  }
});
