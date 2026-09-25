import { Signal } from '../../core/signal';
import type { AuthConfig } from '../../contracts/generated/AuthConfig';
import type { FeatureRecord } from '../../contracts/generated/FeatureRecord';
import type { LayerNode } from '../../contracts/generated/LayerNode';
import type { Me } from '../../contracts/generated/Me';
import type { MembershipView } from '../../contracts/generated/MembershipView';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import type { ProjectList } from '../../contracts/generated/ProjectList';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectRole } from '../../contracts/generated/ProjectRole';
import type { TenantKind } from '../../contracts/generated/TenantKind';
import type { AppContext } from '../context';
import { replaceDrawing } from '../fileIO';
import { AccessWatch } from './accessWatch';
import { ApiFailure, HttpCloudApi, type CloudApi } from './api';
import { browserDraftStore, draftKey, type DraftStore } from './drafts';
import { readIncoming } from './incoming';
import { ProjectSocket, socketUrl, type LinkState } from './socket';
import { ProjectSync } from './sync';

/**
 * The cloud side of the app (`ctx.cloud`, Faz B): who is signed in, which
 * cloud project is open, and the machinery that keeps it in step with the
 * server (autosave: sync.ts, live events: socket.ts). Opening is paged and
 * can be cancelled; a slow answer for a project the user already left is
 * thrown away (`generation`, CLAUDE.md §21.2). Uploading a local drawing
 * creates the project with every layer unlocked, sends the objects in
 * batches and then restores the drawing's own layer tree, locks included.
 * A project can be renamed (a metadata change) and, by its owner or an
 * organisation's admin, deleted; when someone else deletes the open one, its
 * sync stops sending and the drawing stays on screen, its edits kept on this
 * device.
 *
 * What the account may do comes with each project (docs/adr/0015): a tenant
 * role opens no project by itself, so buttons follow the project's own
 * permissions; the membership only says whether projects may be created in a
 * workspace. The server checks every request anyway. While a project is
 * open its permissions follow the server (accessWatch.ts): a changed role
 * holds or resumes the autosave, and access taken away stops it like a
 * deletion, keeps the drawing and its unsent edits on this device, and says
 * so (`accessLost`, TODOS.md CLOUD-13).
 */

export type AuthState = 'unknown' | 'signedOut' | 'signedIn';

export interface CloudProject {
  tenantId: string;
  /** Its workspace as the interface names it (`workspaceName`). */
  tenantName: string;
  tenantKind: TenantKind;
  projectId: string;
  name: string;
  /** This account's role in it. */
  role: ProjectRole;
  /** What this account may do in it. */
  permissions: readonly ProjectPermission[];
  canWrite: boolean;
  canEditMeta: boolean;
}

/** The open project's access was taken away: what the notice tells. */
export interface AccessLost {
  name: string;
  /** Changes not sent to the server; they stay in this device's draft. */
  unsent: number;
  /** Whether edits are kept on this device from now on (a viewer's never are). */
  keeps: boolean;
  /** The server's reason when it said more than “not found” (a membership that may not be used now). */
  reason: string;
}

/**
 * How the interface names a workspace: an organisation by its name, one's own
 * personal space “Kişisel”, someone else's (a project shared from it) by its person.
 */
export function workspaceName(kind: TenantKind, name: string, own: boolean): string {
  if (kind === 'organization') return name;
  return own ? 'Kişisel' : `${name} (kişisel alan)`;
}

export type Progress = (done: number, total: number) => void;

const PAGE = 2000;
const uuid = () => crypto.randomUUID();

function unlocked(nodes: readonly LayerNode[]): LayerNode[] {
  return nodes.map((n) => ({ ...n, locked: false, children: unlocked(n.children) }));
}

function hasLocks(nodes: readonly LayerNode[]): boolean {
  return nodes.some((n) => n.locked || hasLocks(n.children));
}

export class CloudSession {
  readonly auth = new Signal<AuthState>('unknown');
  readonly me = new Signal<Me | null>(null);
  readonly config = new Signal<AuthConfig | null>(null);
  readonly project = new Signal<CloudProject | null>(null);
  readonly sync = new Signal<ProjectSync | null>(null);
  /** The live channel of the open project ('none' when no cloud project is open). */
  readonly link = new Signal<LinkState | 'none'>('none');
  /** Set when the open project's access is taken away (the interface shows a notice); cleared when the project is left. */
  readonly accessLost = new Signal<AccessLost | null>(null);
  /** Whether drafts survive a reload on this browser. */
  readonly durableDrafts: boolean;
  readonly api: CloudApi;
  private readonly ctx: AppContext;
  private readonly drafts: DraftStore;
  private socket: ProjectSocket | null = null;
  private unlink: (() => void) | null = null;
  private generation = 0;

  constructor(ctx: AppContext, api: CloudApi = new HttpCloudApi(), drafts?: DraftStore) {
    this.ctx = ctx;
    this.api = api;
    const store = drafts ? { store: drafts, durable: true } : browserDraftStore();
    this.drafts = store.store;
    this.durableDrafts = store.durable;
  }

  /** The membership of a tenant for the signed-in account. */
  membership(tenantId: string): MembershipView | undefined {
    return this.me.value?.memberships.find((m) => m.tenantId === tenantId);
  }

  /** A right in the workspace itself (`project.create`, `member.manage`). */
  can(tenantId: string, capability: string): boolean {
    return !!this.membership(tenantId)?.capabilities.includes(capability);
  }

  /** Whether this account may do `permission` in the open project. */
  may(permission: ProjectPermission): boolean {
    return !!this.project.value?.permissions.includes(permission);
  }

  /** Asks the server who we are (a quiet no when no one is signed in or the server is away). */
  async refresh(): Promise<void> {
    try {
      this.config.set(await this.api.authConfig());
      this.me.set(await this.api.me());
      this.auth.set('signedIn');
    } catch (e) {
      if (e instanceof ApiFailure && e.status === 401) {
        this.me.set(null);
        this.auth.set('signedOut');
      } else this.auth.set('unknown');
    }
  }

  async signIn(login: string, password: string): Promise<Me> {
    const me = await this.api.login(login, password);
    this.me.set(me);
    this.auth.set('signedIn');
    this.socket?.reconnect();
    return me;
  }

  /** Signs out; an open project's unsent changes stay in this account's device draft. */
  async signOut(): Promise<void> {
    await this.sync.value?.flush().catch(() => false);
    this.detach();
    try {
      await this.api.logout();
    } finally {
      this.me.set(null);
      this.auth.set('signedOut');
    }
  }

  projects(tenantId: string): Promise<ProjectList> {
    return this.api.projects(tenantId);
  }

  /** The account's own projects and the ones shared with it, in every workspace (“Projelerim”). */
  myProjects(): Promise<ProjectList> {
    return this.api.myProjects();
  }

  /** Sends everything waiting now (Ctrl+S on a cloud project). */
  flush(): Promise<boolean> {
    return this.sync.value?.flush() ?? Promise.resolve(true);
  }

  /**
   * Whether the open cloud project keeps this drawing's changes (sent, or
   * waiting in the device draft), so replacing the drawing loses nothing.
   * A viewer's edits are never sent: they are not kept. Nor can a deleted
   * project's, or one whose access was taken away.
   */
  autosaves(): boolean {
    const state = this.sync.value?.state.value;
    return !!this.project.value?.canWrite && !!state && state !== 'deleted' && state !== 'revoked';
  }

  /** The open cloud project, if it is this one. */
  private isOpen(tenantId: string, projectId: string): CloudProject | null {
    const p = this.project.value;
    return p && p.tenantId === tenantId && p.projectId === projectId ? p : null;
  }

  /**
   * Renames a cloud project for everyone. The open one is renamed through
   * its autosave (with whatever else waits, so it cannot conflict with
   * itself); another one with a metadata change based on its current
   * version. True when the server has the new name.
   */
  async rename(tenantId: string, projectId: string, name: string): Promise<boolean> {
    const title = name.trim();
    if (!title || title.length > 200) throw new Error('Proje adı boş olamaz ve en çok 200 karakter olabilir.');
    if (this.isOpen(tenantId, projectId)) {
      this.ctx.doc.name.set(title);
      return this.flush();
    }
    const info = await this.api.project(tenantId, projectId);
    await this.api.command({
      commandName: 'project.changes',
      version: 1,
      tenantId,
      projectId,
      requestId: `web-${uuid()}`,
      idempotencyKey: uuid(),
      expectedVersions: { '@project': info.metaVersion },
      input: { features: [], project: { name: title } },
    });
    return true;
  }

  /**
   * Deletes a cloud project for everyone (`project.delete`: its owner, or an
   * organisation's admin; the server keeps it, so the operator can restore
   * it). The open one is left: the drawing stays on screen as an unsaved
   * local drawing.
   */
  async deleteProject(tenantId: string, projectId: string): Promise<void> {
    await this.api.deleteProject(tenantId, projectId);
    const open = this.isOpen(tenantId, projectId);
    if (!open) return;
    this.detach();
    this.ctx.log.info(`“${open.name}” bulut projesi silindi. Çizim ekranda kaldı; saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`);
  }

  /** Someone deleted the open project: nothing more is sent or heard; the drawing and its unsent edits stay here. */
  private projectDeleted(project: CloudProject): void {
    const open = this.project.value;
    if (open?.projectId !== project.projectId) return;
    this.socket?.stop();
    this.ctx.log.warn(
      `“${open.name}” bulut projesi silindi. Değişiklikleriniz artık buluta kaydedilmiyor; gönderilmemiş olanlar bu cihazda saklanıyor. Çizimi saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`,
    );
  }

  /**
   * This account's access to the open project was taken away (TODOS.md
   * CLOUD-13): nothing more is sent or heard, no action on the project is
   * offered any more, and the drawing and its unsent edits stay here. The
   * notice (`accessLost`) says what the user can do.
   */
  private accessRevoked(project: CloudProject, reason: string): void {
    const open = this.project.value;
    if (open?.projectId !== project.projectId) return;
    this.socket?.stop();
    this.project.set({ ...open, permissions: [], canWrite: false, canEditMeta: false });
    const sync = this.sync.value;
    const keeps = !!sync?.keepsEdits;
    const unsent = keeps ? (sync?.pending.value ?? 0) : 0;
    this.ctx.log.warn(
      `“${open.name}” projesine erişiminiz kaldırıldı${reason ? ` (${reason.replace(/\.$/, '')})` : ''}. Değişiklikleriniz artık buluta kaydedilmiyor${unsent ? `; gönderilmemiş ${unsent} değişiklik bu cihazda saklanıyor` : ''}. Çizimi saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`,
    );
    this.accessLost.set({ name: open.name, unsent, keeps, reason });
  }

  /**
   * Leaves the open cloud project before another drawing takes its place:
   * what waits is sent if the server answers within `waitMs`, and whatever
   * is still unsent stays in this device's draft (it comes back when the
   * project is opened again). Returns how many changes stayed on the device.
   */
  async leave(waitMs = 5000): Promise<number> {
    const sync = this.sync.value;
    if (sync) {
      let timer = 0;
      const gaveUp = new Promise<void>((resolve) => (timer = setTimeout(resolve, waitMs) as unknown as number));
      await Promise.race([sync.flush().catch(() => false), gaveUp]);
      clearTimeout(timer);
      await sync.keepDraft();
    }
    const unsent = sync?.pending.value ?? 0;
    this.detach();
    return unsent;
  }

  /** Leaves the open cloud project (a local file is about to replace the drawing); any open in progress is dropped. */
  detach(dropOpening = true): void {
    if (dropOpening) this.generation++;
    this.socket?.stop();
    this.socket = null;
    this.unlink?.();
    this.unlink = null;
    this.sync.value?.dispose();
    this.sync.set(null);
    this.project.set(null);
    this.link.set('none');
    this.accessLost.set(null);
  }

  private attach(info: ProjectInfo, records: { localId: number; featureId: string; version: string }[], cursor: string): ProjectSync {
    const me = this.me.value!;
    // A project shared from someone else's personal space comes without a membership there.
    const own = !!this.membership(info.tenantId);
    const permissions = info.access.permissions;
    const project: CloudProject = {
      tenantId: info.tenantId,
      tenantName: workspaceName(info.tenantKind, info.tenantName, own),
      tenantKind: info.tenantKind,
      projectId: info.id,
      name: info.name,
      role: info.access.role,
      permissions,
      canWrite: permissions.includes('feature.write'),
      canEditMeta: permissions.includes('project.edit'),
    };
    // Created right after the sync, which asks it when access may have changed.
    let watch: AccessWatch | null = null;
    const sync = new ProjectSync({
      doc: this.ctx.doc,
      api: this.api,
      drafts: this.drafts,
      draftKey: draftKey(me.user.id, info.tenantId, info.id),
      userId: me.user.id,
      tenantId: info.tenantId,
      projectId: info.id,
      canEditMeta: project.canEditMeta,
      canWrite: project.canWrite,
      metaVersion: info.metaVersion,
      cursor,
      records,
      warn: (t) => this.ctx.log.warn(t),
      onDeleted: () => this.projectDeleted(project),
      onRevoked: (reason) => this.accessRevoked(project, reason),
      onAccessChanged: () => void watch?.check(),
    });
    watch = new AccessWatch({
      api: this.api,
      sync,
      project: () => (this.sync.value === sync ? this.project.value : null),
      update: (next) => this.project.set(next),
      resubscribe: () => this.socket?.reconnect(),
      info: (t) => this.ctx.log.info(t),
      warn: (t) => this.ctx.log.warn(t),
    });
    const socket = new ProjectSocket({
      url: socketUrl(),
      tenantId: info.tenantId,
      projectId: info.id,
      cursor: () => sync.cursor,
      onEvents: (events) => void sync.receive(events),
      onResync: () => {
        this.ctx.log.warn('Canlı bağlantı kaçırılan değişiklikleri veremiyor; proje sunucudan yeniden açılıyor.');
        this.open(info.tenantId, info.id).catch((e: unknown) => {
          // Deleted meanwhile (its event was among the ones no longer kept): the same as hearing it.
          if (e instanceof ApiFailure && e.deleted) sync.markDeleted();
          // Or its access was taken away meanwhile.
          else if (e instanceof ApiFailure && e.notFound) sync.markRevoked();
          else this.ctx.log.error(`Proje yeniden açılamadı: ${(e as Error).message}. Dosya → Bulut projesi aç ile yeniden deneyin.`);
        });
      },
      onError: (m, code) => {
        // The server ended the subscription: the project is gone for this account, or its organisation
        // may not be used now. Ask what is left; if the access is still there, subscribe again.
        if (code === 'not_found' || code === 'forbidden') void watch?.check(true);
        else this.ctx.log.warn(`Canlı bağlantı: ${m}`);
      },
    });
    this.socket = socket;
    const unlinkSocket = socket.state.subscribe((s) => this.link.set(s), true);
    // The drawing's name is the project's name: a rename here, or by another editor, shows in the status bar too.
    const unname = this.ctx.doc.name.subscribe((name) => {
      const p = this.project.value;
      if (p && p.name !== name) this.project.set({ ...p, name });
    });
    this.unlink = () => {
      unlinkSocket();
      unname();
    };
    this.sync.set(sync);
    this.project.set(project);
    socket.start();
    this.ctx.files.handle = null;
    return sync;
  }

  /**
   * Opens a cloud project: its metadata, then its objects page by page. True
   * when opened; false when cancelled or overtaken by another open.
   */
  async open(tenantId: string, projectId: string, progress: Progress = () => {}, signal?: AbortSignal): Promise<boolean> {
    const gen = ++this.generation;
    const stale = () => gen !== this.generation || !!signal?.aborted;
    const info = await this.api.project(tenantId, projectId);
    const total = Number(info.featureCount);
    const records: FeatureRecord[] = [];
    let after: string | null = null;
    progress(0, total);
    do {
      const page = await this.api.features(tenantId, projectId, after, PAGE, signal);
      if (stale()) return false;
      records.push(...page.features);
      after = page.next ?? null;
      progress(records.length, Math.max(total, records.length));
    } while (after);
    const read = readIncoming(info, records.map((r) => r.entity));
    if (!read.ok) throw new Error(`“${info.name}” okunamadı: ${read.error}`);
    if (stale()) return false;
    await this.sync.value?.flush().catch(() => false);
    // Another open may have started while the old project's changes went out.
    if (stale()) return false;
    this.detach(false);
    replaceDrawing(this.ctx, read.content);
    const sync = this.attach(info, records.map((r, i) => ({ localId: i + 1, featureId: r.id, version: r.version })), info.eventCursor);
    const draft = await this.drafts.get(draftKey(this.me.value!.user.id, tenantId, projectId)).catch(() => null);
    if (draft) {
      await sync.restore(draft);
      this.ctx.log.info('Bu cihazda gönderilmemiş değişiklikler vardı; çizime geri kondu.');
    }
    this.ctx.log.success(`“${info.name}” bulut projesi açıldı: ${records.length} nesne.`);
    return true;
  }

  /** Makes the current drawing a new cloud project in `tenantId`. */
  async upload(tenantId: string, name: string, progress: Progress = () => {}): Promise<boolean> {
    const doc = this.ctx.doc;
    // The drawing becomes the new project: the previous cloud project is left first (its changes sent).
    await this.sync.value?.flush().catch(() => false);
    this.detach();
    const tree = structuredClone([...doc.layers.tree]) as LayerNode[];
    const info = await this.api.createProject(
      tenantId,
      {
        name,
        settings: doc.settings.toJSON(),
        origin: { ...doc.origin },
        homeView: doc.homeView ? { ...doc.homeView } : undefined,
        layers: unlocked(tree),
        activeLayer: doc.layers.active.value,
        styles: { items: structuredClone([...doc.styles.value.items]), categories: structuredClone([...doc.styles.value.categories]) },
      },
      uuid(),
    );
    const entities = [...doc.all()];
    const records = entities.map((e) => ({ localId: e.id, featureId: uuid(), version: '1' }));
    let cursor = info.eventCursor;
    let metaVersion = info.metaVersion;
    progress(0, entities.length);
    for (let i = 0; i < entities.length; i += PAGE) {
      const part = entities.slice(i, i + PAGE);
      const result = await this.api.command({
        commandName: 'project.changes',
        version: 1,
        tenantId,
        projectId: info.id,
        requestId: `web-${uuid()}`,
        idempotencyKey: uuid(),
        expectedVersions: {},
        input: { features: part.map((e, j) => ({ op: 'create', id: records[i + j].featureId, entity: structuredClone(e) })) },
      });
      cursor = result.eventSeq;
      progress(Math.min(entities.length, i + PAGE), entities.length);
    }
    if (hasLocks(tree)) {
      const result = await this.api.command({
        commandName: 'project.changes',
        version: 1,
        tenantId,
        projectId: info.id,
        requestId: `web-${uuid()}`,
        idempotencyKey: uuid(),
        expectedVersions: { '@project': metaVersion },
        input: { features: [], project: { layers: tree, activeLayer: doc.layers.active.value } },
      });
      cursor = result.eventSeq;
      metaVersion = result.metaVersion;
    }
    doc.applyExternal({ meta: { name } });
    this.attach({ ...info, name, metaVersion }, records, cursor);
    doc.markSaved(doc.revision);
    this.ctx.log.success(`“${name}” buluta yüklendi: ${entities.length} nesne. Bundan sonra değişiklikler kendiliğinden kaydedilir.`);
    return true;
  }
}
