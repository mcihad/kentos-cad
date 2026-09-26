import type { Entity as ContractEntity } from '../../contracts/generated/Entity';
import { CadDocument } from '../../model/document';
import type { Entity } from '../../model/entities';
import { LayerStore } from '../../model/layers';
import { MemoryDraftStore, type Draft } from './drafts';
import { FakeServer } from './fakeServer';
import { readProject } from './incoming';
import { ProjectSync, type SyncOptions } from './sync';

/**
 * Shared set-up of the cloud sync tests (never used by the app): a drawing,
 * the fake server, a sync between them, and a project opened again from the
 * server the way `CloudSession.open` does it.
 */

export type Records = SyncOptions['records'];

export const layers = () => new LayerStore([{ id: 'cizim', name: 'Çizim' }, { id: 'parsel', name: 'Parsel' }], 'cizim');
export const newDoc = () => new CadDocument({ name: 'Ada 101', layers: layers(), origin: { x: 486500, y: 4420200 } });
export const pt = (x: number, layerId = 'cizim') => ({ kind: 'point' as const, layerId, p: { x, y: 4420210 }, attrs: {} });
/** An object as another client sends it: the contract's shape, without the persistent id. */
export const wire = (e: Entity): ContractEntity => {
  const { uid: _uid, ...rest } = e;
  return structuredClone(rest);
};
export const xOf = (e: Entity | undefined) => (e as { p: { x: number } } | undefined)?.p.x;

export const serverFor = (doc: CadDocument) =>
  new FakeServer({ name: doc.name.value, settings: doc.settings.toJSON(), layers: [...doc.layers.tree], activeLayer: 'cizim', styles: structuredClone({ items: [...doc.styles.value.items], categories: [...doc.styles.value.categories] }), origin: doc.origin });

let open: ProjectSync[] = [];

/** Stops every sync a test made (call in `afterEach`). */
export function disposeAll(): void {
  for (const s of open) s.dispose();
  open = [];
}

export function setup(opts: { canEditMeta?: boolean; canWrite?: boolean; drafts?: MemoryDraftStore; doc?: CadDocument; server?: FakeServer; records?: Records } = {}) {
  const doc = opts.doc ?? newDoc();
  const server = opts.server ?? serverFor(doc);
  const warnings: string[] = [];
  const drafts = opts.drafts ?? new MemoryDraftStore();
  const told = { deleted: 0, revoked: [] as string[], asked: 0, archived: 0 };
  const o: SyncOptions = {
    doc,
    api: server,
    drafts,
    draftKey: 'u1/t/p',
    userId: 'u1',
    tenantId: 't',
    projectId: 'p',
    canEditMeta: opts.canEditMeta ?? true,
    canWrite: opts.canWrite,
    metaVersion: String(server.metaVersion),
    cursor: String(server.history.length),
    records: opts.records ?? [],
    warn: (t) => warnings.push(t),
    onDeleted: () => told.deleted++,
    onRevoked: (reason) => told.revoked.push(reason),
    onArchived: () => told.archived++,
    onAccessChanged: () => told.asked++,
    debounceMs: 60_000,
    maxDelayMs: 60_000,
  };
  const sync = new ProjectSync(o);
  open.push(sync);
  return { doc, server, sync, warnings, drafts, told };
}

/**
 * The project opened again from the server into a fresh drawing, as
 * `CloudSession.open` does it: every object under the server's id as its
 * persistent id; `records` for the sync.
 */
export async function reopen(server: FakeServer): Promise<{ doc: CadDocument; records: Records }> {
  const doc = newDoc();
  const info = await server.project();
  const page = await server.features('t', 'p', null, 1_000_000);
  const read = readProject(info, page.features);
  if (!read.ok) throw new Error(read.error);
  doc.replaceWith(read.content);
  return { doc, records: page.features.map((f) => ({ id: f.id, version: f.version })) };
}

/** The draft stored under the test key, as the current code writes it. */
export async function storedDraft(drafts: MemoryDraftStore, key = 'u1/t/p'): Promise<Draft | null> {
  return (await drafts.get(key)) as Draft | null;
}

/** Every persistent id in the drawing is there once; returns the ids, sorted. */
export function uidsOf(doc: CadDocument): string[] {
  const ids = [...doc.all()].map((e) => e.uid!);
  if (new Set(ids).size !== ids.length) throw new Error('aynı kalıcı kimlik iki nesnede');
  return ids.sort();
}
