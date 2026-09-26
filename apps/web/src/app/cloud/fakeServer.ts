import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { CommitResult } from '../../contracts/generated/CommitResult';
import type { Entity as ContractEntity } from '../../contracts/generated/Entity';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FeatureChange } from '../../contracts/generated/FeatureChange';
import type { FeatureConflict } from '../../contracts/generated/FeatureConflict';
import type { FeatureRecord } from '../../contracts/generated/FeatureRecord';
import type { ProjectAccessChange } from '../../contracts/generated/ProjectAccessChange';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectChanges } from '../../contracts/generated/ProjectChanges';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectRole } from '../../contracts/generated/ProjectRole';
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
 * deleted project (410), the caller's role and access taken away (403 and
 * 404, docs/adr/0015), and switches for a dead network and a lost answer. It
 * follows crates/server/application/src/changes.rs, lifecycle.rs and
 * sharing.rs; the real thing is tested against PostgreSQL in Rust and end to
 * end in the browser.
 */
export class FakeServer implements CloudApi {
  store = new Map<string, { version: number; entity: ContractEntity }>();
  meta: Pick<ProjectInfo, 'name' | 'settings' | 'layers' | 'activeLayer' | 'styles' | 'origin'>;
  metaVersion = 1;
  revision = 0;
  history: EventRecord[] = [];
  private readonly log = new Map<string, CommitResult>();
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
  /** The caller's role; `grant` lowers or raises it (a `project.access` event). */
  role: ProjectRole = 'owner';
  /** The caller's access taken away: everything answers 404, retries too (the access is checked first). */
  revoked = false;
  commits = 0;

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

  /** Another editor's commit, straight into the store (returns the event). */
  commitAs(requestId: string, changes: FeatureChange[], meta?: Partial<FakeServer['meta']>): EventRecord {
    const out: EventRecord['features'] = [];
    for (const c of changes) {
      if (c.op === 'delete') {
        this.store.delete(c.id);
        out.push({ id: c.id, op: 'delete' });
      } else {
        const version = (this.store.get(c.id)?.version ?? 0) + 1;
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
    const earlier = this.log.get(envelope.idempotencyKey);
    if (earlier) return { ...earlier, replayed: true };
    this.gone();
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
    this.log.set(envelope.idempotencyKey, result);
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
      ...structuredClone(this.meta),
      metaVersion: String(this.metaVersion),
      dataRevision: String(this.revision),
      featureCount: String(this.store.size),
      eventCursor: String(this.history.length),
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
}
