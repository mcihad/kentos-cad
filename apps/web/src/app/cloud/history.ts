import type { Checkpoint } from '../../contracts/generated/Checkpoint';
import type { CheckpointKind } from '../../contracts/generated/CheckpointKind';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FileRevisions } from '../../contracts/generated/FileRevisions';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { CloudApi } from './api';

/**
 * A cloud project's history as the catalog shows it (docs/adr/0034, 0038;
 * TODOS.md SYNC-11, CLOUD-07): a file project's revisions and every
 * project's named checkpoints, with who may do what with them. The server
 * decides every request; these rules only keep a button from promising
 * what it would refuse, and say which right is missing.
 */

export const KIND_LABEL: Record<CheckpointKind, string> = {
  snapshot: 'Anlık görüntü',
  revision: 'Adlandırılmış revizyon',
};

/** What a checkpoint holds, in one line: a database project's data revision, or a file project's revision. */
export function pointText(c: Checkpoint): string {
  return c.kind === 'revision' ? `revizyon ${c.revision}` : `veri revizyonu ${c.revision}`;
}

export interface HistoryData {
  storage: ProjectStorage;
  /** A file project's revisions, newest first; null for a database project. */
  revisions: FileRevisions | null;
  /** The checkpoints, newest first; null when the account may not see the history (`project.history`). */
  checkpoints: Checkpoint[] | null;
}

/** A project's history: its revisions (a file project) and its checkpoints (with `project.history`). */
export async function loadHistory(api: CloudApi, tenantId: string, projectId: string, storage: ProjectStorage, permissions: readonly ProjectPermission[], signal?: AbortSignal): Promise<HistoryData> {
  const [revisions, checkpoints] = await Promise.all([
    storage === 'file' ? api.fileRevisions(tenantId, projectId, signal) : Promise.resolve(null),
    permissions.includes('project.history') ? api.checkpoints(tenantId, projectId, signal).then((c) => c.checkpoints) : Promise.resolve(null),
  ]);
  return { storage, revisions, checkpoints };
}

/** The events after which the history shows something else. */
export const changesHistory = (events: readonly EventRecord[]): boolean => events.length === 0 || events.some((e) => e.kind === 'project.checkpoint' || e.kind === 'project.file');

const missing = (right: ProjectPermission, what: string) => `Bu projede ${what} yetkiniz yok (${right}); proje sahibine ya da yöneticisine başvurun.`;

/** Why the account may not name a checkpoint here (`feature.write`, not in the archive), or null. */
export function whyNotCreate(permissions: readonly ProjectPermission[], archived: boolean, storage: ProjectStorage, revisions: FileRevisions | null): string | null {
  if (archived) return 'Arşivlenmiş projede kontrol noktası oluşturulmaz; önce arşivden çıkarın.';
  if (!permissions.includes('feature.write')) return missing('feature.write', 'kontrol noktası oluşturma');
  if (storage === 'file' && !revisions?.current) return 'Projenin henüz kaydedilmiş revizyonu yok; önce Kaydet ile bir revizyon yazın.';
  return null;
}

/**
 * Why the account may not remove this checkpoint, or null: as the server
 * decides it (docs/adr/0034), one who may write the project, and then only
 * its maker or someone with `project.edit`; never in the archive.
 */
export function whyNotDelete(c: Checkpoint, me: string | undefined, permissions: readonly ProjectPermission[], archived: boolean): string | null {
  if (archived) return 'Arşivlenmiş projede kontrol noktası silinmez; önce arşivden çıkarın.';
  if (!permissions.includes('feature.write')) return missing('feature.write', 'kontrol noktası silme');
  if (c.createdBy === me || permissions.includes('project.edit')) return null;
  return 'Kontrol noktasını yalnız onu oluşturan ya da projeyi yöneten (project.edit) silebilir.';
}

/** Why the account may not download a file project's revision (`project.download`; listing them is `project.read`), or null. */
export function whyNotDownload(permissions: readonly ProjectPermission[]): string | null {
  return permissions.includes('project.download') ? null : missing('project.download', 'indirme');
}

/** Why the account may not download a checkpoint or restore a point of the history (`project.history`, `project.download`), or null. */
export function whyNotTake(permissions: readonly ProjectPermission[], what: string): string | null {
  if (!permissions.includes('project.history')) return missing('project.history', what);
  if (!permissions.includes('project.download')) return missing('project.download', what);
  return null;
}
