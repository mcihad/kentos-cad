import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { LayerNode } from '../../contracts/generated/LayerNode';
import type { ProjectImported } from '../../contracts/generated/ProjectImported';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import type { DrawingEntity } from '../../model/entities';
import { ApiFailure } from './api';
import type { CloudSession, Progress, UploadCatalog } from './session';
import { importEnvelope } from './transfer';
import { again, uploadObjects } from './upload';
import { UploadFailed, createInput, sendDrawing, type FileStage } from './uploading';

/**
 * The drawing on screen as a new database project (docs/adr/0036, 0038):
 * the project is created, the drawing's `.kcad` uploaded and imported in
 * one transaction (`project.import`). Every object comes in under its
 * persistent id at version 1, or none does: the first object the server
 * does not take refuses the whole file, by its place (`entities[i]`), and
 * the new project stays empty while the drawing stays local
 * (`UploadFailed`). Only a server without the import (an older one) gets
 * the objects the old way, in batches (upload.ts), then the layer tree
 * with its locks.
 */

/**
 * The server has no import path: no upload route at all (its bare 404 or
 * 405), a database project it takes no file into, or no `project.import`
 * command. Anything else is a real answer and is said.
 */
export function importUnavailable(e: unknown): boolean {
  if (!(e instanceof ApiFailure)) return false;
  if ((e.status === 404 && e.code === 'http') || e.status === 405) return true;
  return e.code === 'invalid' && !e.path && /dosya yüklenmez|Bilinmeyen komut: project\.import/.test(e.message);
}

function hasLocks(nodes: readonly LayerNode[]): boolean {
  return nodes.some((n) => n.locked || hasLocks(n.children));
}

/** Where the drawing's objects are on the server once they are in: their versions and the event cursor. */
interface Landed {
  records: { id: string; version: string }[];
  cursor: string;
  metaVersion: string;
}

/**
 * The old way, for a server without the import: the objects in batches
 * (each object under its persistent id), then the layer tree with its
 * locks (the project was created with every layer unlocked).
 */
async function uploadInBatches(s: CloudSession, info: ProjectInfo, progress: Progress): Promise<Landed> {
  const doc = s.ctx.doc;
  const tree = structuredClone([...doc.layers.tree]) as LayerNode[];
  const entities = [...doc.all()] as DrawingEntity[];
  const sent = await uploadObjects(s.api, { tenantId: info.tenantId, projectId: info.id, cursor: info.eventCursor }, entities, progress);
  let cursor = sent.cursor;
  let metaVersion = info.metaVersion;
  if (hasLocks(tree)) {
    const locks: CommandEnvelope = {
      commandName: 'project.changes',
      version: 1,
      tenantId: info.tenantId,
      projectId: info.id,
      requestId: `web-${crypto.randomUUID()}`,
      idempotencyKey: crypto.randomUUID(),
      expectedVersions: { '@project': metaVersion },
      input: { features: [], project: { layers: tree, activeLayer: doc.layers.active.value } },
    };
    const result = await again(() => s.api.command(locks));
    cursor = result.eventSeq;
    metaVersion = result.metaVersion;
  }
  return { records: sent.records, cursor, metaVersion };
}

export async function uploadAsDatabaseProject(s: CloudSession, tenantId: string, name: string, progress: Progress, catalog: UploadCatalog, stage?: (s: FileStage) => void): Promise<boolean> {
  const { ctx, api } = s;
  const doc = ctx.doc;
  // The drawing becomes the new project: the previous cloud project is left first (its changes sent).
  await s.sync.value?.flush().catch(() => false);
  s.detach();
  stage?.('creating');
  // Each step keeps its idempotency key through its tries: a lost answer is answered from the server's log.
  const key = crypto.randomUUID();
  const info = await again(() => api.createProject(tenantId, createInput(doc, name, catalog), key));
  const target = { tenantId, projectId: info.id };
  let landed: Landed;
  let how: 'import' | 'batches' = 'import';
  try {
    const sent = await sendDrawing(api, doc, () => ctx.files.kcad(), target, { stage, progress, warn: (t) => ctx.log.warn(t), part: s.uploadPart });
    stage?.('importing');
    const envelope = importEnvelope(target, sent.upload.id);
    const imported = await again(() => api.lifecycle<ProjectImported>(envelope));
    // The cursor after the import, and the counts to check it by.
    const now = await again(() => api.project(tenantId, info.id));
    const uids = ([...doc.all()] as DrawingEntity[]).map((e) => e.uid);
    if (doc.revision !== sent.encoded.revision || Number(now.featureCount) !== uids.length || Number(imported.objects) !== uids.length) {
      // The drawing changed while it went up (or the server holds another count): the project has the drawing as it
      // was, and the drawing on screen stays local with its changes, rather than being tracked against the wrong copy.
      ctx.log.warn(
        `“${name}” bulut projesi oluşturuldu ve çizim içe aktarıldı (${imported.objects} nesne), ama çizim yükleme sürerken değişti; ekrandaki çizim projeye bağlanmadı ve değişiklikleri yerinde duruyor. Projeyi Bulut projesi aç ile açın.`,
      );
      return false;
    }
    // Every object at version 1 (docs/adr/0036); this import's event is this window's own.
    landed = { records: uids.map((id) => ({ id, version: '1' })), cursor: now.eventCursor, metaVersion: now.metaVersion };
  } catch (e) {
    if (!importUnavailable(e)) throw new UploadFailed(info, e);
    how = 'batches';
    ctx.log.info('Sunucu çizimi tek işlemde içe aktaramıyor (eski sürüm); nesneler parça parça gönderiliyor.');
    stage?.('uploading');
    try {
      landed = await uploadInBatches(s, info, progress);
    } catch (again) {
      throw new UploadFailed(info, again);
    }
  }
  doc.applyExternal({ meta: { name } });
  s.attach({ ...info, name, metaVersion: landed.metaVersion }, landed.records, landed.cursor);
  doc.markSaved(doc.revision);
  ctx.log.success(
    `“${name}” buluta yüklendi: ${landed.records.length} nesne${how === 'import' ? ', tek işlemde içe aktarıldı' : ''}. Bundan sonra değişiklikler kendiliğinden kaydedilir.`,
  );
  return true;
}
