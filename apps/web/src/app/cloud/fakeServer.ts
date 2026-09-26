import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { CommitResult } from '../../contracts/generated/CommitResult';
import type { Entity as ContractEntity } from '../../contracts/generated/Entity';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FeatureChange } from '../../contracts/generated/FeatureChange';
import type { FeatureConflict } from '../../contracts/generated/FeatureConflict';
import type { FeatureRecord } from '../../contracts/generated/FeatureRecord';
import type { ProjectAccessChange } from '../../contracts/generated/ProjectAccessChange';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectCatalogChange } from '../../contracts/generated/ProjectCatalogChange';
import type { ProjectChanges } from '../../contracts/generated/ProjectChanges';
import type { ProjectDetails } from '../../contracts/generated/ProjectDetails';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectRole } from '../../contracts/generated/ProjectRole';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { ApiFailure, type CloudApi } from './api';

/** Every project permission: the fake's caller owns the project unless a test lowers it. */
const ALL: ProjectPermission[] = ['project.read', 'feature.write', 'project.edit', 'project.delete', 'project.comment', 'project.download', 'project.history', 'project.share', 'project.transfer', 'project.jobs.run'];

/** The permissions of a role given by sharing (crates/server/application/src/access.rs). */
export const ROLE_PERMISSIONS: Record<Exclude<ProjectRole, 'owner'>, ProjectPermission[]> = {
  viewer: ['project.read', 'project.download', 'project.history'],
  commenter: ['project.read', 'project.download', 'project.history', 'project.comment'],
  editor: ['project.read', 'project.download', 'project.history', 'project.comment', 'feature.write', 'project.jobs.run'],
  manager: ['project.read', 'project.download', 'project.history', 'project.comment', 'feature.write', 'project.jobs.run', 'project.edit', 'project.share'],
};

/**
 * An in-memory stand-in for the server's edit protocol, for the sync tests:
 * versions and 409s, idempotent replays, events with request ids, a
 * deleted project (410), an archived one (409 `project_archived`, the
 * lifecycle commands of docs/adr/0028), the caller's role and access taken
 * away (403 and 404, docs/adr/0015), and switches for a dead network and a
 * lost answer. It follows crates/server/application/src/changes.rs,
 * lifecycle.rs and sharing.rs; the real thing is tested against PostgreSQL
 * in Rust and end to end in the browser.
 */
export class FakeServer implements CloudApi {
  store = new Map<string, { version: number; entity: ContractEntity }>();
  meta: Pick<ProjectInfo, 'name' | 'settings' | 'layers' | 'activeLayer' | 'styles' | 'origin'>;
  metaVersion = 1;
  revision = 0;
  history: EventRecord[] = [];
  private readonly log = new Map<string, { text: string; result: CommitResult }>();
  /** Fail every call as if the network were down. */
  offline = false;
  /** Commit the next command, then fail as if its answer were lost. */
  loseNextAnswer = false;
  /** While set, commands wait for it before anything happens (one still on its way over a slow network). */
  gate: Promise<void> | null = null;
  /** Commands waiting at the gate. */
  waiting = 0;
  /** Deleted: reading and writing answer 410; the command log still answers retries. */
  deleted = false;
  /** Archived: it reads, writing answers 409 (docs/adr/0028). */
  archived = false;
  /** Lifecycle commands received, by name (the catalog's). */
  readonly lifecycleLog: CommandEnvelope[] = [];
  /** The caller's role; `grant` lowers or raises it (a `project.access` event). */
  role: ProjectRole = 'owner';
  /** The caller's access taken away: everything answers 404, retries too (the access is checked first). */
  revoked = false;
  commits = 0;
  /** Commands answered from the log (a retry after a lost answer). */
  replays = 0;

  constructor(meta: FakeServer['meta']) {
    this.meta = structuredClone(meta);
  }

  private check(): void {
    if (this.offline) throw new ApiFailure(0, { error: 'network' }, 'Sunucuya ulaşılamadı.');
  }

  private gone(): void {
    if (this.deleted) throw new ApiFailure(410, { error: 'project_deleted', message: `“${this.meta.name}” projesi silindi; açılamaz ve değiştirilemez.` }, 'Proje silindi.');
  }

  /** The caller may not see the project (any more): the server's 404. */
  private hidden(): void {
    if (this.revoked) throw new ApiFailure(404, { error: 'not_found', message: 'Proje bulunamadı.' }, 'Proje bulunamadı.');
  }

  private permissions(): ProjectPermission[] {
    return this.role === 'owner' ? ALL : ROLE_PERMISSIONS[this.role];
  }

  /** A manager gives the caller another role, or takes its access away (null): returns the `project.access` event. */
  grant(role: Exclude<ProjectRole, 'owner'> | null, requestId = 'yonetici'): EventRecord {
    if (role) this.role = role;
    else this.revoked = true;
    const e: EventRecord = { seq: String(this.history.length + 1), dataRevision: String(this.revision), kind: 'project.access', requestId, features: [], meta: false };
    this.history.push(e);
    return e;
  }

  /** Someone archives the project (returns its event). */
  archiveAs(requestId: string): EventRecord {
    this.archived = true;
    const e: EventRecord = { seq: String(this.history.length + 1), dataRevision: String(this.revision), kind: 'project.archived', requestId, features: [], meta: false };
    this.history.push(e);
    return e;
  }

  /** Someone deletes the project (returns its event). */
  deleteAs(requestId: string): EventRecord {
    this.deleted = true;
    this.revision++;
    const e: EventRecord = { seq: String(this.history.length + 1), dataRevision: String(this.revision), kind: 'project.deleted', requestId, features: [], meta: false };
    this.history.push(e);
    return e;
  }

  async deleteProject(): Promise<void> {
    this.check();
    if (!this.deleted) this.deleteAs(`web-${crypto.randomUUID()}`);
  }

  private record(id: string): FeatureRecord | undefined {
    const f = this.store.get(id);
    return f && { id, version: String(f.version), entity: structuredClone(f.entity) };
  }

  /**
   * Another editor's commit, straight into the store (returns the event).
   * Versions as the server gives them: a created object's is the commit's
   * data revision, above any the same id had before it was deleted
   * (docs/adr/0026); a change adds one. Deleting removes the object.
   */
  commitAs(requestId: string, changes: FeatureChange[], meta?: Partial<FakeServer['meta']>): EventRecord {
    const out: EventRecord['features'] = [];
    const revision = this.revision + 1;
    for (const c of changes) {
      if (c.op === 'delete') {
        this.store.delete(c.id);
        out.push({ id: c.id, op: 'delete' });
      } else {
        const version = c.op === 'create' ? revision : (this.store.get(c.id)?.version ?? 0) + 1;
        this.store.set(c.id, { version, entity: structuredClone(c.entity) });
        out.push({ id: c.id, op: c.op, version: String(version) });
      }
    }
    if (meta) {
      Object.assign(this.meta, structuredClone(meta));
      this.metaVersion++;
    }
    this.revision++;
    const e: EventRecord = { seq: String(this.history.length + 1), dataRevision: String(this.revision), kind: 'project.changes', requestId, features: out, meta: !!meta };
    this.history.push(e);
    return e;
  }

  async command(envelope: CommandEnvelope): Promise<CommitResult> {
    this.check();
    if (this.gate) {
      this.waiting++;
      await this.gate;
      this.waiting--;
    }
    this.hidden();
    // As the server's command log: the same key answers the same request only (crates/server/application/src/idempotency.rs).
    const text = JSON.stringify({ command: envelope.commandName, version: envelope.version, project: envelope.projectId, expected: envelope.expectedVersions, input: envelope.input });
    const earlier = this.log.get(envelope.idempotencyKey);
    if (earlier && earlier.text !== text)
      throw new ApiFailure(400, { error: 'invalid', message: 'Bu idempotency anahtarı başka bir istek için kullanılmış; her komuta yeni bir anahtar verin.' }, 'Geçersiz istek.');
    if (earlier) {
      this.replays++;
      return { ...earlier.result, replayed: true };
    }
    this.gone();
    if (this.archived)
      throw new ApiFailure(409, { error: 'project_archived', message: `“${this.meta.name}” projesi arşivlenmiş; salt okunurdur.` }, 'Proje arşivlenmiş.');
    const input = envelope.input as ProjectChanges;
    const may = this.permissions();
    if ((input.features.length && !may.includes('feature.write')) || (input.project && !may.includes('project.edit')))
      throw new ApiFailure(403, { error: 'forbidden', message: `“${this.meta.name}” projesinde bu işlem için yetkiniz yok (feature.write); proje sahibinden ya da yöneticisinden isteyin.` }, 'Yetki yok.');
    const conflicts: FeatureConflict[] = [];
    if (input.project && envelope.expectedVersions['@project'] !== String(this.metaVersion))
      conflicts.push({ id: '@project', reason: 'project', expected: envelope.expectedVersions['@project'], actual: String(this.metaVersion) });
    for (const c of input.features) {
      const have = this.store.get(c.id);
      if (c.op === 'create' && have) conflicts.push({ id: c.id, reason: 'exists', actual: String(have.version), current: this.record(c.id) });
      if (c.op !== 'create') {
        const want = envelope.expectedVersions[c.id];
        if (!have) conflicts.push({ id: c.id, reason: 'deleted', expected: want });
        else if (String(have.version) !== want) conflicts.push({ id: c.id, reason: 'changed', expected: want, actual: String(have.version), current: this.record(c.id) });
      }
    }
    if (conflicts.length) throw new ApiFailure(409, { error: 'conflict', message: 'Çakışma', conflicts }, 'Çakışma');
    const event = this.commitAs(envelope.requestId, input.features, input.project ? (input.project as Partial<FakeServer['meta']>) : undefined);
    const versions: Record<string, string> = {};
    for (const f of event.features) if (f.version) versions[f.id] = f.version;
    const result: CommitResult = {
      dataRevision: String(this.revision),
      metaVersion: String(this.metaVersion),
      versions,
      deleted: event.features.filter((f) => f.op === 'delete').map((f) => f.id),
      eventSeq: event.seq,
      replayed: false,
    };
    this.log.set(envelope.idempotencyKey, { text, result });
    this.commits++;
    if (this.loseNextAnswer) {
      this.loseNextAnswer = false;
      throw new ApiFailure(0, { error: 'network' }, 'Yanıt kayboldu.');
    }
    return result;
  }

  async featuresById(_t: string, _p: string, ids: readonly string[]) {
    this.check();
    this.hidden();
    this.gone();
    return { features: ids.map((id) => this.record(id)).filter((f): f is FeatureRecord => !!f) };
  }

  async project(): Promise<ProjectInfo> {
    this.check();
    this.hidden();
    this.gone();
    return {
      id: 'p',
      tenantId: 't',
      tenantName: 'Büro',
      tenantKind: 'organization',
      // The fake's caller owns the project (every permission; docs/adr/0015) unless a test gave it a role.
      access: { role: this.role, via: this.role === 'owner' ? 'owner' : 'grant', permissions: this.permissions() },
      state: this.archived ? 'archived' : 'active',
      ...structuredClone(this.meta),
      metaVersion: String(this.metaVersion),
      dataRevision: String(this.revision),
      featureCount: String(this.store.size),
      eventCursor: String(this.history.length),
      // The fake keeps its content object by object.
      storage: 'database',
    };
  }

  async features(_t: string, _p: string, after: string | null, limit: number) {
    this.check();
    this.hidden();
    this.gone();
    const ids = [...this.store.keys()].sort().filter((id) => !after || id > after);
    const page = ids.slice(0, limit).map((id) => this.record(id)!);
    return { features: page, next: ids.length > limit ? page[page.length - 1].id : undefined };
  }

  async events(_t: string, _p: string, after: string) {
    this.check();
    const list = this.history.filter((e) => Number(e.seq) > Number(after));
    return { events: list, next: list.at(-1)?.seq ?? after };
  }

  authConfig = async () => ({ local: true });
  me = async () => ({ user: { id: 'u1', displayName: 'Ayşe', method: 'local' as const }, memberships: [] });
  login = async () => this.me();
  logout = async () => {};
  projects = async () => ({ projects: [] });
  myProjects = async () => ({ projects: [] });
  createProject = async () => this.project();
  access = async (): Promise<ProjectAccessList> => {
    this.hidden();
    return { tenantKind: 'organization', storage: 'database', adminsAccessAllProjects: true, ownerId: 'u1', ownerName: 'Ayşe', people: [] };
  };
  candidates = async () => ({ candidates: [] });
  accessCommand = async (envelope: CommandEnvelope): Promise<ProjectAccessChange> => {
    this.hidden();
    return { userId: String((envelope.input as { userId?: string }).userId), changed: true, replayed: false };
  };

  /** The project as a catalog list shows it. */
  summary(): ProjectSummary {
    return {
      id: 'p',
      name: this.meta.name,
      srid: 5256,
      dataRevision: String(this.revision),
      updatedAt: '2026-09-26T10:00:00Z',
      tenantId: 't',
      tenantName: 'Büro',
      tenantKind: 'organization',
      ownerName: 'Ayşe',
      access: { role: this.role, via: this.role === 'owner' ? 'owner' : 'grant', permissions: this.permissions() },
      projectType: 'cad',
      description: '',
      tags: [],
      state: this.deleted ? 'trashed' : this.archived ? 'archived' : 'active',
      catalogVersion: '1',
      createdAt: '2026-09-26T09:00:00Z',
      creatorName: 'Ayşe',
      areaUnit: 'm2',
      storage: 'database',
      favorite: false,
      ...(this.deleted ? { trashedAt: '2026-09-26T10:00:00Z', purgeAfter: '2026-10-26T10:00:00Z' } : {}),
    };
  }

  catalog = async () => ({ projects: [this.summary()], total: 1, trashRetentionDays: 30 });

  details = async (): Promise<ProjectDetails> => {
    this.hidden();
    this.gone();
    return { project: this.summary(), featureCount: String(this.store.size), layerCount: 1 };
  };

  /** The lifecycle commands as the server runs them, as far as the sync tests need: archive, unarchive, trash, restore. */
  lifecycle = async <T>(envelope: CommandEnvelope): Promise<T> => {
    this.check();
    this.hidden();
    this.lifecycleLog.push(envelope);
    const event = (kind: string) => {
      const e: EventRecord = { seq: String(this.history.length + 1), dataRevision: String(this.revision), kind, requestId: envelope.requestId, features: [], meta: false };
      this.history.push(e);
      return e.seq;
    };
    let seq: string | undefined;
    switch (envelope.commandName) {
      case 'project.archive':
        this.gone();
        if (!this.archived) {
          this.archived = true;
          seq = event('project.archived');
        }
        break;
      case 'project.unarchive':
        this.gone();
        if (this.archived) {
          this.archived = false;
          seq = event('project.unarchived');
        }
        break;
      case 'project.trash':
        if (!this.deleted) {
          this.deleted = true;
          this.revision++;
          seq = event('project.deleted');
        }
        break;
      case 'project.restore':
        if (this.deleted) {
          this.deleted = false;
          seq = event('project.restored');
        }
        break;
      default:
        this.gone();
    }
    const result: ProjectCatalogChange = { project: this.summary(), changed: !!seq, ...(seq ? { eventSeq: seq } : {}), replayed: false };
    return result as T;
  };
}
