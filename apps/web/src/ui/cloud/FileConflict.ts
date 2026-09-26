import type { AppContext } from '../../app/context';
import { confirmDialog } from '../widgets/confirm';
import { openUploadDialog } from './UploadDialog';

/**
 * A file project's Kaydet that met a newer revision, and the newer
 * revision someone else saved while the project is open (docs/adr/0038;
 * TODOS.md SYNC-06). Two files are never merged byte by byte: the user
 * chooses where the drawing goes — a separate copy (a new file project, or
 * a local file) — or opens the newest revision and drops the drawing's
 * changes. Nothing is reloaded by itself.
 */

type Answer = 'copy' | 'local' | 'latest' | 'stay';

/** The question after a refused Kaydet (or before opening a newer revision over unsaved work). True when the drawing ended up saved. */
export async function resolveFileConflict(ctx: AppContext): Promise<boolean> {
  const cloud = ctx.cloud;
  const file = cloud.file.value;
  const p = cloud.project.value;
  if (!file || !p) return false;
  const newer = file.newer.value;
  const c = file.conflict.value ?? (newer ? { expected: file.base.value, actual: newer.revision } : null);
  if (!c) return false;
  const who = newer?.by ? ` (${newer.by})` : '';
  const answer = await confirmDialog<Answer>({
    title: 'Dosya başka biri tarafından kaydedildi',
    message: `“${p.name}” siz çalışırken başka biri tarafından kaydedildi: sunucuda revizyon ${c.actual}${who} var, sizin çiziminiz revizyon ${c.expected}'e dayanıyor. Hiçbir şey yazılmadı; iki dosya birleştirilmez.`,
    details: [
      `Ayrı kopya olarak kaydet: çiziminiz yeni bir bulut dosya projesi olur ve açık proje o olur; “${p.name}” olduğu gibi kalır.`,
      'Yerel dosyaya kaydet: çiziminiz bu bilgisayara .kcad olarak kaydedilir ve çizim buluttaki projeden ayrılır.',
      `Son revizyonu aç: revizyon ${c.actual} açılır; bu çizimdeki kaydedilmemiş değişiklikler atılır.`,
    ],
    answers: [
      { value: 'latest', label: 'Son revizyonu aç', kind: 'danger', aside: true },
      { value: 'stay', label: 'Vazgeç' },
      { value: 'local', label: 'Yerel dosyaya kaydet' },
      { value: 'copy', label: 'Ayrı kopya olarak kaydet', kind: 'primary' },
    ],
    cancel: 'stay',
  });
  switch (answer) {
    case 'copy':
      // The upload window, on file storage, named as a copy: it says how far it is and what went wrong.
      openUploadDialog(ctx, { storage: 'file', name: `${p.name} (kopya)`, tenantId: p.tenantId });
      return false;
    case 'local': {
      const saved = await ctx.files.saveAs();
      if (saved && cloud.file.value === file) {
        cloud.detach();
        ctx.log.info(`Çizim yerel dosyaya kaydedildi ve “${p.name}” bulut projesinden ayrıldı; proje olduğu gibi duruyor.`);
      }
      return saved;
    }
    case 'latest':
      return openLatest(ctx);
    default:
      return false;
  }
}

/** Opens the newest revision of the open file project; the drawing's unsaved changes are dropped on purpose. */
async function openLatest(ctx: AppContext): Promise<boolean> {
  const p = ctx.cloud.project.value;
  if (!p) return false;
  // Dropped on purpose: their recovery copy goes too (app/recovery.ts).
  if (ctx.doc.dirty.value) ctx.recovery.discard();
  try {
    return await ctx.cloud.open(p.tenantId, p.projectId);
  } catch (e) {
    ctx.log.error(`“${p.name}” son revizyonu açılamadı: ${e instanceof Error ? e.message : String(e)}`);
    return false;
  }
}

/**
 * Someone else saved a newer revision while the project is open: offered.
 * Over unsaved work it is the conflict's question (the work needs a place
 * first); over a clean drawing, a plain question.
 */
export async function offerNewest(ctx: AppContext): Promise<boolean> {
  const file = ctx.cloud.file.value;
  const p = ctx.cloud.project.value;
  if (!file || !p) return false;
  if (ctx.doc.dirty.value || file.conflict.value) return resolveFileConflict(ctx);
  const newer = file.newer.value;
  const answer = await confirmDialog<'open' | 'later'>({
    title: 'Son revizyonu aç',
    message: newer
      ? `“${p.name}” başka bir yerde kaydedildi: revizyon ${newer.revision}${newer.by ? ` (${newer.by})` : ''}. Açık çizim revizyon ${file.base.value}; kaydedilmemiş değişikliği yok.`
      : `“${p.name}” projesinin sunucudaki en yeni revizyonu açılsın mı? Açık çizim revizyon ${file.base.value}; kaydedilmemiş değişikliği yok.`,
    answers: [
      { value: 'later', label: 'Sonra' },
      { value: 'open', label: 'Son revizyonu aç', kind: 'primary' },
    ],
    cancel: 'later',
  });
  return answer === 'open' ? openLatest(ctx) : false;
}
