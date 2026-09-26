import type { FileCommitted } from '../../contracts/generated/FileCommitted';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import { replaceDrawing } from '../fileIO';
import type { NewerRevision } from './fileProject';
import { readProject } from './incoming';
import type { CloudSession, Progress, UploadCatalog } from './session';
import { commitEnvelope, sizeText, verifyDownload } from './transfer';
import { again } from './upload';
import { UploadFailed, createInput, sendDrawing, type FileStage } from './uploading';

export type { FileStage } from './uploading';

/**
 * A file project in the session (docs/adr/0031, 0038): opening one from its
 * newest revision, making the drawing on screen a new one (“Buluta dosya
 * olarak kaydet”), and Kaydet with what it says. The revision's bytes are
 * downloaded inside the open's window (with its progress and Vazgeç),
 * checked against the server's SHA-256, and read and checked in stages as a
 * local file is (app/fileIO.ts); the drawing on screen is replaced only by a
 * revision read whole, and the project is attached at that revision.
 */

/** Someone else saved a newer revision of the open file project: said, never loaded by itself. */
export function newerText(name: string, newer: NewerRevision, base: string): string {
  return `“${name}” başka bir yerde kaydedildi: revizyon ${newer.revision}${newer.by ? ` (${newer.by})` : ''}. Açık çizim revizyon ${base}'e dayanıyor; yeni revizyonu açmak için durum çubuğundaki kayıt durumuna tıklayın. Kendiliğinden yeniden yüklenmez.`;
}

/** The notice of a newer revision for the project `info` (the log; the status bar offers to open it). */
function noticeNewer(s: CloudSession, info: ProjectInfo) {
  return (newer: NewerRevision) => s.ctx.log.warn(newerText(info.name, newer, s.file.value?.base.value ?? '?'));
}

/**
 * Opens the file project `info`: its newest revision, downloaded and
 * checked, then read in stages; a project without a revision yet opens
 * with its own metadata and no objects (the first Kaydet writes revision 1).
 */
export async function openFileProject(s: CloudSession, info: ProjectInfo, progress: Progress, signal: AbortSignal | undefined, stale: () => boolean): Promise<boolean> {
  const { ctx, api } = s;
  const revs = await api.fileRevisions(info.tenantId, info.id, signal);
  if (stale()) return false;
  const current = revs.current;
  const archived = info.state === 'archived';
  const opened = (revision: string) => {
    ctx.log.success(`“${info.name}” bulut projesi açıldı: ${revision === '0' ? 'henüz revizyonu yok' : `revizyon ${revision}`}, ${ctx.doc.size} nesne. Kaydet (Ctrl+S) yeni bir revizyon yazar.`);
    if (archived)
      ctx.log.info(`“${info.name}” arşivlenmiş bir proje: salt okunur açıldı; kaydedilemez. Düzenlemek için arşivden çıkarılmalı ya da kopyası oluşturulmalı.`);
  };
  if (!current) {
    const read = readProject(info, []);
    if (!read.ok) throw new Error(`“${info.name}” okunamadı: ${read.error}`);
    progress(0, 0);
    // What waits of the project open now goes first; it stays in its device draft if the server does not answer.
    await s.sync.value?.flush().catch(() => false);
    if (stale()) return false;
    // A drawing replaced without the question keeps its unsaved work in its recovery copy (app/recovery.ts).
    await ctx.recovery?.flush();
    s.detach(false);
    replaceDrawing(ctx, read.content);
    s.attachFile(info, '0', info.eventCursor, noticeNewer(s, info));
    opened('0');
    return true;
  }
  const listed = revs.revisions.find((r) => r.revision === current);
  await ctx.recovery?.flush();
  return ctx.files.openCloud(
    `“${info.name}” (revizyon ${current})`,
    async (step, stop) => {
      const d = await api.fileRevision(info.tenantId, info.id, current, step, stop);
      // Bytes that changed on the way never become the drawing.
      await verifyDownload(d, listed?.sha256);
      return d.bytes;
    },
    (read) => {
      // The project's name is the catalog's; the revision holds the one it was saved with.
      replaceDrawing(ctx, { ...read.content, name: info.name });
      s.attachFile(info, current, info.eventCursor, noticeNewer(s, info));
      opened(current);
    },
  );
}

/**
 * Makes the drawing on screen a new file project in `tenantId` named
 * `name`: the project is created (`storage: file`), the drawing's `.kcad`
 * uploaded and committed as revision 1 on "0", and the project attached.
 * A failure after the project was created is `UploadFailed` (it stays
 * empty; the drawing stays local, under its own name).
 */
export async function uploadAsFileProject(s: CloudSession, tenantId: string, name: string, progress: Progress, catalog: UploadCatalog, stage?: (s: FileStage) => void): Promise<boolean> {
  const { ctx, api } = s;
  const doc = ctx.doc;
  // The drawing becomes the new project: the previous cloud project is left first (its changes sent).
  await s.sync.value?.flush().catch(() => false);
  s.detach();
  stage?.('creating');
  const key = crypto.randomUUID();
  const info = await again(() => api.createProject(tenantId, createInput(doc, name, catalog, 'file'), key));
  const target = { tenantId, projectId: info.id };
  // The revision holds the project's name; the drawing takes it back if the upload fails.
  const before = doc.name.value;
  if (before !== name) doc.applyExternal({ meta: { name } });
  try {
    const sent = await sendDrawing(api, doc, () => ctx.files.kcad(), target, { stage, progress, warn: (t) => ctx.log.warn(t) });
    const envelope = commitEnvelope(target, sent.upload.id, '0');
    const done = await again(() => api.lifecycle<FileCommitted>(envelope));
    const file = s.attachFile({ ...info, name }, done.revision, info.eventCursor, noticeNewer(s, { ...info, name }));
    file.expect(envelope.requestId);
    file.lastSaved.set({ revision: done.revision, at: Date.now(), sha256: done.sha256, size: done.size });
    doc.markSaved(sent.encoded.revision);
    ctx.log.success(`“${name}” buluta dosya olarak kaydedildi: revizyon ${done.revision}, ${sizeText(done.size)}. Bundan sonra Kaydet (Ctrl+S) yeni bir revizyon yazar; kendiliğinden kaydedilmez.`);
    return true;
  } catch (e) {
    if (before !== name && doc.name.value === name && !doc.busy) doc.applyExternal({ meta: { name: before } });
    throw new UploadFailed(info, e);
  }
}

/** Why nothing is saved to the open file project any more, and what to do. */
function endedText(name: string, why: 'deleted' | 'revoked' | 'archived' | null): string {
  switch (why) {
    case 'deleted':
      return `“${name}” bulut projesi silindi; çizim buluta kaydedilemez. Yerel bir dosyaya kaydedin.`;
    case 'archived':
      return `“${name}” bulut projesi arşivlenmiş; çizim buluta kaydedilemez. Yerel bir dosyaya kaydedin.`;
    default:
      return `“${name}” projesine erişiminiz kaldırıldı; çizim buluta kaydedilemez. Yerel bir dosyaya kaydedin.`;
  }
}

/**
 * Kaydet on the open file project: a new revision on the one the drawing
 * came from, and what happened said in the log. A save that met a newer
 * revision goes to the app's question (`CloudSession.fileConflict`). True
 * when the drawing on screen is saved as a revision.
 */
export async function saveFileProject(s: CloudSession): Promise<boolean> {
  const file = s.file.value;
  const p = s.project.value;
  if (!file || !p) return false;
  const log = s.ctx.log;
  const outcome = await file.save();
  switch (outcome) {
    case 'saved': {
      const last = file.lastSaved.value;
      const later = s.ctx.doc.dirty.value ? ' Kayıt sürerken yapılan değişiklikler henüz kaydedilmedi.' : '';
      log.success(`“${p.name}” buluta kaydedildi: revizyon ${file.base.value}${last ? `, ${sizeText(last.size)}` : ''}.${later}`);
      return true;
    }
    case 'unchanged':
      log.info(`“${p.name}” zaten kaydedilmiş (revizyon ${file.base.value}); kaydedilecek değişiklik yok.`);
      return true;
    case 'conflict':
      return (await s.fileConflict?.()) ?? false;
    case 'readonly':
      log.warn(`“${p.name}” projesinde dosyayı kaydetme yetkiniz yok (feature.write). Değişiklikleri saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.`);
      return false;
    case 'ended':
      log.warn(endedText(p.name, file.endedBy));
      return false;
    default:
      return false;
  }
}
