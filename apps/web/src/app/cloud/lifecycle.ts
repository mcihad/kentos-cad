import type { CheckpointChange } from '../../contracts/generated/CheckpointChange';
import type { CheckpointCreate } from '../../contracts/generated/CheckpointCreate';
import type { CheckpointRestore } from '../../contracts/generated/CheckpointRestore';
import type { ProjectCatalogChange } from '../../contracts/generated/ProjectCatalogChange';
import type { ProjectDuplicated } from '../../contracts/generated/ProjectDuplicated';
import type { ProjectPurged } from '../../contracts/generated/ProjectPurged';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import { catalogEnvelope, day } from './catalog';
import type { CloudSession } from './session';

/**
 * The catalog's commands on a cloud project (docs/adr/0028, TODOS.md
 * CLOUD-05): archive and unarchive, the trash and restoring from it,
 * removing for good, copying, a new project in the other storage mode
 * (docs/adr/0039), checkpoints and restoring a point of the history as a
 * new project (docs/adr/0034), the catalog metadata and favourites. Each is
 * one product command with its own idempotency key; the server checks the
 * rights and answers. When the project is the one open here, the session
 * follows: its unsent edits go first, archiving it makes it read-only,
 * unarchiving opens it again to resume saving, moving it to the trash
 * leaves it (the drawing stays on screen), and a new name goes through its
 * autosave, which knows its own metadata version.
 */

/** A project a command is about: where it is, and its name now (the permanent delete names it). */
export interface ProjectRef {
  tenantId: string;
  projectId: string;
  name: string;
}

/** Catalog metadata to change; absent fields stay. */
export interface MetadataPatch {
  name?: string;
  description?: string;
  projectType?: ProjectType;
  tags?: string[];
}

interface Log {
  info(text: string): void;
}

export class ProjectLifecycle {
  private readonly s: CloudSession;
  private readonly log: Log;

  constructor(session: CloudSession, log: Log) {
    this.s = session;
    this.log = log;
  }

  private send<T>(commandName: string, ref: ProjectRef, input: unknown, expected: Record<string, string> = {}): Promise<T> {
    const envelope = catalogEnvelope(commandName, ref.tenantId, ref.projectId, input, expected);
    // The event of a command sent from here is this window's own (the open project's sync does not announce it).
    if (this.s.openProject(ref.tenantId, ref.projectId)) this.s.sync.value?.expect(envelope.requestId);
    return this.s.api.lifecycle<T>(envelope);
  }

  /** The open project's unsent edits go first (what cannot go stays in its device draft). */
  private async settle(ref: ProjectRef): Promise<void> {
    if (this.s.openProject(ref.tenantId, ref.projectId)) await this.s.flush().catch(() => false);
  }

  async archive(ref: ProjectRef): Promise<ProjectCatalogChange> {
    await this.settle(ref);
    const done = await this.send<ProjectCatalogChange>('project.archive', ref, {});
    const open = this.s.openProject(ref.tenantId, ref.projectId);
    if (open && open.state !== 'archived') {
      this.s.sync.value?.markArchived(true);
      this.s.projectArchived(open, true);
    }
    return done;
  }

  /** Unarchived, the open project is opened again: its saving resumes and edits kept on this device come back. */
  async unarchive(ref: ProjectRef): Promise<ProjectCatalogChange> {
    const done = await this.send<ProjectCatalogChange>('project.unarchive', ref, {});
    if (done.changed && this.s.openProject(ref.tenantId, ref.projectId)) await this.s.open(ref.tenantId, ref.projectId);
    return done;
  }

  async trash(ref: ProjectRef): Promise<ProjectCatalogChange> {
    await this.settle(ref);
    const done = await this.send<ProjectCatalogChange>('project.trash', ref, {});
    const open = this.s.openProject(ref.tenantId, ref.projectId);
    if (open) this.s.leftTrashed(open, done.project.purgeAfter ? `${day(done.project.purgeAfter)} tarihine kadar geri yüklenebilir` : '');
    return done;
  }

  restore(ref: ProjectRef): Promise<ProjectCatalogChange> {
    return this.send<ProjectCatalogChange>('project.restore', ref, {});
  }

  /** Removes a project in the trash for good; the server wants its name as the confirmation. */
  purge(ref: ProjectRef): Promise<ProjectPurged> {
    return this.send<ProjectPurged>('project.purge', ref, { confirmName: ref.name });
  }

  duplicate(ref: ProjectRef, into: { name?: string; tenantId?: string } = {}): Promise<ProjectDuplicated> {
    return this.send<ProjectDuplicated>('project.duplicate', ref, {
      ...(into.name?.trim() ? { name: into.name.trim() } : {}),
      ...(into.tenantId ? { tenantId: into.tenantId } : {}),
    });
  }

  /**
   * A new project kept as `to` from this one's present state (`project.convert`,
   * docs/adr/0039): a file project's newest revision imported into a
   * database project (“PostGIS'e aktar”), or a database project's snapshot
   * as a file project's revision 1. The source does not change; its unsent
   * edits go first. The answer is the new project, as a copy's.
   */
  async convert(ref: ProjectRef, to: ProjectStorage, into: { name?: string; tenantId?: string } = {}): Promise<ProjectDuplicated> {
    await this.settle(ref);
    // The input of `ProjectConvert` v1 (crates/shared/contracts, docs/adr/0039).
    const input: { to: ProjectStorage; name?: string; tenantId?: string } = { to };
    if (into.name?.trim()) input.name = into.name.trim();
    if (into.tenantId) input.tenantId = into.tenantId;
    return this.send<ProjectDuplicated>('project.convert', ref, input);
  }

  /**
   * A named checkpoint (`project.checkpoint.create`, docs/adr/0034): a
   * database project's present state, or a file project's revision (its
   * newest when `fileRevision` is absent). The open project's unsent edits
   * go first, so the checkpoint holds them.
   */
  async createCheckpoint(ref: ProjectRef, input: CheckpointCreate): Promise<CheckpointChange> {
    await this.settle(ref);
    const out: CheckpointCreate = { name: input.name.trim() };
    if (input.note?.trim()) out.note = input.note.trim();
    if (input.fileRevision) out.fileRevision = input.fileRevision;
    return this.send<CheckpointChange>('project.checkpoint.create', ref, out);
  }

  /** Removes a checkpoint (its maker, or someone with `project.edit`); a file project's revision stays. */
  deleteCheckpoint(ref: ProjectRef, checkpointId: string): Promise<CheckpointChange> {
    return this.send<CheckpointChange>('project.checkpoint.delete', ref, { checkpointId });
  }

  /**
   * A point of the history as a new project (`project.checkpoint.restore`):
   * a checkpoint, or a file project's revision. The source does not change;
   * the answer is the new project, as a copy's.
   */
  restoreCheckpoint(ref: ProjectRef, point: { checkpointId: string } | { fileRevision: string }, into: { name?: string; tenantId?: string } = {}): Promise<ProjectDuplicated> {
    const input: CheckpointRestore = { ...point };
    if (into.name?.trim()) input.name = into.name.trim();
    if (into.tenantId) input.tenantId = into.tenantId;
    return this.send<ProjectDuplicated>('project.checkpoint.restore', ref, input);
  }

  setFavorite(ref: ProjectRef, favorite: boolean): Promise<ProjectCatalogChange> {
    return this.send<ProjectCatalogChange>('project.favorite', ref, { favorite });
  }

  /**
   * Changes a project's catalog metadata from `catalogVersion` (the version
   * it was shown at; a newer one on the server refuses, nothing written).
   * The open project's new name goes through its autosave first; the rest
   * follows from the version that rename made. Null when only the open
   * project's name changed.
   */
  async updateMetadata(ref: ProjectRef, patch: MetadataPatch, catalogVersion?: string): Promise<ProjectCatalogChange | null> {
    const rest: MetadataPatch = { ...patch };
    let version = catalogVersion;
    const open = this.s.openProject(ref.tenantId, ref.projectId);
    if (open && rest.name !== undefined) {
      const name = rest.name.trim();
      delete rest.name;
      if (name !== open.name) {
        if (!(await this.s.rename(ref.tenantId, ref.projectId, name)))
          throw new Error('Yeni ad kaydedilemedi; durum çubuğundaki kayıt durumuna bakın (çakışma ya da bağlantı).');
        this.log.info(`Proje “${name}” olarak yeniden adlandırıldı.`);
        if (!Object.keys(rest).length) return null;
        version = (await this.s.api.details(ref.tenantId, ref.projectId)).project.catalogVersion;
      }
    }
    return this.send<ProjectCatalogChange>('project.metadata.update', ref, rest, version ? { '@catalog': version } : {});
  }
}
