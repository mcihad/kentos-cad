import { Emitter } from '../../core/emitter';
import { Signal } from '../../core/signal';
import type { AuthConfig } from '../../contracts/generated/AuthConfig';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FeatureRecord } from '../../contracts/generated/FeatureRecord';
import type { Me } from '../../contracts/generated/Me';
import type { MembershipView } from '../../contracts/generated/MembershipView';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import type { ProjectList } from '../../contracts/generated/ProjectList';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectRole } from '../../contracts/generated/ProjectRole';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import type { TenantKind } from '../../contracts/generated/TenantKind';
import type { AppContext } from '../context';
import { replaceDrawing } from '../fileIO';
import { AccessWatch, type AccessTarget } from './accessWatch';
import { ApiFailure, HttpCloudApi, type CloudApi } from './api';
import { catalogEnvelope } from './catalog';
import { browserDraftStore, draftKey, type DraftStore } from './drafts';
import { FileProjectSave, type NewerRevision } from './fileProject';
import { openFileProject, saveFileProject, uploadAsFileProject, type FileStage } from './fileSession';
import { uploadAsDatabaseProject } from './importing';
import { readProject } from './incoming';
import { ProjectLifecycle } from './lifecycle';
import { ProjectSocket, socketUrl, type LinkState, type SocketOptions } from './socket';
import { ProjectSync } from './sync';
import { ENDED } from './syncCore';

/**
 * The cloud side of the app (`ctx.cloud`, Faz B): who is signed in, which
 * cloud project is open, and the machinery that keeps it in step with the
 * server (autosave: sync.ts, live events: socket.ts). Opening is paged and
 * can be cancelled; a slow answer for a project the user already left is
 * thrown away (`generation`, CLAUDE.md §21.2). An object's persistent id is
 * its id on the server both ways (ADR 0014 slice 3, docs/adr/0026): opening
 * gives each object the server's id as its `uid`, uploading sends each one
 * under its `uid`. Uploading a local drawing creates the project and imports
 * the drawing's `.kcad` into it in one transaction (importing.ts); a server
 * without the import gets the objects in batches (upload.ts). A step whose
 * answer is lost goes again with the same idempotency key.
 * A project is kept object by object in PostGIS (`database`, the autosave
 * above) or as `.kcad` revisions (`file`, docs/adr/0031, 0038): a file
 * project opens from its newest revision through the staged open of a file,
 * is saved only by Kaydet, as a new revision on the one it came from
 * (fileProject.ts, fileSession.ts), and a new database project is filled
 * from the drawing's uploaded `.kcad` in one import (importing.ts).
 * A project can be renamed (a metadata change) and, by its owner or an
 * organisation's admin, deleted (moved to the trash); when someone else
 * deletes or archives the open one, its sync stops sending and the drawing
 * stays on screen, its edits kept on this device. An archived project opens
 * read-only (docs/adr/0028). The catalog's commands (archive, trash,
 * restore, duplicate, metadata, favourites) are `lifecycle` (lifecycle.ts).
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
  /** Active, or archived (read-only until it is unarchived and opened again). */
  state: 'active' | 'archived';
  /** Object by object in PostGIS, or `.kcad` revisions saved by Kaydet (docs/adr/0031). */
  storage: ProjectStorage;
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

/** What the catalog says of a drawing uploaded as a new project. */
export interface UploadCatalog {
  projectType?: ProjectType;
  description?: string;
  tags?: readonly string[];
}

/** Events of the open project as they arrive (the history panel refreshes on `project.checkpoint`). */
export interface ProjectEvents {
  events: { tenantId: string; projectId: string; events: readonly EventRecord[] };
}

/** The live channel of a project (tests pass their own). */
export type SocketFactory = (o: Omit<SocketOptions, 'url'>) => Pick<ProjectSocket, 'state' | 'start' | 'stop' | 'reconnect'>;

const PAGE = 2000;

export class CloudSession {
  readonly auth = new Signal<AuthState>('unknown');
  readonly me = new Signal<Me | null>(null);
  readonly config = new Signal<AuthConfig | null>(null);
  readonly project = new Signal<CloudProject | null>(null);
  readonly sync = new Signal<ProjectSync | null>(null);
  /** The open file project's Kaydet (fileProject.ts); null for a database project or none. */
  readonly file = new Signal<FileProjectSave | null>(null);
  /** The open project's events as they arrive, after the sync took them. */
  readonly events = new Emitter<ProjectEvents>();
  /** The live channel of the open project ('none' when no cloud project is open). */
  readonly link = new Signal<LinkState | 'none'>('none');
  /** Set when the open project's access is taken away (the interface shows a notice); cleared when the project is left. */
  readonly accessLost = new Signal<AccessLost | null>(null);
  /** Whether drafts survive a reload on this browser. */
  readonly durableDrafts: boolean;
  readonly api: CloudApi;
  /** The catalog's commands on any project, the open one included (docs/adr/0028). */
  readonly lifecycle: ProjectLifecycle;
  /**
   * Asks what to do when Kaydet met a newer revision of the open file
   * project (set by the app: ui/cloud/FileConflict.ts); true when the
   * drawing ended up saved somewhere.
   */
  fileConflict: (() => Promise<boolean>) | null = null;
  /**
   * A file larger than this goes to the server in parts of this size
   * (docs/adr/0045); unset, the standard 8 MiB. The cloud end-to-end test
   * sets a small one to send a drawing in parts.
   */
  uploadPart: number | undefined = undefined;
  /** The app's services this session works with (the file project's modules use it too). */
  readonly ctx: AppContext;
  private readonly drafts: DraftStore;
  private readonly sockets: SocketFactory;
  private socket: Pick<ProjectSocket, 'state' | 'start' | 'stop' | 'reconnect'> | null = null;
  private unlink: (() => void) | null = null;
  private generation = 0;

  constructor(ctx: AppContext, api: CloudApi = new HttpCloudApi(), drafts?: DraftStore, sockets?: SocketFactory) {
    this.ctx = ctx;
    this.api = api;
    const store = drafts ? { store: drafts, durable: true } : browserDraftStore();
    this.drafts = store.store;
    this.durableDrafts = store.durable;
    this.sockets = sockets ?? ((o) => new ProjectSocket({ ...o, url: socketUrl() }));
    this.lifecycle = new ProjectLifecycle(this, ctx.log);
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

  /** Sends everything waiting now (Ctrl+S on a database project; a file project is saved by `saveFile`). */
  flush(): Promise<boolean> {
    return this.sync.value?.flush() ?? Promise.resolve(true);
  }

  /** Kaydet on the open file project: a new revision (fileSession.ts). True when the drawing is saved. */
  saveFile(): Promise<boolean> {
    return saveFileProject(this);
  }

  /**
   * Whether the drawing's unsaved changes are kept in a device draft (an
   * open database project's): then the local recovery copy is not needed
   * (app/recovery.ts). A file project's changes live only in the drawing
   * until Kaydet, like a local drawing's.
   */
  keepsDeviceDraft(): boolean {
    return !!this.sync.value;
  }

  /**
   * Whether the open cloud project keeps this drawing's changes (sent, or
   * waiting in the device draft), so replacing the drawing loses nothing.
   * A viewer's edits are never sent: they are not kept. Nor can a deleted
   * project's, or one whose access was taken away.
   */
  autosaves(): boolean {
    const state = this.sync.value?.state.value;
    return !!this.project.value?.canWrite && !!state && !ENDED.includes(state);
  }

  /** The open cloud project, if it is this one. */
  openProject(tenantId: string, projectId: string): CloudProject | null {
    const p = this.project.value;
    return p && p.tenantId === tenantId && p.projectId === projectId ? p : null;
  }

  /**
   * Renames a cloud project for everyone. The open one is renamed through
   * its autosave (with whatever else waits, so it cannot conflict with
   * itself); another one with the `project.rename` command (docs/adr/0028).
   * True when the server has the new name.
   */
  async rename(tenantId: string, projectId: string, name: string): Promise<boolean> {
    const title = name.trim();
    if (!title || title.length > 200) throw new Error('Proje adı boş olamaz ve en çok 200 karakter olabilir.');
    const open = this.openProject(tenantId, projectId);
    if (open && open.storage === 'database') {
      this.ctx.doc.name.set(title);
      return this.flush();
    }
    if (open) {
      // A file project's name is the catalog's: the next revision carries it, the drawing takes it quietly now.
      const envelope = catalogEnvelope('project.rename', tenantId, projectId, { name: title });
      this.file.value?.expect(envelope.requestId);
      await this.api.lifecycle(envelope);
      this.ctx.doc.applyExternal({ meta: { name: title } });
      return true;
    }
    await this.api.lifecycle(catalogEnvelope('project.rename', tenantId, projectId, { name: title }));
    return true;
  }

  /**
   * Deletes a cloud project for everyone: moves it to the trash
   * (`project.trash`: its owner, or an organisation's admin; restorable
   * until the trash's retention ends). The open one is left: the drawing
   * stays on screen as an unsaved local drawing.
   */
  async deleteProject(tenantId: string, projectId: string, name = ''): Promise<void> {
    await this.lifecycle.trash({ tenantId, projectId, name });
  }

  /** The open project was moved to the trash from here: it is left, the drawing stays. */
  leftTrashed(open: CloudProject, until: string): void {
    this.detach();
    this.ctx.log.info(`“${open.name}” bulut projesi çöp kutusuna taşındı${until ? ` (${until})` : ''}. Çizim ekranda kaldı; saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`);
  }

  /**
   * The open project is archived: nothing more is sent or heard; the drawing
   * and its unsent edits stay here (`byMe`: archived from this window).
   */
  projectArchived(project: CloudProject, byMe = false): void {
    const open = this.project.value;
    if (open?.projectId !== project.projectId) return;
    this.socket?.stop();
    this.project.set({ ...open, state: 'archived', canWrite: false, canEditMeta: false });
    if (byMe) this.ctx.log.info(`“${open.name}” arşivlendi: salt okunur. Değiştirmek için arşivden çıkarın; çizim ekranda kalıyor.`);
    else if (open.storage === 'file')
      this.ctx.log.warn(
        `“${open.name}” bulut projesi arşivlendi; çizim artık buluta kaydedilemez. Çizim ekranda kalıyor; kaydedilmemiş değişiklikleri saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`,
      );
    else
      this.ctx.log.warn(
        `“${open.name}” bulut projesi arşivlendi; değişiklikleriniz artık buluta kaydedilmiyor, gönderilmemiş olanlar bu cihazda saklanıyor. Proje arşivden çıkarılınca yeniden açın: saklanan değişiklikler geri gelir. Çizimi saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`,
      );
  }

  /** Someone deleted the open project: nothing more is sent or heard; the drawing and its unsent edits stay here. */
  private projectDeleted(project: CloudProject): void {
    const open = this.project.value;
    if (open?.projectId !== project.projectId) return;
    this.socket?.stop();
    this.ctx.log.warn(
      open.storage === 'file'
        ? `“${open.name}” bulut projesi silindi; çizim artık buluta kaydedilemez. Çizim ekranda kalıyor; saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`
        : `“${open.name}” bulut projesi silindi. Değişiklikleriniz artık buluta kaydedilmiyor; gönderilmemiş olanlar bu cihazda saklanıyor. Çizimi saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`,
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
    this.file.value?.dispose();
    this.file.set(null);
    this.project.set(null);
    this.link.set('none');
    this.accessLost.set(null);
  }

  /**
   * Hears a project's events while something shows it (the catalog's
   * history): the open project's through its own channel, another one's
   * through a channel of its own from its event cursor now. Returns the stop.
   */
  watchProject(tenantId: string, projectId: string, onEvents: (events: readonly EventRecord[]) => void): () => void {
    if (this.openProject(tenantId, projectId))
      return this.events.on('events', (e) => {
        if (e.tenantId === tenantId && e.projectId === projectId) onEvents(e.events);
      });
    let stopped = false;
    let socket: ReturnType<SocketFactory> | null = null;
    void this.api.project(tenantId, projectId).then(
      (info) => {
        if (stopped) return;
        let cursor = info.eventCursor;
        socket = this.sockets({
          tenantId,
          projectId,
          cursor: () => cursor,
          onEvents: (events) => {
            cursor = events[events.length - 1]?.seq ?? cursor;
            onEvents(events);
          },
          onResync: () => onEvents([]),
          onError: () => {},
        });
        socket.start();
      },
      () => {},
    );
    return () => {
      stopped = true;
      socket?.stop();
    };
  }

  /** Starts an open: a later open, or leaving the project, makes it stale (CLAUDE.md §21.2). */
  beginOpen(signal?: AbortSignal): () => boolean {
    const gen = ++this.generation;
    return () => gen !== this.generation || !!signal?.aborted;
  }

  /** The project as the interface keeps it while it is open. */
  private cloudProject(info: ProjectInfo, storage: ProjectStorage): CloudProject {
    // A project shared from someone else's personal space comes without a membership there.
    const own = !!this.membership(info.tenantId);
    const permissions = info.access.permissions;
    // An archived project opens read-only, whatever the role (docs/adr/0028).
    const archived = info.state === 'archived';
    return {
      tenantId: info.tenantId,
      tenantName: workspaceName(info.tenantKind, info.tenantName, own),
      tenantKind: info.tenantKind,
      projectId: info.id,
      name: info.name,
      role: info.access.role,
      state: archived ? 'archived' : 'active',
      storage,
      permissions,
      canWrite: !archived && permissions.includes('feature.write'),
      canEditMeta: !archived && permissions.includes('project.edit'),
    };
  }

  /**
   * The live channel, the access watch and the name of an open project,
   * for either kind: `receive` takes its events, `target` its access
   * changes; the history panel hears the events after them.
   */
  private connect(info: ProjectInfo, project: CloudProject, receive: (events: EventRecord[]) => void, target: AccessTarget, cursor: () => string, explicit: boolean): AccessWatch {
    const watch = new AccessWatch({
      api: this.api,
      sync: target,
      explicit,
      project: () => (this.project.value?.projectId === project.projectId ? this.project.value : null),
      update: (next) => this.project.set(next),
      resubscribe: () => this.socket?.reconnect(),
      info: (t) => this.ctx.log.info(t),
      warn: (t) => this.ctx.log.warn(t),
    });
    const socket = this.sockets({
      tenantId: info.tenantId,
      projectId: info.id,
      cursor,
      onEvents: (events) => {
        receive(events);
        this.events.emit('events', { tenantId: info.tenantId, projectId: info.id, events });
      },
      onResync: () => {
        this.ctx.log.warn('Canlı bağlantı kaçırılan değişiklikleri veremiyor; proje sunucudan yeniden açılıyor.');
        this.open(info.tenantId, info.id).catch((e: unknown) => {
          // Deleted meanwhile (its event was among the ones no longer kept): the same as hearing it.
          if (e instanceof ApiFailure && e.deleted) target.markDeleted();
          // Or its access was taken away meanwhile.
          else if (e instanceof ApiFailure && e.notFound) target.markRevoked();
          else this.ctx.log.error(`Proje yeniden açılamadı: ${(e as Error).message}. Dosya → Bulut projesi aç ile yeniden deneyin.`);
        });
      },
      onError: (m, code) => {
        // The server ended the subscription: the project is gone for this account, or its organisation
        // may not be used now. Ask what is left; if the access is still there, subscribe again.
        if (code === 'not_found' || code === 'forbidden') void watch.check(true);
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
    return watch;
  }

  /**
   * Makes the drawing on screen the open file project `info`, based on
   * revision `base` ("0": none yet); `cursor` is the event cursor it was
   * read at. Kaydet writes its next revision (fileProject.ts).
   */
  attachFile(info: ProjectInfo, base: string, cursor: string, onNewer?: (newer: NewerRevision) => void): FileProjectSave {
    const project = this.cloudProject(info, 'file');
    let watch: AccessWatch | null = null;
    const file = new FileProjectSave({
      doc: this.ctx.doc,
      api: this.api,
      tenantId: info.tenantId,
      projectId: info.id,
      base,
      cursor,
      canWrite: project.canWrite,
      codec: () => this.ctx.files.kcad(),
      warn: (t) => this.ctx.log.warn(t),
      onNewer,
      onDeleted: () => this.projectDeleted(project),
      onRevoked: (reason) => this.accessRevoked(project, reason),
      onArchived: () => this.projectArchived(project),
      onAccessChanged: () => void watch?.check(),
      part: () => this.uploadPart,
    });
    watch = this.connect(info, project, (events) => file.receive(events), file, () => file.cursor, true);
    this.file.set(file);
    this.project.set(project);
    if (info.state === 'archived') file.markArchived(true);
    else this.socket?.start();
    this.ctx.files.handle = null;
    return file;
  }

  /** Makes the drawing on screen the open database project `info` (its objects' ids and versions in `records`). */
  attach(info: ProjectInfo, records: { id: string; version: string }[], cursor: string): ProjectSync {
    const me = this.me.value!;
    const project = this.cloudProject(info, 'database');
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
      onArchived: () => this.projectArchived(project),
      onAccessChanged: () => void watch?.check(),
    });
    watch = this.connect(
      info,
      project,
      (events) => {
        // Someone imported a whole drawing into it (docs/adr/0036): opened again, as the event asks.
        if (events.some((e) => e.kind === 'project.import' && !(e.requestId && sync.ownRequest(e.requestId)))) {
          this.ctx.log.info(`“${info.name}” projesine bir çizim içe aktarıldı; proje sunucudan yeniden açılıyor.`);
          void this.open(info.tenantId, info.id).catch((e: unknown) => this.ctx.log.error(`Proje yeniden açılamadı: ${(e as Error).message}`));
          return;
        }
        void sync.receive(events);
      },
      sync,
      () => sync.cursor,
      false,
    );
    this.sync.set(sync);
    this.project.set(project);
    // Nothing is sent to an archived project, and nothing of it changes: no live channel.
    if (info.state === 'archived') sync.markArchived(true);
    else this.socket?.start();
    this.ctx.files.handle = null;
    return sync;
  }

  /**
   * Opens a cloud project: a database project's metadata, then its objects
   * page by page; a file project's newest revision, downloaded and opened
   * as a file is (fileSession.ts). `storage` when the caller knows it (the
   * catalog); else the server is asked. True when opened; false when
   * cancelled or overtaken by another open.
   */
  async open(tenantId: string, projectId: string, progress: Progress = () => {}, signal?: AbortSignal): Promise<boolean> {
    const stale = this.beginOpen(signal);
    const info = await this.api.project(tenantId, projectId);
    if (stale()) return false;
    // How it keeps its content is the project's own (an older server's answer without it: a database project).
    if (info.storage === 'file') return openFileProject(this, info, progress, signal, stale);
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
    // Each object's persistent id is the server's id for it.
    const read = readProject(info, records);
    if (!read.ok) throw new Error(`“${info.name}” okunamadı: ${read.error}`);
    if (stale()) return false;
    await this.sync.value?.flush().catch(() => false);
    // Another open may have started while the old project's changes went out.
    if (stale()) return false;
    // A drawing replaced without the question keeps its unsaved work in its recovery copy (app/recovery.ts).
    await this.ctx.recovery?.flush();
    if (stale()) return false;
    this.detach(false);
    replaceDrawing(this.ctx, read.content);
    const sync = this.attach(info, records.map((r) => ({ id: r.id, version: r.version })), info.eventCursor);
    const draft = await this.drafts.get(draftKey(this.me.value!.user.id, tenantId, projectId)).catch(() => null);
    if (draft && (await sync.restore(draft))) this.ctx.log.info('Bu cihazda gönderilmemiş değişiklikler vardı; çizime geri kondu.');
    this.ctx.log.success(`“${info.name}” bulut projesi açıldı: ${records.length} nesne.`);
    if (info.state === 'archived')
      this.ctx.log.info(`“${info.name}” arşivlenmiş bir proje: salt okunur açıldı; değişiklikler buluta kaydedilmez. Düzenlemek için arşivden çıkarılmalı ya da kopyası oluşturulmalı.`);
    return true;
  }

  /**
   * Makes the current drawing a new database project in `tenantId`, with
   * its catalog metadata (docs/adr/0028): the project is created, the
   * drawing's `.kcad` uploaded and imported in one transaction (importing.ts,
   * docs/adr/0036). A refused import leaves the new project empty and the
   * drawing local (`UploadFailed`). `stage` hears where it is.
   */
  upload(tenantId: string, name: string, progress: Progress = () => {}, catalog: UploadCatalog = {}, stage?: (s: FileStage) => void): Promise<boolean> {
    return uploadAsDatabaseProject(this, tenantId, name, progress, catalog, stage);
  }

  /**
   * Makes the current drawing a new file project in `tenantId` (“Buluta
   * dosya olarak kaydet”, docs/adr/0031): created with `storage: file`, the
   * drawing's `.kcad` uploaded and committed as revision 1.
   */
  uploadFile(tenantId: string, name: string, progress: Progress = () => {}, catalog: UploadCatalog = {}, stage?: (s: FileStage) => void): Promise<boolean> {
    return uploadAsFileProject(this, tenantId, name, progress, catalog, stage);
  }

}
